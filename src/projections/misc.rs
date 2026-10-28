//! Small spherical projections with no extra parameters.
//!
//! Each kernel follows the named function in PROJ 9.9 `src/projections/`.

#![allow(clippy::pedantic, reason = "formulae follow the PROJ sources")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::aasin;
use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

fn no_inverse() -> Result<(f64, f64), TransformError> {
    Err(TransformError::NoInverse)
}

fn asin_or(v: f64) -> Result<f64, TransformError> {
    aasin(v).map_err(|()| TransformError::OutsideDomain)
}

struct August;
struct Boggs;
struct Denoy;
struct Larr;
struct Lask;
struct Nell;
struct Putp2;
struct Wag7;

impl Kernel for August {
    fn forward(&self, mut lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const M: f64 = 1.333_333_333_333_333;
        let t = (0.5 * phi).tan();
        let c1 = (1.0 - t * t).sqrt();
        lam *= 0.5;
        let c = 1.0 + c1 * lam.cos();
        let x1 = lam.sin() * c1 / c;
        let y1 = t / c;
        let x12 = x1 * x1;
        let y12 = y1 * y1;
        Ok((
            M * x1 * (3.0 + x12 - 3.0 * y12),
            M * y1 * (3.0 + 3.0 * x12 - y12),
        ))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

impl Kernel for Boggs {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const EPS: f64 = 1e-7;
        const FXC: f64 = 2.00276;
        const FXC2: f64 = 1.11072;
        const FYC: f64 = 0.49931;
        let mut theta = phi;
        let x = if (phi.abs() - FRAC_PI_2).abs() < EPS {
            0.0
        } else {
            let c = theta.sin() * PI;
            for _ in 0..20 {
                let step = (theta + theta.sin() - c) / (1.0 + theta.cos());
                theta -= step;
                if step.abs() < EPS {
                    break;
                }
            }
            theta *= 0.5;
            FXC * lam / (1.0 / phi.cos() + FXC2 / theta.cos())
        };
        Ok((x, FYC * (phi + SQRT_2 * theta.sin())))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

impl Kernel for Denoy {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const C0: f64 = 0.95;
        const C1: f64 = -0.083_333_333_333_333_333_33;
        const C3: f64 = 0.001_666_666_666_666_666_66;
        const D1: f64 = 0.9;
        const D5: f64 = 0.03;
        let abs_lam = lam.abs();
        let factor = (C0 + abs_lam * (C1 + abs_lam * abs_lam * C3))
            * (phi * (D1 + D5 * phi * phi * phi * phi));
        Ok((lam * factor.cos(), phi))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

impl Kernel for Larr {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const SIXTH: f64 = 0.166_666_666_666_666_66;
        Ok((
            0.5 * lam * (1.0 + phi.cos().sqrt()),
            phi / ((0.5 * phi).cos() * (SIXTH * lam).cos()),
        ))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

impl Kernel for Lask {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let l2 = lam * lam;
        let p2 = phi * phi;
        let x = lam * (0.975_534 + p2 * (-0.119_161 + l2 * -0.014_305_9 + p2 * -0.054_700_9));
        let y = phi
            * (1.003_84
                + l2 * (0.080_289_4 + p2 * -0.028_550_0 + l2 * 0.000_199_025)
                + p2 * (0.099_890_9 + p2 * -0.049_103_2));
        Ok((x, y))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

impl Kernel for Nell {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let k = 2.0 * phi.sin();
        let p2 = phi * phi;
        phi *= 1.003_71 + p2 * (-0.093_538_2 + p2 * -0.011_412);
        for _ in 0..10 {
            let step = (phi + phi.sin() - k) / (1.0 + phi.cos());
            phi -= step;
            if step.abs() < 1e-7 {
                break;
            }
        }
        Ok((0.5 * lam * (1.0 + phi.cos()), phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        Ok((2.0 * x / (1.0 + y.cos()), asin_or(0.5 * (y + y.sin()))?))
    }
}

impl Kernel for Putp2 {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 1.894_90;
        const C_Y: f64 = 1.718_48;
        const C_P: f64 = 0.614_184_849_304_378_4;
        const PI_DIV_3: f64 = 1.047_197_551_196_597_7;
        let p = C_P * phi.sin();
        let p2 = phi * phi;
        phi *= 0.615_709 + p2 * (0.009_099_53 + p2 * 0.004_629_2);
        let mut converged = false;
        for _ in 0..10 {
            let c = phi.cos();
            let s = phi.sin();
            let step = (phi + s * (c - 1.0) - p) / (1.0 + c * (c - 1.0) - s * s);
            phi -= step;
            if step.abs() < 1e-10 {
                converged = true;
                break;
            }
        }
        if !converged {
            phi = if phi < 0.0 { -PI_DIV_3 } else { PI_DIV_3 };
        }
        Ok((C_X * lam * (phi.cos() - 0.5), C_Y * phi.sin()))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 1.894_90;
        const C_Y: f64 = 1.718_48;
        const C_P: f64 = 0.614_184_849_304_378_4;
        let phi = asin_or(y / C_Y)?;
        let c = phi.cos();
        let lam = x / (C_X * (c - 0.5));
        Ok((lam, asin_or((phi + phi.sin() * (c - 1.0)) / C_P)?))
    }
}

impl Kernel for Wag7 {
    fn forward(&self, mut lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut y = 0.906_307_787_036_649_96 * phi.sin();
        let theta = y.asin();
        let ct = theta.cos();
        lam /= 3.0;
        let mut x = 2.667_23 * ct * lam.sin();
        let d = 1.0 / (0.5 * (1.0 + ct * lam.cos())).sqrt();
        y *= 1.241_04 * d;
        x *= d;
        Ok((x, y))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        no_inverse()
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "august" | "boggs" | "denoy" | "larr" | "lask" | "nell" | "putp2" | "wag7" => Some(&[]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let step = match name {
        "august" => classic_step(frame, August),
        "boggs" => classic_step(frame, Boggs),
        "denoy" => classic_step(frame, Denoy),
        "larr" => classic_step(frame, Larr),
        "lask" => classic_step(frame, Lask),
        "nell" => classic_step(frame, Nell),
        "putp2" => classic_step(frame, Putp2),
        "wag7" => classic_step(frame, Wag7),
        _ => return Ok(None),
    };
    Ok(Some(step))
}
