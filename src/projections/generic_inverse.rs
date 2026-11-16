//! `pj_generic_inverse_2d` of PROJ 9.9 (`src/generic_inverse.cpp`), shared by the projections
//! whose inverse is a Newton search on their forward: `adams_ws2`, `cass`, `wink2`.

use crate::engine::TransformError;
use std::f64::consts::{FRAC_PI_2, PI};

/// Newton iteration from `(lam, phi)` until the forward lands within `tolerance` of `(x, y)` on
/// each axis: 15 iterations, finite-difference step `±1e-6`, Jacobian kept once the residual is
/// below 1e-6, correction clamped to ±0.3 rad, angles clamped to their range. No convergence is
/// PROJ's `PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN`.
///
/// # Errors
///
/// [`TransformError::OutsideDomain`] when the 15 iterations do not converge, or the error of
/// `forward`.
pub(crate) fn generic_inverse_2d(
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
