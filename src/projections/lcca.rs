//! Lambert conformal conic, the historical truncated form.
//!
//! `lat_1` and `lat_2` are accepted and ignored: the cone constant is
//! `sin(lat_0)`. `lat_0 = 0` is rejected. Meridian distance uses the same
//! rectifying series as `pj_mlfn`.

#![allow(clippy::pedantic, reason = "the radius iteration follows lcca.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;

const DEL_TOL: f64 = 1e-12;

struct Lcca {
    meridian: Meridian,
    r0: f64,
    cone: f64,
    m0: f64,
    c_coeff: f64,
    k0: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "lcca" {
        Some(&["lat_1", "lat_2"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "lcca" {
        return Ok(None);
    }
    if frame.phi0 == 0.0 {
        return Err(CreateError::IllegalValue("lat_0"));
    }
    let meridian = Meridian::new(frame.earth.n());
    let cone = frame.phi0.sin();
    let m0 = meridian.forward(frame.phi0);
    let s2 = cone * cone;
    let mut radius = 1.0 / (1.0 - frame.earth.es() * s2);
    let n0 = radius.sqrt();
    radius *= frame.earth.one_es() * n0;
    Ok(Some(classic_step(
        frame,
        Lcca {
            meridian,
            r0: n0 / frame.phi0.tan(),
            cone,
            m0,
            c_coeff: 1.0 / (6.0 * radius * n0),
            k0: frame.k0,
        },
    )))
}

impl Kernel for Lcca {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let distance = self.meridian.forward(phi) - self.m0;
        let radius = self.r0 - distance * (1.0 + distance * distance * self.c_coeff);
        let angle = lam * self.cone;
        Ok((
            self.k0 * radius * angle.sin(),
            self.k0 * (self.r0 - radius * angle.cos()),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let x = x / self.k0;
        let y = y / self.k0;
        let theta = x.atan2(self.r0 - y);
        let dr = y - x * (0.5 * theta).tan();
        let lam = theta / self.cone;
        let mut distance = dr;
        let mut converged = false;
        for _ in 0..10 {
            let slope = 1.0 + 3.0 * distance * distance * self.c_coeff;
            let delta = (distance * (1.0 + distance * distance * self.c_coeff) - dr) / slope;
            distance -= delta;
            if delta.abs() < DEL_TOL {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam, self.meridian.inverse(distance + self.m0)))
    }
}
