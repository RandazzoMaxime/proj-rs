//! Near-sided and tilted perspective.
//!
//! Port of PROJ 9.9 `src/projections/nsper.cpp`. Height is divided by the
//! semi-major axis inside the kernel, which still returns units of `a`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ nsper.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_2;

const EPS10: f64 = 1e-10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct NsPer {
    mode: Mode,
    sinph0: f64,
    cosph0: f64,
    phi0: f64,
    p: f64,
    rp: f64,
    pn1: f64,
    pfact: f64,
    h: f64,
    cg: f64,
    sg: f64,
    sw: f64,
    cw: f64,
    tilt: bool,
}

impl Kernel for NsPer {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let mut coslam = lam.cos();
        let mut y = match self.mode {
            Mode::Oblique => self.sinph0 * sinphi + self.cosph0 * cosphi * coslam,
            Mode::Equator => cosphi * coslam,
            Mode::South => -sinphi,
            Mode::North => sinphi,
        };
        if y < self.rp {
            return Err(TransformError::OutsideDomain);
        }
        y = self.pn1 / (self.p - y);
        let mut x = y * cosphi * lam.sin();
        y = match self.mode {
            Mode::Oblique => y * (self.cosph0 * sinphi - self.sinph0 * cosphi * coslam),
            Mode::Equator => y * sinphi,
            Mode::North => {
                coslam = -coslam;
                y * cosphi * coslam
            }
            Mode::South => y * cosphi * coslam,
        };
        if self.tilt {
            let yt = y * self.cg + x * self.sg;
            let ba = 1.0 / (yt * self.sw * self.h + self.cw);
            x = (x * self.cg - y * self.sg) * self.cw * ba;
            y = yt * ba;
        }
        Ok((x, y))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        if self.tilt {
            let yt = 1.0 / (self.pn1 - y * self.sw);
            let bm = self.pn1 * x * yt;
            let bq = self.pn1 * y * self.cw * yt;
            x = bm * self.cg + bq * self.sg;
            y = bq * self.cg - bm * self.sg;
        }
        let rh = x.hypot(y);
        if rh.abs() <= EPS10 {
            return Ok((0.0, self.phi0));
        }
        let sinz = 1.0 - rh * rh * self.pfact;
        if sinz < 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let sinz = (self.p - sinz.sqrt()) / (self.pn1 / rh + rh / self.pn1);
        let cosz = (1.0 - sinz * sinz).sqrt();
        let phi = match self.mode {
            Mode::Oblique => {
                let phi = (cosz * self.sinph0 + y * sinz * self.cosph0 / rh).asin();
                y = (cosz - self.sinph0 * phi.sin()) * rh;
                x *= sinz * self.cosph0;
                phi
            }
            Mode::Equator => {
                let phi = (y * sinz / rh).asin();
                y = cosz * rh;
                x *= sinz;
                phi
            }
            Mode::North => {
                y = -y;
                cosz.asin()
            }
            Mode::South => -cosz.asin(),
        };
        Ok((x.atan2(y), phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "nsper" => Some(&["h"]),
        "tpers" => Some(&["h", "tilt", "azi"]),
        _ => None,
    }
}

fn finish(
    frame: &Frame,
    height: f64,
    tilt: bool,
    cg: f64,
    sg: f64,
    cw: f64,
    sw: f64,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let pn1 = height / frame.earth.a();
    if pn1 <= 0.0 || pn1 > 1e10 {
        return Err(CreateError::IllegalValue("h"));
    }
    let mode = if (frame.phi0.abs() - FRAC_PI_2).abs() < EPS10 {
        if frame.phi0 < 0.0 {
            Mode::South
        } else {
            Mode::North
        }
    } else if frame.phi0.abs() < EPS10 {
        Mode::Equator
    } else {
        Mode::Oblique
    };
    let (sinph0, cosph0) = if mode == Mode::Oblique {
        (frame.phi0.sin(), frame.phi0.cos())
    } else {
        (0.0, 0.0)
    };
    let p = 1.0 + pn1;
    Ok(classic_step(
        frame,
        NsPer {
            mode,
            sinph0,
            cosph0,
            phi0: frame.phi0,
            p,
            rp: 1.0 / p,
            pn1,
            pfact: (p + 1.0) / pn1,
            h: 1.0 / pn1,
            cg,
            sg,
            sw,
            cw,
            tilt,
        },
    ))
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    match name {
        "nsper" => {
            let height = params.number("h")?.unwrap_or(0.0);
            Ok(Some(finish(frame, height, false, 1.0, 0.0, 1.0, 0.0)?))
        }
        "tpers" => {
            let height = params.number("h")?.unwrap_or(0.0);
            let omega = params.radians("tilt")?.unwrap_or(0.0);
            let gamma = params.radians("azi")?.unwrap_or(0.0);
            Ok(Some(finish(
                frame,
                height,
                true,
                gamma.cos(),
                gamma.sin(),
                omega.cos(),
                omega.sin(),
            )?))
        }
        _ => Ok(None),
    }
}
