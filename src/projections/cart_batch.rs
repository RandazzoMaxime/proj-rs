//! `cart` par lots : `geodetic_to_cartesian` et `cartesian_to_geodetic` en quatre voies.
//!
//! `fwd_prepare` et la remise des unités restent ceux du scalaire, point par point ; seul le
//! calcul tourne sur un morceau. Un point que `prepare_forward` refuse garde l'erreur du
//! scalaire. Un groupe de quatre qu'une voie sort du domaine du noyau (valeur non finie, pôle,
//! `cos φ < 1e-6`) passe par [`geodetic_to_cartesian`] et [`cartesian_to_geodetic`] : ses points
//! valent ceux du chemin scalaire.

use super::{
    adjlon, cartesian_to_geodetic, geodetic_to_cartesian, geographic_to_geocentric,
    prepare_forward, CartStep, Coord, TransformError, BATCH_CHUNK,
};
use crate::ellipsoid::Ellipsoid;
use crate::simd::{run3, Lanes3};

struct Forward(Ellipsoid);

struct Inverse(Ellipsoid);

impl Lanes3 for Forward {
    fn point(&self, lam: f64, phi: f64, height: f64) -> (f64, f64, f64) {
        geodetic_to_cartesian(self.0, lam, phi, height)
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Token,
        lam: crate::simd::V4,
        phi: crate::simd::V4,
        height: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4, crate::simd::V4)> {
        vector::forward(self.0, tok, lam, phi, height)
    }
}

impl Lanes3 for Inverse {
    fn point(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        cartesian_to_geodetic(self.0, x, y, z)
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes(
        &self,
        tok: crate::simd::Token,
        x: crate::simd::V4,
        y: crate::simd::V4,
        z: crate::simd::V4,
    ) -> Option<(crate::simd::V4, crate::simd::V4, crate::simd::V4)> {
        vector::inverse(self.0, tok, x, y, z)
    }
}

pub(super) fn forward(
    step: &CartStep,
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    assert_eq!(coords.len(), failed.len(), "one slot per point");
    let frame = &step.frame;
    let kernel = Forward(frame.earth);
    for (coords, failed) in coords
        .chunks_mut(BATCH_CHUNK)
        .zip(failed.chunks_mut(BATCH_CHUNK))
    {
        let count = coords.len();
        let (mut lam, mut phi, mut height) =
            ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        for (slot, (coord, failed)) in coords.iter().zip(failed.iter_mut()).enumerate() {
            if failed.is_some() {
                continue;
            }
            match prepare_forward(frame, coord.x, coord.y) {
                Ok((l, p)) => (lam[slot], phi[slot], height[slot]) = (l, p, coord.z),
                Err(error) => *failed = Some(error),
            }
        }
        let (mut x, mut y, mut z) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        run3(
            &kernel,
            [&lam[..count], &phi[..count], &height[..count]],
            [&mut x[..count], &mut y[..count], &mut z[..count]],
        );
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter()).enumerate() {
            if failed.is_none() {
                *coord = Coord {
                    x: frame.fr_meter * x[slot],
                    y: frame.fr_meter * y[slot],
                    z: frame.fr_meter * z[slot],
                    t: coord.t,
                };
            }
        }
    }
}

pub(super) fn inverse(
    step: &CartStep,
    coords: &mut [Coord],
    failed: &mut [Option<TransformError>],
) {
    assert_eq!(coords.len(), failed.len(), "one slot per point");
    let frame = &step.frame;
    let kernel = Inverse(frame.earth);
    // A point already failed gets an ordinary one, so it does not send its group to the scalar.
    let on_surface = frame.earth.a();
    for (coords, failed) in coords
        .chunks_mut(BATCH_CHUNK)
        .zip(failed.chunks_mut(BATCH_CHUNK))
    {
        let count = coords.len();
        let (mut x, mut y, mut z) = ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        for (slot, (coord, failed)) in coords.iter().zip(failed.iter()).enumerate() {
            if failed.is_some() {
                x[slot] = on_surface;
                continue;
            }
            x[slot] = frame.to_meter * coord.x;
            y[slot] = frame.to_meter * coord.y;
            z[slot] = frame.to_meter * coord.z;
        }
        let (mut lam, mut phi, mut height) =
            ([0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK], [0.0; BATCH_CHUNK]);
        run3(
            &kernel,
            [&x[..count], &y[..count], &z[..count]],
            [&mut lam[..count], &mut phi[..count], &mut height[..count]],
        );
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed.iter()).enumerate() {
            if failed.is_some() {
                continue;
            }
            let mut lon = lam[slot] + frame.from_greenwich + frame.lam0;
            if !frame.over {
                lon = adjlon(lon);
            }
            let mut latitude = phi[slot];
            if frame.geoc {
                latitude = geographic_to_geocentric(latitude, frame.earth.one_es());
            }
            *coord = Coord {
                x: lon,
                y: latitude,
                z: height[slot],
                t: coord.t,
            };
        }
    }
}

#[cfg(simd_lanes)]
mod vector {
    use crate::ellipsoid::Ellipsoid;
    use crate::simd::{Token, V4};

    /// [`super::geodetic_to_cartesian`] sur quatre points.
    #[inline(always)]
    pub(super) fn forward(
        earth: Ellipsoid,
        tok: Token,
        lam: V4,
        phi: V4,
        height: V4,
    ) -> Option<(V4, V4, V4)> {
        let k = |value| V4::splat(tok, value);
        if !lam
            .is_finite()
            .and(phi.is_finite())
            .and(height.is_finite())
            .all()
        {
            return None;
        }
        let (sin_phi, cos_phi) = phi.sin_cos();
        let (sin_lam, cos_lam) = lam.sin_cos();
        let n = if earth.es() == 0.0 {
            k(earth.a())
        } else {
            k(earth.a()) / (k(1.0) - k(earth.es()) * sin_phi * sin_phi).sqrt()
        };
        let radius = (n + height) * cos_phi;
        let z = (n * k(1.0 - earth.es()) + height) * sin_phi;
        Some((radius * cos_lam, radius * sin_lam, z))
    }

    /// [`super::cartesian_to_geodetic`] sur quatre points.
    #[inline(always)]
    pub(super) fn inverse(
        earth: Ellipsoid,
        tok: Token,
        x: V4,
        y: V4,
        z: V4,
    ) -> Option<(V4, V4, V4)> {
        let k = |value| V4::splat(tok, value);
        let b_div = 1.0 - earth.flattening();
        let (x_div, y_div, z_div) = (x * k(earth.ra()), y * k(earth.ra()), z * k(earth.ra()));
        let p_div = x_div.mul_add(x_div, y_div * y_div).sqrt();
        let p_b = p_div * k(b_div);
        let norm = z_div.mul_add(z_div, p_b * p_b).sqrt();
        let inv_norm = k(1.0) / norm;
        let (c, s) = (p_b * inv_norm, z_div * inv_norm);
        let y_phi = (k(earth.e2s() * b_div) * s * s).mul_add(s, z_div);
        let x_phi = p_div - k(earth.es()) * c * c * c;
        let norm_phi = y_phi.mul_add(y_phi, x_phi * x_phi).sqrt();
        let inv_norm_phi = k(1.0) / norm_phi;
        let (cos_phi, sin_phi) = (x_phi * inv_norm_phi, y_phi * inv_norm_phi);
        let phi = (y_phi / x_phi).atan();
        let lam = y_div.atan2(x_div);
        let n = if earth.es() == 0.0 {
            k(earth.a())
        } else {
            k(earth.a()) / (k(1.0) - k(earth.es()) * sin_phi * sin_phi).sqrt()
        };
        let height = (k(earth.a()) * p_div) / cos_phi - n;
        // `x_phi <= 0` (pole), `norm = 0` and `cos φ < 1e-6` have their own branches in the
        // scalar, and a non-finite input has none: all four go there.
        let in_domain = x_phi
            .gt(k(0.0))
            .and(norm.gt(k(0.0)))
            .and(cos_phi.ge(k(1e-6)))
            .and(x_div.is_finite())
            .and(y_div.is_finite())
            .and(z_div.is_finite());
        in_domain.all().then_some((lam, phi, height))
    }
}

#[cfg(test)]
#[path = "cart_batch_tests.rs"]
mod tests;
