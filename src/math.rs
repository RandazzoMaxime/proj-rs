//! Mathematical helpers PROJ exposes to its projections (`pj_phi2`).

use std::fmt;

/// The inverse isometric latitude did not converge (PROJ returns an error for
/// the same case from `pj_sinhpsi2tanphi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phi2Error;

impl fmt::Display for Phi2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("inverse isometric latitude did not converge")
    }
}

impl std::error::Error for Phi2Error {}

/// The latitude `phi` whose isometric latitude is `ts`, with eccentricity `e`
/// (PROJ `pj_phi2`). `ts` is `exp(-psi)`.
///
/// Special values follow PROJ: `phi2(0, e) = pi/2`, `phi2(inf, e) = -pi/2`,
/// `phi2(-0, e) = -pi/2`, `phi2(-inf, e) = pi/2`, `phi2(1, e) = 0`,
/// `phi2(-1, e) = 0`. A NaN argument gives NaN.
///
/// # Errors
///
/// [`Phi2Error`] when the Newton iteration fails to converge.
pub fn phi2(ts: f64, e: f64) -> Result<f64, Phi2Error> {
    crate::projmath::phi2(ts, e).map_err(|()| Phi2Error)
}
