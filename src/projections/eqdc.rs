//! Equidistant conic.
//!
//! Port of PROJ 9.9 `src/projections/eqdc.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ eqdc.cpp")]

use crate::engine::{classic_step, proj_msfn, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;
use std::f64::consts::FRAC_PI_2;

const EPS10: f64 = 1e-10;

pub(crate) const KEYS: &[&str] = &["lat_1", "lat_2"];

struct Eqdc {
    n: f64,
    rho0: f64,
    c: f64,
    meridian: Option<Meridian>,
}

impl Kernel for Eqdc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let along = if let Some(meridian) = self.meridian {
            meridian.forward(phi)
        } else {
            phi
        };
        let rho = self.c - along;
        let angle = lam * self.n;
        Ok((rho * angle.sin(), self.rho0 - rho * angle.cos()))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y = self.rho0 - y;
        let mut rho = x.hypot(y);
        if rho == 0.0 {
            let phi = if self.n > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
            return Ok((0.0, phi));
        }
        if self.n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }
        let mut phi = self.c - rho;
        if let Some(meridian) = self.meridian {
            phi = meridian.inverse(phi);
        }
        Ok((x.atan2(y) / self.n, phi))
    }
}

pub(crate) fn build(
    params: &Params,
    frame: &Frame,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
    let phi2 = params.radians("lat_2")?.unwrap_or(0.0);
    if phi1.abs() > FRAC_PI_2 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    if phi2.abs() > FRAC_PI_2 {
        return Err(CreateError::IllegalValue("lat_2"));
    }
    if (phi1 + phi2).abs() < EPS10 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let secant = (phi1 - phi2).abs() >= EPS10;
    let mut n = phi1.sin();
    let cosphi = phi1.cos();
    let ellips = frame.earth.es() > 0.0;
    let (c, rho0, meridian) = if ellips {
        let meridian = Meridian::new(frame.earth.n());
        let m1 = proj_msfn(phi1.sin(), cosphi, frame.earth.es());
        let ml1 = meridian.forward(phi1);
        if secant {
            let m2 = proj_msfn(phi2.sin(), phi2.cos(), frame.earth.es());
            let ml2 = meridian.forward(phi2);
            if ml1 == ml2 || (m1 - m2) == 0.0 {
                return Err(CreateError::IllegalValue("lat_1"));
            }
            n = (m1 - m2) / (ml2 - ml1);
            if n == 0.0 {
                return Err(CreateError::IllegalValue("lat_1"));
            }
        }
        let c = ml1 + m1 / n;
        let rho0 = c - meridian.forward(frame.phi0);
        (c, rho0, Some(meridian))
    } else {
        if secant {
            n = (cosphi - phi2.cos()) / (phi2 - phi1);
        }
        if n == 0.0 {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        let c = phi1 + cosphi / n;
        (c, c - frame.phi0, None)
    };
    Ok(classic_step(
        frame,
        Eqdc {
            n,
            rho0,
            c,
            meridian,
        },
    ))
}
