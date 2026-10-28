//! Stereographic and Universal Polar Stereographic.
//!
//! Port of PROJ 9.9 `src/projections/stere.cpp`. UPS overwrites the origin,
//! the scale and the false offsets; those replacements are applied in the
//! kernel so the classic wrapper does not add them twice.

#![allow(clippy::pedantic, reason = "formulae follow PROJ stere.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::tsfn;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const EPS10: f64 = 1e-10;
const TOL: f64 = 1e-8;
const CONV: f64 = 1e-10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    South,
    North,
    Oblique,
    Equator,
}

struct Stere {
    mode: Mode,
    sin_x1: f64,
    cos_x1: f64,
    akm1: f64,
    sinph0: f64,
    cosph0: f64,
    phi0: f64,
    e: f64,
    ellips: bool,
    shift: f64,
    over: bool,
    /// Extra false offset, in units of the semi-major axis.
    false_x: f64,
    false_y: f64,
}

fn ssfn(phit: f64, mut sinphi: f64, e: f64) -> f64 {
    sinphi *= e;
    (0.5 * (FRAC_PI_2 + phit)).tan() * ((1.0 - sinphi) / (1.0 + sinphi)).powf(0.5 * e)
}

impl Stere {
    fn relative(&self, lam: f64) -> f64 {
        let lam = lam - self.shift;
        if self.over {
            lam
        } else {
            adjlon(lam)
        }
    }
}

impl Kernel for Stere {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.relative(lam);
        let (x, y) = if self.ellips {
            self.forward_ellipsoid(lam, phi)?
        } else {
            self.forward_sphere(lam, phi)?
        };
        Ok((x + self.false_x, y + self.false_y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let x = x - self.false_x;
        let y = y - self.false_y;
        let (lam, phi) = if self.ellips {
            self.inverse_ellipsoid(x, y)?
        } else {
            self.inverse_sphere(x, y)?
        };
        Ok((lam + self.shift, phi))
    }
}

impl Stere {
    fn forward_sphere(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let mut coslam = lam.cos();
        let sinlam = lam.sin();
        match self.mode {
            Mode::Equator | Mode::Oblique => {
                let y = if self.mode == Mode::Equator {
                    1.0 + cosphi * coslam
                } else {
                    1.0 + self.sinph0 * sinphi + self.cosph0 * cosphi * coslam
                };
                if y <= EPS10 {
                    return Err(TransformError::OutsideDomain);
                }
                let y = self.akm1 / y;
                let scaled = if self.mode == Mode::Equator {
                    sinphi
                } else {
                    self.cosph0 * sinphi - self.sinph0 * cosphi * coslam
                };
                Ok((y * cosphi * sinlam, y * scaled))
            }
            Mode::North | Mode::South => {
                if self.mode == Mode::North {
                    coslam = -coslam;
                    phi = -phi;
                }
                if (phi - FRAC_PI_2).abs() < TOL {
                    return Err(TransformError::OutsideDomain);
                }
                let y = self.akm1 * (FRAC_PI_4 + 0.5 * phi).tan();
                Ok((sinlam * y, y * coslam))
            }
        }
    }

    fn forward_ellipsoid(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let mut coslam = lam.cos();
        let sinlam = lam.sin();
        let mut sinphi = phi.sin();
        let (sin_x, cos_x) = if matches!(self.mode, Mode::Oblique | Mode::Equator) {
            let x = 2.0 * ssfn(phi, sinphi, self.e).atan() - FRAC_PI_2;
            (x.sin(), x.cos())
        } else {
            (0.0, 0.0)
        };
        let (x, y) = match self.mode {
            Mode::Oblique => {
                let denom =
                    self.cos_x1 * (1.0 + self.sin_x1 * sin_x + self.cos_x1 * cos_x * coslam);
                if denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                let scale = self.akm1 / denom;
                (
                    scale * cos_x,
                    scale * (self.cos_x1 * sin_x - self.sin_x1 * cos_x * coslam),
                )
            }
            Mode::Equator => {
                let (scale, y) = if 1.0 + cos_x * coslam == 0.0 {
                    (0.0, f64::INFINITY)
                } else {
                    let scale = self.akm1 / (1.0 + cos_x * coslam);
                    (scale, scale * sin_x)
                };
                (scale * cos_x, y)
            }
            Mode::South | Mode::North => {
                if self.mode == Mode::South {
                    phi = -phi;
                    coslam = -coslam;
                    sinphi = -sinphi;
                }
                let radial = if (phi - FRAC_PI_2).abs() < 1e-15 {
                    0.0
                } else {
                    self.akm1 * tsfn(phi, sinphi, self.e)
                };
                (radial, -radial * coslam)
            }
        };
        Ok((x * sinlam, y))
    }

    fn inverse_sphere(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let rh = x.hypot(y);
        let c = 2.0 * (rh / self.akm1).atan();
        let sinc = c.sin();
        let cosc = c.cos();
        match self.mode {
            Mode::Equator => {
                let phi = if rh.abs() <= EPS10 {
                    0.0
                } else {
                    (y * sinc / rh).asin()
                };
                let lam = if cosc != 0.0 || x != 0.0 {
                    (x * sinc).atan2(cosc * rh)
                } else {
                    0.0
                };
                Ok((lam, phi))
            }
            Mode::Oblique => {
                let phi = if rh.abs() <= EPS10 {
                    self.phi0
                } else {
                    (cosc * self.sinph0 + y * sinc * self.cosph0 / rh).asin()
                };
                let c = cosc - self.sinph0 * phi.sin();
                let lam = if c != 0.0 || x != 0.0 {
                    (x * sinc * self.cosph0).atan2(c * rh)
                } else {
                    0.0
                };
                Ok((lam, phi))
            }
            Mode::North | Mode::South => {
                if self.mode == Mode::North {
                    y = -y;
                }
                let phi = if rh.abs() <= EPS10 {
                    self.phi0
                } else if self.mode == Mode::South {
                    (-cosc).asin()
                } else {
                    cosc.asin()
                };
                let lam = if x == 0.0 && y == 0.0 {
                    0.0
                } else {
                    x.atan2(y)
                };
                Ok((lam, phi))
            }
        }
    }

    fn inverse_ellipsoid(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let rho = x.hypot(y);
        let (tp, mut phi_l, halfpi, halfe) = match self.mode {
            Mode::Oblique | Mode::Equator => {
                let tp = 2.0 * (rho * self.cos_x1).atan2(self.akm1);
                let cosphi = tp.cos();
                let sinphi = tp.sin();
                let phi_l = if rho == 0.0 {
                    (cosphi * self.sin_x1).asin()
                } else {
                    (cosphi * self.sin_x1 + y * sinphi * self.cos_x1 / rho).asin()
                };
                let tp = (0.5 * (FRAC_PI_2 + phi_l)).tan();
                x *= sinphi;
                y = rho * self.cos_x1 * cosphi - y * self.sin_x1 * sinphi;
                (tp, phi_l, FRAC_PI_2, 0.5 * self.e)
            }
            Mode::North | Mode::South => {
                if self.mode == Mode::North {
                    y = -y;
                }
                let tp = -rho / self.akm1;
                let phi_l = FRAC_PI_2 - 2.0 * tp.atan();
                (tp, phi_l, -FRAC_PI_2, -0.5 * self.e)
            }
        };
        for _ in 0..8 {
            let sinphi = self.e * phi_l.sin();
            let phi = 2.0 * (tp * ((1.0 + sinphi) / (1.0 - sinphi)).powf(halfe)).atan() - halfpi;
            if (phi_l - phi).abs() < CONV {
                let phi = if self.mode == Mode::South { -phi } else { phi };
                let lam = if x == 0.0 && y == 0.0 {
                    0.0
                } else {
                    x.atan2(y)
                };
                return Ok((lam, phi));
            }
            phi_l = phi;
        }
        Err(TransformError::OutsideDomain)
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "stere" => Some(&["lat_ts"]),
        "ups" => Some(&["south"]),
        _ => None,
    }
}

struct Origin {
    phi0: f64,
    k0: f64,
    phits: f64,
    shift: f64,
    false_x: f64,
    false_y: f64,
}

fn origin_stere(params: &Params, frame: &Frame) -> Result<Origin, CreateError> {
    let phits = params.radians("lat_ts")?.unwrap_or(FRAC_PI_2).abs();
    Ok(Origin {
        phi0: frame.phi0,
        k0: frame.k0,
        phits,
        shift: 0.0,
        false_x: 0.0,
        false_y: 0.0,
    })
}

fn origin_ups(params: &Params, frame: &Frame) -> Result<Origin, CreateError> {
    if frame.earth.es() == 0.0 {
        return Err(CreateError::IllegalValue("ellps"));
    }
    let phi0 = if params.flag("south") {
        -FRAC_PI_2
    } else {
        FRAC_PI_2
    };
    Ok(Origin {
        phi0,
        k0: 0.994,
        phits: FRAC_PI_2,
        shift: -frame.lam0,
        false_x: (2_000_000.0 - frame.x0) / frame.earth.a(),
        false_y: (2_000_000.0 - frame.y0) / frame.earth.a(),
    })
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let origin = match name {
        "stere" => origin_stere(params, frame)?,
        "ups" => origin_ups(params, frame)?,
        _ => return Ok(None),
    };
    let t = origin.phi0.abs();
    let mode = if (t - FRAC_PI_2).abs() < EPS10 {
        if origin.phi0 < 0.0 {
            Mode::South
        } else {
            Mode::North
        }
    } else if t > EPS10 {
        Mode::Oblique
    } else {
        Mode::Equator
    };
    let ellips = frame.earth.es() != 0.0;
    let e = frame.earth.e();
    let (akm1, sin_x1, cos_x1, sinph0, cosph0) = if ellips {
        match mode {
            Mode::North | Mode::South => {
                let akm1 = if (origin.phits - FRAC_PI_2).abs() < EPS10 {
                    2.0 * origin.k0 / ((1.0 + e).powf(1.0 + e) * (1.0 - e).powf(1.0 - e)).sqrt()
                } else {
                    let t = origin.phits.sin();
                    let mut akm1 = origin.phits.cos() / tsfn(origin.phits, t, e);
                    let t = t * e;
                    akm1 /= (1.0 - t * t).sqrt();
                    akm1
                };
                (akm1, 0.0, 0.0, 0.0, 0.0)
            }
            Mode::Equator | Mode::Oblique => {
                let t = origin.phi0.sin();
                let x = 2.0 * ssfn(origin.phi0, t, e).atan() - FRAC_PI_2;
                let te = t * e;
                let akm1 = 2.0 * origin.k0 * origin.phi0.cos() / (1.0 - te * te).sqrt();
                (akm1, x.sin(), x.cos(), 0.0, 0.0)
            }
        }
    } else {
        match mode {
            Mode::Oblique => (
                2.0 * origin.k0,
                0.0,
                0.0,
                origin.phi0.sin(),
                origin.phi0.cos(),
            ),
            Mode::Equator => (2.0 * origin.k0, 0.0, 0.0, 0.0, 0.0),
            Mode::South | Mode::North => {
                let akm1 = if (origin.phits - FRAC_PI_2).abs() >= EPS10 {
                    origin.phits.cos() / (FRAC_PI_4 - 0.5 * origin.phits).tan()
                } else {
                    2.0 * origin.k0
                };
                (akm1, 0.0, 0.0, 0.0, 0.0)
            }
        }
    };
    Ok(Some(classic_step(
        frame,
        Stere {
            mode,
            sin_x1,
            cos_x1,
            akm1,
            sinph0,
            cosph0,
            phi0: origin.phi0,
            e,
            ellips,
            shift: origin.shift,
            over: frame.over,
            false_x: origin.false_x,
            false_y: origin.false_y,
        },
    )))
}
