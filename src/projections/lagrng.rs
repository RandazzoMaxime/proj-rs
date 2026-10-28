//! Lagrange.
//!
//! Port of PROJ 9.9 `src/projections/lagrng.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ lagrng.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_2;

const TOL: f64 = 1e-10;

struct Lagrange {
    a1: f64,
    a2: f64,
    hrw: f64,
    hw: f64,
    rw: f64,
    w: f64,
}

impl Kernel for Lagrange {
    fn forward(&self, mut lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sin_phi = phi.sin();
        if (sin_phi.abs() - 1.0).abs() < TOL {
            return Ok((0.0, if phi < 0.0 { -2.0 } else { 2.0 }));
        }
        let v = self.a1 * ((1.0 + sin_phi) / (1.0 - sin_phi)).powf(self.hrw);
        lam *= self.rw;
        let c = 0.5 * (v + 1.0 / v) + lam.cos();
        if c < TOL {
            return Err(TransformError::OutsideDomain);
        }
        Ok((2.0 * lam.sin() / c, (v - 1.0 / v) / c))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if (y.abs() - 2.0).abs() < TOL {
            return Ok((0.0, if y < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 }));
        }
        let x2 = x * x;
        let y2p = 2.0 + y;
        let y2m = 2.0 - y;
        let c = y2p * y2m - x2;
        if c.abs() < TOL {
            return Err(TransformError::OutsideDomain);
        }
        let phi = 2.0
            * ((y2p * y2p + x2) / (self.a2 * (y2m * y2m + x2)))
                .powf(self.hw)
                .atan()
            - FRAC_PI_2;
        Ok((self.w * (4.0 * x).atan2(c), phi))
    }
}

const KEYS: &[&str] = &["W", "lat_1"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "lagrng" {
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
    if name != "lagrng" {
        return Ok(None);
    }
    let w = params.number("W")?.unwrap_or(2.0);
    if w <= 0.0 {
        return Err(CreateError::IllegalValue("W"));
    }
    let hw = 0.5 * w;
    let rw = 1.0 / w;
    let hrw = 0.5 * rw;
    let sin_phi1 = params.radians("lat_1")?.unwrap_or(0.0).sin();
    if (sin_phi1.abs() - 1.0).abs() < TOL {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let a1 = ((1.0 - sin_phi1) / (1.0 + sin_phi1)).powf(hrw);
    Ok(Some(classic_step(
        frame,
        Lagrange {
            a1,
            a2: a1 * a1,
            hrw,
            hw,
            rw,
            w,
        },
    )))
}
