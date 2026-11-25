//! Mercator par lots, `N` points à la fois, sphère et ellipsoïde.
//!
//! Les formules sont celles de [`Mercator::fwd_unit`] et [`Mercator::inv_unit`], écrites contre
//! [`V`]. Un groupe de `N` qu'une voie sort du domaine du noyau (valeur non
//! finie, latitude hors de ±90°) passe par le chemin scalaire : ses points valent exactement
//! ceux de [`Mercator::forward`] et [`Mercator::inverse`].

use super::Mercator;
use crate::simd::{Lanes, Vector};

/// `forward` en lot : degrés vers mètres.
pub(super) struct Forward<'a>(pub(super) &'a Mercator);

/// `forward` en lot : radians absolus vers mètres.
pub(super) struct ForwardRad<'a>(pub(super) &'a Mercator);

/// `inverse` en lot : mètres vers degrés.
pub(super) struct Inverse<'a>(pub(super) &'a Mercator);

/// `inv_unit` en lot : unités du rayon vers radians, sans ramener la longitude.
pub(super) struct InverseUnit<'a>(pub(super) &'a Mercator);

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

impl Lanes for ForwardRad<'_> {
    fn point(&self, lon_rad: f64, lat_rad: f64) -> (f64, f64) {
        self.0
            .forward_rad(lon_rad, lat_rad)
            .map_or((f64::NAN, f64::NAN), |xy| (xy.east, xy.north))
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        lon_rad: V,
        lat_rad: V,
    ) -> Option<(V, V)> {
        vector::forward_rad(self.0, tok, lon_rad, lat_rad)
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

impl Lanes for InverseUnit<'_> {
    fn point(&self, x: f64, y: f64) -> (f64, f64) {
        self.0.inv_unit(x, y).unwrap_or((f64::NAN, f64::NAN))
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(&self, tok: V::Token, x: V, y: V) -> Option<(V, V)> {
        vector::inverse_unit(self.0, tok, x, y)
    }
}

#[cfg(simd_lanes)]
mod vector {
    use std::f64::consts::FRAC_PI_2;

    use super::Mercator;
    use crate::merc::Model;
    use crate::simd::kernels::{self, DEG_TO_RAD, RAD_TO_DEG};
    use crate::simd::{Mask, Vector, VectorMath};

    /// [`Mercator::forward`] sur `N` points.
    #[inline(always)]
    pub(super) fn forward<V: Vector<N>, const N: usize>(
        m: &Mercator,
        tok: V::Token,
        lon_deg: V,
        lat_deg: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        forward_rad(m, tok, lon_deg * k(DEG_TO_RAD), lat_deg * k(DEG_TO_RAD))
    }

    /// [`Mercator::forward_rad`] sur `N` points.
    #[inline(always)]
    pub(super) fn forward_rad<V: Vector<N>, const N: usize>(
        m: &Mercator,
        tok: V::Token,
        lon_r: V,
        lat_r: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        if !kernels::latitude_in_domain(tok, lat_r)
            .and(lon_r.is_finite())
            .all()
        {
            return None;
        }
        let phi = lat_r.max(k(-FRAC_PI_2)).min(k(FRAC_PI_2));
        let lam = kernels::adjlon(tok, kernels::adjlon(tok, lon_r) - k(m.lam0));
        let y = match m.model {
            Model::Sphere => phi.tan().asinh(),
            Model::Ellipsoid { e } => {
                let (sin_phi, cos_phi) = phi.sin_cos();
                -kernels::tsfn(tok, sin_phi, cos_phi, e).log()
            }
        };
        let x = k(m.k0) * lam;
        let y = k(m.k0) * y;
        // PROJ refuse un point dont une coordonnée n'est pas finie : `NaN`, comme le scalaire.
        let valid = x.is_finite().and(y.is_finite());
        let nan = k(f64::NAN);
        Some((
            V::select(valid, k(m.a).mul_add(x, k(m.x0)), nan),
            V::select(valid, k(m.a).mul_add(y, k(m.y0)), nan),
        ))
    }

    /// [`Mercator::inverse`] sur `N` points.
    #[inline(always)]
    pub(super) fn inverse<V: Vector<N>, const N: usize>(
        m: &Mercator,
        tok: V::Token,
        east: V,
        north: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        let (x, y) = ((east - k(m.x0)) / k(m.a), (north - k(m.y0)) / k(m.a));
        let (lam, phi) = inverse_unit(m, tok, x, y)?;
        let lon = kernels::adjlon(tok, lam + k(m.lam0));
        Some((lon * k(RAD_TO_DEG), phi * k(RAD_TO_DEG)))
    }

    /// [`Mercator::inv_unit`] sur `N` points : lam et phi en radians, lam non ramenée.
    #[inline(always)]
    pub(super) fn inverse_unit<V: Vector<N>, const N: usize>(
        m: &Mercator,
        tok: V::Token,
        x: V,
        y: V,
    ) -> Option<(V, V)> {
        let k = |value| V::splat(tok, value);
        if !x.is_finite().and(y.is_finite()).all() {
            return None;
        }
        let lam = x / k(m.k0);
        let scaled = y / k(m.k0);
        let phi = match m.model {
            Model::Sphere => scaled.sinh().atan(),
            Model::Ellipsoid { e } => kernels::phi2(tok, (-scaled).exp(), e),
        };
        Some((lam, phi))
    }
}
