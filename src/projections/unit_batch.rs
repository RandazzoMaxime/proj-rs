//! Le chemin par lots d'un noyau `Classic` écrit en unités du demi-grand axe (`sterea`,
//! `somerc`, `laea`) : le noyau ne donne que sa formule en quatre voies ; la mise à l'échelle
//! (`a`, `1/a`) et les degrés sont ici, comme `Classic` les fait. Un groupe de quatre qu'une voie
//! sort du domaine du noyau passe par [`Kernel`] : ses points valent ceux du chemin scalaire.

use crate::ellipsoid::Ellipsoid;
use crate::engine::{Kernel, ProjectedBatch};
use crate::simd::{run, Lanes};

/// Un noyau `Classic` avec sa formule en quatre voies.
pub(crate) trait UnitLanes: Kernel + Copy + 'static {
    /// [`Kernel::forward`] sur quatre points : radians relatifs au méridien central vers unités
    /// du demi-grand axe. `None` si une voie sort du domaine que le noyau vectoriel traite.
    #[cfg(target_arch = "x86_64")]
    fn forward_lanes(
        &self,
        tok: crate::simd::Avx2,
        lam: crate::simd::V4,
        phi: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)>;

    /// [`Kernel::inverse`] sur quatre points : unités du demi-grand axe vers radians.
    #[cfg(target_arch = "x86_64")]
    fn inverse_lanes(
        &self,
        tok: crate::simd::Avx2,
        x: crate::simd::V4,
        y: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)>;
}

/// Le chemin par lots de `K` sur son ellipsoïde.
pub(crate) struct UnitBatch<K>(pub(crate) K, pub(crate) Ellipsoid);

struct Forward<K>(K, Ellipsoid);

struct Inverse<K>(K, Ellipsoid);

impl<K: UnitLanes + Send + Sync> ProjectedBatch for UnitBatch<K> {
    fn forward(&self, lon_rad: &[f64], lat_rad: &[f64], east: &mut [f64], north: &mut [f64]) {
        run(&Forward(self.0, self.1), lon_rad, lat_rad, east, north);
    }

    fn inverse(&self, east: &[f64], north: &[f64], lon_deg: &mut [f64], lat_deg: &mut [f64]) {
        run(&Inverse(self.0, self.1), east, north, lon_deg, lat_deg);
    }
}

impl<K: UnitLanes> Lanes for Forward<K> {
    fn point(&self, lam: f64, phi: f64) -> (f64, f64) {
        self.0
            .forward(lam, phi)
            .map_or((f64::NAN, f64::NAN), |(x, y)| {
                (self.1.a() * x, self.1.a() * y)
            })
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Avx2,
        lam: crate::simd::V4,
        phi: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        let a = crate::simd::V4::splat(tok, self.1.a());
        let (x, y) = self.0.forward_lanes(tok, lam, phi)?;
        Some((a * x, a * y))
    }
}

impl<K: UnitLanes> Lanes for Inverse<K> {
    fn point(&self, east: f64, north: f64) -> (f64, f64) {
        let ra = self.1.ra();
        self.0
            .inverse(east * ra, north * ra)
            .map_or((f64::NAN, f64::NAN), |(lam, phi)| {
                (lam.to_degrees(), phi.to_degrees())
            })
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Avx2,
        east: crate::simd::V4,
        north: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4)> {
        use crate::simd::kernels::RAD_TO_DEG;
        let ra = crate::simd::V4::splat(tok, self.1.ra());
        let to_degrees = crate::simd::V4::splat(tok, RAD_TO_DEG);
        let (lam, phi) = self.0.inverse_lanes(tok, east * ra, north * ra)?;
        Some((lam * to_degrees, phi * to_degrees))
    }
}
