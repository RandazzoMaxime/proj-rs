//! Briques de calcul que plusieurs projections partagent, en quatre voies.
//!
//! Chaque fonction suit sa version scalaire opération par opération : mêmes constantes, même
//! ordre des additions. L'écart avec le scalaire vient des fonctions transcendantes
//! (`math.rs`), pas de la formule.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{Avx2, Mask, V4};

/// Degrés vers radians, comme `f64::to_radians`.
pub(crate) const DEG_TO_RAD: f64 = PI / 180.0;
/// Radians vers degrés, comme `f64::to_degrees`.
pub(crate) const RAD_TO_DEG: f64 = 180.0 / PI;

/// `adjlon` de `crate::angle` : ramène une longitude dans ]−π, π].
#[inline(always)]
pub(crate) fn adjlon(tok: Avx2, lon: V4) -> V4 {
    let k = |value| V4::splat(tok, value);
    let in_range = lon.abs().lt(k(PI + 1e-12));
    let shifted = lon + k(PI);
    let wrapped = shifted - k(TAU) * (shifted / k(TAU)).floor();
    V4::select(in_range, lon, wrapped - k(PI))
}

/// Latitudes (radians) finies et au plus `π/2 + 1e-12` en valeur absolue : le domaine où les
/// projections acceptent le point.
#[inline(always)]
pub(crate) fn latitude_in_domain(tok: Avx2, lat_rad: V4) -> Mask {
    lat_rad
        .is_finite()
        .and(lat_rad.abs().le(V4::splat(tok, FRAC_PI_2 + 1e-12)))
}

/// `tsfn` de `crate::lcc`, avec le sinus et le cosinus déjà calculés.
#[inline(always)]
pub(crate) fn tsfn(tok: Avx2, sin_phi: V4, cos_phi: V4, e: f64) -> V4 {
    let k = |value| V4::splat(tok, value);
    let one = k(1.0);
    let positive = cos_phi / (one + sin_phi);
    let negative = (one - sin_phi) / cos_phi;
    let ratio = V4::select(sin_phi.gt(V4::zero(tok)), positive, negative);
    (k(e) * (k(e) * sin_phi).atanh()).exp() * ratio
}

/// `phi2` de `crate::lcc` : la latitude (radians) de `ts`, par le Newton de `GeographicLib`.
///
/// Les voies convergent à leur rythme, comme dans la boucle scalaire : une voie qui a convergé
/// est figée, et la boucle s'arrête quand toutes l'ont fait ou après cinq tours.
#[inline(always)]
pub(crate) fn phi2(tok: Avx2, ts: V4, e: f64) -> V4 {
    const NUMIT: usize = 5;
    let k = |value| V4::splat(tok, value);
    let one = k(1.0);
    let rooteps = f64::EPSILON.sqrt();
    let tol = rooteps / 10.0;
    let tmax = 2.0 / rooteps;
    let e2m = 1.0 - e * e;

    let taup = (one / ts - ts) / k(2.0);
    let stol = k(tol) * one.max(taup.abs());
    let large = taup.abs().gt(k(70.0));
    let mut tau = V4::select(large, taup * k((e * e.atanh()).exp()), taup / k(e2m));
    // `!(|tau| < tmax)` rend `tau` tel quel, `NaN` compris.
    let mut active = tau.abs().lt(k(tmax));
    for _ in 0..NUMIT {
        if !active.any() {
            break;
        }
        let tau1 = tau.mul_add(tau, one).sqrt();
        let sig = (k(e) * (k(e) * tau / tau1).atanh()).sinh();
        let taupa = sig.mul_add(sig, one).sqrt().mul_add(tau, -(sig * tau1));
        let dtau = ((taup - taupa) * k(e2m).mul_add(tau * tau, one))
            / (k(e2m) * tau1 * taupa.mul_add(taupa, one).sqrt());
        tau = V4::select(active, tau + dtau, tau);
        // `!(|dtau| >= stol)` arrête la voie, une voie `NaN` comprise.
        active = active.and(dtau.abs().ge(stol));
    }
    tau.atan()
}
