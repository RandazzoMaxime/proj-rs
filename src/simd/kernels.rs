//! Briques de calcul que plusieurs projections partagent, en quatre voies.
//!
//! Chaque fonction suit sa version scalaire opération par opération : mêmes constantes, même
//! ordre des additions. L'écart avec le scalaire vient des fonctions transcendantes
//! (`math.rs`), pas de la formule.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{Mask, Token, V4};

/// Degrés vers radians, comme `f64::to_radians`.
pub(crate) const DEG_TO_RAD: f64 = PI / 180.0;
/// Radians vers degrés, comme `f64::to_degrees`.
pub(crate) const RAD_TO_DEG: f64 = 180.0 / PI;

/// `adjlon` de `crate::angle` : ramène une longitude dans ]−π, π].
#[inline(always)]
pub(crate) fn adjlon(tok: Token, lon: V4) -> V4 {
    let k = |value| V4::splat(tok, value);
    let in_range = lon.abs().lt(k(PI + 1e-12));
    let shifted = lon + k(PI);
    let wrapped = shifted - k(TAU) * (shifted / k(TAU)).floor();
    V4::select(in_range, lon, wrapped - k(PI))
}

/// Latitudes (radians) finies et au plus `π/2 + 1e-12` en valeur absolue : le domaine où les
/// projections acceptent le point.
#[inline(always)]
pub(crate) fn latitude_in_domain(tok: Token, lat_rad: V4) -> Mask {
    lat_rad
        .is_finite()
        .and(lat_rad.abs().le(V4::splat(tok, FRAC_PI_2 + 1e-12)))
}

/// `tsfn` de `crate::lcc`, avec le sinus et le cosinus déjà calculés.
#[inline(always)]
pub(crate) fn tsfn(tok: Token, sin_phi: V4, cos_phi: V4, e: f64) -> V4 {
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
pub(crate) fn phi2(tok: Token, ts: V4, e: f64) -> V4 {
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

/// `asin` d'un argument de `[-1, 1]` : `atan2(x, sqrt((1 - x)(1 + x)))`. L'erreur croît comme
/// `1 / sqrt(1 - x²)` : l'appelant garde `|x|` sous [`ASIN_LIMIT`] et renvoie le groupe au
/// scalaire au-delà.
#[inline(always)]
pub(crate) fn asin(tok: Token, x: V4) -> V4 {
    let one = V4::splat(tok, 1.0);
    x.atan2(((one - x) * (one + x)).sqrt())
}

/// Au-delà de cette valeur absolue, [`asin`] amplifie l'erreur d'entrée de plus de quatre fois
/// (`1 / sqrt(1 - 0,97²)`), et la longitude qui en dépend, de `1 / cos φ` en plus, sort du 1e-11°
/// du dépôt à 86° de latitude (mesuré sur `somerc`). La zone d'usage reste bien en deçà.
pub(crate) const ASIN_LIMIT: f64 = 0.97;

/// Voies dont l'argument de [`asin`] est fini et sous [`ASIN_LIMIT`] en valeur absolue.
#[inline(always)]
pub(crate) fn asin_in_domain(tok: Token, x: V4) -> Mask {
    x.abs().le(V4::splat(tok, ASIN_LIMIT))
}

/// `auxlat_convert_sincos` de `crate::projmath` : `zeta` plus la série de Clenshaw des
/// coefficients `f`, avec le sinus et le cosinus de `zeta` déjà calculés.
#[inline(always)]
pub(crate) fn auxlat_convert(tok: Token, zeta: V4, sin_zeta: V4, cos_zeta: V4, f: &[f64]) -> V4 {
    let k = |value| V4::splat(tok, value);
    let x = k(2.0) * (cos_zeta - sin_zeta) * (cos_zeta + sin_zeta);
    let mut u0 = V4::zero(tok);
    let mut u1 = V4::zero(tok);
    for coefficient in f.iter().rev() {
        let next = x.mul_add(u0, k(*coefficient) - u1);
        u1 = u0;
        u0 = next;
    }
    zeta + k(2.0) * sin_zeta * cos_zeta * u0
}
