//! Central conic. `lat_1` is the standard parallel and must not be zero.
//! The result is in units of the semi-major axis.

#![allow(clippy::pedantic, reason = "the cone radius follows ccon.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};

const EPS10: f64 = 1e-10;

struct Ccon {
    phi1: f64,
    cot_phi1: f64,
    sin_phi1: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "ccon" {
        Some(&["lat_1"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "ccon" {
        return Ok(None);
    }
    let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
    if phi1.abs() < EPS10 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let sin_phi1 = phi1.sin();
    Ok(Some(classic_step(
        frame,
        Ccon {
            phi1,
            cot_phi1: phi1.cos() / sin_phi1,
            sin_phi1,
        },
    )))
}

impl Kernel for Ccon {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let radius = self.cot_phi1 - (phi - self.phi1).tan();
        let angle = lam * self.sin_phi1;
        Ok((radius * angle.sin(), self.cot_phi1 - radius * angle.cos()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y = self.cot_phi1 - y;
        let phi = self.phi1 - (x.hypot(y) - self.cot_phi1).atan();
        let lam = x.atan2(y) / self.sin_phi1;
        Ok((lam, phi))
    }
}
