//! Oblated equal area.
//!
//! Port of PROJ 9.9 `src/projections/oea.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ oea.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aacos, aasin, aatan2};

struct Oea {
    theta: f64,
    m: f64,
    n: f64,
    two_r_m: f64,
    two_r_n: f64,
    rm: f64,
    rn: f64,
    hm: f64,
    hn: f64,
    cp0: f64,
    sp0: f64,
}

fn asin_or(v: f64) -> Result<f64, TransformError> {
    aasin(v).map_err(|()| TransformError::OutsideDomain)
}

fn acos_or(v: f64) -> Result<f64, TransformError> {
    aacos(v).map_err(|()| TransformError::OutsideDomain)
}

impl Kernel for Oea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let cp = phi.cos();
        let sp = phi.sin();
        let cl = lam.cos();
        let az = aatan2(cp * lam.sin(), self.cp0 * sp - self.sp0 * cp * cl) + self.theta;
        let shz = (0.5 * acos_or(self.sp0 * sp + self.cp0 * cp * cl)?).sin();
        let m = asin_or(shz * az.sin())?;
        let n = asin_or(shz * az.cos() * m.cos() / (m * self.two_r_m).cos())?;
        let y = self.n * (n * self.two_r_n).sin();
        let x = self.m * (m * self.two_r_m).sin() * n.cos() / (n * self.two_r_n).cos();
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let n = self.hn * asin_or(y * self.rn)?;
        let m = self.hm * asin_or(x * self.rm * (n * self.two_r_n).cos() / n.cos())?;
        let xp = 2.0 * m.sin();
        let yp = 2.0 * n.sin() * (m * self.two_r_m).cos() / m.cos();
        let az = aatan2(xp, yp) - self.theta;
        let caz = az.cos();
        let z = 2.0 * asin_or(0.5 * xp.hypot(yp))?;
        let sz = z.sin();
        let cz = z.cos();
        let phi = asin_or(self.sp0 * cz + self.cp0 * sz * caz)?;
        let lam = aatan2(sz * az.sin(), self.cp0 * cz - self.sp0 * sz * caz);
        Ok((lam, phi))
    }
}

const KEYS: &[&str] = &["n", "m", "theta"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "oea" {
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
    if name != "oea" {
        return Ok(None);
    }
    let n = params.number("n")?.unwrap_or(0.0);
    if n <= 0.0 {
        return Err(CreateError::IllegalValue("n"));
    }
    let m = params.number("m")?.unwrap_or(0.0);
    if m <= 0.0 {
        return Err(CreateError::IllegalValue("m"));
    }
    let theta = params.radians("theta")?.unwrap_or(0.0);
    let rn = 1.0 / n;
    let rm = 1.0 / m;
    Ok(Some(classic_step(
        frame,
        Oea {
            theta,
            m,
            n,
            two_r_m: 2.0 * rm,
            two_r_n: 2.0 * rn,
            rm,
            rn,
            hm: 0.5 * m,
            hn: 0.5 * n,
            cp0: frame.phi0.cos(),
            sp0: frame.phi0.sin(),
        },
    )))
}
