//! Mercator par lots, quatre points à la fois, sphère et ellipsoïde.
//!
//! Les formules sont celles de [`Mercator::fwd_unit`] et [`Mercator::inv_unit`], écrites contre
//! [`crate::simd::V4`]. Un groupe de quatre qu'une voie sort du domaine du noyau (valeur non
//! finie, latitude hors de ±90°) passe par le chemin scalaire : ses points valent exactement
//! ceux de [`Mercator::forward`] et [`Mercator::inverse`].

use super::Mercator;
use crate::simd::Lanes;

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
    fn lanes(
        &self,
        tok: crate::simd::Token,
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

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Token,
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

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Token,
        east: crate::simd::V4,
        north: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        vector::inverse(self.0, tok, east, north)
    }
}

impl Lanes for InverseUnit<'_> {
    fn point(&self, x: f64, y: f64) -> (f64, f64) {
        self.0.inv_unit(x, y).unwrap_or((f64::NAN, f64::NAN))
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Token,
        x: crate::simd::V4,
        y: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        vector::inverse_unit(self.0, tok, x, y)
    }
}

#[cfg(simd_lanes)]
mod vector {
    use std::f64::consts::FRAC_PI_2;

    use super::Mercator;
    use crate::merc::Model;
    use crate::simd::kernels::{self, DEG_TO_RAD, RAD_TO_DEG};
    use crate::simd::{Token, V4};

    /// [`Mercator::forward`] sur quatre points.
    #[inline(always)]
    pub(super) fn forward(m: &Mercator, tok: Token, lon_deg: V4, lat_deg: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
        forward_rad(m, tok, lon_deg * k(DEG_TO_RAD), lat_deg * k(DEG_TO_RAD))
    }

    /// [`Mercator::forward_rad`] sur quatre points.
    #[inline(always)]
    pub(super) fn forward_rad(m: &Mercator, tok: Token, lon_r: V4, lat_r: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
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
            V4::select(valid, k(m.a).mul_add(x, k(m.x0)), nan),
            V4::select(valid, k(m.a).mul_add(y, k(m.y0)), nan),
        ))
    }

    /// [`Mercator::inverse`] sur quatre points.
    #[inline(always)]
    pub(super) fn inverse(m: &Mercator, tok: Token, east: V4, north: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
        let (x, y) = ((east - k(m.x0)) / k(m.a), (north - k(m.y0)) / k(m.a));
        let (lam, phi) = inverse_unit(m, tok, x, y)?;
        let lon = kernels::adjlon(tok, lam + k(m.lam0));
        Some((lon * k(RAD_TO_DEG), phi * k(RAD_TO_DEG)))
    }

    /// [`Mercator::inv_unit`] sur quatre points : lam et phi en radians, lam non ramenée.
    #[inline(always)]
    pub(super) fn inverse_unit(m: &Mercator, tok: Token, x: V4, y: V4) -> Option<(V4, V4)> {
        let k = |value| V4::splat(tok, value);
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
