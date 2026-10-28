//! Meta interrupted projection.
//!
//! Port of PROJ 9.9 `src/projections/interrupted.cpp`. The base is a fresh
//! `sinu`, `tmerc`, `cass`, or `poly`: the caller's ellipsoid and `k0` are
//! kept, and `lat_0`, `lon_0`, false offsets, and the prime meridian are not.
//! Those bases already return metres, so this kernel divides by `a` and lets
//! the classic wrapper multiply once.

#![allow(
    clippy::pedantic,
    reason = "gore edges follow the loops in interrupted.cpp"
)]

use crate::engine::{
    classic_step, Coord, CreateError, Frame, IoKind, Kernel, Operation, Params, Step,
    TransformError,
};
use crate::{Projected, Tmerc};
use std::f64::consts::PI;

struct Interrupted {
    base: Box<dyn Step>,
    a: f64,
    ra: f64,
    lam_borders: Vec<f64>,
    lam_centers: Vec<f64>,
    x_borders: Vec<f64>,
    x_centers: Vec<f64>,
}

impl Kernel for Interrupted {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut center_lam = 0.0;
        let mut center_x = 0.0;
        if let Some(index) = gore_index(&self.lam_borders, lam) {
            center_lam = self.lam_centers[index];
            center_x = self.x_centers[index];
        }
        let projected = self.base.forward(Coord::xy(lam - center_lam, phi))?;
        Ok((projected.x * self.ra + center_x, projected.y * self.ra))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut center_lam = 0.0;
        let mut center_x = 0.0;
        let mut start_lam = 0.0;
        let mut end_lam = 0.0;
        if let Some(index) = gore_index(&self.x_borders, x) {
            center_lam = self.lam_centers[index];
            center_x = self.x_centers[index];
            let last = index + 1 == self.x_borders.len();
            end_lam = if last { PI } else { self.lam_borders[index] };
            start_lam = if index == 0 {
                -PI
            } else {
                self.lam_borders[index - 1]
            };
        }
        let geographic = self
            .base
            .inverse(Coord::xy((x - center_x) * self.a, y * self.a))?;
        let lam = geographic.x + center_lam;
        if lam > end_lam || lam < start_lam {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam, geographic.y))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "interrupted" {
        Some(&["base", "gores"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "interrupted" {
        return Ok(None);
    }
    let base_name = params.text("base").unwrap_or("");
    if !matches!(base_name, "cass" | "poly" | "sinu" | "tmerc") {
        return Err(CreateError::IllegalValue("base"));
    }
    let sizes = gore_sizes(params)?;
    let child = child_frame(frame);
    let base = base_step(base_name, &child)?;
    let layout = layout(&*base, &sizes, frame.earth.ra())?;
    Ok(Some(classic_step(
        frame,
        Interrupted {
            base,
            a: frame.earth.a(),
            ra: frame.earth.ra(),
            lam_borders: layout.lam_borders,
            lam_centers: layout.lam_centers,
            x_borders: layout.x_borders,
            x_centers: layout.x_centers,
        },
    )))
}

/// PROJ allocates a new `PJ` and copies only `a`, `es`, and `k0`.
fn child_frame(frame: &Frame) -> Frame {
    let mut child = frame.clone();
    child.lam0 = 0.0;
    child.phi0 = 0.0;
    child.x0 = 0.0;
    child.y0 = 0.0;
    child.z0 = 0.0;
    child.from_greenwich = 0.0;
    child.geoc = false;
    child.over = true;
    child.to_meter = 1.0;
    child.fr_meter = 1.0;
    child.vto_meter = 1.0;
    child.vfr_meter = 1.0;
    child
}

fn base_step(name: &str, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let bare = Params::parse("")?;
    let built = match name {
        "sinu" => crate::projections::sphere2::build(name, &bare, frame)?,
        "cass" => crate::projections::cass::build(name, &bare, frame)?,
        "poly" => crate::projections::more::build(name, &bare, frame)?,
        "tmerc" => return tmerc_step(frame),
        _ => return Err(CreateError::IllegalValue("base")),
    };
    built.ok_or(CreateError::IllegalValue("base"))
}

fn tmerc_step(frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    if frame.earth.es() == 0.0 {
        let definition = format!(
            "+proj=tmerc +R={} +k_0={} +over",
            format_f64(frame.earth.a()),
            format_f64(frame.k0)
        );
        let operation = Operation::create(&definition)?;
        return Ok(Box::new(SphereTmerc {
            operation,
            a: frame.earth.a(),
            es: 0.0,
        }));
    }
    let tmerc = Tmerc::try_new(frame.earth, 0.0, 0.0, frame.k0, 0.0, 0.0)
        .map_err(|_| CreateError::IllegalValue("base"))?;
    Ok(Box::new(ExactTmerc {
        tmerc,
        a: frame.earth.a(),
        es: frame.earth.es(),
    }))
}

fn format_f64(value: f64) -> String {
    format!("{value:?}")
}

/// Ellipsoidal transverse Mercator, Poder/Engsager, equator-centered.
///
/// `forward_rad` reduces longitude with `adjlon`. Gore offsets used here stay
/// inside `(-π, π]`, where that reduction is the identity.
struct ExactTmerc {
    tmerc: Tmerc,
    a: f64,
    es: f64,
}

impl Step for ExactTmerc {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let projected = self.tmerc.forward_rad(coord.x, coord.y)?;
        Ok(Coord {
            x: projected.east,
            y: projected.north,
            z: coord.z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let geographic = self.tmerc.inverse(Projected {
            east: coord.x,
            north: coord.y,
        })?;
        Ok(Coord {
            x: geographic.lon_deg.to_radians(),
            y: geographic.lat_deg.to_radians(),
            z: coord.z,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

struct SphereTmerc {
    operation: Operation,
    a: f64,
    es: f64,
}

impl Step for SphereTmerc {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.operation.forward(coord)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        self.operation.inverse(coord)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

struct Layout {
    lam_borders: Vec<f64>,
    lam_centers: Vec<f64>,
    x_borders: Vec<f64>,
    x_centers: Vec<f64>,
}

fn layout(base: &dyn Step, sizes: &[f64], ra: f64) -> Result<Layout, CreateError> {
    let total: f64 = sizes.iter().copied().sum();
    if total == 0.0 || !total.is_finite() {
        return Err(CreateError::IllegalValue("gores"));
    }
    let mut lam_start = -PI;
    let mut x_semis = Vec::with_capacity(sizes.len());
    let mut lam_centers = Vec::with_capacity(sizes.len());
    let mut lam_borders = Vec::with_capacity(sizes.len());
    for size in sizes {
        let lam_semi = size * PI / total;
        let width = base
            .forward(Coord::xy(lam_semi, 0.0))
            .map_err(|_| CreateError::IllegalValue("gores"))?;
        if !width.x.is_finite() {
            return Err(CreateError::IllegalValue("gores"));
        }
        x_semis.push(width.x * ra);
        lam_centers.push(lam_start + lam_semi);
        let lam_end = lam_start + lam_semi * 2.0;
        lam_borders.push(lam_end);
        lam_start = lam_end;
    }
    let mut x_start = -x_semis.iter().copied().sum::<f64>();
    let mut x_centers = Vec::with_capacity(x_semis.len());
    let mut x_borders = Vec::with_capacity(x_semis.len());
    for width in x_semis {
        x_centers.push(x_start + width);
        let x_end = x_start + width * 2.0;
        x_borders.push(x_end);
        x_start = x_end;
    }
    // Selection only. The inverse still closes the last gore at `+π`.
    if let Some(border) = lam_borders.last_mut() {
        *border += 1.0;
    }
    if let Some(border) = x_borders.last_mut() {
        *border += 100.0;
    }
    Ok(Layout {
        lam_borders,
        lam_centers,
        x_borders,
        x_centers,
    })
}

fn gore_index(borders: &[f64], value: f64) -> Option<usize> {
    borders.iter().position(|&border| value <= border)
}

fn gore_sizes(params: &Params) -> Result<Vec<f64>, CreateError> {
    let mut sizes = if params.flag("gores") {
        parse_gore_list(params.text("gores").unwrap_or(""))?
    } else {
        vec![3.0]
    };
    if sizes.len() == 1 {
        let count = equal_gore_count(sizes[0])?;
        sizes = vec![1.0; count];
    }
    if sizes.is_empty() {
        return Err(CreateError::IllegalValue("gores"));
    }
    Ok(sizes)
}

fn parse_gore_list(text: &str) -> Result<Vec<f64>, CreateError> {
    let mut sizes = Vec::new();
    for piece in text.split(',') {
        if piece.is_empty() {
            continue;
        }
        let token = piece.trim();
        if token.is_empty() {
            return Err(CreateError::IllegalValue("gores"));
        }
        let value: f64 = token
            .parse()
            .map_err(|_| CreateError::IllegalValue("gores"))?;
        if !value.is_finite() {
            return Err(CreateError::IllegalValue("gores"));
        }
        sizes.push(value);
    }
    Ok(sizes)
}

/// A single `+gores=` entry is a count, truncated toward zero the way
/// `static_cast<size_t>` does. Counts above one million are rejected instead
/// of allocating.
fn equal_gore_count(value: f64) -> Result<usize, CreateError> {
    if !value.is_finite() || value < 0.0 {
        return Err(CreateError::IllegalValue("gores"));
    }
    let truncated = value.trunc();
    if truncated > 1_000_000.0 {
        return Err(CreateError::IllegalValue("gores"));
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "truncated count was checked to lie in 0..=1_000_000 before the size_t-style cast"
    )]
    Ok(truncated as usize)
}

#[cfg(test)]
mod tests {
    use super::CreateError;
    use crate::engine::{lpz_dist, Coord, Operation};

    fn expect_forward(definition: &str, lon: f64, lat: f64, east: f64, north: f64) {
        let op =
            Operation::create(definition).unwrap_or_else(|error| panic!("{definition}: {error}"));
        let fwd = op
            .forward(Coord::xy(lon.to_radians(), lat.to_radians()))
            .unwrap_or_else(|error| panic!("{definition} forward: {error}"));
        assert!(
            (fwd.x - east).abs() <= 1e-3,
            "{definition} east {fwd_x} != {east}",
            fwd_x = fwd.x
        );
        assert!(
            (fwd.y - north).abs() <= 1e-3,
            "{definition} north {fwd_y} != {north}",
            fwd_y = fwd.y
        );
        let back = op
            .inverse(fwd)
            .unwrap_or_else(|error| panic!("{definition} inverse: {error}"));
        let (a, es) = op.distance_ellipsoid();
        let miss = lpz_dist(a, es, Coord::xy(lon.to_radians(), lat.to_radians()), back);
        assert!(miss <= 1e-3, "{definition} roundtrip {miss} m");
    }

    #[test]
    fn gie_gores_over_the_four_bases() {
        expect_forward("+proj=interrupted +base=sinu", 0.0, 0.0, 0.0, 0.0);
        expect_forward(
            "+proj=interrupted +base=sinu",
            120.0,
            30.0,
            13_358_338.895_2,
            3_320_113.397_8,
        );
        expect_forward(
            "+proj=interrupted +base=sinu +gores=1,2,3",
            120.0,
            30.0,
            12_913_342.578_9,
            3_320_113.397_8,
        );
        expect_forward(
            "+proj=interrupted +base=tmerc +gores=5",
            -120.0,
            30.0,
            -14_862_738.269_9,
            3_575_648.808_3,
        );
        expect_forward(
            "+proj=interrupted +base=cass +gores=5",
            -120.0,
            30.0,
            -13_732_123.899_9,
            3_575_100.452_0,
        );
        expect_forward(
            "+proj=interrupted +base=poly +gores=5",
            -120.0,
            30.0,
            -13_731_228.303_9,
            3_561_724.740_5,
        );
    }

    #[test]
    fn sinu_ignores_k0_and_a_missing_base_is_rejected() {
        let plain = Operation::create("+proj=interrupted +base=sinu").expect("plain");
        let scaled = Operation::create("+proj=interrupted +base=sinu +k_0=0.5").expect("scaled");
        let point = Coord::xy(120.0_f64.to_radians(), 30.0_f64.to_radians());
        let left = plain.forward(point).expect("plain forward");
        let right = scaled.forward(point).expect("scaled forward");
        assert!((left.x - right.x).abs() <= 1e-6);
        assert!((left.y - right.y).abs() <= 1e-6);
        let missing = Operation::create("+proj=interrupted").expect_err("base");
        assert!(matches!(missing, CreateError::IllegalValue("base")));
        let bad = Operation::create("+proj=interrupted +base=moll").expect_err("moll");
        assert!(matches!(bad, CreateError::IllegalValue("base")));
        let empty = Operation::create("+proj=interrupted +base=sinu +gores=").expect_err("gores");
        assert!(matches!(empty, CreateError::IllegalValue("gores")));
    }
}
