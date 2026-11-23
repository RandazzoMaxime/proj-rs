//! [`V4`] : quatre `f64` dans un registre AVX2, avec une API sûre.
//!
//! Les intrinsèques passent par `pulp` (`pulp::x86::V3`) : un jeton que l'on n'obtient qu'après
//! avoir détecté le processeur, et dont les méthodes appellent l'intrinsèque du même nom. C'est
//! `pulp` qui porte le `unsafe` ; ce module n'en a pas. Un [`V4`] ou un [`Mask`] garde son jeton
//! (de taille nulle) : il n'existe donc qu'à partir d'un [`Avx2`]. Le débit dépend d'une autre
//! condition, qui n'est pas de sûreté : le code appelant doit être inséré dans une fonction
//! `target_feature`, ce que fait le driver (`drive.rs`, par [`Avx2::vectorize`]). Hors d'une
//! telle fonction, les mêmes appels restent corrects mais ne sont plus en ligne.

use pulp::bytemuck;
use pulp::x86::V3;
use std::arch::x86_64::{
    __m256d, __m256i, _CMP_EQ_OQ, _CMP_GE_OQ, _CMP_GT_OQ, _CMP_LE_OQ, _CMP_LT_OQ, _CMP_TRUE_UQ,
    _MM_FROUND_NO_EXC, _MM_FROUND_TO_NEAREST_INT,
};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Preuve que le processeur supporte AVX2 et FMA (le niveau x86-64-v3 de `pulp` : AVX, AVX2,
/// FMA, BMI1, BMI2, LZCNT, POPCNT et les SSE, ce qu'ont tous les processeurs AVX2 et FMA).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Avx2(V3);

impl Avx2 {
    /// La preuve, si ce processeur supporte AVX2 et FMA.
    #[must_use]
    pub(crate) fn detect() -> Option<Self> {
        V3::try_new().map(Self)
    }

    /// Exécute `f` dans une fonction compilée avec les extensions du jeton (`avx2`, `fma`, …) :
    /// les méthodes `#[inline(always)]` de [`V4`] y deviennent des instructions en ligne.
    #[inline(always)]
    pub(super) fn vectorize<F: pulp::NullaryFnOnce>(self, f: F) -> F::Output {
        self.0.vectorize(f)
    }
}

/// Quatre `f64`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct V4 {
    lanes: __m256d,
    simd: V3,
}

/// Le résultat d'une comparaison, un bit par voie.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mask {
    lanes: __m256d,
    simd: V3,
}

impl V4 {
    #[inline(always)]
    fn with(self, lanes: __m256d) -> Self {
        Self {
            lanes,
            simd: self.simd,
        }
    }

    #[inline(always)]
    fn mask(self, lanes: __m256d) -> Mask {
        Mask {
            lanes,
            simd: self.simd,
        }
    }

    /// Quatre fois `value`.
    #[inline(always)]
    pub(crate) fn splat(tok: Avx2, value: f64) -> Self {
        Self {
            lanes: tok.0.avx._mm256_set1_pd(value),
            simd: tok.0,
        }
    }

    /// Quatre zéros.
    #[inline(always)]
    pub(crate) fn zero(tok: Avx2) -> Self {
        Self {
            lanes: tok.0.avx._mm256_setzero_pd(),
            simd: tok.0,
        }
    }

    /// Charge quatre valeurs (`vmovupd` : une copie de 32 octets, sans alignement exigé).
    #[inline(always)]
    pub(crate) fn load(tok: Avx2, lanes: &[f64; 4]) -> Self {
        Self {
            lanes: bytemuck::cast(*lanes),
            simd: tok.0,
        }
    }

    /// Écrit les quatre voies.
    #[inline(always)]
    pub(crate) fn store(self, lanes: &mut [f64; 4]) {
        *lanes = bytemuck::cast(self.lanes);
    }

    /// Quatre fois `value`, sans [`Avx2`] : `self` en est la preuve.
    #[inline(always)]
    pub(super) fn splat_like(self, value: f64) -> Self {
        self.with(self.simd.avx._mm256_set1_pd(value))
    }

    /// Un registre de ces quatre valeurs, sans [`Avx2`] : `self` en est la preuve.
    #[inline(always)]
    pub(super) fn reload(self, lanes: &[f64; 4]) -> Self {
        self.with(bytemuck::cast(*lanes))
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
        self.with(
            self.simd
                .avx
                ._mm256_round_pd::<{ _MM_FROUND_TO_NEAREST_INT | _MM_FROUND_NO_EXC }>(self.lanes),
        )
    }

    /// La grandeur de `self` avec le signe de `sign` (un zéro négatif compris).
    #[inline(always)]
    pub(crate) fn copysign(self, sign: Self) -> Self {
        let avx = self.simd.avx;
        let sign_bit = avx._mm256_set1_pd(-0.0);
        let magnitude = avx._mm256_andnot_pd(sign_bit, self.lanes);
        let sign_only = avx._mm256_and_pd(sign_bit, sign.lanes);
        // Les deux moitiés n'ont aucun bit en commun : un OU les réunit.
        self.with(avx._mm256_or_pd(magnitude, sign_only))
    }

    /// Les voies dont le bit de signe est à un : négatifs, zéro négatif, `NaN` de signe négatif.
    #[inline(always)]
    pub(crate) fn sign_mask(self) -> Mask {
        // `blendv` et `movemask` ne lisent que le bit de signe : le registre est son propre masque.
        self.mask(self.lanes)
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
        let bias = self.simd.avx._mm256_set1_epi64x(1023);
        let biased = self.simd.avx2._mm256_add_epi64(bits, bias);
        let shifted = self.simd.avx2._mm256_slli_epi64::<52>(biased);
        self.with(self.simd.avx._mm256_castsi256_pd(shifted))
    }

    /// La valeur entière de `self`, supposée entière et de valeur absolue inférieure à `2^51`,
    /// en entier 64 bits : `self + 1.5 * 2^52` a `self` dans ses bits de mantisse.
    #[inline(always)]
    fn integer_bits(self) -> __m256i {
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        let sum = self + shifter;
        let sum_bits = self.simd.avx._mm256_castpd_si256(sum.lanes);
        let shifter_bits = self.simd.avx._mm256_castpd_si256(shifter.lanes);
        self.simd.avx2._mm256_sub_epi64(sum_bits, shifter_bits)
    }

    /// Le masque des voies dont l'entier (`self` est entier, `|self| < 2^51`) a à un le bit
    /// `63 - SHIFT` : `SHIFT = 63` pour le bit 0, `SHIFT = 62` pour le bit 1.
    #[inline(always)]
    pub(super) fn integer_bit_mask<const SHIFT: i32>(self) -> Mask {
        let bits = self.integer_bits();
        // Le bit visé monte en position de signe, la seule que `blendv` et `movemask` lisent.
        let moved = self.simd.avx2._mm256_slli_epi64::<SHIFT>(bits);
        self.mask(self.simd.avx._mm256_castsi256_pd(moved))
    }

    /// Mantisse et exposant d'un `f64` positif normal : `self = m * 2^e` avec `m` dans `[1, 2[`.
    /// `e` est rendu en `f64`. Pour une valeur nulle, négative, sous-normale, infinie ou `NaN`,
    /// le résultat n'a pas de sens : l'appelant écarte ces voies.
    #[inline(always)]
    pub(super) fn split_exponent(self) -> (Self, Self) {
        let (avx, avx2) = (self.simd.avx, self.simd.avx2);
        let bits = avx._mm256_castpd_si256(self.lanes);
        let exponent_field = avx2._mm256_srli_epi64::<52>(bits);
        let mantissa_mask = avx._mm256_set1_epi64x(0x000F_FFFF_FFFF_FFFF);
        let mantissa = avx2._mm256_and_si256(bits, mantissa_mask);
        let one_bits = avx._mm256_set1_epi64x(0x3FF0_0000_0000_0000);
        let normalized = avx2._mm256_or_si256(mantissa, one_bits);
        let mantissa_value = self.with(avx._mm256_castsi256_pd(normalized));
        // e = field - 1023, reconverti en f64 par la même astuce de `1.5 * 2^52`.
        let bias = avx._mm256_set1_epi64x(1023);
        let exponent = avx2._mm256_sub_epi64(exponent_field, bias);
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        let shifter_bits = avx._mm256_castpd_si256(shifter.lanes);
        let summed = avx2._mm256_add_epi64(shifter_bits, exponent);
        let exponent_value = self.with(avx._mm256_castsi256_pd(summed)) - shifter;
        (mantissa_value, exponent_value)
    }

    /// Valeur absolue.
    #[inline(always)]
    pub(crate) fn abs(self) -> Self {
        let sign = self.simd.avx._mm256_set1_pd(-0.0);
        self.with(self.simd.avx._mm256_andnot_pd(sign, self.lanes))
    }

    /// Racine carrée.
    #[inline(always)]
    pub(crate) fn sqrt(self) -> Self {
        self.with(self.simd.avx._mm256_sqrt_pd(self.lanes))
    }

    /// `self * a + b` en une seule opération, arrondie une fois (FMA).
    ///
    /// Le résultat diffère de `self * a + b` en deux opérations par le seul arrondi du produit :
    /// il est plus proche de la valeur exacte, mais pas égal au chemin scalaire, qui n'a pas
    /// d'FMA (`f64::mul_add` y serait logiciel sans FMA matériel).
    #[inline(always)]
    pub(crate) fn mul_add(self, a: Self, b: Self) -> Self {
        self.with(self.simd.fma._mm256_fmadd_pd(self.lanes, a.lanes, b.lanes))
    }

    /// Plus grand entier inférieur ou égal.
    #[inline(always)]
    pub(crate) fn floor(self) -> Self {
        self.with(self.simd.avx._mm256_floor_pd(self.lanes))
    }

    /// Le plus grand des deux. Une voie `NaN` rend la seconde opérande.
    #[inline(always)]
    pub(crate) fn max(self, other: Self) -> Self {
        self.with(self.simd.avx._mm256_max_pd(self.lanes, other.lanes))
    }

    /// Le plus petit des deux. Une voie `NaN` rend la seconde opérande.
    #[inline(always)]
    pub(crate) fn min(self, other: Self) -> Self {
        self.with(self.simd.avx._mm256_min_pd(self.lanes, other.lanes))
    }

    /// Voie par voie : `self` si le masque est vrai, `other` sinon.
    #[inline(always)]
    pub(crate) fn select(mask: Mask, if_true: Self, if_false: Self) -> Self {
        if_true.with(
            mask.simd
                .avx
                ._mm256_blendv_pd(if_false.lanes, if_true.lanes, mask.lanes),
        )
    }

    /// Voies finies (ni `NaN` ni infinie).
    #[inline(always)]
    pub(crate) fn is_finite(self) -> Mask {
        // Fini : `|x| <= f64::MAX`, faux pour `NaN` et pour l'infini.
        let largest = self.splat_like(f64::MAX);
        self.abs().le(largest)
    }
}

macro_rules! compare {
    ($name:ident, $predicate:ident, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self, other: Self) -> Mask {
                self.mask(
                    self.simd
                        .avx
                        ._mm256_cmp_pd::<$predicate>(self.lanes, other.lanes),
                )
            }
        }
    };
}

compare!(eq, _CMP_EQ_OQ, "`self == other`, false if a lane is `NaN`.");
compare!(lt, _CMP_LT_OQ, "`self < other`, false if a lane is `NaN`.");
compare!(le, _CMP_LE_OQ, "`self <= other`, false if a lane is `NaN`.");
compare!(gt, _CMP_GT_OQ, "`self > other`, false if a lane is `NaN`.");
compare!(ge, _CMP_GE_OQ, "`self >= other`, false if a lane is `NaN`.");

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $intrinsic:ident) => {
        impl $trait for V4 {
            type Output = V4;

            #[inline(always)]
            fn $method(self, rhs: V4) -> V4 {
                self.with(self.simd.avx.$intrinsic(self.lanes, rhs.lanes))
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
        let sign = self.simd.avx._mm256_set1_pd(-0.0);
        self.with(self.simd.avx._mm256_xor_pd(self.lanes, sign))
    }
}

impl Mask {
    #[inline(always)]
    fn with(self, lanes: __m256d) -> Self {
        Self {
            lanes,
            simd: self.simd,
        }
    }

    /// Voies vraies des deux masques.
    #[inline(always)]
    pub(crate) fn and(self, other: Self) -> Self {
        self.with(self.simd.avx._mm256_and_pd(self.lanes, other.lanes))
    }

    /// Voies vraies d'au moins un des deux masques.
    #[inline(always)]
    pub(crate) fn or(self, other: Self) -> Self {
        self.with(self.simd.avx._mm256_or_pd(self.lanes, other.lanes))
    }

    /// Les voies que le masque ne retient pas.
    #[inline(always)]
    pub(crate) fn not(self) -> Self {
        let avx = self.simd.avx;
        let zero = avx._mm256_setzero_pd();
        // Le prédicat `TRUE` met tous les bits à un.
        let ones = avx._mm256_cmp_pd::<_CMP_TRUE_UQ>(zero, zero);
        self.with(avx._mm256_xor_pd(self.lanes, ones))
    }

    /// Un bit par voie, le bit 0 pour la première : les voies vraies.
    #[inline(always)]
    pub(crate) fn bits(self) -> u32 {
        let bits = self.simd.avx._mm256_movemask_pd(self.lanes);
        u32::try_from(bits).unwrap_or(0)
    }

    /// Au moins une voie est vraie.
    #[inline(always)]
    pub(crate) fn any(self) -> bool {
        self.simd.avx._mm256_movemask_pd(self.lanes) != 0
    }

    /// Les quatre voies sont vraies.
    #[inline(always)]
    pub(crate) fn all(self) -> bool {
        self.simd.avx._mm256_movemask_pd(self.lanes) == 0b1111
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
