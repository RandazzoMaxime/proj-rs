//! Airy. Forward only.
//!
//! Port of PROJ 9.9 `src/projections/airy.cpp`. The projection forces a sphere
//! and keeps the semi-major axis.

#![allow(clippy::pedantic, reason = "formulae follow PROJ airy.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_2;

const EPS: f64 = 1e-10;

#[derive(Clone, Copy)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct Airy {
    mode: Mode,
    sinph0: f64,
    cosph0: f64,
    p_halfpi: f64,
    cb: f64,
    no_cut: bool,
}

impl Kernel for Airy {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let sinlam = lam.sin();
        let coslam = lam.cos();
        match self.mode {
            Mode::Equator | Mode::Oblique => {
                let sinphi = phi.sin();
                let cosphi = phi.cos();
                let mut cosz = cosphi * coslam;
                if matches!(self.mode, Mode::Oblique) {
                    cosz = self.sinph0 * sinphi + self.cosph0 * cosz;
                }
                if !self.no_cut && cosz < -EPS {
                    return Err(TransformError::OutsideDomain);
                }
                let s = 1.0 - cosz;
                let krho = if s.abs() > EPS {
                    let t = 0.5 * (1.0 + cosz);
                    if t == 0.0 {
                        return Err(TransformError::OutsideDomain);
                    }
                    -t.ln() / s - self.cb / t
                } else {
                    0.5 - self.cb
                };
                let y = if matches!(self.mode, Mode::Oblique) {
                    krho * (self.cosph0 * sinphi - self.sinph0 * cosphi * coslam)
                } else {
                    krho * sinphi
                };
                Ok((krho * cosphi * sinlam, y))
            }
            Mode::South | Mode::North => {
                phi = (self.p_halfpi - phi).abs();
                if !self.no_cut && phi - EPS > FRAC_PI_2 {
                    return Err(TransformError::OutsideDomain);
                }
                phi *= 0.5;
                if phi > EPS {
                    let t = phi.tan();
                    let krho = -2.0 * (phi.cos().ln() / t + t * self.cb);
                    let y = if matches!(self.mode, Mode::North) {
                        -krho * coslam
                    } else {
                        krho * coslam
                    };
                    Ok((krho * sinlam, y))
                } else {
                    Ok((0.0, 0.0))
                }
            }
        }
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

const KEYS: &[&str] = &["no_cut", "lat_b"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "airy" {
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
    if name != "airy" {
        return Ok(None);
    }
    let no_cut = params.flag("no_cut");
    let beta = 0.5 * (FRAC_PI_2 - params.radians("lat_b")?.unwrap_or(0.0));
    let cb = if beta.abs() < EPS {
        -0.5
    } else {
        let cot = 1.0 / beta.tan();
        cot * cot * beta.cos().ln()
    };
    let (mode, p_halfpi, sinph0, cosph0) = if (frame.phi0.abs() - FRAC_PI_2).abs() < EPS {
        if frame.phi0 < 0.0 {
            (Mode::South, -FRAC_PI_2, 0.0, 0.0)
        } else {
            (Mode::North, FRAC_PI_2, 0.0, 0.0)
        }
    } else if frame.phi0.abs() < EPS {
        (Mode::Equator, 0.0, 0.0, 0.0)
    } else {
        (Mode::Oblique, 0.0, frame.phi0.sin(), frame.phi0.cos())
    };
    Ok(Some(classic_step(
        frame,
        Airy {
            mode,
            sinph0,
            cosph0,
            p_halfpi,
            cb,
            no_cut,
        },
    )))
}
