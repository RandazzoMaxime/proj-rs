//! Transverse Mercator exact par lots, quatre points à la fois.
//!
//! Les formules sont celles de [`Tmerc::fwd_unit`] et [`Tmerc::inv_unit`], opération par
//! opération, écrites contre [`crate::simd::V4`]. Un groupe de quatre qu'une voie sort du
//! domaine du noyau (valeur non finie, latitude hors de ±90°, `|Ce|` au-delà de la limite) passe
//! par le chemin scalaire : ses points valent exactement ceux de [`Tmerc::forward`] et
//! [`Tmerc::inverse`].

use super::Tmerc;
use crate::simd::Lanes;

/// `forward` en lot : degrés vers mètres.
pub(super) struct Forward<'a>(pub(super) &'a Tmerc);

/// `forward_rad` en lot : radians vers mètres.
pub(super) struct ForwardRad<'a>(pub(super) &'a Tmerc);

/// `inverse` en lot : mètres vers degrés.
pub(super) struct Inverse<'a>(pub(super) &'a Tmerc);

impl Lanes for Forward<'_> {
    fn point(&self, lon_deg: f64, lat_deg: f64) -> (f64, f64) {
        self.0
            .forward(lon_deg, lat_deg)
            .map_or((f64::NAN, f64::NAN), |xy| (xy.east, xy.north))
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Avx2,
        lon_deg: crate::simd::V4,
        lat_deg: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        vector::forward(self.0, tok, lon_deg, lat_deg)
    }
}

impl Lanes for ForwardRad<'_> {
    fn point(&self, lon_rad: f64, lat_rad: f64) -> (f64, f64) {
        self.0
            .forward_rad(lon_rad, lat_rad)
            .map_or((f64::NAN, f64::NAN), |xy| (xy.east, xy.north))
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Avx2,
        lon_rad: crate::simd::V4,
        lat_rad: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        vector::forward_rad(self.0, tok, lon_rad, lat_rad)
    }
}

impl Lanes for Inverse<'_> {
    fn point(&self, east: f64, north: f64) -> (f64, f64) {
        self.0
            .inverse(crate::Projected { east, north })
            .map_or((f64::NAN, f64::NAN), |geo| (geo.lon_deg, geo.lat_deg))
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Avx2,
        east: crate::simd::V4,
        north: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        vector::inverse(self.0, tok, east, north)
    }
}

#[cfg(target_arch = "x86_64")]
mod vector {
    use std::f64::consts::FRAC_PI_2;

    use super::Tmerc;
    use crate::simd::kernels::{self, DEG_TO_RAD, RAD_TO_DEG};
    use crate::simd::{Avx2, V4};
    use crate::tmerc::{CE_LIMIT, ORDER};

    /// `gatg` : la série de Clenshaw réelle, coefficients `p1`.
    #[inline(always)]
    fn gatg(tok: Avx2, p1: &[f64; ORDER], b: V4, cos_2b: V4, sin_2b: V4) -> V4 {
        let k = |value| V4::splat(tok, value);
        let two = k(2.0) * cos_2b;
        let mut h2 = V4::zero(tok);
        let mut h1 = k(p1[ORDER - 1]);
        let mut h = V4::zero(tok);
        for idx in (0..ORDER - 1).rev() {
            // `-h2 + two * h1 + p` : un produit fusionné et une soustraction.
            h = two.mul_add(h1, k(p1[idx]) - h2);
            h2 = h1;
            h1 = h;
        }
        h.mul_add(sin_2b, b)
    }

    /// `clenshaw_complex` : la série de Clenshaw complexe, coefficients `a`.
    #[inline(always)]
    fn clenshaw_complex(
        tok: Avx2,
        a: &[f64; ORDER],
        sin_arg_r: V4,
        cos_arg_r: V4,
        sinh_arg_i: V4,
        cosh_arg_i: V4,
    ) -> (V4, V4) {
        let k = |value| V4::splat(tok, value);
        let mut r = k(2.0) * cos_arg_r * cosh_arg_i;
        let mut i = k(-2.0) * sin_arg_r * sinh_arg_i;
        let mut hr1 = V4::zero(tok);
        let mut hi1 = V4::zero(tok);
        let mut hi = V4::zero(tok);
        let mut hr = k(a[ORDER - 1]);
        for idx in (0..ORDER - 1).rev() {
            let hr2 = hr1;
            let hi2 = hi1;
            hr1 = hr;
            hi1 = hi;
            hr = (-i).mul_add(hi1, r.mul_add(hr1, k(a[idx]) - hr2));
            hi = r.mul_add(hi1, i.mul_add(hr1, -hi2));
        }
        r = sin_arg_r * cosh_arg_i;
        i = cos_arg_r * sinh_arg_i;
        ((-i).mul_add(hi, r * hr), r.mul_add(hi, i * hr))
    }

    /// [`Tmerc::forward`] sur quatre points.
    #[inline(always)]
    pub(super) fn forward(t: &Tmerc, tok: Avx2, lon_deg: V4, lat_deg: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
        forward_rad(t, tok, lon_deg * k(DEG_TO_RAD), lat_deg * k(DEG_TO_RAD))
    }

    /// [`Tmerc::forward_rad`] sur quatre points.
    #[inline(always)]
    #[expect(
        clippy::similar_names,
        reason = "Cn and Ce are the paired terms of PROJ exact_e_fwd"
    )]
    pub(super) fn forward_rad(t: &Tmerc, tok: Avx2, lon_r: V4, lat_r: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
        if !kernels::latitude_in_domain(tok, lat_r)
            .and(lon_r.is_finite())
            .all()
        {
            return None;
        }
        let phi = lat_r.max(k(-FRAC_PI_2)).min(k(FRAC_PI_2));
        let lam = kernels::adjlon(tok, kernels::adjlon(tok, lon_r) - k(t.lam0));

        let two_phi = phi + phi;
        let mut cn = gatg(tok, &t.cbg, phi, two_phi.cos(), two_phi.sin());
        let sin_cn = cn.sin();
        let cos_cn = cn.cos();
        let sin_ce = lam.sin();
        let cos_ce = lam.cos();
        let cos_cn_cos_ce = cos_cn * cos_ce;
        cn = sin_cn.atan2(cos_cn_cos_ce);

        let inv_denom = k(1.0) / sin_cn.hypot(cos_cn_cos_ce);
        let tan_ce = sin_ce * cos_cn * inv_denom;
        let mut ce = tan_ce.asinh();

        let two_inv = k(2.0) * inv_denom;
        let two_inv_sq = two_inv * inv_denom;
        let tmp_r = cos_cn_cos_ce * two_inv_sq;
        let sin_arg_r = sin_cn * tmp_r;
        let cos_arg_r = cos_cn_cos_ce.mul_add(tmp_r, k(-1.0));
        let sinh_arg_i = tan_ce * two_inv;
        let cosh_arg_i = two_inv_sq - k(1.0);

        let (d_cn, d_ce) =
            clenshaw_complex(tok, &t.gtu, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        cn = cn + d_cn;
        ce = ce + d_ce;

        let inside = ce.abs().le(k(CE_LIMIT));
        let east = k(t.a).mul_add(k(t.qn) * ce, k(t.x0));
        let north = k(t.a).mul_add(k(t.qn).mul_add(cn, k(t.zb)), k(t.y0));
        let nan = k(f64::NAN);
        Some((
            V4::select(inside, east, nan),
            V4::select(inside, north, nan),
        ))
    }

    /// [`Tmerc::inverse`] sur quatre points.
    #[inline(always)]
    #[expect(
        clippy::similar_names,
        reason = "Cn and Ce are the paired terms in PROJ exact_e_inv"
    )]
    pub(super) fn inverse(t: &Tmerc, tok: Avx2, east: V4, north: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
        let x = (east - k(t.x0)) * k(t.ra);
        let y = (north - k(t.y0)) * k(t.ra);
        let mut cn = (y - k(t.zb)) / k(t.qn);
        let mut ce = x / k(t.qn);
        if !x
            .is_finite()
            .and(y.is_finite())
            .and(ce.abs().le(k(CE_LIMIT)))
            .all()
        {
            return None;
        }
        let two_cn = cn + cn;
        let sin_arg_r = two_cn.sin();
        let cos_arg_r = two_cn.cos();
        let exp_2_ce = (ce + ce).exp();
        let half_inv = k(0.5) / exp_2_ce;
        let sinh_arg_i = k(0.5).mul_add(exp_2_ce, -half_inv);
        let cosh_arg_i = k(0.5).mul_add(exp_2_ce, half_inv);
        let (d_cn, d_ce) =
            clenshaw_complex(tok, &t.utg, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        cn = cn + d_cn;
        ce = ce + d_ce;

        let sin_cn = cn.sin();
        let cos_cn = cn.cos();
        let sinh_ce = ce.sinh();
        ce = sinh_ce.atan2(cos_cn);
        let modulus = sinh_ce.hypot(cos_cn);
        cn = sin_cn.atan2(modulus);

        let tmp = k(2.0) * modulus / sinh_ce.mul_add(sinh_ce, k(1.0));
        let sin_2_cn = sin_cn * tmp;
        let cos_2_cn = tmp.mul_add(modulus, k(-1.0));
        let phi = gatg(tok, &t.cgb, cn, cos_2_cn, sin_2_cn);

        let lon = kernels::adjlon(tok, ce + k(t.lam0));
        Some((lon * k(RAD_TO_DEG), phi * k(RAD_TO_DEG)))
    }
}
