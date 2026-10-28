//! Geostationary satellite view.
//!
//! Port of PROJ 9.9 `src/projections/geos.cpp`. The kernel works in units of
//! the semi-major axis: `h` is divided by `a` here.

#![allow(clippy::pedantic, reason = "formulae follow PROJ geos.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};

struct Geos {
    radius_p: f64,
    radius_p2: f64,
    radius_p_inv2: f64,
    radius_g: f64,
    radius_g_1: f64,
    c: f64,
    flip_axis: bool,
    ellips: bool,
}

impl Kernel for Geos {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let (vx, vy, vz) = if self.ellips {
            phi = (self.radius_p2 * phi.tan()).atan();
            let r = self.radius_p / (self.radius_p * phi.cos()).hypot(phi.sin());
            let vx = r * lam.cos() * phi.cos();
            let vy = r * lam.sin() * phi.cos();
            let vz = r * phi.sin();
            if (self.radius_g - vx) * vx - vy * vy - vz * vz * self.radius_p_inv2 < 0.0 {
                return Err(TransformError::OutsideDomain);
            }
            (vx, vy, vz)
        } else {
            let tmp = phi.cos();
            (lam.cos() * tmp, lam.sin() * tmp, phi.sin())
        };
        let tmp = self.radius_g - vx;
        let (x, y) = if self.flip_axis {
            (
                self.radius_g_1 * (vy / vz.hypot(tmp)).atan(),
                self.radius_g_1 * (vz / tmp).atan(),
            )
        } else {
            (
                self.radius_g_1 * (vy / tmp).atan(),
                self.radius_g_1 * (vz / vy.hypot(tmp)).atan(),
            )
        };
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let vx0 = -1.0;
        let (vy, vz) = if self.flip_axis {
            let vz = (y / self.radius_g_1).tan();
            ((x / self.radius_g_1).tan() * (1.0 + vz * vz).sqrt(), vz)
        } else {
            let vy = (x / self.radius_g_1).tan();
            (vy, (y / self.radius_g_1).tan() * (1.0 + vy * vy).sqrt())
        };
        let a = if self.ellips {
            let scaled = vz / self.radius_p;
            vy * vy + scaled * scaled + vx0 * vx0
        } else {
            vy * vy + vz * vz + vx0 * vx0
        };
        let b = 2.0 * self.radius_g * vx0;
        let det = b * b - 4.0 * a * self.c;
        if det < 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let k = (-b - det.sqrt()) / (2.0 * a);
        let vx = self.radius_g + k * vx0;
        let vy = vy * k;
        let vz = vz * k;
        let lam = vy.atan2(vx);
        let phi = if self.ellips {
            (self.radius_p_inv2 * vz / vx.hypot(vy)).atan()
        } else {
            (vz / vx.hypot(vy)).atan()
        };
        Ok((lam, phi))
    }
}

const KEYS: &[&str] = &["h", "sweep"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "geos" {
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
    if name != "geos" {
        return Ok(None);
    }
    let flip_axis = match params.text("sweep") {
        None => false,
        Some("x") => true,
        Some("y") => false,
        Some(_) => return Err(CreateError::IllegalValue("sweep")),
    };
    let radius_g_1 = params.number("h")?.unwrap_or(0.0) / frame.earth.a();
    if radius_g_1 <= 0.0 || radius_g_1 > 1e10 {
        return Err(CreateError::IllegalValue("h"));
    }
    let ellips = frame.earth.es() != 0.0;
    let one_es = frame.earth.one_es();
    Ok(Some(classic_step(
        frame,
        Geos {
            radius_p: if ellips { one_es.sqrt() } else { 1.0 },
            radius_p2: if ellips { one_es } else { 1.0 },
            radius_p_inv2: if ellips { 1.0 / one_es } else { 1.0 },
            radius_g: 1.0 + radius_g_1,
            radius_g_1,
            c: (1.0 + radius_g_1) * (1.0 + radius_g_1) - 1.0,
            flip_axis,
            ellips,
        },
    )))
}
