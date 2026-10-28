//! Swiss oblique Mercator and Gauss-Schreiber transverse Mercator.
//!
//! Kernel coordinates stay in units of the semi-major axis. `gstmerc` multiplies
//! by `ra` inside the C kernel because its radius `n2` is already in metres.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{phi2, tsfn};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

fn asin_or(value: f64) -> Result<f64, TransformError> {
    crate::projmath::aasin(value).map_err(|()| TransformError::OutsideDomain)
}

struct Somerc {
    k: f64,
    c: f64,
    hlf_e: f64,
    kr: f64,
    cosp0: f64,
    sinp0: f64,
    e: f64,
    rone_es: f64,
}

impl Kernel for Somerc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sp = self.e * phi.sin();
        let phip = 2.0
            * ((self.c
                * ((FRAC_PI_4 + 0.5 * phi).tan().ln()
                    - self.hlf_e * ((1.0 + sp) / (1.0 - sp)).ln())
                + self.k)
                .exp())
            .atan()
            - FRAC_PI_2;
        let lamp = self.c * lam;
        let cp = phip.cos();
        let phipp = asin_or(self.cosp0 * phip.sin() - self.sinp0 * cp * lamp.cos())?;
        let lampp = asin_or(cp * lamp.sin() / phipp.cos())?;
        Ok((
            self.kr * lampp,
            self.kr * (FRAC_PI_4 + 0.5 * phipp).tan().ln(),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phipp = 2.0 * ((y / self.kr).exp().atan() - FRAC_PI_4);
        let lampp = x / self.kr;
        let cp = phipp.cos();
        let mut phip = asin_or(self.cosp0 * phipp.sin() + self.sinp0 * cp * lampp.cos())?;
        let lamp = asin_or(cp * lampp.sin() / phip.cos())?;
        let con = (self.k - (FRAC_PI_4 + 0.5 * phip).tan().ln()) / self.c;
        let mut guard = 6;
        while guard != 0 {
            let esp = self.e * phip.sin();
            let delp = (con + (FRAC_PI_4 + 0.5 * phip).tan().ln()
                - self.hlf_e * ((1.0 + esp) / (1.0 - esp)).ln())
                * (1.0 - esp * esp)
                * phip.cos()
                * self.rone_es;
            phip -= delp;
            if delp.abs() < 1e-10 {
                break;
            }
            guard -= 1;
        }
        if guard == 0 {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lamp / self.c, phip))
    }
}

struct Gstmerc {
    c: f64,
    n1: f64,
    n2: f64,
    ys: f64,
    e: f64,
}

impl Kernel for Gstmerc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let l = self.n1 * lam;
        let ls = self.c + self.n1 * tsfn(-phi, -phi.sin(), self.e).ln();
        let sin_ls1 = l.sin() / ls.cosh();
        let ls1 = tsfn(-sin_ls1.asin(), -sin_ls1, 0.0).ln();
        Ok((
            self.n2 * ls1,
            self.ys + self.n2 * (ls.sinh() / l.cos()).atan(),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let xn = x / self.n2;
        let yn = (y - self.ys) / self.n2;
        let l = (xn.sinh() / yn.cos()).atan();
        let sin_c = yn.sin() / xn.cosh();
        let lc = tsfn(-sin_c.asin(), -sin_c, 0.0).ln();
        let phi = -phi2(((lc - self.c) / self.n1).exp(), self.e)
            .map_err(|()| TransformError::OutsideDomain)?;
        Ok((l / self.n1, phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "somerc" || name == "gstmerc" {
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
    let earth = frame.earth;
    let built = match name {
        "somerc" => {
            let e = earth.e();
            let es = earth.es();
            let phi0 = frame.phi0;
            let hlf_e = 0.5 * e;
            let cp = phi0.cos().powi(2);
            let rone_es = 1.0 / earth.one_es();
            let c = (1.0 + es * cp * cp * rone_es).sqrt();
            let mut sp = phi0.sin();
            let sinp0 = sp / c;
            let phip0 = asin_or(sinp0).map_err(|_| CreateError::IllegalValue("lat_0"))?;
            sp *= e;
            let k = (FRAC_PI_4 + 0.5 * phip0).tan().ln()
                - c * ((FRAC_PI_4 + 0.5 * phi0).tan().ln()
                    - hlf_e * ((1.0 + sp) / (1.0 - sp)).ln());
            let kr = frame.k0 * earth.one_es().sqrt() / (1.0 - sp * sp);
            classic_step(
                frame,
                Somerc {
                    k,
                    c,
                    hlf_e,
                    kr,
                    cosp0: phip0.cos(),
                    sinp0,
                    e,
                    rone_es,
                },
            )
        }
        "gstmerc" => {
            let e = earth.e();
            let es = earth.es();
            let phi0 = frame.phi0;
            let n1 = (1.0 + es * phi0.cos().powi(4) / (1.0 - es)).sqrt();
            let phic = (phi0.sin() / n1).asin();
            let c = tsfn(-phic, -phi0.sin() / n1, 0.0).ln() - n1 * tsfn(-phi0, -phi0.sin(), e).ln();
            let n2 = frame.k0 * earth.one_es().sqrt() / (1.0 - es * phi0.sin().powi(2));
            classic_step(
                frame,
                Gstmerc {
                    c,
                    n1,
                    n2,
                    ys: -n2 * phic,
                    e,
                },
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(built))
}
