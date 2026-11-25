//! Lambert conique conforme par lots, `N` points à la fois.
//!
//! Les formules sont celles de [`Lcc::fwd_unit`] et [`Lcc::inv_unit`], écrites contre
//! [`V`]. Un groupe de `N` qu'une voie sort du domaine du noyau (valeur non
//! finie, pôle, `rho` nul) passe par le chemin scalaire : ses points valent exactement ceux de
//! [`Lcc::forward`] et [`Lcc::inverse`].

use super::Lcc;
use crate::simd::{Lanes, Vector};

/// `forward` en lot : degrés vers mètres.
pub(super) struct Forward<'a>(pub(super) &'a Lcc);

/// `inverse` en lot : mètres vers degrés.
pub(super) struct Inverse<'a>(pub(super) &'a Lcc);

impl Lanes for Forward<'_> {
    fn point(&self, lon_deg: f64, lat_deg: f64) -> (f64, f64) {
        self.0
            .forward(lon_deg, lat_deg)
            .map_or((f64::NAN, f64::NAN), |xy| (xy.east, xy.north))
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        lon_deg: V,
        lat_deg: V,
    ) -> Option<(V, V)> {
        vector::forward(self.0, tok, lon_deg, lat_deg)
    }
}

impl Lanes for Inverse<'_> {
    fn point(&self, east: f64, north: f64) -> (f64, f64) {
        self.0
            .inverse(crate::Projected { east, north })
            .map_or((f64::NAN, f64::NAN), |geo| (geo.lon_deg, geo.lat_deg))
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        east: V,
        north: V,
    ) -> Option<(V, V)> {
        vector::inverse(self.0, tok, east, north)
    }
}

#[cfg(simd_lanes)]
mod vector {
    use std::f64::consts::FRAC_PI_2;

    use super::Lcc;
    use crate::lcc::EPS10;
    use crate::simd::kernels::{self, DEG_TO_RAD, RAD_TO_DEG};
    use crate::simd::{Mask, Vector, VectorMath};

    /// [`Lcc::forward`] sur `N` points.
    #[inline(always)]
    pub(super) fn forward<V: Vector<N>, const N: usize>(
        l: &Lcc,
        tok: V::Token,
        lon_deg: V,
        lat_deg: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        let lon_r = lon_deg * k(DEG_TO_RAD);
        let lat_r = lat_deg * k(DEG_TO_RAD);
        // Un pôle (`rho` nul ou refusé selon le cône) reste au chemin scalaire.
        let at_pole = (lat_r.abs() - k(FRAC_PI_2)).abs().lt(k(EPS10));
        if !kernels::latitude_in_domain(tok, lat_r)
            .and(lon_r.is_finite())
            .and(at_pole.not())
            .all()
        {
            return None;
        }
        let phi = lat_r.max(k(-FRAC_PI_2)).min(k(FRAC_PI_2));
        let lam = kernels::adjlon(tok, kernels::adjlon(tok, lon_r) - k(l.lam0));

        let (sin_phi, cos_phi) = phi.sin_cos();
        let ts = kernels::tsfn(tok, sin_phi, cos_phi, l.e);
        let rho = k(l.c) * (k(l.n) * ts.log()).exp();
        let lam_n = lam * k(l.n);
        let (sin_lam_n, cos_lam_n) = lam_n.sin_cos();
        let easting = k(l.k0) * (rho * sin_lam_n);
        let northing = k(l.k0) * (-rho).mul_add(cos_lam_n, k(l.rho0));
        Some((
            k(l.a).mul_add(easting, k(l.x0)),
            k(l.a).mul_add(northing, k(l.y0)),
        ))
    }

    /// [`Lcc::inverse`] sur `N` points.
    #[inline(always)]
    pub(super) fn inverse<V: Vector<N>, const N: usize>(
        l: &Lcc,
        tok: V::Token,
        east: V,
        north: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        let mut x = (east - k(l.x0)) * k(l.ra) / k(l.k0);
        let mut y = k(l.rho0) - (north - k(l.y0)) * k(l.ra) / k(l.k0);
        let mut rho = x.hypot(y);
        // `rho` nul est le sommet du cône, et une entrée non finie une erreur : le scalaire.
        if !x
            .is_finite()
            .and(y.is_finite())
            .and(rho.eq(V::zero(tok)).not())
            .all()
        {
            return None;
        }
        if l.n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }
        let ts = (rho / k(l.c)).powf(k(1.0 / l.n));
        let phi = kernels::phi2(tok, ts, l.e);
        // Une latitude non finie est `OutsideDomain` : le scalaire.
        if !phi.is_finite().all() {
            return None;
        }
        let lam = x.atan2(y) / k(l.n);
        let lon = kernels::adjlon(tok, lam + k(l.lam0));
        Some((lon * k(RAD_TO_DEG), phi * k(RAD_TO_DEG)))
    }
}
