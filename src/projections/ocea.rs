//! Oblique cylindrical equal area.
//!
//! Port of PROJ 9.9 `src/projections/ocea.cpp`. Setup replaces `lam0` with
//! the oblique pole's longitude plus π/2.

#![allow(clippy::pedantic, reason = "formulae follow PROJ ocea.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

struct Ocea {
    rok: f64,
    rtk: f64,
    sinphi: f64,
    cosphi: f64,
    shift: f64,
    over: bool,
}

impl Ocea {
    fn relative(&self, lam: f64) -> f64 {
        let lam = lam - self.shift;
        if self.over {
            lam
        } else {
            adjlon(lam)
        }
    }
}

impl Kernel for Ocea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.relative(lam);
        let sinlam = lam.sin();
        let t = lam.cos();
        let mut x = ((phi.tan() * self.cosphi + self.sinphi * sinlam) / t).atan();
        if t < 0.0 {
            x += PI;
        }
        x *= self.rtk;
        let y = self.rok * (self.sinphi * phi.sin() - self.cosphi * phi.cos() * sinlam);
        Ok((x, y))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y /= self.rok;
        x /= self.rtk;
        let t = (1.0 - y * y).sqrt();
        let s = x.sin();
        let phi = (y * self.sinphi + t * self.cosphi * s).asin();
        let lam = (t * self.sinphi * s - y * self.cosphi).atan2(t * x.cos());
        Ok((lam + self.shift, phi))
    }
}

const KEYS: &[&str] = &["alpha", "lonc", "lat_1", "lat_2", "lon_1", "lon_2"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "ocea" {
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
    if name != "ocea" {
        return Ok(None);
    }
    let (lam_p, phi_p) = if params.flag("alpha") {
        let alpha = PI + params.radians("alpha")?.unwrap_or(0.0);
        let lonz = params.radians("lonc")?.unwrap_or(0.0);
        let lam_p = (-alpha.cos()).atan2(-frame.phi0.sin() * alpha.sin()) + lonz;
        let phi_p = (frame.phi0.cos() * alpha.sin()).asin();
        (lam_p, phi_p)
    } else {
        let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
        let phi2 = params.radians("lat_2")?.unwrap_or(0.0);
        let lam1 = params.radians("lon_1")?.unwrap_or(0.0);
        let lam2 = params.radians("lon_2")?.unwrap_or(0.0);
        let y = phi1.cos() * phi2.sin() * lam1.cos() - phi1.sin() * phi2.cos() * lam2.cos();
        let x = phi1.sin() * phi2.cos() * lam2.sin() - phi1.cos() * phi2.sin() * lam1.sin();
        let mut lam_p = y.atan2(x);
        if lam1 == -FRAC_PI_2 {
            lam_p = -lam_p;
        }
        let cos_diff = (lam_p - lam1).cos();
        let tan_phi1 = phi1.tan();
        let phi_p = if tan_phi1 == 0.0 {
            if cos_diff >= 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            }
        } else {
            (-cos_diff / tan_phi1).atan()
        };
        (lam_p, phi_p)
    };
    let lam0 = lam_p + FRAC_PI_2;
    Ok(Some(classic_step(
        frame,
        Ocea {
            rok: 1.0 / frame.k0,
            rtk: frame.k0,
            sinphi: phi_p.sin(),
            cosphi: phi_p.cos(),
            shift: lam0 - frame.lam0,
            over: frame.over,
        },
    )))
}
