//! Bonne, and Werner when the standard parallel is a pole.
//!
//! Port of PROJ 9.9 `src/projections/bonne.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ bonne.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;
use std::f64::consts::FRAC_PI_2;

const EPS10: f64 = 1e-10;

pub(crate) const KEYS: &[&str] = &["lat_1"];

struct Bonne {
    phi1: f64,
    /// Sphere: cot(lat_1). Ellipsoid: the PROJ `am1` term.
    am1: f64,
    m1: f64,
    es: f64,
    meridian: Option<Meridian>,
}

impl Kernel for Bonne {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let (rh, scale) = if let Some(meridian) = self.meridian {
            let rh = self.am1 + self.m1 - meridian.forward(phi);
            let scale = (1.0 - self.es * sinphi * sinphi).sqrt();
            (rh, scale)
        } else {
            (self.am1 + self.phi1 - phi, 1.0)
        };
        if rh.abs() <= EPS10 {
            return Ok((0.0, 0.0));
        }
        let angle = if self.meridian.is_some() {
            cosphi * lam / (rh * scale)
        } else {
            lam * cosphi / rh
        };
        let y0 = if self.meridian.is_some() {
            self.am1
        } else {
            self.am1
        };
        Ok((rh * angle.sin(), y0 - rh * angle.cos()))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let origin = if self.meridian.is_some() {
            self.am1
        } else {
            self.am1
        };
        y = origin - y;
        let rh = x.hypot(y).copysign(self.phi1);
        let phi = if let Some(meridian) = self.meridian {
            meridian.inverse(self.am1 + self.m1 - rh)
        } else {
            self.am1 + self.phi1 - rh
        };
        let abs_phi = phi.abs();
        if self.meridian.is_some() {
            if abs_phi < FRAC_PI_2 {
                let sinphi = phi.sin();
                let lm = rh * (1.0 - self.es * sinphi * sinphi).sqrt() / phi.cos();
                let lam = if self.phi1 > 0.0 {
                    lm * x.atan2(y)
                } else {
                    lm * (-x).atan2(-y)
                };
                return Ok((lam, phi));
            }
            if abs_phi - FRAC_PI_2 <= EPS10 {
                return Ok((0.0, phi));
            }
            return Err(TransformError::OutsideDomain);
        }
        if abs_phi > FRAC_PI_2 {
            return Err(TransformError::OutsideDomain);
        }
        if FRAC_PI_2 - abs_phi <= EPS10 {
            return Ok((0.0, phi));
        }
        let lm = rh / phi.cos();
        let lam = if self.phi1 > 0.0 {
            lm * x.atan2(y)
        } else {
            lm * (-x).atan2(-y)
        };
        Ok((lam, phi))
    }
}

pub(crate) fn build(
    params: &Params,
    frame: &Frame,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
    if phi1.abs() < EPS10 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let (am1, m1, meridian) = if frame.earth.es() == 0.0 {
        let cot = if phi1.abs() + EPS10 >= FRAC_PI_2 {
            0.0
        } else {
            1.0 / phi1.tan()
        };
        (cot, 0.0, None)
    } else {
        let meridian = Meridian::new(frame.earth.n());
        let sin1 = phi1.sin();
        let cos1 = phi1.cos();
        let m1 = meridian.forward(phi1);
        let am1 = cos1 / ((1.0 - frame.earth.es() * sin1 * sin1).sqrt() * sin1);
        (am1, m1, Some(meridian))
    };
    Ok(classic_step(
        frame,
        Bonne {
            phi1,
            am1,
            m1,
            es: frame.earth.es(),
            meridian,
        },
    ))
}
