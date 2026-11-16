//! [`V4`] : quatre `f64` dans un registre AVX2, avec une API sûre.
//!
//! L'invariant qui rend l'API sûre : un [`V4`] ou un [`Mask`] n'est construit qu'à partir d'un
//! [`Avx2`], donc le processeur supporte AVX2. Chaque méthode appelle alors l'intrinsèque
//! correspondante. Le débit dépend d'une autre condition, qui n'est pas de sûreté : le code
//! appelant doit être inséré dans une fonction `target_feature(enable = "avx2,fma")`, ce que
//! fait le driver (`drive.rs`). Hors d'une telle fonction, les mêmes appels restent corrects
//! mais ne sont plus en ligne.

use std::arch::x86_64::{
    __m256d, _mm256_add_epi64, _mm256_add_pd, _mm256_and_pd, _mm256_and_si256, _mm256_andnot_pd,
    _mm256_blendv_pd, _mm256_castpd_si256, _mm256_castsi256_pd, _mm256_cmp_pd, _mm256_div_pd,
    _mm256_floor_pd, _mm256_fmadd_pd, _mm256_loadu_pd, _mm256_max_pd, _mm256_min_pd,
    _mm256_movemask_pd, _mm256_mul_pd, _mm256_or_pd, _mm256_or_si256, _mm256_round_pd,
    _mm256_set1_epi64x, _mm256_set1_pd, _mm256_setzero_pd, _mm256_slli_epi64, _mm256_sqrt_pd,
    _mm256_srli_epi64, _mm256_storeu_pd, _mm256_sub_epi64, _mm256_sub_pd, _mm256_xor_pd,
    _CMP_EQ_OQ, _CMP_GE_OQ, _CMP_GT_OQ, _CMP_LE_OQ, _CMP_LT_OQ, _CMP_TRUE_UQ, _MM_FROUND_NO_EXC,
    _MM_FROUND_TO_NEAREST_INT,
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

    /// Quatre fois `value`, sans [`Avx2`] : `self` en est la preuve.
    #[inline(always)]
    pub(super) fn splat_like(self, value: f64) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_set1_pd(value) })
    }

    /// Un registre de ces quatre valeurs, sans [`Avx2`] : `self` en est la preuve.
    #[inline(always)]
    pub(super) fn reload(self, lanes: &[f64; 4]) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté ; `lanes` désigne quatre `f64`.
        Self(unsafe { _mm256_loadu_pd(lanes.as_ptr()) })
    }

    /// Les quatre voies, pour un calcul voie par voie.
    #[inline(always)]
    pub(crate) fn to_array(self) -> [f64; 4] {
        let mut lanes = [0.0; 4];
        self.store(&mut lanes);
        lanes
    }

    /// L'entier le plus proche, au pair en cas d'égalité (`vroundpd`).
    #[inline(always)]
    pub(crate) fn round(self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe {
            _mm256_round_pd::<{ _MM_FROUND_TO_NEAREST_INT | _MM_FROUND_NO_EXC }>(self.0)
        })
    }

    /// La grandeur de `self` avec le signe de `sign` (un zéro négatif compris).
    #[inline(always)]
    pub(crate) fn copysign(self, sign: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let sign_bit = unsafe { _mm256_set1_pd(-0.0) };
        // SAFETY: idem.
        let magnitude = unsafe { _mm256_andnot_pd(sign_bit, self.0) };
        // SAFETY: idem.
        let sign_only = unsafe { _mm256_and_pd(sign_bit, sign.0) };
        // SAFETY: idem. Les deux moitiés n'ont aucun bit en commun : un OU les réunit.
        Self(unsafe { _mm256_or_pd(magnitude, sign_only) })
    }

    /// Les voies dont le bit de signe est à un : négatifs, zéro négatif, `NaN` de signe négatif.
    #[inline(always)]
    pub(crate) fn sign_mask(self) -> Mask {
        // `blendv` et `movemask` ne lisent que le bit de signe : le registre est son propre masque.
        Mask(self.0)
    }

    /// `self * 2^k` pour des `k` entiers, `|k| <= 2000`, en deux multiplications pour que la
    /// puissance de deux reste représentable.
    #[inline(always)]
    pub(super) fn scale_pow2(self, k: Self) -> Self {
        let half = (k * self.splat_like(0.5)).round();
        self * half.pow2() * (k - half).pow2()
    }

    /// `2^k` pour un `k` entier de `[-1022, 1023]`, par l'exposant : `(k + 1023) << 52`.
    #[inline(always)]
    fn pow2(self) -> Self {
        let bits = self.integer_bits();
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let bias = unsafe { _mm256_set1_epi64x(1023) };
        // SAFETY: idem.
        let biased = unsafe { _mm256_add_epi64(bits, bias) };
        // SAFETY: idem.
        let shifted = unsafe { _mm256_slli_epi64::<52>(biased) };
        // SAFETY: idem.
        Self(unsafe { _mm256_castsi256_pd(shifted) })
    }

    /// La valeur entière de `self`, supposée entière et de valeur absolue inférieure à `2^51`,
    /// en entier 64 bits : `self + 1.5 * 2^52` a `self` dans ses bits de mantisse.
    #[inline(always)]
    fn integer_bits(self) -> std::arch::x86_64::__m256i {
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        let sum = self + shifter;
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let sum_bits = unsafe { _mm256_castpd_si256(sum.0) };
        // SAFETY: idem.
        let shifter_bits = unsafe { _mm256_castpd_si256(shifter.0) };
        // SAFETY: idem.
        unsafe { _mm256_sub_epi64(sum_bits, shifter_bits) }
    }

    /// Le masque des voies dont l'entier (`self` est entier, `|self| < 2^51`) a à un le bit
    /// `63 - SHIFT` : `SHIFT = 63` pour le bit 0, `SHIFT = 62` pour le bit 1.
    #[inline(always)]
    pub(super) fn integer_bit_mask<const SHIFT: i32>(self) -> Mask {
        let bits = self.integer_bits();
        // SAFETY: `self` n'existe que si AVX2 est supporté. Le bit visé monte en position de
        // signe, la seule que `blendv` et `movemask` lisent.
        let moved = unsafe { _mm256_slli_epi64::<SHIFT>(bits) };
        // SAFETY: idem.
        Mask(unsafe { _mm256_castsi256_pd(moved) })
    }

    /// Mantisse et exposant d'un `f64` positif normal : `self = m * 2^e` avec `m` dans `[1, 2[`.
    /// `e` est rendu en `f64`. Pour une valeur nulle, négative, sous-normale, infinie ou `NaN`,
    /// le résultat n'a pas de sens : l'appelant écarte ces voies.
    #[inline(always)]
    pub(super) fn split_exponent(self) -> (Self, Self) {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let bits = unsafe { _mm256_castpd_si256(self.0) };
        // SAFETY: idem.
        let exponent_field = unsafe { _mm256_srli_epi64::<52>(bits) };
        // SAFETY: idem.
        let mantissa_mask = unsafe { _mm256_set1_epi64x(0x000F_FFFF_FFFF_FFFF) };
        // SAFETY: idem.
        let mantissa = unsafe { _mm256_and_si256(bits, mantissa_mask) };
        // SAFETY: idem.
        let one_bits = unsafe { _mm256_set1_epi64x(0x3FF0_0000_0000_0000) };
        // SAFETY: idem.
        let normalized = unsafe { _mm256_or_si256(mantissa, one_bits) };
        // SAFETY: idem.
        let mantissa_value = Self(unsafe { _mm256_castsi256_pd(normalized) });
        // e = field - 1023, reconverti en f64 par la même astuce de `1.5 * 2^52`.
        // SAFETY: idem.
        let bias = unsafe { _mm256_set1_epi64x(1023) };
        // SAFETY: idem.
        let exponent = unsafe { _mm256_sub_epi64(exponent_field, bias) };
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        // SAFETY: idem.
        let shifter_bits = unsafe { _mm256_castpd_si256(shifter.0) };
        // SAFETY: idem.
        let summed = unsafe { _mm256_add_epi64(shifter_bits, exponent) };
        // SAFETY: idem.
        let exponent_value = Self(unsafe { _mm256_castsi256_pd(summed) }) - shifter;
        (mantissa_value, exponent_value)
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

    /// Voies vraies d'au moins un des deux masques.
    #[inline(always)]
    pub(crate) fn or(self, other: Self) -> Self {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        Self(unsafe { _mm256_or_pd(self.0, other.0) })
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

    /// Un bit par voie, le bit 0 pour la première : les voies vraies.
    #[inline(always)]
    pub(crate) fn bits(self) -> u32 {
        // SAFETY: `self` n'existe que si AVX2 est supporté.
        let bits = unsafe { _mm256_movemask_pd(self.0) };
        u32::try_from(bits).unwrap_or(0)
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
