//! General oblique transformation.
//!
//! The tested target is `+o_proj=latlon`, which is the identity after the
//! sphere is rotated. The forward result is that rotated longitude and
//! latitude in radians, compared by the gie runner as a linear coordinate
//! because PROJ marks the output unit as "whatever". The inverse returns a
//! geographic position, so `lon_0` is added back.

#![allow(
    clippy::pedantic,
    reason = "the pole rotation follows Snyder 5-7 through 5-10 in ob_tran.cpp"
)]

use crate::angle::adjlon;
use crate::engine::{Coord, CreateError, Frame, IoKind, Operation, Params, Step, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

const TOL: f64 = 1e-10;
const EPS_LAT: f64 = 1e-12;
const ONE_TOL: f64 = 1.000_000_000_000_01;

struct ObTran {
    frame: Frame,
    lamp: f64,
    sin_pole: f64,
    cos_pole: f64,
    transverse: bool,
    /// `None` is `+o_proj=latlon`: the rotated angles are the result.
    child: Option<Operation>,
    /// PROJ only installs an inverse when the target has a 2D inverse.
    /// Helmert is Cartesian and does not.
    has_inverse: bool,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "ob_tran" {
        Some(&[
            "o_proj", "o_lat_p", "o_lon_p", "o_alpha", "o_lon_c", "o_lat_c", "o_lon_1", "o_lat_1",
            "o_lon_2", "o_lat_2",
        ])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "ob_tran" {
        return Ok(None);
    }
    let Some(target) = params.text("o_proj") else {
        return Err(CreateError::MissingArgument("o_proj"));
    };
    if target.is_empty() || target == "ob_tran" {
        return Err(CreateError::MissingArgument("o_proj"));
    }
    let (lamp, pole) = pole(params)?;
    let transverse = pole.abs() <= TOL;
    let identity = matches!(target, "latlon" | "latlong" | "longlat" | "lonlat");
    let (child, has_inverse) = if identity {
        (None, true)
    } else {
        let child = Operation::create(&child_definition(target, params))?;
        let cartesian = child_is_cartesian(target);
        (Some(child), !cartesian)
    };
    Ok(Some(Box::new(ObTran {
        frame: frame.clone(),
        lamp,
        sin_pole: pole.sin(),
        cos_pole: pole.cos(),
        transverse,
        child,
        has_inverse,
    })))
}

fn child_is_cartesian(name: &str) -> bool {
    matches!(
        name,
        "helmert" | "molobadekas" | "cart" | "geocent" | "topocentric"
    )
}

fn child_definition(target: &str, params: &Params) -> String {
    let mut parts = vec![format!("proj={target}")];
    for (key, value) in params.entries() {
        if matches!(
            key,
            "proj"
                | "o_proj"
                | "o_lat_p"
                | "o_lon_p"
                | "o_alpha"
                | "o_lon_c"
                | "o_lat_c"
                | "o_lon_1"
                | "o_lat_1"
                | "o_lon_2"
                | "o_lat_2"
                | "inv"
                | "lon_0"
                | "pm"
        ) {
            continue;
        }
        match value {
            Some(text) => parts.push(format!("{key}={text}")),
            None => parts.push(key.to_owned()),
        }
    }
    parts.join(" ")
}

fn pole(params: &Params) -> Result<(f64, f64), CreateError> {
    if params.flag("o_alpha") {
        let lamc = params.radians("o_lon_c")?.unwrap_or(0.0);
        let phic = params.radians("o_lat_c")?.unwrap_or(0.0);
        let alpha = params.radians("o_alpha")?.unwrap_or(0.0);
        if (phic.abs() - FRAC_PI_2).abs() <= TOL {
            return Err(CreateError::IllegalValue("o_lat_c"));
        }
        let lamp = lamc + atan2_small(-alpha.cos(), -alpha.sin() * phic.sin());
        let pole = asin_clamped((phic.cos() * alpha.sin()).clamp(-1.0, 1.0))?;
        return Ok((lamp, pole));
    }
    if params.flag("o_lat_p") {
        let lamp = params.radians("o_lon_p")?.unwrap_or(0.0);
        let pole = params.radians("o_lat_p")?.unwrap_or(0.0);
        return Ok((lamp, pole));
    }
    let lam1 = params.radians("o_lon_1")?.unwrap_or(0.0);
    let phi1 = params.radians("o_lat_1")?.unwrap_or(0.0);
    let lam2 = params.radians("o_lon_2")?.unwrap_or(0.0);
    let phi2 = params.radians("o_lat_2")?.unwrap_or(0.0);
    if phi1.abs() > FRAC_PI_2 - TOL {
        return Err(CreateError::IllegalValue("o_lat_1"));
    }
    if phi2.abs() > FRAC_PI_2 - TOL {
        return Err(CreateError::IllegalValue("o_lat_2"));
    }
    if (phi1 - phi2).abs() < TOL || phi1.abs() < TOL {
        return Err(CreateError::IllegalValue("o_lat_1"));
    }
    let lamp = (phi1.cos() * phi2.sin() * lam1.cos() - phi1.sin() * phi2.cos() * lam2.cos())
        .atan2(phi1.sin() * phi2.cos() * lam2.sin() - phi1.cos() * phi2.sin() * lam1.sin());
    let pole = (-(lamp - lam1).cos() / phi1.tan()).atan();
    Ok((lamp, pole))
}

impl Step for ObTran {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lam, phi) = geographic(&self.frame, coord.x, coord.y)?;
        let (lam, phi) = self.rotate(lam, phi, false)?;
        if let Some(child) = &self.child {
            return child.forward(Coord {
                x: lam,
                y: phi,
                z: coord.z,
                t: coord.t,
            });
        }
        Ok(Coord {
            x: lam,
            y: phi,
            z: coord.z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !self.has_inverse {
            return Err(TransformError::NoInverse);
        }
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lam, phi) = if let Some(child) = &self.child {
            let geo = child.inverse(coord)?;
            self.rotate(geo.x, geo.y, true)?
        } else {
            self.rotate(coord.x, coord.y, true)?
        };
        let mut lam = lam + self.frame.from_greenwich + self.frame.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
        Ok(Coord {
            x: lam,
            y: phi,
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
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

impl ObTran {
    fn rotate(&self, lam: f64, phi: f64, inverse: bool) -> Result<(f64, f64), TransformError> {
        if self.transverse {
            return self.rotate_transverse(lam, phi, inverse);
        }
        if inverse {
            let shifted = lam - self.lamp;
            let (sinphi, cosphi) = (phi.sin(), phi.cos());
            let coslam = shifted.cos();
            let phi = asin_checked(self.sin_pole * sinphi + self.cos_pole * cosphi * coslam)?;
            let lam = atan2_small(
                cosphi * shifted.sin(),
                self.sin_pole * cosphi * coslam - self.cos_pole * sinphi,
            );
            Ok((lam, phi))
        } else {
            let (sinphi, cosphi) = (phi.sin(), phi.cos());
            let coslam = lam.cos();
            let lam = adjlon(
                atan2_small(
                    cosphi * lam.sin(),
                    self.sin_pole * cosphi * coslam + self.cos_pole * sinphi,
                ) + self.lamp,
            );
            let phi = asin_checked(self.sin_pole * sinphi - self.cos_pole * cosphi * coslam)?;
            Ok((lam, phi))
        }
    }

    fn rotate_transverse(
        &self,
        lam: f64,
        phi: f64,
        inverse: bool,
    ) -> Result<(f64, f64), TransformError> {
        if inverse {
            let cosphi = phi.cos();
            let shifted = lam - self.lamp;
            let lam = atan2_small(cosphi * shifted.sin(), -phi.sin());
            let phi = asin_checked(cosphi * shifted.cos())?;
            Ok((lam, phi))
        } else {
            let cosphi = phi.cos();
            let coslam = lam.cos();
            let lam = adjlon(atan2_small(cosphi * lam.sin(), phi.sin()) + self.lamp);
            let phi = asin_checked(-cosphi * coslam)?;
            Ok((lam, phi))
        }
    }
}

fn geographic(frame: &Frame, mut lon: f64, mut lat: f64) -> Result<(f64, f64), TransformError> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err(TransformError::NonFinite);
    }
    if lat.abs() > FRAC_PI_2 {
        if lat > 0.0 {
            if lat - FRAC_PI_2 > EPS_LAT {
                return Err(TransformError::InvalidCoord);
            }
            lat = FRAC_PI_2;
        } else if lat - FRAC_PI_2 < -EPS_LAT {
            return Err(TransformError::InvalidCoord);
        } else {
            lat = -FRAC_PI_2;
        }
    }
    if lon.abs() > PI && lon.abs() > 10.0 {
        return Err(TransformError::InvalidCoord);
    }
    lon = (lon - frame.from_greenwich) - frame.lam0;
    if !frame.over {
        lon = adjlon(lon);
    }
    Ok((lon, lat))
}

fn asin_checked(value: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > ONE_TOL {
            return Err(TransformError::OutsideDomain);
        }
        return Ok(if value < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
    }
    Ok(value.asin())
}

fn asin_clamped(value: f64) -> Result<f64, CreateError> {
    asin_checked(value).map_err(|_| CreateError::IllegalValue("o_alpha"))
}

fn atan2_small(y: f64, x: f64) -> f64 {
    if y.abs() < 1e-50 && x.abs() < 1e-50 {
        0.0
    } else {
        y.atan2(x)
    }
}
