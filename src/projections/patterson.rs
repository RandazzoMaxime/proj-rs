//! Patterson cylindrical. The sphere formulas ignore the ellipsoid.
//! Coordinates are in units of the semi-major axis.

#![allow(
    clippy::pedantic,
    reason = "the Newton inverse follows patterson.cpp, including the clamped target"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};

const K1: f64 = 1.0148;
const K2: f64 = 0.23185;
const K3: f64 = -0.14499;
const K4: f64 = 0.02406;
const C1: f64 = K1;
const C2: f64 = 5.0 * K2;
const C3: f64 = 7.0 * K3;
const C4: f64 = 9.0 * K4;
const EPS11: f64 = 1.0e-11;
const MAX_Y: f64 = 1.790_857_183;

struct Patterson;

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "patterson" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name == "patterson" {
        Ok(Some(classic_step(frame, Patterson)))
    } else {
        Ok(None)
    }
}

impl Kernel for Patterson {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let phi2 = phi * phi;
        let y = phi * (K1 + phi2 * phi2 * (K2 + phi2 * (K3 + K4 * phi2)));
        Ok((lam, y))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let mut yc = y;
        if y > MAX_Y {
            y = MAX_Y;
        } else if y < -MAX_Y {
            y = -MAX_Y;
        }
        let mut converged = false;
        for _ in 0..100 {
            let y2 = yc * yc;
            let y4 = y2 * y2;
            let f = yc * (K1 + y4 * (K2 + y2 * (K3 + K4 * y2))) - y;
            let fder = C1 + y4 * (C2 + y2 * (C3 + C4 * y2));
            let tol = f / fder;
            yc -= tol;
            if tol.abs() < EPS11 {
                converged = true;
                break;
            }
        }
        if converged {
            Ok((x, yc))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}
