//! Lambert azimuthal equal area.
//!
//! Port of PROJ 9.9 `src/projections/laea.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ laea.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_forward, authalic_inverse, authalic_q};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const EPS10: f64 = 1e-10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct Laea {
    mode: Mode,
    sinb1: f64,
    cosb1: f64,
    xmf: f64,
    ymf: f64,
    qp: f64,
    dd: f64,
    rq: f64,
    phi0: f64,
    ellips: bool,
    n: f64,
    e: f64,
    es: f64,
    one_es: f64,
}

impl Kernel for Laea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if self.ellips {
            self.forward_ellipsoid(lam, phi)
        } else {
            self.forward_sphere(lam, phi)
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if self.ellips {
            self.inverse_ellipsoid(x, y)
        } else {
            self.inverse_sphere(x, y)
        }
    }
}

impl Laea {
    fn beta(&self, phi: f64) -> f64 {
        authalic_forward(phi, phi.sin(), self.n, self.e, self.one_es, self.qp)
    }

    fn forward_sphere(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let mut coslam = lam.cos();
        match self.mode {
            Mode::Equator | Mode::Oblique => {
                let y = if self.mode == Mode::Equator {
                    1.0 + cosphi * coslam
                } else {
                    1.0 + self.sinb1 * sinphi + self.cosb1 * cosphi * coslam
                };
                if y <= EPS10 {
                    return Err(TransformError::OutsideDomain);
                }
                let y = (2.0 / y).sqrt();
                let x = y * cosphi * lam.sin();
                let y = y * if self.mode == Mode::Equator {
                    sinphi
                } else {
                    self.cosb1 * sinphi - self.sinb1 * cosphi * coslam
                };
                Ok((x, y))
            }
            Mode::North | Mode::South => {
                if self.mode == Mode::North {
                    coslam = -coslam;
                }
                if (phi + self.phi0).abs() < EPS10 {
                    return Err(TransformError::OutsideDomain);
                }
                let reduced = FRAC_PI_4 - phi * 0.5;
                let y = 2.0
                    * if self.mode == Mode::South {
                        reduced.cos()
                    } else {
                        reduced.sin()
                    };
                Ok((y * lam.sin(), y * coslam))
            }
        }
    }

    fn forward_ellipsoid(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let coslam = lam.cos();
        let sinlam = lam.sin();
        let xi = self.beta(phi);
        let mut q = xi.sin() * self.qp;
        let (sinb, cosb) = if matches!(self.mode, Mode::Oblique | Mode::Equator) {
            (xi.sin(), xi.cos())
        } else {
            (0.0, 0.0)
        };
        let mut b = match self.mode {
            Mode::Oblique => 1.0 + self.sinb1 * sinb + self.cosb1 * cosb * coslam,
            Mode::Equator => 1.0 + cosb * coslam,
            Mode::North => {
                q = self.qp - q;
                FRAC_PI_2 + phi
            }
            Mode::South => {
                q = self.qp + q;
                phi - FRAC_PI_2
            }
        };
        if b.abs() < EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        match self.mode {
            Mode::Oblique => {
                b = (2.0 / b).sqrt();
                let y = self.ymf * b * (self.cosb1 * sinb - self.sinb1 * cosb * coslam);
                Ok((self.xmf * b * cosb * sinlam, y))
            }
            Mode::Equator => {
                b = (2.0 / (1.0 + cosb * coslam)).sqrt();
                Ok((self.xmf * b * cosb * sinlam, b * sinb * self.ymf))
            }
            Mode::North | Mode::South => {
                if q >= 1e-15 {
                    b = q.sqrt();
                    let y = coslam * if self.mode == Mode::South { b } else { -b };
                    Ok((b * sinlam, y))
                } else {
                    Ok((0.0, 0.0))
                }
            }
        }
    }

    fn inverse_sphere(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let rh = x.hypot(y);
        if rh * 0.5 > 1.0 {
            return Err(TransformError::OutsideDomain);
        }
        let mut phi = 2.0 * (rh * 0.5).asin();
        let (sinz, cosz) = if matches!(self.mode, Mode::Oblique | Mode::Equator) {
            (phi.sin(), phi.cos())
        } else {
            (0.0, 0.0)
        };
        match self.mode {
            Mode::Equator => {
                phi = if rh.abs() <= EPS10 {
                    0.0
                } else {
                    (y * sinz / rh).asin()
                };
                x *= sinz;
                y = cosz * rh;
            }
            Mode::Oblique => {
                phi = if rh.abs() <= EPS10 {
                    self.phi0
                } else {
                    (cosz * self.sinb1 + y * sinz * self.cosb1 / rh).asin()
                };
                x *= sinz * self.cosb1;
                y = (cosz - phi.sin() * self.sinb1) * rh;
            }
            Mode::North => {
                y = -y;
                phi = FRAC_PI_2 - phi;
            }
            Mode::South => phi -= FRAC_PI_2,
        }
        let lam = if y == 0.0 && matches!(self.mode, Mode::Equator | Mode::Oblique) {
            0.0
        } else {
            x.atan2(y)
        };
        Ok((lam, phi))
    }

    fn inverse_ellipsoid(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let ab = match self.mode {
            Mode::Equator | Mode::Oblique => {
                x /= self.dd;
                y *= self.dd;
                let rho = x.hypot(y);
                if rho < EPS10 {
                    return Ok((0.0, self.phi0));
                }
                let argument = 0.5 * rho / self.rq;
                if argument > 1.0 {
                    return Err(TransformError::OutsideDomain);
                }
                let sce = 2.0 * argument.asin();
                let cce = sce.cos();
                let sce = sce.sin();
                x *= sce;
                if self.mode == Mode::Oblique {
                    let ab = cce * self.sinb1 + y * sce * self.cosb1 / rho;
                    y = rho * self.cosb1 * cce - y * self.sinb1 * sce;
                    ab
                } else {
                    let ab = y * sce / rho;
                    y = rho * cce;
                    ab
                }
            }
            Mode::North | Mode::South => {
                if self.mode == Mode::North {
                    y = -y;
                }
                let q = x * x + y * y;
                if q == 0.0 {
                    return Ok((0.0, self.phi0));
                }
                let mut ab = 1.0 - q / self.qp;
                if self.mode == Mode::South {
                    ab = -ab;
                }
                ab
            }
        };
        let beta = if ab.abs() >= 1.0 {
            if ab > 0.0 {
                FRAC_PI_2
            } else {
                -FRAC_PI_2
            }
        } else {
            ab.asin()
        };
        let phi = authalic_inverse(beta, self.n, self.e, self.es, self.one_es, self.qp);
        Ok((x.atan2(y), phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "laea" {
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
    if name != "laea" {
        return Ok(None);
    }
    let t = frame.phi0.abs();
    let mode = if (t - FRAC_PI_2).abs() < EPS10 {
        if frame.phi0 < 0.0 {
            Mode::South
        } else {
            Mode::North
        }
    } else if t < EPS10 {
        Mode::Equator
    } else {
        Mode::Oblique
    };
    let ellips = frame.earth.es() != 0.0;
    let (sinb1, cosb1, xmf, ymf, qp, dd, rq) = if ellips {
        let e = frame.earth.e();
        let qp = authalic_q(1.0, e, frame.earth.one_es());
        match mode {
            Mode::North | Mode::South => (0.0, 0.0, 0.0, 0.0, qp, 1.0, 0.0),
            Mode::Equator => {
                let rq = (0.5 * qp).sqrt();
                (0.0, 0.0, 1.0, 0.5 * qp, qp, 1.0 / rq, rq)
            }
            Mode::Oblique => {
                let rq = (0.5 * qp).sqrt();
                let sinphi = frame.phi0.sin();
                let b1 = authalic_forward(
                    frame.phi0,
                    sinphi,
                    frame.earth.n(),
                    e,
                    frame.earth.one_es(),
                    qp,
                );
                let sinb1 = b1.sin();
                let cosb1 = b1.cos();
                let dd = frame.phi0.cos()
                    / ((1.0 - frame.earth.es() * sinphi * sinphi).sqrt() * rq * cosb1);
                let xmf = rq * dd;
                let ymf = rq / dd;
                (sinb1, cosb1, xmf, ymf, qp, dd, rq)
            }
        }
    } else {
        let (sinb1, cosb1) = if mode == Mode::Oblique {
            (frame.phi0.sin(), frame.phi0.cos())
        } else {
            (0.0, 0.0)
        };
        (sinb1, cosb1, 0.0, 0.0, 0.0, 0.0, 0.0)
    };
    Ok(Some(classic_step(
        frame,
        Laea {
            mode,
            sinb1,
            cosb1,
            xmf,
            ymf,
            qp,
            dd,
            rq,
            phi0: frame.phi0,
            ellips,
            n: frame.earth.n(),
            e: frame.earth.e(),
            es: frame.earth.es(),
            one_es: frame.earth.one_es(),
        },
    )))
}
