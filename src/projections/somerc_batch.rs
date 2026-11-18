//! `somerc` en quatre voies : [`Somerc::forward`] et [`Somerc::inverse`], opération par
//! opération. Passe par le scalaire, pour un groupe de quatre : une valeur non finie, un argument
//! d'`asin` près de ±1, un Newton qui ne converge pas en six tours.

use super::Somerc;
use crate::projections::unit_batch::UnitLanes;

#[cfg(target_arch = "x86_64")]
mod vector {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    use super::{Somerc, UnitLanes};
    use crate::simd::kernels;
    use crate::simd::{Avx2, V4};

    /// `ln(tan(π/4 + φ/2))`, la latitude isométrique.
    #[inline(always)]
    fn isometric(tok: Avx2, phi: V4) -> V4 {
        (V4::splat(tok, FRAC_PI_4) + V4::splat(tok, 0.5) * phi)
            .tan()
            .log()
    }

    /// `ln((1 + sp) / (1 - sp))`.
    #[inline(always)]
    fn log_ratio(tok: Avx2, sp: V4) -> V4 {
        let one = V4::splat(tok, 1.0);
        ((one + sp) / (one - sp)).log()
    }

    impl UnitLanes for Somerc {
        #[inline(always)]
        fn forward_lanes(&self, tok: Avx2, lam: V4, phi: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            if !kernels::latitude_in_domain(tok, phi)
                .and(lam.is_finite())
                .all()
            {
                return None;
            }
            let sp = k(self.e) * phi.sin_cos().0;
            let inner = k(self.c) * (isometric(tok, phi) - k(self.hlf_e) * log_ratio(tok, sp));
            let phip = k(2.0) * (inner + k(self.k)).exp().atan() - k(FRAC_PI_2);
            let (sin_phip, cp) = phip.sin_cos();
            let (sin_lamp, cos_lamp) = (k(self.c) * lam).sin_cos();
            let sine_phipp = k(self.cosp0) * sin_phip - k(self.sinp0) * cp * cos_lamp;
            if !kernels::asin_in_domain(tok, sine_phipp).all() {
                return None;
            }
            let phipp = kernels::asin(tok, sine_phipp);
            let sine_lampp = cp * sin_lamp / phipp.sin_cos().1;
            if !kernels::asin_in_domain(tok, sine_lampp).all() {
                return None;
            }
            let lampp = kernels::asin(tok, sine_lampp);
            let east = k(self.kr) * lampp;
            let north = k(self.kr) * isometric(tok, phipp);
            east.is_finite()
                .and(north.is_finite())
                .all()
                .then_some((east, north))
        }

        #[inline(always)]
        fn inverse_lanes(&self, tok: Avx2, x: V4, y: V4) -> Option<(V4, V4)> {
            let k = |value| V4::splat(tok, value);
            let phipp = k(2.0) * ((y / k(self.kr)).exp().atan() - k(FRAC_PI_4));
            let lampp = x / k(self.kr);
            let (sin_phipp, cp) = phipp.sin_cos();
            let (sin_lampp, cos_lampp) = lampp.sin_cos();
            let sine_phip = k(self.cosp0) * sin_phipp + k(self.sinp0) * cp * cos_lampp;
            if !kernels::asin_in_domain(tok, sine_phip).all() {
                return None;
            }
            let mut phip = kernels::asin(tok, sine_phip);
            let sine_lamp = cp * sin_lampp / phip.sin_cos().1;
            if !kernels::asin_in_domain(tok, sine_lamp).all() {
                return None;
            }
            let lamp = kernels::asin(tok, sine_lamp);
            let con = (k(self.k) - isometric(tok, phip)) / k(self.c);
            if !con.is_finite().all() {
                return None;
            }
            let mut done = k(0.0).gt(k(0.0));
            for _ in 0..6 {
                let (sin_phip, cos_phip) = phip.sin_cos();
                let esp = k(self.e) * sin_phip;
                let delp = (con + isometric(tok, phip) - k(self.hlf_e) * log_ratio(tok, esp))
                    * (k(1.0) - esp * esp)
                    * cos_phip
                    * k(self.rone_es);
                phip = V4::select(done, phip, phip - delp);
                done = done.or(delp.abs().lt(k(1e-10)));
                if done.all() {
                    break;
                }
            }
            done.all().then_some((lamp / k(self.c), phip))
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl UnitLanes for Somerc {}

#[cfg(test)]
#[path = "somerc_batch_tests.rs"]
mod tests;
