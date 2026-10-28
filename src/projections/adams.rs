//! Adams hemisphere-in-a-square and the two world-in-a-square projections.
//!
//! Port of PROJ 9.9 `src/projections/adams.cpp` for `adams_hemi`, `adams_ws1`
//! and `adams_ws2` only. Guyou and Peirce quincuncial share that file and are
//! not built here. Setup forces a sphere (`P->es = 0`) and keeps the caller's
//! semi-major axis; the classic wrapper multiplies by it. The forward never
//! reads `k0`.

#![allow(
    clippy::pedantic,
    reason = "coefficients, clamps and the Newton inverse follow PROJ adams.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

/// `|lam|` margin in `adams_hemi`, `TOL` in `adams.cpp`.
const TOL: f64 = 1e-9;
/// `RSQRT2` in `adams.cpp`, truncated on purpose (not `1/sqrt(2)` from libm).
const RSQRT2: f64 = 0.707_106_781_186_547_524_400_844_362_0;
/// `M_2_PI` from PROJ `proj_internal.h`, not `2/π`.
const M_2_PI: f64 = 0.636_619_772_367_581_343_08;
/// `ONE_TOL` in `aasincos.cpp`. Past this, `aasin` / `aacos` set the domain errno.
const ONE_TOL: f64 = 1.000_000_000_000_01;
/// Northing of the north pole for `adams_ws2` at `R = 1` (`adams_inverse`).
const WS2_PHI_SCALE: f64 = 2.621_813_47;
/// Easting of longitude 180° on the equator for `adams_ws2` at `R = 1`.
const WS2_LAM_SCALE: f64 = 2.622_057_60;
/// `deltaXYTolerance` passed to `pj_generic_inverse_2d` for `adams_ws2`.
const INV_TOL: f64 = 1e-10;

const ELL_C0: f64 = 2.191_745_708_310_38;
const ELL_C: [f64; 7] = [
    -8.586_910_036_364_95e-7,
    2.026_921_156_536_89e-7,
    3.129_604_807_653_14e-5,
    5.303_947_399_210_63e-5,
    -0.001_280_464_468_061_3,
    -0.005_755_748_368_302_88,
    0.091_420_303_340_821_1,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Hemi,
    Ws1,
    Ws2,
}

struct Adams {
    mode: Mode,
}

/// Clamped arc-sine or arc-cosine, plus the PROJ domain flag.
///
/// `aasin` / `aacos` still return a finite angle when `|v| > 1`. The flag is
/// the errno `pj_fwd` turns into a failed transform. `pj_generic_inverse_2d`
/// calls `P->fwd` directly, so the inverse must keep the angle and remember
/// the flag: a later successful Newton step does not clear it.
struct Clamped {
    value: f64,
    outside: bool,
}

fn aasin(v: f64) -> Clamped {
    let magnitude = v.abs();
    if magnitude >= 1.0 {
        Clamped {
            value: if v < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 },
            outside: magnitude > ONE_TOL,
        }
    } else {
        Clamped {
            value: v.asin(),
            outside: false,
        }
    }
}

fn aacos(v: f64) -> Clamped {
    let magnitude = v.abs();
    if magnitude >= 1.0 {
        Clamped {
            value: if v < 0.0 { PI } else { 0.0 },
            outside: magnitude > ONE_TOL,
        }
    } else {
        Clamped {
            value: v.acos(),
            outside: false,
        }
    }
}

fn take(angle: Clamped, outside: &mut bool) -> f64 {
    *outside |= angle.outside;
    angle.value
}

/// `std::min` / `std::max` from the C++ standard library, not `f64::min`.
fn cpp_min(a: f64, b: f64) -> f64 {
    if b < a {
        b
    } else {
        a
    }
}

fn cpp_max(a: f64, b: f64) -> f64 {
    if a < b {
        b
    } else {
        a
    }
}

/// Even Chebyshev approximation of the incomplete elliptic integral of the
/// first kind at `k² = 1/2`. Coefficients and Clenshaw order are `ell_int_5`.
fn ell_int_5(phi: f64) -> f64 {
    let mut y = phi * M_2_PI;
    y = 2.0 * y * y - 1.0;
    let y2 = 2.0 * y;
    let mut d1 = 0.0;
    let mut d2 = 0.0;
    for coefficient in ELL_C {
        let temp = d1;
        d1 = y2 * d1 - d2 + coefficient;
        d2 = temp;
    }
    phi * (y * d1 - d2 + 0.5 * ELL_C0)
}

struct Raw {
    x: f64,
    y: f64,
    outside: bool,
}

fn adams_forward(mode: Mode, lam: f64, phi: f64) -> Result<Raw, TransformError> {
    let mut outside = false;
    let (a, b, negate_m, negate_n) = match mode {
        Mode::Hemi => {
            if (lam.abs() - TOL) > FRAC_PI_2 {
                return Err(TransformError::OutsideDomain);
            }
            let sin_phi = phi.sin();
            let mut a = phi.cos() * lam.sin();
            let negate_m = (sin_phi + a) < 0.0;
            let negate_n = (sin_phi - a) < 0.0;
            a = take(aacos(a), &mut outside);
            (a, FRAC_PI_2 - phi, negate_m, negate_n)
        }
        Mode::Ws1 => {
            let sp = (0.5 * phi).tan();
            let mut b = take(aasin(sp), &mut outside).cos() * (0.5 * lam).sin();
            let a = take(aacos((b - sp) * RSQRT2), &mut outside);
            b = take(aacos((b + sp) * RSQRT2), &mut outside);
            (a, b, lam < 0.0, phi < 0.0)
        }
        Mode::Ws2 => {
            let spp = (0.5 * phi).tan();
            let mut a = take(aasin(spp), &mut outside).cos() * (0.5 * lam).sin();
            let negate_m = (spp + a) < 0.0;
            let negate_n = (spp - a) < 0.0;
            let b = take(aacos(spp), &mut outside);
            a = take(aacos(a), &mut outside);
            (a, b, negate_m, negate_n)
        }
    };

    let mut m = take(
        aasin((1.0 + cpp_min(0.0, (a + b).cos())).sqrt()),
        &mut outside,
    );
    if negate_m {
        m = -m;
    }
    let mut n = take(
        aasin((1.0 - cpp_max(0.0, (a - b).cos())).abs().sqrt()),
        &mut outside,
    );
    if negate_n {
        n = -n;
    }

    let mut x = ell_int_5(m);
    let mut y = ell_int_5(n);
    if matches!(mode, Mode::Hemi | Mode::Ws2) {
        let temp = x;
        x = RSQRT2 * (x - y);
        y = RSQRT2 * (temp + y);
    }
    Ok(Raw { x, y, outside })
}

fn ws2_inverse(x: f64, y: f64) -> Result<(f64, f64), TransformError> {
    let mut phi = cpp_max(cpp_min(y / WS2_PHI_SCALE, 1.0), -1.0) * FRAC_PI_2;
    let mut lam = if phi.abs() >= FRAC_PI_2 {
        0.0
    } else {
        cpp_max(cpp_min(x / WS2_LAM_SCALE / phi.cos(), 1.0), -1.0) * PI
    };
    let mut poisoned = false;
    let mut dlam_x = 0.0;
    let mut dlam_y = 0.0;
    let mut dphi_x = 0.0;
    let mut dphi_y = 0.0;
    for iteration in 0..15 {
        let approx = adams_forward(Mode::Ws2, lam, phi)?;
        poisoned |= approx.outside;
        let dx = approx.x - x;
        let dy = approx.y - y;
        if dx.abs() < INV_TOL && dy.abs() < INV_TOL {
            return if poisoned {
                Err(TransformError::OutsideDomain)
            } else {
                Ok((lam, phi))
            };
        }
        if iteration == 0 || dx.abs() > 1e-6 || dy.abs() > 1e-6 {
            let step_lam = if lam > 0.0 { -1e-6 } else { 1e-6 };
            let at_lam = adams_forward(Mode::Ws2, lam + step_lam, phi)?;
            poisoned |= at_lam.outside;
            let dx_lam = (at_lam.x - approx.x) / step_lam;
            let dy_lam = (at_lam.y - approx.y) / step_lam;
            let step_phi = if phi > 0.0 { -1e-6 } else { 1e-6 };
            let at_phi = adams_forward(Mode::Ws2, lam, phi + step_phi)?;
            poisoned |= at_phi.outside;
            let dx_phi = (at_phi.x - approx.x) / step_phi;
            let dy_phi = (at_phi.y - approx.y) / step_phi;
            let det = dx_lam * dy_phi - dx_phi * dy_lam;
            if det != 0.0 {
                dlam_x = dy_phi / det;
                dlam_y = -dx_phi / det;
                dphi_x = -dy_lam / det;
                dphi_y = dx_lam / det;
            }
        }
        lam -= cpp_max(cpp_min(dx * dlam_x + dy * dlam_y, 0.3), -0.3);
        if lam < -PI {
            lam = -PI;
        } else if lam > PI {
            lam = PI;
        }
        phi -= cpp_max(cpp_min(dx * dphi_x + dy * dphi_y, 0.3), -0.3);
        if phi < -FRAC_PI_2 {
            phi = -FRAC_PI_2;
        } else if phi > FRAC_PI_2 {
            phi = FRAC_PI_2;
        }
    }
    Err(TransformError::OutsideDomain)
}

impl Kernel for Adams {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let raw = adams_forward(self.mode, lam, phi)?;
        if raw.outside {
            Err(TransformError::OutsideDomain)
        } else {
            Ok((raw.x, raw.y))
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if self.mode == Mode::Ws2 {
            ws2_inverse(x, y)
        } else {
            Err(TransformError::NoInverse)
        }
    }
}

const NO_KEYS: &[&str] = &[];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if matches!(name, "adams_hemi" | "adams_ws1" | "adams_ws2") {
        Some(NO_KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let mode = match name {
        "adams_hemi" => Mode::Hemi,
        "adams_ws1" => Mode::Ws1,
        "adams_ws2" => Mode::Ws2,
        _ => return Ok(None),
    };
    Ok(Some(classic_step(frame, Adams { mode })))
}
