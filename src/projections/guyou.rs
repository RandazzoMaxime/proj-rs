//! Guyou hemisphere-in-a-square.
//!
//! Port of PROJ 9.9 `src/projections/adams.cpp` (`PJ_PROJECTION(guyou)` only).
//! Setup forces a sphere (`P->es = 0`) and keeps the caller's semi-major axis.
//! The kernel does not read `k0`. There is no inverse.

#![allow(clippy::pedantic, reason = "formulae follow PROJ adams.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aacos, aasin};
use std::f64::consts::FRAC_PI_2;

/// `|λ|` may pass the hemisphere by this much before the domain check fires.
const TOL: f64 = 1e-9;
/// PROJ `RSQRT2`, not a recomputed `1/sqrt(2)`.
const RSQRT2: f64 = 0.707_106_781_186_547_524_400_844_362_0;
/// Truncated pole ordinate from `adams.cpp`. Not `ell_int_5(±π/2)`.
const POLE_Y: f64 = 1.85407;
/// PROJ `M_2_PI` (`2/π`) as the literal in `proj_internal.h`.
const M_2_PI: f64 = 0.636_619_772_367_581_343_08;

struct Guyou;

impl Kernel for Guyou {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        // Same order as `adams_forward`: a longitude outside the hemisphere
        // is rejected even when the latitude is a pole.
        if (lam.abs() - TOL) > FRAC_PI_2 {
            return Err(TransformError::OutsideDomain);
        }
        if (phi.abs() - FRAC_PI_2).abs() < TOL {
            let y = if phi < 0.0 { -POLE_Y } else { POLE_Y };
            return Ok((0.0, y));
        }
        let sl = lam.sin();
        let sp = phi.sin();
        let cp = phi.cos();
        let a = acos_proj((cp * sl - sp) * RSQRT2)?;
        let b = acos_proj((cp * sl + sp) * RSQRT2)?;
        let sm = lam < 0.0;
        let sn = phi < 0.0;

        // `std::min(0, cos(a+b))`: the cosine wins only when it is strictly smaller.
        let cos_sum = (a + b).cos();
        let min_cos = if cos_sum < 0.0 { cos_sum } else { 0.0 };
        let mut m = asin_proj((1.0 + min_cos).sqrt())?;
        if sm {
            m = -m;
        }

        // `std::max(0, cos(a-b))`: the cosine wins only when it is strictly larger.
        let cos_diff = (a - b).cos();
        let max_cos = if 0.0 < cos_diff { cos_diff } else { 0.0 };
        let mut n = asin_proj((1.0 - max_cos).abs().sqrt())?;
        if sn {
            n = -n;
        }

        Ok((ell_int_5(m), ell_int_5(n)))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

fn acos_proj(value: f64) -> Result<f64, TransformError> {
    aacos(value).map_err(|()| TransformError::OutsideDomain)
}

fn asin_proj(value: f64) -> Result<f64, TransformError> {
    aasin(value).map_err(|()| TransformError::OutsideDomain)
}

/// Elliptic integral of the first kind at `k² = 0.5`.
///
/// Even Chebyshev series from `ell_int_5`. Clenshaw stores the constant term
/// so the tail adds `0.5 * C0`, then the whole sum is multiplied by `phi`.
fn ell_int_5(phi: f64) -> f64 {
    const C0: f64 = 2.191_745_708_310_38;
    const C: [f64; 7] = [
        -8.586_910_036_364_95e-07,
        2.026_921_156_536_89e-07,
        3.129_604_807_653_14e-05,
        5.303_947_399_210_63e-05,
        -0.001_280_464_468_061_3,
        -0.005_755_748_368_302_88,
        0.091_420_303_340_821_1,
    ];
    let y = phi * M_2_PI;
    let y = 2.0 * y * y - 1.0;
    let y2 = 2.0 * y;
    let mut d1 = 0.0;
    let mut d2 = 0.0;
    for coefficient in C {
        let temp = d1;
        d1 = y2 * d1 - d2 + coefficient;
        d2 = temp;
    }
    phi * (y * d1 - d2 + 0.5 * C0)
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "guyou" {
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
    if name == "guyou" {
        Ok(Some(classic_step(frame, Guyou)))
    } else {
        Ok(None)
    }
}
