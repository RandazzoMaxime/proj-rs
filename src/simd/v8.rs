// The AVX-512 types of `std::arch` are stable since Rust 1.89, above the `rust-version` floor of
// this crate (1.83): clippy's `incompatible_msrv` fires on them here, but this module is only
// compiled with the `avx512` feature, which itself requires Rust 1.89+ (it enables the `x86-v4`
// feature of `pulp`, which does not build below 1.89).
#![allow(
    clippy::incompatible_msrv,
    reason = "this module is only compiled with the `avx512` feature, which requires Rust 1.89+"
)]

//! [`V8`] : huit `f64` dans un registre AVX-512, la réalisation x86-64 à huit voies de
//! [`Vector`], derrière la feature `avx512`.
//!
//! Ce module n'est compilé qu'avec cette feature : les types 512 bits de `std::arch` exigent
//! Rust 1.89, au-dessus du plancher du dépôt (1.83), et la feature `x86-v4` de `pulp` ne compile
//! pas sous 1.83. Sur un hôte sans AVX-512, [`Avx512::detect`] rend `None` et le driver prend le
//! chemin AVX2 (`v4.rs`).
//!
//! Les intrinsèques passent par `pulp` (`pulp::x86::V4`, le niveau x86-64-v4) : un jeton que l'on
//! n'obtient qu'après avoir détecté le processeur, et dont les méthodes appellent l'intrinsèque
//! du même nom. C'est `pulp` qui porte le `unsafe` ; ce module n'en a pas. Un [`V8`] ou un
//! [`Mask`] garde son jeton (de taille nulle) : il n'existe donc qu'à partir d'un [`Avx512`].
//!
//! Chaque opération rend les bits de son homologue AVX2 (`v4.rs`) : les deux jeux d'instructions
//! arrondissent de la même façon (IEEE 754, FMA fusionné), les comparaisons rendent un masque
//! dont on ne lit que le bit de chaque voie, et `math.rs` ne les compose que par l'API de
//! [`Vector`]. Les trois endroits où les instructions diffèrent sont rendus identiques :
//!
//! - les masques sont des registres de bits (`__mmask8`) et non des voies entières : `select`
//!   passe par `vblendmpd`, les comparaisons rendent directement le masque ;
//! - `round` et `floor` passent par `vrndscalepd` (l'arrondi vers un entier est exact : les bits
//!   sont ceux de `vroundpd`) ;
//! - `sign_mask` et `integer_bit_mask` lisent le bit de signe de chaque voie 64 bits par
//!   `vpmovq2m`, comme `movemask` le fait sur le registre AVX2.

use pulp::bytemuck;
use pulp::x86::V4 as Cpu;
use std::arch::x86_64::{
    __m512d, __m512i, __mmask8, _CMP_EQ_OQ, _CMP_GE_OQ, _CMP_GT_OQ, _CMP_LE_OQ, _CMP_LT_OQ,
    _MM_FROUND_TO_NEAREST_INT, _MM_FROUND_TO_NEG_INF,
};
use std::ops::{Add, Div, Mul, Neg, Sub};

use super::vec::{SimdToken, Vector};

/// Preuve que le processeur supporte AVX-512 (le niveau x86-64-v4 de `pulp` : AVX512F, BW, CD,
/// DQ, VL, plus tout le niveau v3 qu'ont tous ces processeurs).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Avx512(Cpu);

impl Avx512 {
    /// La preuve, si ce processeur supporte le niveau x86-64-v4.
    #[must_use]
    pub(crate) fn detect() -> Option<Self> {
        Cpu::try_new().map(Self)
    }
}

impl SimdToken for Avx512 {
    /// Exécute `f` dans une fonction compilée avec les extensions du jeton (`avx512f`, …) : les
    /// méthodes `#[inline(always)]` de [`V8`] y deviennent des instructions en ligne.
    #[inline(always)]
    fn vectorize<F: pulp::NullaryFnOnce>(self, f: F) -> F::Output {
        self.0.vectorize(f)
    }
}

/// Huit `f64`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct V8 {
    lanes: __m512d,
    simd: Cpu,
}

/// Le résultat d'une comparaison : un bit par voie.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mask {
    lanes: __mmask8,
    simd: Cpu,
}

impl V8 {
    #[inline(always)]
    fn with(self, lanes: __m512d) -> Self {
        Self {
            lanes,
            simd: self.simd,
        }
    }

    #[inline(always)]
    fn mask(self, lanes: __mmask8) -> Mask {
        Mask {
            lanes,
            simd: self.simd,
        }
    }

    /// `2^k` pour un `k` entier de `[-1022, 1023]`, par l'exposant : `(k + 1023) << 52`.
    #[inline(always)]
    fn pow2(self) -> Self {
        let f = self.simd.avx512f;
        let bits = self.integer_bits();
        let bias = f._mm512_set1_epi64(1023);
        let biased = f._mm512_add_epi64(bits, bias);
        let shifted = f._mm512_slli_epi64::<52>(biased);
        self.with(f._mm512_castsi512_pd(shifted))
    }

    /// La valeur entière de `self`, supposée entière et de valeur absolue inférieure à `2^51`,
    /// en entier 64 bits : `self + 1.5 * 2^52` a `self` dans ses bits de mantisse.
    #[inline(always)]
    fn integer_bits(self) -> __m512i {
        let f = self.simd.avx512f;
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        let sum = self + shifter;
        let sum_bits = f._mm512_castpd_si512(sum.lanes);
        let shifter_bits = f._mm512_castpd_si512(shifter.lanes);
        f._mm512_sub_epi64(sum_bits, shifter_bits)
    }
}

macro_rules! compare {
    ($name:ident, $predicate:ident, $doc:literal) => {
        #[doc = $doc]
        #[inline(always)]
        fn $name(self, other: Self) -> Self::Mask {
            self.mask(
                self.simd
                    .avx512f
                    ._mm512_cmp_pd_mask::<$predicate>(self.lanes, other.lanes),
            )
        }
    };
}

impl Vector<8> for V8 {
    type Token = Avx512;
    type Mask = Mask;

    /// Huit fois `value`.
    #[inline(always)]
    fn splat(tok: Avx512, value: f64) -> Self {
        Self {
            lanes: tok.0.avx512f._mm512_set1_pd(value),
            simd: tok.0,
        }
    }

    /// Huit zéros.
    #[inline(always)]
    fn zero(tok: Avx512) -> Self {
        Self {
            lanes: tok.0.avx512f._mm512_setzero_pd(),
            simd: tok.0,
        }
    }

    /// Charge huit valeurs (`vmovupd` : une copie de 64 octets, sans alignement exigé).
    #[inline(always)]
    fn load(tok: Avx512, lanes: &[f64; 8]) -> Self {
        Self {
            lanes: bytemuck::cast(*lanes),
            simd: tok.0,
        }
    }

    /// Écrit les huit voies.
    #[inline(always)]
    fn store(self, lanes: &mut [f64; 8]) {
        *lanes = bytemuck::cast(self.lanes);
    }

    /// Huit fois `value`, sans [`Avx512`] : `self` en est la preuve.
    #[inline(always)]
    fn splat_like(self, value: f64) -> Self {
        self.with(self.simd.avx512f._mm512_set1_pd(value))
    }

    /// Un registre de ces huit valeurs, sans [`Avx512`] : `self` en est la preuve.
    #[inline(always)]
    fn reload(self, lanes: &[f64; 8]) -> Self {
        self.with(bytemuck::cast(*lanes))
    }

    /// Les huit voies, pour un calcul voie par voie.
    #[inline(always)]
    fn to_array(self) -> [f64; 8] {
        let mut lanes = [0.0; 8];
        self.store(&mut lanes);
        lanes
    }

    /// L'entier le plus proche, au pair en cas d'égalité (`vrndscalepd`, arrondi exact).
    #[inline(always)]
    fn round(self) -> Self {
        self.with(
            self.simd
                .avx512f
                ._mm512_roundscale_pd::<{ _MM_FROUND_TO_NEAREST_INT }>(self.lanes),
        )
    }

    /// La grandeur de `self` avec le signe de `sign` (un zéro négatif compris).
    #[inline(always)]
    fn copysign(self, sign: Self) -> Self {
        let f = self.simd.avx512f;
        let dq = self.simd.avx512dq;
        let sign_bit = f._mm512_set1_pd(-0.0);
        let magnitude = dq._mm512_andnot_pd(sign_bit, self.lanes);
        let sign_only = dq._mm512_and_pd(sign_bit, sign.lanes);
        // Les deux moitiés n'ont aucun bit en commun : un OU les réunit.
        self.with(dq._mm512_or_pd(magnitude, sign_only))
    }

    /// Les voies dont le bit de signe est à un : négatifs, zéro négatif, `NaN` de signe négatif.
    #[inline(always)]
    fn sign_mask(self) -> Self::Mask {
        let f = self.simd.avx512f;
        let dq = self.simd.avx512dq;
        // `vpmovq2m` rend le bit de signe de chaque voie 64 bits, comme `movemask` sur AVX2.
        self.mask(dq._mm512_movepi64_mask(f._mm512_castpd_si512(self.lanes)))
    }

    /// `self * 2^k` pour des `k` entiers, `|k| <= 2000`, en deux multiplications pour que la
    /// puissance de deux reste représentable.
    #[inline(always)]
    fn scale_pow2(self, k: Self) -> Self {
        let half = (k * self.splat_like(0.5)).round();
        self * half.pow2() * (k - half).pow2()
    }

    /// Le masque des voies dont l'entier (`self` est entier, `|self| < 2^51`) a à un le bit
    /// `63 - SHIFT` : `SHIFT = 63` pour le bit 0, `SHIFT = 62` pour le bit 1.
    #[inline(always)]
    fn integer_bit_mask<const SHIFT: i32>(self) -> Self::Mask {
        let f = self.simd.avx512f;
        let dq = self.simd.avx512dq;
        let bits = self.integer_bits();
        // Le bit visé monte en position de signe : `vpsllvq` décale par le compte d'un registre,
        // `SHIFT` y est passé en scalaire.
        let count = self.simd.sse2._mm_set1_epi64x(i64::from(SHIFT));
        let moved = f._mm512_sll_epi64(bits, count);
        self.mask(dq._mm512_movepi64_mask(moved))
    }

    /// Mantisse et exposant d'un `f64` positif normal : `self = m * 2^e` avec `m` dans `[1, 2[`.
    /// `e` est rendu en `f64`. Pour une valeur nulle, négative, sous-normale, infinie ou `NaN`,
    /// le résultat n'a pas de sens : l'appelant écarte ces voies.
    #[inline(always)]
    fn split_exponent(self) -> (Self, Self) {
        let f = self.simd.avx512f;
        let bits = f._mm512_castpd_si512(self.lanes);
        let exponent_field = f._mm512_srli_epi64::<52>(bits);
        let mantissa_mask = f._mm512_set1_epi64(0x000F_FFFF_FFFF_FFFF);
        let mantissa = f._mm512_and_si512(bits, mantissa_mask);
        let one_bits = f._mm512_set1_epi64(0x3FF0_0000_0000_0000);
        let normalized = f._mm512_or_si512(mantissa, one_bits);
        let mantissa_value = self.with(f._mm512_castsi512_pd(normalized));
        // e = field - 1023, reconverti en f64 par la même astuce de `1.5 * 2^52`.
        let bias = f._mm512_set1_epi64(1023);
        let exponent = f._mm512_sub_epi64(exponent_field, bias);
        let shifter = self.splat_like(6_755_399_441_055_744.0);
        let shifter_bits = f._mm512_castpd_si512(shifter.lanes);
        let summed = f._mm512_add_epi64(shifter_bits, exponent);
        let exponent_value = self.with(f._mm512_castsi512_pd(summed)) - shifter;
        (mantissa_value, exponent_value)
    }

    /// Valeur absolue.
    #[inline(always)]
    fn abs(self) -> Self {
        let sign = self.simd.avx512f._mm512_set1_pd(-0.0);
        self.with(self.simd.avx512dq._mm512_andnot_pd(sign, self.lanes))
    }

    /// Racine carrée.
    #[inline(always)]
    fn sqrt(self) -> Self {
        self.with(self.simd.avx512f._mm512_sqrt_pd(self.lanes))
    }

    /// `self * a + b` en une seule opération, arrondie une fois (FMA).
    ///
    /// Le résultat diffère de `self * a + b` en deux opérations par le seul arrondi du produit :
    /// il est plus proche de la valeur exacte, mais pas égal au chemin scalaire, qui n'a pas
    /// d'FMA (`f64::mul_add` y serait logiciel sans FMA matériel). C'est le même `vfmadd` que
    /// celui d'AVX2, voie par voie.
    #[inline(always)]
    fn mul_add(self, a: Self, b: Self) -> Self {
        self.with(
            self.simd
                .avx512f
                ._mm512_fmadd_pd(self.lanes, a.lanes, b.lanes),
        )
    }

    /// Plus grand entier inférieur ou égal (`vrndscalepd` vers le bas, arrondi exact).
    #[inline(always)]
    fn floor(self) -> Self {
        self.with(
            self.simd
                .avx512f
                ._mm512_roundscale_pd::<{ _MM_FROUND_TO_NEG_INF }>(self.lanes),
        )
    }

    /// Le plus grand des deux. Une voie `NaN` rend la seconde opérande, comme `vmaxpd` d'AVX2.
    #[inline(always)]
    fn max(self, other: Self) -> Self {
        self.with(self.simd.avx512f._mm512_max_pd(self.lanes, other.lanes))
    }

    /// Le plus petit des deux. Une voie `NaN` rend la seconde opérande, comme `vminpd` d'AVX2.
    #[inline(always)]
    fn min(self, other: Self) -> Self {
        self.with(self.simd.avx512f._mm512_min_pd(self.lanes, other.lanes))
    }

    /// Voie par voie : `if_true` si le masque est vrai, `if_false` sinon.
    #[inline(always)]
    fn select(mask: Mask, if_true: Self, if_false: Self) -> Self {
        if_true.with(mask.simd.avx512f._mm512_mask_blend_pd(
            mask.lanes,
            if_false.lanes,
            if_true.lanes,
        ))
    }

    /// Voies finies (ni `NaN` ni infinie).
    #[inline(always)]
    fn is_finite(self) -> Mask {
        // Fini : `|x| <= f64::MAX`, faux pour `NaN` et pour l'infini.
        let largest = self.splat_like(f64::MAX);
        self.abs().le(largest)
    }

    compare!(eq, _CMP_EQ_OQ, "`self == other`, false if a lane is `NaN`.");
    compare!(lt, _CMP_LT_OQ, "`self < other`, false if a lane is `NaN`.");
    compare!(le, _CMP_LE_OQ, "`self <= other`, false if a lane is `NaN`.");
    compare!(gt, _CMP_GT_OQ, "`self > other`, false if a lane is `NaN`.");
    compare!(ge, _CMP_GE_OQ, "`self >= other`, false if a lane is `NaN`.");
}

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $intrinsic:ident) => {
        impl $trait for V8 {
            type Output = V8;

            #[inline(always)]
            fn $method(self, rhs: V8) -> V8 {
                self.with(self.simd.avx512f.$intrinsic(self.lanes, rhs.lanes))
            }
        }
    };
}

arithmetic!(Add, add, _mm512_add_pd);
arithmetic!(Sub, sub, _mm512_sub_pd);
arithmetic!(Mul, mul, _mm512_mul_pd);
arithmetic!(Div, div, _mm512_div_pd);

impl Neg for V8 {
    type Output = V8;

    #[inline(always)]
    fn neg(self) -> V8 {
        let sign = self.simd.avx512f._mm512_set1_pd(-0.0);
        self.with(self.simd.avx512dq._mm512_xor_pd(self.lanes, sign))
    }
}

impl Mask {
    #[inline(always)]
    fn with(self, lanes: __mmask8) -> Self {
        Self {
            lanes,
            simd: self.simd,
        }
    }
}

impl super::vec::Mask for Mask {
    /// Voies vraies des deux masques.
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.with(self.lanes & other.lanes)
    }

    /// Voies vraies d'au moins un des deux masques.
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.with(self.lanes | other.lanes)
    }

    /// Les voies que le masque ne retient pas.
    #[inline(always)]
    fn not(self) -> Self {
        self.with(!self.lanes)
    }

    /// Un bit par voie, le bit 0 pour la première : les voies vraies.
    #[inline(always)]
    fn bits(self) -> u32 {
        u32::from(self.lanes)
    }

    /// Au moins une voie est vraie.
    #[inline(always)]
    fn any(self) -> bool {
        self.lanes != 0
    }

    /// Les huit voies sont vraies.
    #[inline(always)]
    fn all(self) -> bool {
        self.lanes == 0xFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simd::vec::Mask as _;

    /// `mul_add` est un vrai FMA : un seul arrondi, celui de `f64::mul_add`.
    #[test]
    fn mul_add_rounds_once() {
        let Some(tok) = Avx512::detect() else {
            return;
        };
        let x = 1.0 + 2.0_f64.powi(-30);
        let exact = 2.0_f64.powi(-29) + 2.0_f64.powi(-60);
        let unfused = x * x - 1.0;
        assert_ne!(unfused, exact, "the test value does tell the two apart");
        assert_eq!(x.mul_add(x, -1.0), exact, "f64::mul_add is the reference");

        let lanes = [x, 0.1, -3.7, 1e150, 2.5, -0.0, 1e-300, 42.0];
        let others = [x, 0.3, 2.25, 1e10, -1.5, 7.0, 3.0, 0.0];
        let adds = [-1.0, 0.7, 1e-5, -1e160, 0.25, -0.0, 1e300, 1.0];
        let mut got = [0.0; 8];
        V8::load(tok, &lanes)
            .mul_add(V8::load(tok, &others), V8::load(tok, &adds))
            .store(&mut got);
        for lane in 0..8 {
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

    /// Les opérations de base rendent les bits de la même opération scalaire, voie par voie.
    #[test]
    fn basic_operations_match_f64_bits() {
        let Some(tok) = Avx512::detect() else {
            return;
        };
        let samples = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            2.5,
            -3.75,
            1e-300,
            1e300,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            0.1,
            -9.87e-5,
        ];
        let quad = |offset: usize| -> [f64; 8] {
            std::array::from_fn(|lane| samples[(offset + lane) % samples.len()])
        };
        for offset in 0..samples.len() {
            for shift in [0, 5, 11] {
                let (a, b) = (quad(offset), quad(offset + shift));
                let (va, vb) = (V8::load(tok, &a), V8::load(tok, &b));
                let mut got = [0.0; 8];
                (va + vb).store(&mut got);
                for lane in 0..8 {
                    assert_eq!(
                        got[lane].to_bits(),
                        (a[lane] + b[lane]).to_bits(),
                        "add lane {lane}"
                    );
                }
                (va * vb).store(&mut got);
                for lane in 0..8 {
                    assert_eq!(
                        got[lane].to_bits(),
                        (a[lane] * b[lane]).to_bits(),
                        "mul lane {lane}"
                    );
                }
                (va / vb).store(&mut got);
                for lane in 0..8 {
                    let want = a[lane] / b[lane];
                    if want.is_nan() {
                        assert!(got[lane].is_nan(), "div lane {lane}");
                    } else {
                        assert_eq!(got[lane].to_bits(), want.to_bits(), "div lane {lane}");
                    }
                }
                (-va).store(&mut got);
                for lane in 0..8 {
                    assert_eq!(got[lane].to_bits(), (-a[lane]).to_bits(), "neg lane {lane}");
                }
                va.abs().store(&mut got);
                for lane in 0..8 {
                    assert_eq!(
                        got[lane].to_bits(),
                        a[lane].abs().to_bits(),
                        "abs lane {lane}"
                    );
                }
                va.sqrt().store(&mut got);
                for lane in 0..8 {
                    let want = a[lane].sqrt();
                    if want.is_nan() {
                        assert!(got[lane].is_nan(), "sqrt lane {lane}");
                    } else {
                        assert_eq!(got[lane].to_bits(), want.to_bits(), "sqrt lane {lane}");
                    }
                }
                va.floor().store(&mut got);
                for lane in 0..8 {
                    let want = a[lane].floor();
                    if want.is_nan() {
                        assert!(got[lane].is_nan(), "floor lane {lane}");
                    } else {
                        assert_eq!(got[lane].to_bits(), want.to_bits(), "floor lane {lane}");
                    }
                }
                va.round().store(&mut got);
                for lane in 0..8 {
                    let want = a[lane].round_ties_even();
                    if want.is_nan() {
                        assert!(got[lane].is_nan(), "round lane {lane}");
                    } else {
                        assert_eq!(got[lane].to_bits(), want.to_bits(), "round lane {lane}");
                    }
                }
                va.copysign(vb).store(&mut got);
                for lane in 0..8 {
                    assert_eq!(
                        got[lane].to_bits(),
                        a[lane].copysign(b[lane]).to_bits(),
                        "copysign lane {lane}"
                    );
                }
                // `max` et `min` rendent la seconde opérande sur un `NaN`, comme `vmaxpd`.
                va.max(vb).store(&mut got);
                for lane in 0..8 {
                    let want = if a[lane] > b[lane] { a[lane] } else { b[lane] };
                    if want.is_nan() {
                        assert!(got[lane].is_nan(), "max lane {lane}");
                    } else {
                        assert_eq!(got[lane].to_bits(), want.to_bits(), "max lane {lane}");
                    }
                }
                // Les masques : un bit par voie, et `select` suit.
                let mask = va.lt(vb);
                let want_bits: u32 = (0..8)
                    .map(|lane| u32::from(a[lane] < b[lane]) << lane)
                    .sum();
                assert_eq!(mask.bits(), want_bits, "lt bits at {offset}");
                assert_eq!(mask.any(), want_bits != 0);
                assert_eq!(mask.all(), want_bits == 0xFF);
                assert_eq!(mask.not().bits(), !want_bits & 0xFF, "not");
                assert_eq!(
                    mask.and(va.is_finite()).bits(),
                    want_bits & {
                        let mut m = 0u32;
                        for lane in 0..8 {
                            if a[lane].is_finite() {
                                m |= 1 << lane;
                            }
                        }
                        m
                    }
                );
                let picked = V8::select(mask, V8::splat(tok, 1.0), V8::splat(tok, 2.0)).to_array();
                for lane in 0..8 {
                    assert_eq!(picked[lane], if a[lane] < b[lane] { 1.0 } else { 2.0 });
                }
                assert_eq!(
                    va.sign_mask().bits(),
                    (0..8)
                        .map(|lane| u32::from(a[lane].is_sign_negative()) << lane)
                        .sum::<u32>(),
                    "sign bits at {offset}"
                );
            }
        }
    }

    /// `scale_pow2`, `split_exponent` et `integer_bit_mask` : les briques de `math.rs`.
    #[test]
    fn exponent_helpers() {
        let Some(tok) = Avx512::detect() else {
            return;
        };
        let values = [0.75, 1.0, 3.0e-5, 6.02e23, 1.0e-200, 12.5, 9.999, 2.0];
        let (mantissa, exponent) = V8::load(tok, &values).split_exponent();
        let (m, e) = (mantissa.to_array(), exponent.to_array());
        for lane in 0..8 {
            let rebuilt = m[lane] * 2.0_f64.powf(e[lane]);
            assert!((1.0..2.0).contains(&m[lane]), "mantissa {}", m[lane]);
            assert_eq!(rebuilt, values[lane], "lane {lane}");
        }
        let powers = [-1000_i32, -3, 0, 1000, -1, 1, 500, -500];
        let ks = powers.map(f64::from);
        let xs = [1.5, -2.25, 7.0, 1.0e-300, 3.0, -8.0, 1e10, -1e-10];
        let mut got = [0.0; 8];
        V8::load(tok, &xs)
            .scale_pow2(V8::load(tok, &ks))
            .store(&mut got);
        // Une multiplication par une puissance de deux est exacte : le découpage ne change rien.
        let want: [f64; 8] = std::array::from_fn(|lane| {
            xs[lane]
                * 2.0_f64.powi(powers[lane] / 2)
                * 2.0_f64.powi(powers[lane] - powers[lane] / 2)
        });
        for lane in 0..8 {
            assert_eq!(
                got[lane].to_bits(),
                want[lane].to_bits(),
                "scale_pow2 lane {lane}"
            );
        }
        let ints = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        assert_eq!(
            V8::load(tok, &ints).integer_bit_mask::<63>().bits(),
            0b1010_1010
        );
        assert_eq!(
            V8::load(tok, &ints).integer_bit_mask::<62>().bits(),
            0b1100_1100
        );
        let negatives = [-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0, -8.0];
        // -1 = ...1111, -2 = ...1110, -3 = ...1101, -4 = ...1100 : le bit 0 est à un pour les
        // impairs, le bit 1 pour -1, -2, -5 et -6.
        assert_eq!(
            V8::load(tok, &negatives).integer_bit_mask::<63>().bits(),
            0b0101_0101
        );
        assert_eq!(
            V8::load(tok, &negatives).integer_bit_mask::<62>().bits(),
            0b0011_0011
        );
    }
}
