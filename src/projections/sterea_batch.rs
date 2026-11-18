//! `sterea` en quatre voies : [`Sterea::forward`] et [`Sterea::inverse`], opération par
//! opération (`gauss.cpp`, `sterea.cpp`). Sort du domaine vectoriel, et passe donc par le scalaire :
//! une valeur non finie, un dénominateur nul, un argument d'`asin` près de ±1, un Newton qui ne
//! converge pas en vingt tours.

use super::Sterea;
use crate::projections::unit_batch::UnitLanes;

#[cfg(target_arch = "x86_64")]
mod vector {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    use super::{Sterea, UnitLanes};
    use crate::projections::sterea::DEL_TOL;
    use crate::simd::kernels;
    use crate::simd::{Avx2, V4};

    /// `srat` de `sterea.rs`.
    #[inline(always)]
    fn srat(tok: Avx2, esinp: V4, ratexp: f64) -> V4 {
        let one = V4::splat(tok, 1.0);
        ((one - esinp) / (one + esinp)).powf(V4::splat(tok, ratexp))
    }

    impl UnitLanes for Sterea {
        #[inline(always)]
        fn forward_lanes(&self, tok: Avx2, lam: V4, phi: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            let g = &self.gauss;
            if !kernels::latitude_in_domain(tok, phi)
                .and(lam.is_finite())
                .all()
            {
                return None;
            }
            let tangent = (k(0.5) * phi + k(FRAC_PI_4)).tan();
            let chi = k(2.0)
                * (k(g.k) * tangent.powf(k(g.c)) * srat(tok, k(g.e) * phi.sin_cos().0, g.ratexp))
                    .atan()
                - k(FRAC_PI_2);
            let (sinc, cosc) = chi.sin_cos();
            let (sinl, cosl) = (k(g.c) * lam).sin_cos();
            let denom = k(1.0) + k(self.sinc0) * sinc + k(self.cosc0) * cosc * cosl;
            let scale = k(self.k0) * k(self.r2) / denom;
            let east = scale * cosc * sinl;
            let north = scale * (k(self.cosc0) * sinc - k(self.sinc0) * cosc * cosl);
            // A zero denominator is the scalar's refused point.
            let usable = denom
                .abs()
                .gt(k(0.0))
                .and(east.is_finite())
                .and(north.is_finite());
            usable.all().then_some((east, north))
        }

        #[inline(always)]
        fn inverse_lanes(&self, tok: Avx2, x: V4, y: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            let g = &self.gauss;
            let (x, y) = (x / k(self.k0), y / k(self.k0));
            let rho = x.hypot(y);
            let (sinc, cosc) = (k(2.0) * rho.atan2(k(self.r2))).sin_cos();
            let sine_phi = cosc * k(self.sinc0) + y * sinc * k(self.cosc0) / rho;
            let lam = (x * sinc).atan2(rho * k(self.cosc0) * cosc - y * k(self.sinc0) * sinc);
            // The origin of the projection (`rho = 0`) has its own branch in the scalar.
            let in_domain = rho
                .gt(k(0.0))
                .and(kernels::asin_in_domain(tok, sine_phi))
                .and(lam.is_finite());
            if !in_domain.all() {
                return None;
            }
            let mut phi = kernels::asin(tok, sine_phi);
            let num = ((k(0.5) * phi + k(FRAC_PI_4)).tan() / k(g.k)).powf(k(1.0 / g.c));
            let tolerance = k(DEL_TOL);
            let mut done = tolerance.lt(k(0.0));
            for _ in 0..20 {
                let next = k(2.0) * (num * srat(tok, k(g.e) * phi.sin_cos().0, -0.5 * g.e)).atan()
                    - k(FRAC_PI_2);
                let converged = (next - phi).abs().lt(tolerance);
                phi = V4::select(done, phi, next);
                done = done.or(converged);
                if done.all() {
                    break;
                }
            }
            done.all().then_some((lam / k(g.c), phi))
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl UnitLanes for Sterea {}

#[cfg(test)]
#[path = "sterea_batch_tests.rs"]
mod tests;
