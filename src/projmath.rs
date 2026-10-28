#![allow(
    clippy::pedantic,
    reason = "series coefficients and Clenshaw summation are transcribed from PROJ latitudes.cpp"
)]

//! Auxiliary latitudes and the small helpers shared by the classic projections.
//!
//! The series are `pj_auxlat_coeffs` from PROJ 9.9 `src/latitudes.cpp`
//! (Karney, order 6). `aasin` / `aacos` / `asqrt` follow `src/aasincos.cpp`.

use std::f64::consts::FRAC_PI_2;

const ONE_TOL: f64 = 1.000_000_000_000_01;
const ATOL: f64 = 1e-50;
const ORDER: usize = 6;
const AUX_NUMBER: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) enum AuxLat {
    Geographic = 0,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parametric latitude is used by later projections")
    )]
    Parametric = 1,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "geocentric latitude is used by later projections")
    )]
    Geocentric = 2,
    Rectifying = 3,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "conformal latitude is used by the transverse Mercator port"
        )
    )]
    Conformal = 4,
    Authalic = 5,
}

impl AuxLat {
    fn index(self) -> usize {
        match self {
            Self::Geographic => 0,
            Self::Parametric => 1,
            Self::Geocentric => 2,
            Self::Rectifying => 3,
            Self::Conformal => 4,
            Self::Authalic => 5,
        }
    }
}

/// `asin` that clamps a value just outside [-1, 1] instead of returning NaN.
pub(crate) fn aasin(v: f64) -> Result<f64, ()> {
    let av = v.abs();
    if av >= 1.0 {
        if av > ONE_TOL {
            return Err(());
        }
        return Ok(if v < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
    }
    Ok(v.asin())
}

/// `acos` with the same clamp as [`aasin`].
pub(crate) fn aacos(v: f64) -> Result<f64, ()> {
    let av = v.abs();
    if av >= 1.0 {
        if av > ONE_TOL {
            return Err(());
        }
        return Ok(if v < 0.0 { std::f64::consts::PI } else { 0.0 });
    }
    Ok(v.acos())
}

/// `sqrt` that returns 0 for a non-positive argument, as PROJ `asqrt`.
#[must_use]
pub(crate) fn asqrt(v: f64) -> f64 {
    if v <= 0.0 {
        0.0
    } else {
        v.sqrt()
    }
}

/// `atan2` that returns 0 when both arguments are tiny.
#[must_use]
pub(crate) fn aatan2(n: f64, d: f64) -> f64 {
    if n.abs() < ATOL && d.abs() < ATOL {
        0.0
    } else {
        n.atan2(d)
    }
}

/// Snyder `ts(φ) = exp(−ψ)`, PROJ 9.9 `pj_tsfn`.
#[must_use]
pub(crate) fn tsfn(phi: f64, sinphi: f64, e: f64) -> f64 {
    let cosphi = phi.cos();
    let factor = if sinphi > 0.0 {
        cosphi / (1.0 + sinphi)
    } else {
        (1.0 - sinphi) / cosphi
    };
    (e * (e * sinphi).atanh()).exp() * factor
}

/// Inverse isometric latitude, PROJ 9.9 `pj_phi2`.
///
/// `ts` is `exp(−ψ)`. The error is the Newton failure `pj_sinhpsi2tanphi`
/// reports as an illegal argument; callers treat that as a rejected point.
pub(crate) fn phi2(ts: f64, e: f64) -> Result<f64, ()> {
    let taup = (1.0 / ts - ts) / 2.0;
    Ok(sinhpsi2tanphi(taup, e)?.atan())
}

/// `tan φ` from `sinh ψ`, GeographicLib's `tauf` as ported in `phi2.cpp`.
pub(crate) fn sinhpsi2tanphi(taup: f64, e: f64) -> Result<f64, ()> {
    let rooteps = f64::EPSILON.sqrt();
    let tol = rooteps / 10.0;
    let tmax = 2.0 / rooteps;
    let e2m = 1.0 - e * e;
    let stol = tol * 1.0_f64.max(taup.abs());
    let mut tau = if taup.abs() > 70.0 {
        taup * (e * e.atanh()).exp()
    } else {
        taup / e2m
    };
    if !(tau.abs() < tmax) {
        return Ok(tau);
    }
    let mut i = 5;
    loop {
        if i == 0 {
            return Err(());
        }
        let tau1 = (1.0 + tau * tau).sqrt();
        let sig = (e * (e * tau / tau1).atanh()).sinh();
        let taupa = (1.0 + sig * sig).sqrt() * tau - sig * tau1;
        let dtau = ((taup - taupa) * (1.0 + e2m * tau * tau))
            / (e2m * tau1 * (1.0 + taupa * taupa).sqrt());
        tau += dtau;
        if !(dtau.abs() >= stol) {
            break;
        }
        i -= 1;
    }
    Ok(tau)
}

/// Snyder `m`, `cos φ / sqrt(1 − e² sin² φ)`.
#[must_use]
pub(crate) fn msfn(sinphi: f64, cosphi: f64, es: f64) -> f64 {
    cosphi / (1.0 - es * sinphi * sinphi).sqrt()
}

/// Authalic latitude factor `q`. Sphere (`e` below 1e-7) returns `2 sin φ`.
#[must_use]
pub(crate) fn authalic_q(sinphi: f64, e: f64, one_es: f64) -> f64 {
    const EPSILON: f64 = 1e-7;
    if e >= EPSILON {
        let e_sinphi = e * sinphi;
        let one_minus = 1.0 - e_sinphi * e_sinphi;
        if one_minus == 0.0 {
            return f64::INFINITY;
        }
        one_es * (sinphi / one_minus + e_sinphi.atanh() / e)
    } else {
        2.0 * sinphi
    }
}

fn polyval(x: f64, p: &[f64], mut n: i32) -> f64 {
    if n < 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "n is a non-negative polynomial degree"
    )]
    let mut y = p[n as usize];
    while n > 0 {
        n -= 1;
        #[expect(
            clippy::cast_sign_loss,
            clippy::as_conversions,
            reason = "n stays a non-negative polynomial degree"
        )]
        {
            y = y * x + p[n as usize];
        }
    }
    y
}

fn clenshaw(szeta: f64, czeta: f64, f: &[f64]) -> f64 {
    let mut u0 = 0.0;
    let mut u1 = 0.0;
    let x = 2.0 * (czeta - szeta) * (czeta + szeta);
    let mut k = f.len();
    while k > 0 {
        k -= 1;
        let t = x * u0 - u1 + f[k];
        u1 = u0;
        u0 = t;
    }
    2.0 * szeta * czeta * u0
}

/// Fourier coefficients converting `auxin` to `auxout`, length [`ORDER`].
pub(crate) fn auxlat_coeffs(n: f64, auxin: AuxLat, auxout: AuxLat) -> Option<[f64; ORDER]> {
    let k = AUX_NUMBER * auxout.index() + auxin.index();
    let mut o = PTRS[k];
    if o == PTRS[k + 1] {
        return None;
    }
    let mut d = n;
    let n2 = n * n;
    let mut f = [0.0; ORDER];
    if auxin <= AuxLat::Rectifying && auxout <= AuxLat::Rectifying {
        for (l, slot) in f.iter_mut().enumerate() {
            let m = (ORDER - l - 1) / 2;
            *slot = d * polyval(n2, &COEFFS[o..], i32::try_from(m).unwrap_or(0));
            o += m + 1;
            d *= n;
        }
    } else {
        for (l, slot) in f.iter_mut().enumerate() {
            let m = ORDER - l - 1;
            *slot = d * polyval(n, &COEFFS[o..], i32::try_from(m).unwrap_or(0));
            o += m + 1;
            d *= n;
        }
    }
    Some(f)
}

/// Convert auxiliary latitude `zeta` with coefficients from [`auxlat_coeffs`].
#[must_use]
pub(crate) fn auxlat_convert(zeta: f64, f: &[f64]) -> f64 {
    zeta + clenshaw(zeta.sin(), zeta.cos(), f)
}

/// Meridional distance `M/a` and its inverse, order 6 in the third flattening.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Meridian {
    radius: f64,
    to_mu: [f64; ORDER],
    to_phi: [f64; ORDER],
}

impl Meridian {
    /// Coefficients for third flattening `n`.
    ///
    /// # Panics
    ///
    /// Panics if the generated tables omit the rectifying pair. They include it.
    #[must_use]
    pub(crate) fn new(n: f64) -> Self {
        Self {
            radius: rectifying_radius(n),
            to_mu: auxlat_coeffs(n, AuxLat::Geographic, AuxLat::Rectifying)
                .expect("geographic to rectifying is generated"),
            to_phi: auxlat_coeffs(n, AuxLat::Rectifying, AuxLat::Geographic)
                .expect("rectifying to geographic is generated"),
        }
    }

    /// Distance from the equator, in units of the semi-major axis.
    #[must_use]
    pub(crate) fn forward(self, phi: f64) -> f64 {
        self.radius * auxlat_convert(phi, &self.to_mu)
    }

    /// Latitude from a meridional distance in units of the semi-major axis.
    #[must_use]
    pub(crate) fn inverse(self, distance: f64) -> f64 {
        auxlat_convert(distance / self.radius, &self.to_phi)
    }
}

/// Quarter-meridian factor `(quarter meridian) / (a π / 2)`.
#[must_use]
pub(crate) fn rectifying_radius(n: f64) -> f64 {
    const COEFF_RAD: [f64; 4] = [1.0, 0.25, 1.0 / 64.0, 1.0 / 256.0];
    polyval(n * n, &COEFF_RAD, 3) / (1.0 + n)
}

const SERIES_N: f64 = 0.01;

/// Authalic latitude from geographic latitude `phi`.
#[must_use]
pub(crate) fn authalic_forward(phi: f64, sinphi: f64, n: f64, e: f64, one_es: f64, qp: f64) -> f64 {
    if n.abs() < SERIES_N {
        return auxlat_coeffs(n, AuxLat::Geographic, AuxLat::Authalic)
            .map_or(phi, |coeffs| auxlat_convert(phi, &coeffs));
    }
    let mut ratio = authalic_q(sinphi, e, one_es) / qp;
    if ratio.abs() > 1.0 {
        ratio = if ratio > 0.0 { 1.0 } else { -1.0 };
    }
    ratio.asin()
}

/// Geographic latitude from authalic latitude `beta`.
#[must_use]
pub(crate) fn authalic_inverse(beta: f64, n: f64, e: f64, es: f64, one_es: f64, qp: f64) -> f64 {
    let to_geo = auxlat_coeffs(n, AuxLat::Authalic, AuxLat::Geographic);
    let mut phi = to_geo.map_or(beta, |c| auxlat_convert(beta, &c));
    if n.abs() < SERIES_N {
        return phi;
    }
    let q = beta.sin() * qp;
    let q_div = q / one_es;
    for _ in 0..10 {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let one_minus = 1.0 - es * sinphi * sinphi;
        let dphi = (one_minus * one_minus) / (2.0 * cosphi)
            * (q_div - sinphi / one_minus - (e * sinphi).atanh() / e);
        // Negated so a NaN step stops the iteration, as PROJ's Newton loop does.
        #[expect(
            clippy::neg_cmp_op_on_partial_ord,
            reason = "NaN must break the loop, matching pj_authalic_lat_inverse"
        )]
        if !(dphi.abs() >= 1e-15) {
            break;
        }
        phi += dphi;
    }
    phi
}

/// Coefficients, machine-generated in PROJ 9.9 `latitudes.cpp` (2025-03-23).
#[rustfmt::skip]
const COEFFS: [f64; 150] = [
    3.0 / 2.0, -27.0 / 32.0, 269.0 / 512.0, 21.0 / 16.0, -55.0 / 32.0,
    6759.0 / 4096.0, 151.0 / 96.0, -417.0 / 128.0, 1097.0 / 512.0,
    -15543.0 / 2560.0, 8011.0 / 2560.0, 293_393.0 / 61440.0,
    2.0, -2.0 / 3.0, -2.0, 116.0 / 45.0, 26.0 / 45.0, -2854.0 / 675.0,
    7.0 / 3.0, -8.0 / 5.0, -227.0 / 45.0, 2704.0 / 315.0, 2323.0 / 945.0,
    56.0 / 15.0, -136.0 / 35.0, -1262.0 / 105.0, 73814.0 / 2835.0,
    4279.0 / 630.0, -332.0 / 35.0, -399_572.0 / 14175.0, 4174.0 / 315.0,
    -144_838.0 / 6237.0, 601_676.0 / 22275.0,
    4.0 / 3.0, 4.0 / 45.0, -16.0 / 35.0, -2582.0 / 14175.0,
    60136.0 / 467_775.0, 28_112_932.0 / 212_837_625.0, 46.0 / 45.0,
    152.0 / 945.0, -11966.0 / 14175.0, -21016.0 / 51975.0,
    251_310_128.0 / 638_512_875.0, 3044.0 / 2835.0, 3802.0 / 14175.0,
    -94388.0 / 66825.0, -8_797_648.0 / 10_945_935.0, 6059.0 / 4725.0,
    41072.0 / 93555.0, -1_472_637_812.0 / 638_512_875.0, 768_272.0 / 467_775.0,
    455_935_736.0 / 638_512_875.0, 4_210_684_958.0 / 1_915_538_625.0,
    -3.0 / 2.0, 9.0 / 16.0, -3.0 / 32.0, 15.0 / 16.0, -15.0 / 32.0,
    135.0 / 2048.0, -35.0 / 48.0, 105.0 / 256.0, 315.0 / 512.0,
    -189.0 / 512.0, -693.0 / 1280.0, 1001.0 / 2048.0,
    1.0 / 2.0, -2.0 / 3.0, 5.0 / 16.0, 41.0 / 180.0, -127.0 / 288.0,
    7891.0 / 37800.0, 13.0 / 48.0, -3.0 / 5.0, 557.0 / 1440.0,
    281.0 / 630.0, -1_983_433.0 / 1_935_360.0, 61.0 / 240.0, -103.0 / 140.0,
    15061.0 / 26880.0, 167_603.0 / 181_440.0, 49561.0 / 161_280.0,
    -179.0 / 168.0, 6_601_661.0 / 7_257_600.0, 34729.0 / 80640.0,
    -3_418_889.0 / 1_995_840.0, 212_378_941.0 / 319_334_400.0,
    -2.0, 2.0 / 3.0, 4.0 / 3.0, -82.0 / 45.0, 32.0 / 45.0, 4642.0 / 4725.0,
    5.0 / 3.0, -16.0 / 15.0, -13.0 / 9.0, 904.0 / 315.0, -1522.0 / 945.0,
    -26.0 / 15.0, 34.0 / 21.0, 8.0 / 5.0, -12686.0 / 2835.0, 1237.0 / 630.0,
    -12.0 / 5.0, -24832.0 / 14175.0, -734.0 / 315.0, 109_598.0 / 31185.0,
    444_337.0 / 155_925.0,
    -1.0 / 2.0, 2.0 / 3.0, -37.0 / 96.0, 1.0 / 360.0, 81.0 / 512.0,
    -96199.0 / 604_800.0, -1.0 / 48.0, -1.0 / 15.0, 437.0 / 1440.0,
    -46.0 / 105.0, 1_118_711.0 / 3_870_720.0, -17.0 / 480.0, 37.0 / 840.0,
    209.0 / 4480.0, -5569.0 / 90720.0, -4397.0 / 161_280.0, 11.0 / 504.0,
    830_251.0 / 7_257_600.0, -4583.0 / 161_280.0, 108_847.0 / 3_991_680.0,
    -20_648_693.0 / 638_668_800.0,
    -4.0 / 3.0, -4.0 / 45.0, 88.0 / 315.0, 538.0 / 4725.0,
    20824.0 / 467_775.0, -44732.0 / 2_837_835.0, 34.0 / 45.0, 8.0 / 105.0,
    -2482.0 / 14175.0, -37192.0 / 467_775.0, -12_467_764.0 / 212_837_625.0,
    -1532.0 / 2835.0, -898.0 / 14175.0, 54968.0 / 467_775.0,
    100_320_856.0 / 1_915_538_625.0, 6007.0 / 14175.0, 24496.0 / 467_775.0,
    -5_884_124.0 / 70_945_875.0, -23356.0 / 66825.0, -839_792.0 / 19_348_875.0,
    570_284_222.0 / 1_915_538_625.0,
];

const PTRS: [usize; 37] = [
    0, 0, 0, 0, 12, 33, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 66, 66, 66, 66, 87, 87,
    108, 108, 108, 129, 129, 129, 150, 150, 150, 150, 150, 150,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coeff_table_matches_proj_length() {
        assert_eq!(COEFFS.len(), PTRS[AUX_NUMBER * AUX_NUMBER]);
    }

    #[test]
    fn clamps_are_defined() {
        assert!(aasin(0.0).is_ok());
        assert!(aacos(0.0).is_ok());
        assert_eq!(asqrt(-1.0), 0.0);
        assert_eq!(aatan2(0.0, 0.0), 0.0);
        assert!(rectifying_radius(0.0) > 0.0);
        let _ = (AuxLat::Parametric, AuxLat::Geocentric);
    }

    #[test]
    fn conformal_roundtrip_on_grs80() {
        let n = crate::Ellipsoid::grs80().n();
        let to_chi = auxlat_coeffs(n, AuxLat::Geographic, AuxLat::Conformal).expect("coeffs");
        let to_phi = auxlat_coeffs(n, AuxLat::Conformal, AuxLat::Geographic).expect("coeffs");
        let phi = 0.7;
        let chi = auxlat_convert(phi, &to_chi);
        let back = auxlat_convert(chi, &to_phi);
        assert!((back - phi).abs() < 1e-14, "{back}");
    }
}
