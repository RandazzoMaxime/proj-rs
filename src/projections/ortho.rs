//! Orthographic.
//!
//! Port of PROJ 9.9 `src/projections/ortho.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ ortho.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS10: f64 = 1e-10;

pub(crate) const KEYS: &[&str] = &["alpha"];

#[derive(Clone, Copy)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct Ortho {
    mode: Mode,
    sinph0: f64,
    cosph0: f64,
    phi0: f64,
    k0: f64,
    sinalpha: f64,
    cosalpha: f64,
    es: f64,
    one_es: f64,
    nu0: f64,
    y_shift: f64,
    y_scale: f64,
    ellips: bool,
}

impl Ortho {
    fn rotate_fwd(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x * self.cosalpha - y * self.sinalpha) * self.k0,
            (x * self.sinalpha + y * self.cosalpha) * self.k0,
        )
    }

    fn rotate_inv(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (self.cosalpha * x + self.sinalpha * y) / self.k0,
            (-self.sinalpha * x + self.cosalpha * y) / self.k0,
        )
    }

    fn sphere_forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let cosphi = phi.cos();
        let coslam = lam.cos();
        let sinphi = phi.sin();
        let y = match self.mode {
            Mode::Equator => {
                if cosphi * coslam < -EPS10 {
                    return Err(TransformError::OutsideDomain);
                }
                sinphi
            }
            Mode::Oblique => {
                if self.sinph0 * sinphi + self.cosph0 * cosphi * coslam < -EPS10 {
                    return Err(TransformError::OutsideDomain);
                }
                self.cosph0 * sinphi - self.sinph0 * cosphi * coslam
            }
            Mode::North => {
                if (phi - self.phi0).abs() - EPS10 > FRAC_PI_2 {
                    return Err(TransformError::OutsideDomain);
                }
                cosphi * -coslam
            }
            Mode::South => {
                if (phi - self.phi0).abs() - EPS10 > FRAC_PI_2 {
                    return Err(TransformError::OutsideDomain);
                }
                cosphi * coslam
            }
        };
        Ok(self.rotate_fwd(cosphi * lam.sin(), y))
    }

    fn sphere_inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let (x, y) = self.rotate_inv(x, y);
        let rh = x.hypot(y);
        let mut sinc = rh;
        if sinc > 1.0 {
            if sinc - 1.0 > EPS10 {
                return Err(TransformError::OutsideDomain);
            }
            sinc = 1.0;
        }
        let cosc = (1.0 - sinc * sinc).sqrt();
        if rh.abs() <= EPS10 {
            return Ok((0.0, self.phi0));
        }
        let (phi, xx, yy) = match self.mode {
            Mode::North => (sinc.acos(), x, -y),
            Mode::South => (-sinc.acos(), x, y),
            Mode::Equator => {
                let phi = clamp_asin(y * sinc / rh);
                (phi, x * sinc, cosc * rh)
            }
            Mode::Oblique => {
                let raw = cosc * self.sinph0 + y * sinc * self.cosph0 / rh;
                let phi = clamp_asin(raw);
                let yy = (cosc - self.sinph0 * raw) * rh;
                (phi, x * sinc * self.cosph0, yy)
            }
        };
        let lam = if yy == 0.0 && matches!(self.mode, Mode::Oblique | Mode::Equator) {
            if xx == 0.0 {
                0.0
            } else if xx < 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            }
        } else {
            xx.atan2(yy)
        };
        Ok((lam, phi))
    }
}

fn clamp_asin(phi: f64) -> f64 {
    if phi.abs() >= 1.0 {
        if phi < 0.0 {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        }
    } else {
        phi.asin()
    }
}

impl Kernel for Ortho {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if !self.ellips {
            return self.sphere_forward(lam, phi);
        }
        let cosphi = phi.cos();
        let sinphi = phi.sin();
        let coslam = lam.cos();
        let sinlam = lam.sin();
        if self.sinph0 * sinphi + self.cosph0 * cosphi * coslam < -EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        let nu = 1.0 / (1.0 - self.es * sinphi * sinphi).sqrt();
        let x = nu * cosphi * sinlam;
        let y = nu * (sinphi * self.cosph0 - cosphi * self.sinph0 * coslam)
            + self.es * (self.nu0 * self.sinph0 - nu * sinphi) * self.cosph0;
        Ok(self.rotate_fwd(x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if !self.ellips {
            return self.sphere_inverse(x, y);
        }
        let (x, y) = self.rotate_inv(x, y);
        match self.mode {
            Mode::North | Mode::South => {
                let rh2 = x * x + y * y;
                let phi = if rh2 >= 1.0 - 1e-15 {
                    if rh2 - 1.0 > EPS10 {
                        return Err(TransformError::OutsideDomain);
                    }
                    0.0
                } else {
                    (rh2 * self.one_es / (1.0 - self.es * rh2)).sqrt().acos()
                        * if matches!(self.mode, Mode::North) {
                            1.0
                        } else {
                            -1.0
                        }
                };
                let sign = if matches!(self.mode, Mode::North) {
                    -1.0
                } else {
                    1.0
                };
                Ok((x.atan2(y * sign), phi))
            }
            Mode::Equator => {
                let b_ratio = 1.0 / self.one_es.sqrt();
                if x * x + (y * b_ratio) * (y * b_ratio) > 1.0 + 1e-11 {
                    return Err(TransformError::OutsideDomain);
                }
                let sinphi2 = if y == 0.0 {
                    0.0
                } else {
                    1.0 / (((1.0 - self.es) / y).powi(2) + self.es)
                };
                if sinphi2 > 1.0 - 1e-11 {
                    return Ok((0.0, FRAC_PI_2.copysign(y)));
                }
                let phi = sinphi2.sqrt().asin().copysign(y);
                let sinlam = x * ((1.0 - self.es * sinphi2) / (1.0 - sinphi2)).sqrt();
                let lam = if sinlam.abs() - 1.0 > -1e-15 {
                    FRAC_PI_2.copysign(x)
                } else {
                    sinlam.asin()
                };
                Ok((lam, phi))
            }
            Mode::Oblique => {
                let yy = (y - self.y_shift) / self.y_scale;
                if x * x + yy * yy > 1.0 + 1e-11 {
                    return Err(TransformError::OutsideDomain);
                }
                let mut guess = self.sphere_inverse(x, yy)?;
                for _ in 0..20 {
                    let cosphi = guess.1.cos();
                    let sinphi = guess.1.sin();
                    let coslam = guess.0.cos();
                    let sinlam = guess.0.sin();
                    let one = 1.0 - self.es * sinphi * sinphi;
                    let nu = 1.0 / one.sqrt();
                    let xn = nu * cosphi * sinlam;
                    let yn = nu * (sinphi * self.cosph0 - cosphi * self.sinph0 * coslam)
                        + self.es * (self.nu0 * self.sinph0 - nu * sinphi) * self.cosph0;
                    let rho = (1.0 - self.es) * nu / one;
                    let j11 = -rho * sinphi * sinlam;
                    let j12 = nu * cosphi * coslam;
                    let j21 = rho * (cosphi * self.cosph0 + sinphi * self.sinph0 * coslam);
                    let j22 = nu * self.sinph0 * cosphi * sinlam;
                    let det = j11 * j22 - j12 * j21;
                    if det == 0.0 {
                        return Err(TransformError::OutsideDomain);
                    }
                    let dx = x - xn;
                    let dy = y - yn;
                    let dphi = (j22 * dx - j12 * dy) / det;
                    let dlam = (-j21 * dx + j11 * dy) / det;
                    guess.1 += dphi;
                    if guess.1 > FRAC_PI_2 {
                        guess.1 = FRAC_PI_2 - (guess.1 - FRAC_PI_2);
                        guess.0 = adjlon(guess.0 + PI);
                    } else if guess.1 < -FRAC_PI_2 {
                        guess.1 = -FRAC_PI_2 + (-FRAC_PI_2 - guess.1);
                        guess.0 = adjlon(guess.0 + PI);
                    }
                    guess.0 += dlam;
                    if dphi.abs() < 1e-12 && dlam.abs() < 1e-12 {
                        return Ok(guess);
                    }
                }
                Err(TransformError::OutsideDomain)
            }
        }
    }
}

pub(crate) fn build(
    params: &Params,
    frame: &Frame,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let phi0 = frame.phi0;
    let sinph0 = phi0.sin();
    let cosph0 = phi0.cos();
    let mode = if (phi0.abs() - FRAC_PI_2).abs() <= EPS10 {
        if phi0 < 0.0 {
            Mode::South
        } else {
            Mode::North
        }
    } else if phi0.abs() > EPS10 {
        Mode::Oblique
    } else {
        Mode::Equator
    };
    let ellips = frame.earth.es() > 0.0;
    let nu0 = if ellips {
        1.0 / (1.0 - frame.earth.es() * sinph0 * sinph0).sqrt()
    } else {
        1.0
    };
    let alpha = params.radians("alpha")?.unwrap_or(0.0);
    Ok(classic_step(
        frame,
        Ortho {
            mode,
            sinph0,
            cosph0,
            phi0,
            k0: frame.k0,
            sinalpha: alpha.sin(),
            cosalpha: alpha.cos(),
            es: frame.earth.es(),
            one_es: frame.earth.one_es(),
            nu0,
            y_shift: frame.earth.es() * nu0 * sinph0 * cosph0,
            y_scale: 1.0 / (1.0 - frame.earth.es() * cosph0 * cosph0).sqrt(),
            ellips,
        },
    ))
}
