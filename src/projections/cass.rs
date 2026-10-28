//! Cassini.
//!
//! Port of PROJ 9.9 `src/projections/cass.cpp`. The ellipsoidal inverse starts
//! from the series and then runs the same finite-difference correction as
//! `pj_generic_inverse_2d` (tolerance 1e-12).

#![allow(clippy::pedantic, reason = "formulae follow PROJ cass.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;

const C1: f64 = 0.166_666_666_666_666_666_66;
const C2: f64 = 0.008_333_333_333_333_333_33;
const C3: f64 = 0.041_666_666_666_666_666_66;
const C4: f64 = 0.333_333_333_333_333_333_33;
const C5: f64 = 0.066_666_666_666_666_666_66;

struct Cass {
    phi0: f64,
    es: f64,
    m0: f64,
    meridian: Option<Meridian>,
    hyperbolic: bool,
}

impl Cass {
    fn forward_ellipsoid(&self, lam: f64, phi: f64) -> (f64, f64) {
        let meridian = self.meridian.expect("ellipsoid cass has a meridian");
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let nu_square = 1.0 / (1.0 - self.es * sinphi * sinphi);
        let nu = nu_square.sqrt();
        let tanphi = phi.tan();
        let t = tanphi * tanphi;
        let a = lam * cosphi;
        let c = self.es * cosphi * cosphi / (1.0 - self.es);
        let a2 = a * a;
        let x = nu * a * (1.0 - a2 * t * (C1 + (8.0 - t + 8.0 * c) * a2 * C2));
        let mut y = meridian.forward(phi) - self.m0
            + nu * tanphi * a2 * (0.5 + (5.0 - t + 6.0 * c) * a2 * C3);
        if self.hyperbolic {
            let rho = nu_square * (1.0 - self.es) * nu;
            y -= y * y * y / (6.0 * rho * nu);
        }
        (x, y)
    }
}

impl Kernel for Cass {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if self.meridian.is_none() {
            let x = (phi.cos() * lam.sin()).asin();
            let y = phi.tan().atan2(lam.cos()) - self.phi0;
            return Ok((x, y));
        }
        Ok(self.forward_ellipsoid(lam, phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let Some(meridian) = self.meridian else {
            let dd = y + self.phi0;
            let phi = (dd.sin() * x.cos()).asin();
            let lam = x.tan().atan2(dd.cos());
            return Ok((lam, phi));
        };
        let phi1 = meridian.inverse(self.m0 + y);
        let tanphi1 = phi1.tan();
        let t1 = tanphi1 * tanphi1;
        let sinphi1 = phi1.sin();
        let nu1_square = 1.0 / (1.0 - self.es * sinphi1 * sinphi1);
        let nu1 = nu1_square.sqrt();
        let rho1 = nu1_square * (1.0 - self.es) * nu1;
        let d = x / nu1;
        let d2 = d * d;
        let mut phi = phi1 - (nu1 * tanphi1 / rho1) * d2 * (0.5 - (1.0 + 3.0 * t1) * d2 * C3);
        let mut lam = d * (1.0 + t1 * d2 * (-C4 + (1.0 + 3.0 * t1) * d2 * C5)) / phi1.cos();
        for _ in 0..6 {
            let (fx, fy) = self.forward_ellipsoid(lam, phi);
            let dx = fx - x;
            let dy = fy - y;
            if dx.hypot(dy) < 1e-12 {
                break;
            }
            const STEP: f64 = 1e-8;
            let (fx_l, fy_l) = self.forward_ellipsoid(lam + STEP, phi);
            let (fx_p, fy_p) = self.forward_ellipsoid(lam, phi + STEP);
            let dxl = (fx_l - fx) / STEP;
            let dyl = (fy_l - fy) / STEP;
            let dxp = (fx_p - fx) / STEP;
            let dyp = (fy_p - fy) / STEP;
            let det = dxl * dyp - dxp * dyl;
            if det == 0.0 {
                break;
            }
            lam -= (dyp * dx - dxp * dy) / det;
            phi -= (dxl * dy - dyl * dx) / det;
        }
        Ok((lam, phi))
    }
}

const KEYS: &[&str] = &["hyperbolic"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "cass" {
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
    if name != "cass" {
        return Ok(None);
    }
    let meridian = if frame.earth.es() == 0.0 {
        None
    } else {
        let meridian = Meridian::new(frame.earth.n());
        Some(meridian)
    };
    let m0 = meridian.map_or(0.0, |meridian| meridian.forward(frame.phi0));
    Ok(Some(classic_step(
        frame,
        Cass {
            phi0: frame.phi0,
            es: frame.earth.es(),
            m0,
            meridian,
            hyperbolic: params.flag("hyperbolic"),
        },
    )))
}
