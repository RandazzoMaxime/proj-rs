//! [`V4`] : quatre `f64` dans un registre AVX2, avec une API sûre.
//!
//! L'invariant qui rend l'API sûre : un [`V4`] ou un [`Mask`] n'est construit qu'à partir d'un
//! [`Avx2`], donc le processeur supporte AVX2. Chaque méthode appelle alors l'intrinsèque
//! correspondante. Le débit dépend d'une autre condition, qui n'est pas de sûreté : le code
//! appelant doit être inséré dans une fonction `target_feature(enable = "avx2,fma")`, ce que
//! fait le driver (`drive.rs`). Hors d'une telle fonction, les mêmes appels restent corrects
//! mais ne sont plus en ligne.

use std::arch::x86_64::{
    __m256d, _mm256_add_pd, _mm256_and_pd, _mm256_andnot_pd, _mm256_blendv_pd, _mm256_cmp_pd,
    _mm256_div_pd, _mm256_floor_pd, _mm256_fmadd_pd, _mm256_loadu_pd, _mm256_max_pd, _mm256_min_pd,
    _mm256_movemask_pd, _mm256_mul_pd, _mm256_set1_pd, _mm256_setzero_pd, _mm256_sqrt_pd,
    _mm256_storeu_pd, _mm256_sub_pd, _mm256_xor_pd, _CMP_EQ_OQ, _CMP_GE_OQ, _CMP_GT_OQ, _CMP_LE_OQ,
    _CMP_LT_OQ, _CMP_TRUE_UQ,
};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Preuve que le processeur supporte AVX2 et FMA.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Avx2(());

impl Avx2 {
    /// La preuve, si ce processeur supporte AVX2 et FMA.
    #[must_use]
    pub(crate) fn detect() -> Option<Self> {
        (std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma"))
            .then_some(Self(()))
    }
}

/// Quatre `f64`.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct V4(pub(super) __m256d);

/// Le résultat d'une comparaison, un bit par voie.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct Mask(__m256d);

impl V4 {
    /// Quatre fois `value`.
    #[inline(always)]
    pub(crate) fn splat(_: Avx2, value: f64) -> Self {
        // SAFETY: `Avx2` prouve le support AVX2.
        Self(unsafe { _mm256_set1_pd(value) })
    }

    /// Quatre zéros.
    #[inline(always)]
    pub(crate) fn zero(_: Avx2) -> Self {
        // SAFETY: `Avx2` prouve le support AVX2.
        Self(unsafe { _mm256_setzero_pd() })
    }

    /// Charge quatre valeurs.
    #[inline(always)]
    pub(crate) fn load(_: Avx2, lanes: &[f64; 4]) -> Self {
        // SAFETY: `Avx2` prouve AVX2 ; `lanes` désigne quatre `f64` lisibles, et `loadu`
        // n'exige pas d'alignement.
        Self(unsafe { _mm256_loadu_pd(lanes.as_ptr()) })
    }

    /// Écrit les quatre voies.
    #[inline(always)]
    pub(crate) fn store(self, lanes: &mut [f64; 4]) {
        // SAFETY: `self` n'existe que si AVX2 est supporté ; `lanes` désigne quatre `f64`
        // inscriptibles, et `storeu` n'exige pas d'alignement.
        unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), self.0) };
    }

    /// Valeur absolue.
    #[inline(always)]
    pub(crate) fn abs(self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let sign = unsafe { _mm256_set1_pd(-0.0) };
        // SAFETY: idem.
        Self(unsafe { _mm256_andnot_pd(sign, self.0) })
    }

    /// Racine carrée.
    #[inline(always)]
    pub(crate) fn sqrt(self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_sqrt_pd(self.0) })
    }

    /// `self * a + b` en une seule opération, arrondie une fois (FMA).
    ///
    /// Le résultat diffère de `self * a + b` en deux opérations par le seul arrondi du produit :
    /// il est plus proche de la valeur exacte, mais pas égal au chemin scalaire, qui n'a pas
    /// d'FMA (`f64::mul_add` y serait logiciel sans FMA matériel).
    #[inline(always)]
    pub(crate) fn mul_add(self, a: Self, b: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 et FMA sont supportés (`Avx2::detect`).
        Self(unsafe { _mm256_fmadd_pd(self.0, a.0, b.0) })
    }

    /// Plus grand entier inférieur ou égal.
    #[inline(always)]
    pub(crate) fn floor(self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_floor_pd(self.0) })
    }

    /// Le plus grand des deux. Une voie `NaN` rend la seconde opérande.
    #[inline(always)]
    pub(crate) fn max(self, other: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_max_pd(self.0, other.0) })
    }

    /// Le plus petit des deux. Une voie `NaN` rend la seconde opérande.
    #[inline(always)]
    pub(crate) fn min(self, other: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_min_pd(self.0, other.0) })
    }

    /// Voie par voie : `self` si le masque est vrai, `other` sinon.
    #[inline(always)]
    pub(crate) fn select(mask: Mask, if_true: Self, if_false: Self) -> Self {
        // SAFETY: `mask` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_blendv_pd(if_false.0, if_true.0, mask.0) })
    }

    /// Voies finies (ni `NaN` ni infinie).
    #[inline(always)]
    pub(crate) fn is_finite(self) -> Mask {
        // Fini : `|x| <= f64::MAX`, faux pour `NaN` et pour l'infini.
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let largest = Self(unsafe { _mm256_set1_pd(f64::MAX) });
        self.abs().le(largest)
    }
}

macro_rules! compare {
    ($name:ident, $predicate:ident, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self, other: Self) -> Mask {
                // SAFETY: `self` n'existe que si AVX2 est supporté.
                Mask(unsafe { _mm256_cmp_pd::<$predicate>(self.0, other.0) })
            }
        }
    };
}

compare!(
    eq,
    _CMP_EQ_OQ,
    "`self == other`, faux si une voie est `NaN`."
);
compare!(
    lt,
    _CMP_LT_OQ,
    "`self < other`, faux si une voie est `NaN`."
);
compare!(
    le,
    _CMP_LE_OQ,
    "`self <= other`, faux si une voie est `NaN`."
);
compare!(
    gt,
    _CMP_GT_OQ,
    "`self > other`, faux si une voie est `NaN`."
);
compare!(
    ge,
    _CMP_GE_OQ,
    "`self >= other`, faux si une voie est `NaN`."
);

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $intrinsic:ident) => {
        impl $trait for V4 {
            type Output = V4;

            #[inline(always)]
            fn $method(self, rhs: V4) -> V4 {
                // SAFETY: `self` n'existe que si AVX2 est supporté.
                V4(unsafe { $intrinsic(self.0, rhs.0) })
            }
        }
    };
}

arithmetic!(Add, add, _mm256_add_pd);
arithmetic!(Sub, sub, _mm256_sub_pd);
arithmetic!(Mul, mul, _mm256_mul_pd);
arithmetic!(Div, div, _mm256_div_pd);

impl Neg for V4 {
    type Output = V4;

    #[inline(always)]
    fn neg(self) -> V4 {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let sign = unsafe { _mm256_set1_pd(-0.0) };
        // SAFETY: idem.
        V4(unsafe { _mm256_xor_pd(self.0, sign) })
    }
}

impl Mask {
    /// Voies vraies des deux masques.
    #[inline(always)]
    pub(crate) fn and(self, other: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_and_pd(self.0, other.0) })
    }

    /// Les voies que le masque ne retient pas.
    #[inline(always)]
    pub(crate) fn not(self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let zero = unsafe { _mm256_setzero_pd() };
        // SAFETY: idem. Le prédicat `TRUE` met tous les bits à un.
        let ones = unsafe { _mm256_cmp_pd::<_CMP_TRUE_UQ>(zero, zero) };
        // SAFETY: idem.
        Self(unsafe { _mm256_xor_pd(self.0, ones) })
    }

    /// Au moins une voie est vraie.
    #[inline(always)]
    pub(crate) fn any(self) -> bool {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        unsafe { _mm256_movemask_pd(self.0) != 0 }
    }

    /// Les quatre voies sont vraies.
    #[inline(always)]
    pub(crate) fn all(self) -> bool {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        unsafe { _mm256_movemask_pd(self.0) == 0b1111 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mul_add` est un vrai FMA : un seul arrondi, celui de `f64::mul_add`. Avec `x = 1 + 2^-30`,
    /// `x * x - 1` vaut exactement `2^-29 + 2^-60`. Deux opérations arrondissent le produit à
    /// `1 + 2^-29` et perdent `2^-60` ; l'FMA le garde.
    #[test]
    fn mul_add_rounds_once() {
        let Some(tok) = Avx2::detect() else {
            return;
        };
        let x = 1.0 + 2.0_f64.powi(-30);
        let exact = 2.0_f64.powi(-29) + 2.0_f64.powi(-60);
        let unfused = x * x - 1.0;
        assert_ne!(unfused, exact, "the test value does tell the two apart");
        assert_eq!(x.mul_add(x, -1.0), exact, "f64::mul_add is the reference");

        let lanes = [x, 0.1, -3.7, 1e150];
        let others = [x, 0.3, 2.25, 1e10];
        let adds = [-1.0, 0.7, 1e-5, -1e160];
        let mut got = [0.0; 4];
        V4::load(tok, &lanes)
            .mul_add(V4::load(tok, &others), V4::load(tok, &adds))
            .store(&mut got);
        for lane in 0..4 {
            let want = lanes[lane].mul_add(others[lane], adds[lane]);
            assert_eq!(
                got[lane].to_bits(),
                want.to_bits(),
                "lane {lane}: {} vs {want}",
                got[lane]
            );
        }
        assert_eq!(got[0], exact);
    }
}
