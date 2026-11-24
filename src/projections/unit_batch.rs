//! Le chemin par lots d'un noyau `Classic` écrit en unités du demi-grand axe (`sterea`,
//! `somerc`, `laea`) : le noyau ne donne que sa formule en `N` voies ; la mise à l'échelle
//! (`a`, `1/a`) et les degrés sont ici, comme `Classic` les fait. Un groupe de `N` qu'une voie
//! sort du domaine du noyau passe par [`Kernel`] : ses points valent ceux du chemin scalaire.

use crate::ellipsoid::Ellipsoid;
use crate::engine::{Kernel, ProjectedBatch};
use crate::simd::{run, Lanes, Vector};

/// Un noyau `Classic` avec sa formule en `N` voies.
pub(crate) trait UnitLanes: Kernel + Copy + 'static {
    /// [`Kernel::forward`] sur `N` points : radians relatifs au méridien central vers unités
    /// du demi-grand axe. `None` si une voie sort du domaine que le noyau vectoriel traite.
    #[cfg(simd_lanes)]
    fn forward_lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        lam: V,
        phi: V,
    ) -> Option<(V, V)>;

    /// [`Kernel::inverse`] sur `N` points : unités du demi-grand axe vers radians.
    #[cfg(simd_lanes)]
    fn inverse_lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        x: V,
        y: V,
    ) -> Option<(V, V)>;
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

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(&self, tok: V::Token, lam: V, phi: V) -> Option<(V, V)> {
        let a = V::splat(tok, self.1.a());
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

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        east: V,
        north: V,
    ) -> Option<(V, V)> {
        use crate::simd::kernels::RAD_TO_DEG;
        let ra = V::splat(tok, self.1.ra());
        let to_degrees = V::splat(tok, RAD_TO_DEG);
        let (lam, phi) = self.0.inverse_lanes(tok, east * ra, north * ra)?;
        Some((lam * to_degrees, phi * to_degrees))
    }
}
