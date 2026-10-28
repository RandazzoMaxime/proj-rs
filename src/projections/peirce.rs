//! Peirce quincuncial.
//!
//! Port of the `peirce_q` path in PROJ 9.9 `src/projections/adams.cpp`
//! (`ell_int_5`, the `PEIRCE_Q` arm of `adams_forward`,
//! `peirce_q_square_inverse`, `peirce_q_diamond_inverse`, `pj_adams_setup`).
//! Guyou and the Adams projections in that file are not built here.
//!
//! PROJ sets `P->es = 0` and leaves `a` alone. The kernel is spherical, in
//! units of that semi-major axis. `adams_forward` never reads `P->k0`.

#![allow(
    clippy::pedantic,
    reason = "coefficients and inverse heuristics are copied from PROJ adams.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::Ellipsoid;
use std::f64::consts::{FRAC_PI_2, PI};

/// Same clamp as PROJ `aasincos.cpp`. Past this, `aacos` / `aasin` set errno.
const ONE_TOL: f64 = 1.000_000_000_000_01;
/// Hemisphere gate in `adams_forward`. Not `EPS10`.
const TOL: f64 = 1e-9;
/// `RSQRT2` in adams.cpp, not a recomputed `1/sqrt(2)`.
const RSQRT2: f64 = 0.707_106_781_186_547_524_400_844_362_0;
/// `M_2_PI` from `proj_internal.h`, the value `ell_int_5` multiplies by.
const M_2_PI: f64 = 0.636_619_772_367_581_343_08;
/// Complete elliptic integral used as the quincuncial shift (`shd` before `* 2`).
const ELLIPTIC_SHIFT: f64 = 1.854_074_677_301_371_9;
/// Trial constants in `peirce_q_square_inverse`. The two axes differ on purpose.
const SQUARE_NEG_Y: f64 = 2.622_057_580_396;
const SQUARE_NEG_X: f64 = 2.622_057_574_224;

const ELL_C0: f64 = 2.191_745_708_310_38;
const ELL_C: [f64; 7] = [
    -8.586_910_036_364_95e-07,
    2.026_921_156_536_89e-07,
    3.129_604_807_653_14e-05,
    5.303_947_399_210_63e-05,
    -0.001_280_464_468_061_3,
    -0.005_755_748_368_302_88,
    0.091_420_303_340_821_1,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Square,
    Diamond,
    North,
    South,
    Horizontal,
    Vertical,
}

enum Start {
    /// `(0, 0)` is the north pole. PROJ returns it before Newton.
    Exact(f64, f64),
    Guess(f64, f64),
}

struct PeirceQ {
    shape: Shape,
    scrollx: f64,
    scrolly: f64,
}

impl Kernel for PeirceQ {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if hemisphere_rejected(self.shape, phi) {
            return Err(TransformError::OutsideDomain);
        }
        let (x, y, outside) = self.project(lam, phi);
        if outside {
            Err(TransformError::OutsideDomain)
        } else {
            Ok((x, y))
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let (lam, phi) = match self.shape {
            Shape::Square => match square_start(x, y) {
                Start::Exact(lam, phi) => return Ok((lam, phi)),
                Start::Guess(lam, phi) => (lam, phi),
            },
            Shape::Diamond => match diamond_start(x, y) {
                Start::Exact(lam, phi) => return Ok((lam, phi)),
                Start::Guess(lam, phi) => (lam, phi),
            },
            Shape::North | Shape::South | Shape::Horizontal | Shape::Vertical => {
                return Err(TransformError::NoInverse);
            }
        };
        // `pj_generic_inverse_2d` calls `P->fwd`, which still returns a finite
        // point after `aacos` clamps. The errno flag is not an abort here.
        generic_inverse(x, y, lam, phi, 1e-10, |lam, phi| {
            let (px, py, _) = self.project(lam, phi);
            (px, py)
        })
    }
}

impl PeirceQ {
    fn project(&self, lam: f64, phi: f64) -> (f64, f64, bool) {
        let (x, y, outside) = conformal(lam, phi);
        let (x, y) = self.arrange(lam, phi, x, y);
        (x, y, outside)
    }

    fn arrange(&self, lam: f64, phi: f64, mut x: f64, mut y: f64) -> (f64, f64) {
        let shd = ELLIPTIC_SHIFT * 2.0;
        if matches!(self.shape, Shape::Square | Shape::Diamond) && phi < 0.0 {
            // Separate tests, not `else if`: the longitude intervals are the
            // same ones adams.cpp writes, including the shared endpoints.
            if lam < -0.75 * PI {
                y = shd - y;
            }
            if lam < -0.25 * PI && lam >= -0.75 * PI {
                x = -shd - x;
            }
            if lam < 0.25 * PI && lam >= -0.25 * PI {
                y = -shd - y;
            }
            if lam < 0.75 * PI && lam >= 0.25 * PI {
                x = shd - x;
            }
            if lam >= 0.75 * PI {
                y = shd - y;
            }
        }
        if self.shape == Shape::Square {
            let temp = x;
            x = RSQRT2 * (x - y);
            y = RSQRT2 * (temp + y);
        }
        if self.shape == Shape::Horizontal {
            if phi < 0.0 {
                x = shd - x;
            }
            x -= shd / 2.0;
        }
        if self.shape == Shape::Vertical {
            if phi < 0.0 {
                y = shd - y;
            }
            y -= shd / 2.0;
        }
        if self.scrollx != 0.0 && self.shape == Shape::Horizontal {
            let xscale = 2.0;
            let xthresh = shd / 2.0;
            x += self.scrollx * (xthresh * 2.0 * xscale);
            if x >= xthresh * xscale {
                x -= shd * xscale;
            } else if x < -(xthresh * xscale) {
                x += shd * xscale;
            }
        }
        if self.scrolly != 0.0 && self.shape == Shape::Vertical {
            let yscale = 2.0;
            let ythresh = shd / 2.0;
            y += self.scrolly * (ythresh * 2.0 * yscale);
            if y >= ythresh * yscale {
                y -= shd * yscale;
            } else if y < -(ythresh * yscale) {
                y += shd * yscale;
            }
        }
        (x, y)
    }
}

fn hemisphere_rejected(shape: Shape, phi: f64) -> bool {
    match shape {
        Shape::North => phi < -TOL,
        Shape::South => phi > -TOL,
        Shape::Square | Shape::Diamond | Shape::Horizontal | Shape::Vertical => false,
    }
}

fn conformal(lam: f64, phi: f64) -> (f64, f64, bool) {
    let sl = lam.sin();
    let cl = lam.cos();
    let cp = phi.cos();
    let (a, bad_a) = aacos(cp * (sl + cl) * RSQRT2);
    let (b, bad_b) = aacos(cp * (sl - cl) * RSQRT2);
    let sm = sl < 0.0;
    let sn = cl > 0.0;
    let (mut m, bad_m) = aasin((1.0 + cpp_min(0.0, (a + b).cos())).sqrt());
    if sm {
        m = -m;
    }
    let (mut n, bad_n) = aasin((1.0 - cpp_max(0.0, (a - b).cos())).abs().sqrt());
    if sn {
        n = -n;
    }
    (ell_int_5(m), ell_int_5(n), bad_a || bad_b || bad_m || bad_n)
}

/// `std::min` / `std::max`: `(b < a) ? b : a`, which is not `f64::min` on NaN.
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

/// Clamp like PROJ `aacos`. The bool is the errno PROJ would set; the angle
/// is still returned and the caller keeps going.
fn aacos(v: f64) -> (f64, bool) {
    let av = v.abs();
    if av >= 1.0 {
        let angle = if v < 0.0 { PI } else { 0.0 };
        (angle, av > ONE_TOL)
    } else {
        (v.acos(), false)
    }
}

fn aasin(v: f64) -> (f64, bool) {
    let av = v.abs();
    if av >= 1.0 {
        let angle = if v < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
        (angle, av > ONE_TOL)
    } else {
        (v.asin(), false)
    }
}

/// Even Chebyshev series for the elliptic integral of the first kind, `k² = 0.5`.
/// Coefficients stay in this order: the loop is Clenshaw from the high term.
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

fn square_start(x: f64, y: f64) -> Start {
    let mut phi = 0.0;
    let lam = if x == 0.0 && y < 0.0 {
        if y.abs() < SQUARE_NEG_Y {
            phi = PI / 4.0;
        }
        -PI / 4.0
    } else if x > 0.0 && y.abs() < 1e-7 {
        PI / 4.0
    } else if x < 0.0 && y.abs() < 1e-7 {
        phi = PI / 2.0 / SQUARE_NEG_X * x + PI / 2.0;
        -3.0 * PI / 4.0
    } else if x.abs() < 1e-7 && y > 0.0 {
        3.0 * PI / 4.0
    } else if x >= 0.0 && y <= 0.0 {
        if x == 0.0 && y == 0.0 {
            return Start::Exact(0.0, PI / 2.0);
        }
        0.0
    } else if x >= 0.0 && y >= 0.0 {
        PI / 2.0
    } else if x <= 0.0 && y >= 0.0 {
        if x.abs() < y.abs() {
            PI * 0.9
        } else {
            -PI * 0.9
        }
    } else {
        -PI / 2.0
    };
    Start::Guess(lam, phi)
}

fn diamond_start(x: f64, y: f64) -> Start {
    let mut phi = 0.0;
    let lam = if x >= 0.0 && y <= 0.0 {
        if x > 0.0 && y == 0.0 {
            PI / 2.0
        } else if x == 0.0 && y == 0.0 {
            return Start::Exact(0.0, PI / 2.0);
        } else if x == 0.0 && y < 0.0 {
            phi = PI / 4.0;
            0.0
        } else {
            PI / 4.0
        }
    } else if x >= 0.0 && y >= 0.0 {
        3.0 * PI / 4.0
    } else if x <= 0.0 && y >= 0.0 {
        -3.0 * PI / 4.0
    } else {
        -PI / 4.0
    };
    if x.abs() > ELLIPTIC_SHIFT + 1e-3 || y.abs() > ELLIPTIC_SHIFT + 1e-3 {
        phi = -PI / 4.0;
    }
    Start::Guess(lam, phi)
}

/// `pj_generic_inverse_2d`: 15 Newton steps, Jacobian step `1e-6`, correction
/// clamped to `±0.3`, longitude to `±π`, latitude to `±π/2`.
fn generic_inverse(
    x: f64,
    y: f64,
    mut lam: f64,
    mut phi: f64,
    tolerance: f64,
    mut forward: impl FnMut(f64, f64) -> (f64, f64),
) -> Result<(f64, f64), TransformError> {
    let mut dlam_x = 0.0;
    let mut dlam_y = 0.0;
    let mut dphi_x = 0.0;
    let mut dphi_y = 0.0;
    for i in 0..15 {
        let (approx_x, approx_y) = forward(lam, phi);
        let dx = approx_x - x;
        let dy = approx_y - y;
        if dx.abs() < tolerance && dy.abs() < tolerance {
            return Ok((lam, phi));
        }
        if i == 0 || dx.abs() > 1e-6 || dy.abs() > 1e-6 {
            let step_lam = if lam > 0.0 { -1e-6 } else { 1e-6 };
            let (x_lam, y_lam) = forward(lam + step_lam, phi);
            let dx_lam = (x_lam - approx_x) / step_lam;
            let dy_lam = (y_lam - approx_y) / step_lam;
            let step_phi = if phi > 0.0 { -1e-6 } else { 1e-6 };
            let (x_phi, y_phi) = forward(lam, phi + step_phi);
            let dx_phi = (x_phi - approx_x) / step_phi;
            let dy_phi = (y_phi - approx_y) / step_phi;
            let det = dx_lam * dy_phi - dx_phi * dy_lam;
            if det != 0.0 {
                dlam_x = dy_phi / det;
                dlam_y = -dx_phi / det;
                dphi_x = -dy_lam / det;
                dphi_y = dx_lam / det;
            }
        }
        lam -= (dx * dlam_x + dy * dlam_y).clamp(-0.3, 0.3);
        lam = lam.clamp(-PI, PI);
        phi -= (dx * dphi_x + dy * dphi_y).clamp(-0.3, 0.3);
        phi = phi.clamp(-FRAC_PI_2, FRAC_PI_2);
    }
    Err(TransformError::OutsideDomain)
}

const KEYS: &[&str] = &["shape", "scrollx", "scrolly"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "peirce_q" {
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
    if name != "peirce_q" {
        return Ok(None);
    }
    if params.flag("shape") && params.text("shape").is_none() {
        return Err(CreateError::IllegalValue("shape"));
    }
    let shape = match params.text("shape") {
        None => Shape::Diamond,
        Some("square") => Shape::Square,
        Some("diamond") => Shape::Diamond,
        Some("nhemisphere") => Shape::North,
        Some("shemisphere") => Shape::South,
        Some("horizontal") => Shape::Horizontal,
        Some("vertical") => Shape::Vertical,
        Some(_) => return Err(CreateError::IllegalValue("shape")),
    };
    let scrollx = if shape == Shape::Horizontal {
        scroll_shift(params, "scrollx")?
    } else {
        0.0
    };
    let scrolly = if shape == Shape::Vertical {
        scroll_shift(params, "scrolly")?
    } else {
        0.0
    };
    let mut frame = frame.clone();
    frame.earth = Ellipsoid::try_from_a_es(frame.earth.a(), 0.0)
        .map_err(|_| CreateError::IllegalValue("a"))?;
    Ok(Some(classic_step(
        &frame,
        PeirceQ {
            shape,
            scrollx,
            scrolly,
        },
    )))
}

fn scroll_shift(params: &Params, key: &'static str) -> Result<f64, CreateError> {
    if !params.flag(key) {
        return Ok(0.0);
    }
    let shift = params.number(key)?.unwrap_or(0.0);
    // `|scroll| == 1` is legal. NaN fails both comparisons and is kept, as in C.
    if shift > 1.0 || shift < -1.0 {
        Err(CreateError::IllegalValue(key))
    } else {
        Ok(shift)
    }
}
