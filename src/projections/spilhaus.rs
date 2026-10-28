//! Spilhaus, an oblique Adams World in a Square II.
//!
//! Port of PROJ 9.9 `src/projections/spilhaus.cpp`. The square is
//! `adams_forward` / `adams_inverse` for `ADAMS_WS2` in
//! `src/projections/adams.cpp` (`ell_int_5` and `pj_generic_inverse_2d`).

#![allow(
    clippy::pedantic,
    reason = "formulae follow PROJ spilhaus.cpp and adams.cpp"
)]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aacos, aasin, aatan2, msfn, sinhpsi2tanphi};
use std::f64::consts::{FRAC_2_PI, FRAC_PI_2, PI};

/// `+lon_0` when the parameter is omitted. PROJ writes this into `P->lam0`.
const DEFAULT_LON0_DEG: f64 = 66.94970198;
/// `+lat_0` when the parameter is omitted.
const DEFAULT_LAT0_DEG: f64 = -49.56371678;
/// `+azi` when the parameter is omitted.
const DEFAULT_AZI_DEG: f64 = 40.17823482;
/// `+rot` when the parameter is omitted.
const DEFAULT_ROT_DEG: f64 = 45.0;

/// `RSQRT2` in `adams.cpp`. The 45° rotation of Adams WS2 uses this literal.
const RSQRT2: f64 = 0.707_106_781_186_547_6;

/// Newton tolerance of `adams_inverse`, in units of the semi-major axis.
const ADAMS_INV_TOL: f64 = 1e-10;

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

struct Spilhaus {
    cosalpha: f64,
    sinalpha: f64,
    beta: f64,
    lambda_0: f64,
    conformal_distortion: f64,
    cosrot: f64,
    sinrot: f64,
    e: f64,
    k0: f64,
    /// Central meridian PROJ stored in `P->lam0` because `+lon_0` was absent.
    /// The frame has already subtracted an explicit `+lon_0`.
    default_lam0: Option<f64>,
    over: bool,
}

impl Kernel for Spilhaus {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.reduced_lon(lam);
        let phi_c = conformal_lat(phi, self.e);
        let cosphi_c = phi_c.cos();
        let sinphi_c = phi_c.sin();
        let coslam = (lam - self.lambda_0).cos();
        let sinlam = (lam - self.lambda_0).sin();
        let phi_a = domain_err(aasin(
            self.sinalpha * sinphi_c - self.cosalpha * cosphi_c * coslam,
        ))?;
        // Raw `atan2`, not `aatan2`: a signed zero must stay on the branch cut.
        let mut lam_a = self.beta
            + (cosphi_c * sinlam)
                .atan2(self.sinalpha * cosphi_c * coslam + self.cosalpha * sinphi_c);
        while lam_a > PI {
            lam_a -= PI * 2.0;
        }
        while lam_a < -PI {
            lam_a += PI * 2.0;
        }
        let (xa, ya) = adams_ws2_forward(lam_a, phi_a)?;
        let factor = self.conformal_distortion * self.k0;
        let x = -(xa * self.cosrot + ya * self.sinrot) * factor;
        let y = -(xa * -self.sinrot + ya * self.cosrot) * factor;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let factor = 1.0 / (self.conformal_distortion * self.k0);
        let xa = -(x * self.cosrot + y * -self.sinrot) * factor;
        let ya = -(x * self.sinrot + y * self.cosrot) * factor;
        let (lam_s, phi_s) = adams_ws2_inverse(xa, ya)?;
        let cosphi_s = phi_s.cos();
        let sinphi_s = phi_s.sin();
        let coslam_s = (lam_s - self.beta).cos();
        let sinlam_s = (lam_s - self.beta).sin();
        let phi_c = domain_err(aasin(
            self.sinalpha * sinphi_s + self.cosalpha * cosphi_s * coslam_s,
        ))?;
        let lam = self.lambda_0
            + aatan2(
                cosphi_s * sinlam_s,
                self.sinalpha * cosphi_s * coslam_s - self.cosalpha * sinphi_s,
            );
        let phi = conformal_lat_inverse(phi_c, self.e)?;
        Ok((self.restore_lon(lam), phi))
    }
}

impl Spilhaus {
    fn reduced_lon(&self, lam: f64) -> f64 {
        let Some(lam0) = self.default_lam0 else {
            return lam;
        };
        let shifted = lam - lam0;
        if self.over {
            shifted
        } else {
            adjlon(shifted)
        }
    }

    fn restore_lon(&self, lam: f64) -> f64 {
        match self.default_lam0 {
            Some(lam0) => lam + lam0,
            None => lam,
        }
    }
}

/// Closed form from `pj_conformal_lat` (`latitudes.cpp`), not the order-6 series.
fn conformal_lat(phi: f64, e: f64) -> f64 {
    if e == 0.0 {
        return phi;
    }
    let sphi = phi.sin();
    let cphi = phi.cos();
    ((sphi / cphi).asinh() - e * (e * sphi).atanh())
        .sinh()
        .atan()
}

fn conformal_lat_inverse(chi: f64, e: f64) -> Result<f64, TransformError> {
    if e == 0.0 {
        return Ok(chi);
    }
    let tau = sinhpsi2tanphi(chi.tan(), e).map_err(|()| TransformError::OutsideDomain)?;
    Ok(tau.atan())
}

/// Even Chebyshev approximation of the incomplete elliptic integral, `k² = 0.5`.
fn ell_int_5(phi: f64) -> f64 {
    let mut y = phi * FRAC_2_PI;
    y = 2.0 * y * y - 1.0;
    let y2 = 2.0 * y;
    let mut d1 = 0.0;
    let mut d2 = 0.0;
    for coeff in ELL_C {
        let temp = d1;
        d1 = y2 * d1 - d2 + coeff;
        d2 = temp;
    }
    phi * (y * d1 - d2 + 0.5 * ELL_C0)
}

fn domain_err<T>(result: Result<T, ()>) -> Result<T, TransformError> {
    result.map_err(|()| TransformError::OutsideDomain)
}

fn adams_ws2_forward(lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
    let spp = (0.5 * phi).tan();
    let a_lin = domain_err(aasin(spp))?.cos() * (0.5 * lam).sin();
    let sm = (spp + a_lin) < 0.0;
    let sn = (spp - a_lin) < 0.0;
    let b = domain_err(aacos(spp))?;
    let a = domain_err(aacos(a_lin))?;
    let mut m = domain_err(aasin((1.0 + (a + b).cos().min(0.0)).sqrt()))?;
    if sm {
        m = -m;
    }
    let mut n = domain_err(aasin((1.0 - (a - b).cos().max(0.0)).abs().sqrt()))?;
    if sn {
        n = -n;
    }
    let x = ell_int_5(m);
    let y = ell_int_5(n);
    Ok((RSQRT2 * (x - y), RSQRT2 * (x + y)))
}

fn adams_ws2_inverse(x: f64, y: f64) -> Result<(f64, f64), TransformError> {
    // Unit-sphere images of (0°, 90°) and (180°, 0°), from the C++ comments.
    let phi = (y / 2.621_813_47).clamp(-1.0, 1.0) * FRAC_PI_2;
    let lam = if phi.abs() >= FRAC_PI_2 {
        0.0
    } else {
        (x / 2.622_057_60 / phi.cos()).clamp(-1.0, 1.0) * PI
    };
    generic_inverse(x, y, lam, phi, ADAMS_INV_TOL, adams_ws2_forward)
}

/// `pj_generic_inverse_2d`: 15 iterations, finite-difference step `1e-6`,
/// correction clamped to ±0.3 rad.
fn generic_inverse(
    x: f64,
    y: f64,
    mut lam: f64,
    mut phi: f64,
    tolerance: f64,
    mut forward: impl FnMut(f64, f64) -> Result<(f64, f64), TransformError>,
) -> Result<(f64, f64), TransformError> {
    let mut dlam_x = 0.0;
    let mut dlam_y = 0.0;
    let mut dphi_x = 0.0;
    let mut dphi_y = 0.0;
    for i in 0..15 {
        let (approx_x, approx_y) = forward(lam, phi)?;
        let dx = approx_x - x;
        let dy = approx_y - y;
        if dx.abs() < tolerance && dy.abs() < tolerance {
            return Ok((lam, phi));
        }
        if i == 0 || dx.abs() > 1e-6 || dy.abs() > 1e-6 {
            let step_lam = if lam > 0.0 { -1e-6 } else { 1e-6 };
            let (x_lam, y_lam) = forward(lam + step_lam, phi)?;
            let dx_lam = (x_lam - approx_x) / step_lam;
            let dy_lam = (y_lam - approx_y) / step_lam;
            let step_phi = if phi > 0.0 { -1e-6 } else { 1e-6 };
            let (x_phi, y_phi) = forward(lam, phi + step_phi)?;
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

const KEYS: &[&str] = &["azi", "rot"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "spilhaus" {
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
    if name != "spilhaus" {
        return Ok(None);
    }
    let phi0 = if params.flag("lat_0") {
        frame.phi0
    } else {
        DEFAULT_LAT0_DEG.to_radians()
    };
    let azimuth = params
        .radians("azi")?
        .unwrap_or(DEFAULT_AZI_DEG.to_radians());
    let rotation = params
        .radians("rot")?
        .unwrap_or(DEFAULT_ROT_DEG.to_radians());
    let e = frame.earth.e();
    let chi0 = conformal_lat(phi0, e);
    let sinalpha = -chi0.cos() * azimuth.cos();
    // `sqrt`, not `asqrt`: a tiny negative under the radical stays NaN, as in C.
    let cosalpha = (1.0 - sinalpha * sinalpha).sqrt();
    // `atan2(+0, -0) = π` and `atan2(-0, -0) = -π`. Rewriting the zeros drops
    // `+azi=0 +lat_0=0` off Adams WS2.
    let lambda_0 = azimuth.tan().atan2(-chi0.sin());
    let beta = PI + (-azimuth.sin()).atan2(-chi0.tan());
    let conformal_distortion = msfn(phi0.sin(), phi0.cos(), frame.earth.es()) / chi0.cos();
    let default_lam0 = if params.flag("lon_0") {
        None
    } else {
        Some(DEFAULT_LON0_DEG.to_radians())
    };
    Ok(Some(classic_step(
        frame,
        Spilhaus {
            cosalpha,
            sinalpha,
            beta,
            lambda_0,
            conformal_distortion,
            cosrot: rotation.cos(),
            sinrot: rotation.sin(),
            e,
            k0: frame.k0,
            default_lam0,
            over: frame.over,
        },
    )))
}
