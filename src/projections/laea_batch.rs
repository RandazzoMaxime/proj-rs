//! `laea` ellipsoïdal, modes oblique et équatorial, en quatre voies : `Laea::forward_ellipsoid`
//! et `Laea::inverse_ellipsoid`, opération par opération, avec la série authalique de
//! `projmath` (coefficients calculés une fois à la création, le scalaire les refait par point).
//! Les modes polaires, la sphère et un `n` hors de la série n'ont pas de noyau vectoriel. Un groupe
//! de quatre qu'une voie sort du domaine passe par le scalaire : une valeur non finie, un
//! dénominateur sous `EPS10`, un argument d'`asin` près de ±1, le centre (`rho < EPS10`).

use super::{Laea, Mode, EPS10};
use crate::projections::unit_batch::UnitLanes;

#[cfg(target_arch = "x86_64")]
mod vector {
    use super::{Laea, Mode, UnitLanes, EPS10};
    use crate::simd::kernels;
    use crate::simd::{Avx2, V4};

    impl UnitLanes for Laea {
        #[inline(always)]
        fn forward_lanes(&self, tok: Avx2, lam: V4, phi: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            if !kernels::latitude_in_domain(tok, phi)
                .and(lam.is_finite())
                .all()
            {
                return None;
            }
            let (sin_phi, cos_phi) = phi.sin_cos();
            let xi = kernels::auxlat_convert(tok, phi, sin_phi, cos_phi, &self.fwd_coeffs);
            let (sin_lam, cos_lam) = lam.sin_cos();
            let (sinb, cosb) = xi.sin_cos();
            let b = match self.mode {
                Mode::Oblique => k(1.0) + k(self.sinb1) * sinb + k(self.cosb1) * cosb * cos_lam,
                _ => k(1.0) + cosb * cos_lam,
            };
            if !b.abs().ge(k(EPS10)).all() {
                return None;
            }
            let b = (k(2.0) / b).sqrt();
            let east = k(self.xmf) * b * cosb * sin_lam;
            let north = match self.mode {
                Mode::Oblique => {
                    k(self.ymf) * b * (k(self.cosb1) * sinb - k(self.sinb1) * cosb * cos_lam)
                }
                _ => b * sinb * k(self.ymf),
            };
            east.is_finite()
                .and(north.is_finite())
                .all()
                .then_some((east, north))
        }

        #[inline(always)]
        fn inverse_lanes(&self, tok: Avx2, x: V4, y: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            let (x, y) = (x / k(self.dd), y * k(self.dd));
            let rho = x.hypot(y);
            let argument = k(0.5) * rho / k(self.rq);
            let in_domain = rho
                .is_finite()
                .and(rho.ge(k(EPS10)))
                .and(kernels::asin_in_domain(tok, argument));
            if !in_domain.all() {
                return None;
            }
            let (sce, cce) = (k(2.0) * kernels::asin(tok, argument)).sin_cos();
            let x = x * sce;
            let (ab, y) = match self.mode {
                Mode::Oblique => (
                    cce * k(self.sinb1) + y * sce * k(self.cosb1) / rho,
                    rho * k(self.cosb1) * cce - y * k(self.sinb1) * sce,
                ),
                _ => (y * sce / rho, rho * cce),
            };
            if !kernels::asin_in_domain(tok, ab).all() {
                return None;
            }
            let beta = kernels::asin(tok, ab);
            let (sin_beta, cos_beta) = beta.sin_cos();
            let phi = kernels::auxlat_convert(tok, beta, sin_beta, cos_beta, &self.inv_coeffs);
            let lam = x.atan2(y);
            lam.is_finite()
                .and(phi.is_finite())
                .all()
                .then_some((lam, phi))
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl UnitLanes for Laea {}

#[cfg(test)]
#[path = "laea_batch_tests.rs"]
mod tests;
