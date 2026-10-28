//! Transverse Mercator Zoned Grid System.
//!
//! Port of PROJ 9.9 `src/projections/tmerczoned.cpp`. The inner transverse
//! Mercator is built with this ellipsoid and `k0`, and with a zero origin:
//! `pj_tmerc` on a fresh `PJ` does not copy the parent `phi0`.

use std::f64::consts::TAU;

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::tmerc::Tmerc;
use crate::{Projected, ProjectionError};

pub(crate) const KEYS: &[&str] = &["width"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "tmerczoned" {
        Some(KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "tmerczoned" {
        return Ok(None);
    }
    Ok(Some(build_zoned(params, frame)?))
}

struct Zoned {
    tmerc: Tmerc,
    zone_width: f64,
    /// `1_000_000 / a`, in the kernel's semi-major units.
    offset: f64,
    ra: f64,
}

impl Kernel for Zoned {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut lam = lam;
        if lam < 0.0 {
            lam += TAU;
        }
        // Bias the lower bound inclusive and the upper bound exclusive.
        const EPS: f64 = 1e-12;
        let zone = trunc_i32(lam / self.zone_width + EPS) + 1;
        let center = (f64::from(zone) - 0.5) * self.zone_width;
        let xy = self
            .tmerc
            .forward_rad(lam - center, phi)
            .map_err(step_err)?;
        Ok((
            xy.east * self.ra + f64::from(zone) * self.offset,
            xy.north * self.ra,
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let zone = trunc_i32(x / self.offset + 0.5);
        let local = x - f64::from(zone) * self.offset;
        let geo = self
            .tmerc
            .inverse(Projected {
                east: local / self.ra,
                north: y / self.ra,
            })
            .map_err(step_err)?;
        let center = (f64::from(zone) - 0.5) * self.zone_width;
        Ok((geo.lon_deg.to_radians() + center, geo.lat_deg.to_radians()))
    }
}

fn build_zoned(
    params: &Params,
    frame: &Frame,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let Some(zone_width) = params.radians("width")? else {
        return Err(CreateError::MissingArgument("width"));
    };
    if !(zone_width.is_finite() && zone_width > 0.0) {
        return Err(CreateError::IllegalValue("width"));
    }
    let tmerc =
        Tmerc::try_new(frame.earth, 0.0, 0.0, frame.k0, 0.0, 0.0).map_err(|error| match error {
            ProjectionError::InvalidScale => CreateError::IllegalValue("k_0"),
            ProjectionError::NonFinite => CreateError::IllegalValue("a"),
            _ => CreateError::IllegalValue("parameter"),
        })?;
    Ok(classic_step(
        frame,
        Zoned {
            tmerc,
            zone_width,
            offset: 1_000_000.0 * frame.earth.ra(),
            ra: frame.earth.ra(),
        },
    ))
}

fn step_err(error: ProjectionError) -> TransformError {
    match error {
        ProjectionError::NonFinite => TransformError::NonFinite,
        ProjectionError::InvalidLatitude => TransformError::InvalidCoord,
        _ => TransformError::OutsideDomain,
    }
}

/// C++ `static_cast<int>` truncates toward zero.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "zone index matches PROJ's cast of a small finite quotient to int"
)]
fn trunc_i32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let truncated = value.trunc();
    if truncated < f64::from(i32::MIN) || truncated > f64::from(i32::MAX) {
        return 0;
    }
    truncated as i32
}
