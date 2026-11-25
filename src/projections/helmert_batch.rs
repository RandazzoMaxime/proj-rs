//! `helmert` et `molobadekas` par lots, pour une transformation sans dérive dans le temps.
//!
//! Sans taux (`dx`, `drx`, `ds`…), la matrice, l'échelle et les translations ne dépendent pas
//! du point : le scalaire les refait pourtant à chaque point (`rotation_matrix`). Ici ils sont
//! calculés une fois à la création, et un morceau de points passe par un noyau à `N` voies
//! qui suit `spatial_kernel` opération par opération. Une transformation à taux, ou la
//! variante plane (`theta`), garde la boucle scalaire.

use super::{rotation_matrix, Coord, EpochShift, Law, TransformError};
use crate::simd::{run3, Lanes3, Vector};

const CHUNK: usize = 256;

/// Les constantes d'un Helmert spatial sans taux.
#[derive(Clone, Copy)]
pub(super) struct Fixed {
    matrix: [[f64; 3]; 3],
    factor: f64,
    translation: [f64; 3],
    reference: [f64; 3],
}

impl Fixed {
    /// `None` quand la transformation dépend de l'époque, ou n'est pas spatiale.
    pub(super) fn new(shift: &EpochShift) -> Option<Self> {
        let Law::Spatial {
            angles,
            convention,
            ppm,
            dppm,
            rx,
            ry,
            rz,
            drx,
            dry,
            drz,
            px,
            py,
            pz,
        } = shift.law
        else {
            return None;
        };
        let rates = [shift.dtx, shift.dty, shift.dtz, dppm, drx, dry, drz];
        if rates.iter().any(|rate| *rate != 0.0) || !shift.t_epoch.is_finite() {
            return None;
        }
        Some(Self {
            matrix: rotation_matrix(rx, ry, rz, angles, convention),
            factor: 1.0 + ppm * 1e-6,
            translation: [shift.tx, shift.ty, shift.tz],
            reference: [px, py, pz],
        })
    }

    /// `spatial_kernel`, sens aller.
    fn forward(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let m = &self.matrix;
        let (u, v, w) = (
            x - self.reference[0],
            y - self.reference[1],
            z - self.reference[2],
        );
        let rotated = (
            m[0][0] * u + m[0][1] * v + m[0][2] * w,
            m[1][0] * u + m[1][1] * v + m[1][2] * w,
            m[2][0] * u + m[2][1] * v + m[2][2] * w,
        );
        (
            self.factor * rotated.0 + self.translation[0],
            self.factor * rotated.1 + self.translation[1],
            self.factor * rotated.2 + self.translation[2],
        )
    }

    /// `spatial_kernel`, sens inverse : la transposée, pas l'inverse de la matrice.
    fn inverse(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let m = &self.matrix;
        let u = (x - self.translation[0]) / self.factor;
        let v = (y - self.translation[1]) / self.factor;
        let w = (z - self.translation[2]) / self.factor;
        (
            m[0][0] * u + m[1][0] * v + m[2][0] * w + self.reference[0],
            m[0][1] * u + m[1][1] * v + m[2][1] * w + self.reference[1],
            m[0][2] * u + m[1][2] * v + m[2][2] * w + self.reference[2],
        )
    }
}

struct Forward(Fixed);

struct Inverse(Fixed);

impl Lanes3 for Forward {
    fn point(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        self.0.forward(x, y, z)
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        x: V,
        y: V,
        z: V,
    ) -> Option<(V, V, V)> {
        let fixed = &self.0;
        let k = |value| V::splat(tok, value);
        let m = &fixed.matrix;
        let u = x - k(fixed.reference[0]);
        let v = y - k(fixed.reference[1]);
        let w = z - k(fixed.reference[2]);
        let row = |r: usize, offset: usize| {
            k(m[r][0])
                .mul_add(u, k(m[r][1]).mul_add(v, k(m[r][2]) * w))
                .mul_add(k(fixed.factor), k(fixed.translation[offset]))
        };
        Some((row(0, 0), row(1, 1), row(2, 2)))
    }
}

impl Lanes3 for Inverse {
    fn point(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        self.0.inverse(x, y, z)
    }

    #[cfg(simd_lanes)]
    #[inline(always)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        x: V,
        y: V,
        z: V,
    ) -> Option<(V, V, V)> {
        let fixed = &self.0;
        let k = |value| V::splat(tok, value);
        let m = &fixed.matrix;
        let u = (x - k(fixed.translation[0])) / k(fixed.factor);
        let v = (y - k(fixed.translation[1])) / k(fixed.factor);
        let w = (z - k(fixed.translation[2])) / k(fixed.factor);
        let column = |c: usize| {
            k(m[0][c]).mul_add(u, k(m[1][c]).mul_add(v, k(m[2][c]) * w)) + k(fixed.reference[c])
        };
        Some((column(0), column(1), column(2)))
    }
}

/// Un morceau de points à la fois : en mètres vers le noyau, puis les unités de sortie.
pub(super) fn run(
    fixed: Fixed,
    forward: bool,
    to_meter: f64,
    fr_meter: f64,
    coords: &mut [Coord],
    failed: &[Option<TransformError>],
) {
    for (coords, failed) in coords.chunks_mut(CHUNK).zip(failed.chunks(CHUNK)) {
        let count = coords.len();
        let (mut x, mut y, mut z) = ([0.0; CHUNK], [0.0; CHUNK], [0.0; CHUNK]);
        for (slot, coord) in coords.iter().enumerate() {
            // Forward reads metres as they are: the scalar multiplies by exactly 1.
            let meter = if forward { 1.0 } else { to_meter };
            (x[slot], y[slot], z[slot]) = (meter * coord.x, meter * coord.y, meter * coord.z);
        }
        let (mut ox, mut oy, mut oz) = ([0.0; CHUNK], [0.0; CHUNK], [0.0; CHUNK]);
        let input = [&x[..count], &y[..count], &z[..count]];
        let output = [&mut ox[..count], &mut oy[..count], &mut oz[..count]];
        if forward {
            run3(&Forward(fixed), input, output);
        } else {
            run3(&Inverse(fixed), input, output);
        }
        let out = if forward { fr_meter } else { 1.0 };
        for (slot, (coord, failed)) in coords.iter_mut().zip(failed).enumerate() {
            if failed.is_none() {
                *coord = Coord {
                    x: out * ox[slot],
                    y: out * oy[slot],
                    z: out * oz[slot],
                    t: coord.t,
                };
            }
        }
    }
}

#[cfg(test)]
#[path = "helmert_batch_tests.rs"]
mod tests;
