//! Two-point equidistant.
//!
//! Port of PROJ 9.9 `src/projections/tpeqd.cpp`. The setup replaces `lam0`
//! with the midpoint of the two longitudes. The kernel undoes the wrapper's
//! user `lon_0` and applies that midpoint instead.

#![allow(clippy::pedantic, reason = "formulae follow PROJ tpeqd.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aacos, aasin, asqrt};
use std::f64::consts::FRAC_PI_2;

struct Tpeqd {
    cp1: f64,
    sp1: f64,
    cp2: f64,
    sp2: f64,
    ccs: f64,
    cs: f64,
    sc: f64,
    r2z0: f64,
    z02: f64,
    dlam2: f64,
    hz0: f64,
    thz0: f64,
    rhshz0: f64,
    ca: f64,
    sa: f64,
    lp: f64,
    lamc: f64,
    /// `computed lam0 − frame.lam0`.
    shift: f64,
    over: bool,
}

fn asin_or(v: f64) -> Result<f64, TransformError> {
    aasin(v).map_err(|()| TransformError::OutsideDomain)
}

fn acos_or(v: f64) -> Result<f64, TransformError> {
    aacos(v).map_err(|()| TransformError::OutsideDomain)
}

impl Tpeqd {
    fn relative(&self, lam: f64) -> f64 {
        let lam = lam - self.shift;
        if self.over {
            lam
        } else {
            adjlon(lam)
        }
    }
}

impl Kernel for Tpeqd {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.relative(lam);
        let sp = phi.sin();
        let cp = phi.cos();
        let dl1 = lam + self.dlam2;
        let dl2 = lam - self.dlam2;
        let mut z1 = acos_or(self.sp1 * sp + self.cp1 * cp * dl1.cos())?;
        let mut z2 = acos_or(self.sp2 * sp + self.cp2 * cp * dl2.cos())?;
        z1 *= z1;
        z2 *= z2;
        let t = z1 - z2;
        let x = self.r2z0 * t;
        let t = self.z02 - t;
        let mut y = self.r2z0 * asqrt(4.0 * self.z02 * z2 - t * t);
        if self.ccs * sp - cp * (self.cs * dl1.sin() - self.sc * dl2.sin()) < 0.0 {
            y = -y;
        }
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let cz1 = y.hypot(x + self.hz0).cos();
        let cz2 = y.hypot(x - self.hz0).cos();
        let s = cz1 + cz2;
        let d = cz1 - cz2;
        let mut lam = -d.atan2(s * self.thz0);
        let mut phi = acos_or((self.thz0 * s).hypot(d) * self.rhshz0)?;
        if y < 0.0 {
            phi = -phi;
        }
        let sp = phi.sin();
        let cp = phi.cos();
        lam -= self.lp;
        let side = lam.cos();
        phi = asin_or(self.sa * sp + self.ca * cp * side)?;
        lam = (cp * lam.sin()).atan2(self.sa * cp * side - self.ca * sp) + self.lamc;
        Ok((lam + self.shift, phi))
    }
}

const KEYS: &[&str] = &["lat_1", "lon_1", "lat_2", "lon_2"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "tpeqd" {
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
    if name != "tpeqd" {
        return Ok(None);
    }
    let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
    let lam1 = params.radians("lon_1")?.unwrap_or(0.0);
    let phi2 = params.radians("lat_2")?.unwrap_or(0.0);
    let lam2 = params.radians("lon_2")?.unwrap_or(0.0);
    if phi1 == phi2 && lam1 == lam2 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let lam0 = adjlon(0.5 * (lam1 + lam2));
    let mut dlam2 = adjlon(lam2 - lam1);
    let cp1 = phi1.cos();
    let cp2 = phi2.cos();
    let sp1 = phi1.sin();
    let sp2 = phi2.sin();
    let cs = cp1 * sp2;
    let sc = sp1 * cp2;
    let ccs = cp1 * cp2 * dlam2.sin();
    let along = cs - sc * dlam2.cos();
    let z02 = ((cp2 * dlam2.sin()).hypot(along)).atan2(sp1 * sp2 + cp1 * cp2 * dlam2.cos());
    // Both control latitudes on a pole make the central angle vanish. `cos`
    // of exactly π/2 is not always 0, so the `z02 == 0` test in tpeqd.cpp
    // is repeated as an explicit pole check.
    if z02 == 0.0 || (phi1.abs() == FRAC_PI_2 && phi2.abs() == FRAC_PI_2) {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let hz0 = 0.5 * z02;
    let a12 = (cp2 * dlam2.sin()).atan2(along);
    let pp = aasin(cp1 * a12.sin()).map_err(|()| CreateError::IllegalValue("lat_1"))?;
    let lp = adjlon((cp1 * a12.cos()).atan2(sp1) - hz0);
    dlam2 *= 0.5;
    let lamc = FRAC_PI_2 - (a12.sin() * sp1).atan2(a12.cos()) - dlam2;
    Ok(Some(classic_step(
        frame,
        Tpeqd {
            cp1,
            sp1,
            cp2,
            sp2,
            ccs,
            cs,
            sc,
            r2z0: 0.5 / z02,
            z02: z02 * z02,
            dlam2,
            hz0,
            thz0: hz0.tan(),
            rhshz0: 0.5 / hz0.sin(),
            ca: pp.cos(),
            sa: pp.sin(),
            lp,
            lamc,
            shift: lam0 - frame.lam0,
            over: frame.over,
        },
    )))
}
