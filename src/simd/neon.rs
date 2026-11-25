//! [`V4`] : quatre `f64` dans deux registres NEON, la réalisation aarch64 de [`Vector`], avec la
//! même API sûre que celle d'AVX2 (`v4.rs`).
//!
//! Les intrinsèques passent par `pulp` (`pulp::aarch64::Neon`) : un jeton que l'on n'obtient
//! qu'après avoir détecté le processeur, et dont les méthodes appellent l'intrinsèque du même
//! nom. C'est `pulp` qui porte le `unsafe` ; ce module n'en a pas. NEON fait partie du socle des
//! cibles aarch64 : il n'y a pas de fonction `target_feature` à entrer, le code est en ligne là
//! où il est appelé.
//!
//! Chaque opération rend les bits de son homologue AVX2, car les deux jeux d'instructions
//! arrondissent de la même façon (IEEE 754, FMA fusionné) et que `math.rs` ne les compose que par
//! cette API. Les trois endroits où les instructions diffèrent sont rendus identiques :
//!
//! - `max` et `min` : `maxpd` rend la seconde opérande sur un `NaN` ou une égalité, `fmax` rend
//!   le `NaN`. Les deux passent ici par une comparaison et un `select`.
//! - les masques : AVX2 lit le bit de signe de chaque voie, NEON veut des voies toutes à un. Un
//!   [`Mask`] est toujours une voie entière, ce qui satisfait aussi tous les usages de l'autre.
//! - le passage d'un entier valant un `f64` à l'entier 64 bits : AVX2 passe par l'astuce de
//!   `1,5 * 2^52`, NEON convertit directement, avec le même résultat sur un entier exact.

use pulp::aarch64::Neon as Cpu;
use pulp::bytemuck;
use std::arch::aarch64::{float64x2_t, int64x2_t, uint64x2_t};
use std::ops::{Add, Div, Mul, Neg, Sub};

use super::vec::{SimdToken, Vector};

/// Preuve que le processeur supporte NEON, que toute cible aarch64 a : `detect` rend toujours
/// `Some`, mais l'API reste celle de la preuve d'AVX2 (`Avx2`), pour que les noyaux soient les
/// mêmes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Neon(Cpu);

impl Neon {
    /// La preuve, si ce processeur supporte NEON.
    #[must_use]
    pub(crate) fn detect() -> Option<Self> {
        Cpu::try_new().map(Self)
    }
}

impl SimdToken for Neon {
    /// Exécute `f`. NEON étant dans le socle de la cible, `f` est déjà compilé avec ses
    /// instructions : les méthodes `#[inline(always)]` de [`V4`] y deviennent des instructions en
    /// ligne sans passer par une fonction `target_feature`.
    #[inline(always)]
    fn vectorize<F: pulp::NullaryFnOnce>(self, f: F) -> F::Output {
        f.call()
    }
}

/// Quatre `f64`, deux par registre : les voies 0 et 1 dans `low`, 2 et 3 dans `high`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct V4 {
    low: float64x2_t,
    high: float64x2_t,
    simd: Cpu,
}

/// Le résultat d'une comparaison : chaque voie est toute à un ou toute à zéro.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mask {
    low: uint64x2_t,
    high: uint64x2_t,
    simd: Cpu,
}

const SIGN_BIT: u64 = 0x8000_0000_0000_0000;
const MANTISSA_BITS: u64 = 0x000F_FFFF_FFFF_FFFF;
const ONE_BITS: u64 = 0x3FF0_0000_0000_0000;

impl V4 {
    #[inline(always)]
    fn with(self, low: float64x2_t, high: float64x2_t) -> Self {
        Self {
            low,
            high,
            simd: self.simd,
        }
    }

    #[inline(always)]
    fn mask(self, low: uint64x2_t, high: uint64x2_t) -> Mask {
        Mask {
            low,
            high,
            simd: self.simd,
        }
    }

    /// `2^k` pour un `k` entier de `[-1022, 1023]`, par l'exposant : `(k + 1023) << 52`.
    #[inline(always)]
    fn pow2(self) -> Self {
        let neon = self.simd.neon;
        let (low, high) = self.integer_bits();
        let bias = neon.vdupq_n_s64(1023);
        let low = neon.vshlq_n_s64::<52>(neon.vaddq_s64(low, bias));
        let high = neon.vshlq_n_s64::<52>(neon.vaddq_s64(high, bias));
        self.with(bytemuck::cast(low), bytemuck::cast(high))
    }

    /// La valeur entière de `self`, supposée entière et de valeur absolue inférieure à `2^51`,
    /// en entier 64 bits. NEON convertit directement : sur un entier exact, c'est ce que donne
    /// l'astuce de `1,5 * 2^52` d'AVX2.
    #[inline(always)]
    fn integer_bits(self) -> (int64x2_t, int64x2_t) {
        (
            self.simd.neon.vcvtq_s64_f64(self.low),
            self.simd.neon.vcvtq_s64_f64(self.high),
        )
    }
}

macro_rules! compare {
    ($name:ident, $intrinsic:ident, $doc:literal) => {
        #[doc = $doc]
        #[inline(always)]
        fn $name(self, other: Self) -> Self::Mask {
            self.mask(
                self.simd.neon.$intrinsic(self.low, other.low),
                self.simd.neon.$intrinsic(self.high, other.high),
            )
        }
    };
}

impl Vector<4> for V4 {
    type Token = Neon;
    type Mask = Mask;

    /// Quatre fois `value`.
    #[inline(always)]
    fn splat(tok: Neon, value: f64) -> Self {
        Self {
            low: tok.0.neon.vdupq_n_f64(value),
            high: tok.0.neon.vdupq_n_f64(value),
            simd: tok.0,
        }
    }

    /// Quatre zéros.
    #[inline(always)]
    fn zero(tok: Neon) -> Self {
        Self::splat(tok, 0.0)
    }

    /// Charge quatre valeurs (deux copies de 16 octets, sans alignement exigé).
    #[inline(always)]
    fn load(tok: Neon, lanes: &[f64; 4]) -> Self {
        let [a, b, c, d] = *lanes;
        Self {
            low: bytemuck::cast([a, b]),
            high: bytemuck::cast([c, d]),
            simd: tok.0,
        }
    }

    /// Écrit les quatre voies.
    #[inline(always)]
    fn store(self, lanes: &mut [f64; 4]) {
        let low: [f64; 2] = bytemuck::cast(self.low);
        let high: [f64; 2] = bytemuck::cast(self.high);
        *lanes = [low[0], low[1], high[0], high[1]];
    }

    /// Quatre fois `value`, sans [`Neon`] : `self` en est la preuve.
    #[inline(always)]
    fn splat_like(self, value: f64) -> Self {
        self.with(
            self.simd.neon.vdupq_n_f64(value),
            self.simd.neon.vdupq_n_f64(value),
        )
    }

    /// Un registre de ces quatre valeurs, sans [`Neon`] : `self` en est la preuve.
    #[inline(always)]
    fn reload(self, lanes: &[f64; 4]) -> Self {
        let [a, b, c, d] = *lanes;
        self.with(bytemuck::cast([a, b]), bytemuck::cast([c, d]))
    }

    /// Les quatre voies, pour un calcul voie par voie.
    #[inline(always)]
    fn to_array(self) -> [f64; 4] {
        let mut lanes = [0.0; 4];
        self.store(&mut lanes);
        lanes
    }

    /// L'entier le plus proche, au pair en cas d'égalité (`frintn`).
    #[inline(always)]
    fn round(self) -> Self {
        self.with(
            self.simd.neon.vrndnq_f64(self.low),
            self.simd.neon.vrndnq_f64(self.high),
        )
    }

    /// La grandeur de `self` avec le signe de `sign` (un zéro négatif compris).
    #[inline(always)]
    fn copysign(self, sign: Self) -> Self {
        let neon = self.simd.neon;
        let sign_bit = neon.vdupq_n_u64(SIGN_BIT);
        // `bsl` prend le bit de `sign` là où le masque est à un, celui de `self` ailleurs.
        self.with(
            neon.vbslq_f64(sign_bit, sign.low, self.low),
            neon.vbslq_f64(sign_bit, sign.high, self.high),
        )
    }

    /// Les voies dont le bit de signe est à un : négatifs, zéro négatif, `NaN` de signe négatif.
    #[inline(always)]
    fn sign_mask(self) -> Self::Mask {
        let neon = self.simd.neon;
        let zero = neon.vdupq_n_s64(0);
        let low: int64x2_t = bytemuck::cast(self.low);
        let high: int64x2_t = bytemuck::cast(self.high);
        // Un entier est négatif exactement quand son bit de signe est à un.
        self.mask(neon.vcltq_s64(low, zero), neon.vcltq_s64(high, zero))
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
        let neon = self.simd.neon;
        let (low, high) = self.integer_bits();
        // Le bit visé monte en position de signe, puis le décalage arithmétique le recopie sur
        // toute la voie.
        let low = neon.vshrq_n_s64::<63>(neon.vshlq_n_s64::<SHIFT>(low));
        let high = neon.vshrq_n_s64::<63>(neon.vshlq_n_s64::<SHIFT>(high));
        self.mask(bytemuck::cast(low), bytemuck::cast(high))
    }

    /// Mantisse et exposant d'un `f64` positif normal : `self = m * 2^e` avec `m` dans `[1, 2[`.
    /// `e` est rendu en `f64`. Pour une valeur nulle, négative, sous-normale, infinie ou `NaN`,
    /// le résultat n'a pas de sens : l'appelant écarte ces voies.
    #[inline(always)]
    fn split_exponent(self) -> (Self, Self) {
        let neon = self.simd.neon;
        let mantissa_mask = neon.vdupq_n_u64(MANTISSA_BITS);
        let one_bits = neon.vdupq_n_u64(ONE_BITS);
        let bias = neon.vdupq_n_s64(1023);
        let half = |lanes: float64x2_t| {
            let bits: uint64x2_t = bytemuck::cast(lanes);
            let exponent_field = neon.vshrq_n_u64::<52>(bits);
            let mantissa = neon.vorrq_u64(neon.vandq_u64(bits, mantissa_mask), one_bits);
            // e = field - 1023 : le champ est positif et sous `2^11`, donc le même en signé.
            let exponent = neon.vsubq_s64(bytemuck::cast(exponent_field), bias);
            (
                bytemuck::cast::<_, float64x2_t>(mantissa),
                neon.vcvtq_f64_s64(exponent),
            )
        };
        let (mantissa_low, exponent_low) = half(self.low);
        let (mantissa_high, exponent_high) = half(self.high);
        (
            self.with(mantissa_low, mantissa_high),
            self.with(exponent_low, exponent_high),
        )
    }

    /// Valeur absolue.
    #[inline(always)]
    fn abs(self) -> Self {
        self.with(
            self.simd.neon.vabsq_f64(self.low),
            self.simd.neon.vabsq_f64(self.high),
        )
    }

    /// Racine carrée.
    #[inline(always)]
    fn sqrt(self) -> Self {
        self.with(
            self.simd.neon.vsqrtq_f64(self.low),
            self.simd.neon.vsqrtq_f64(self.high),
        )
    }

    /// `self * a + b` en une seule opération, arrondie une fois (FMA).
    ///
    /// Le résultat diffère de `self * a + b` en deux opérations par le seul arrondi du produit :
    /// il est plus proche de la valeur exacte. C'est le même que celui d'AVX2 (`vfmadd`), et que
    /// celui de `f64::mul_add`, que le chemin scalaire appelle.
    #[inline(always)]
    fn mul_add(self, a: Self, b: Self) -> Self {
        let neon = self.simd.neon;
        // `fmla` calcule `acc + x * y`.
        self.with(
            neon.vfmaq_f64(b.low, self.low, a.low),
            neon.vfmaq_f64(b.high, self.high, a.high),
        )
    }

    /// Plus grand entier inférieur ou égal.
    #[inline(always)]
    fn floor(self) -> Self {
        self.with(
            self.simd.neon.vrndmq_f64(self.low),
            self.simd.neon.vrndmq_f64(self.high),
        )
    }

    /// Le plus grand des deux. Une voie `NaN` rend la seconde opérande, comme `maxpd`.
    #[inline(always)]
    fn max(self, other: Self) -> Self {
        Self::select(self.gt(other), self, other)
    }

    /// Le plus petit des deux. Une voie `NaN` rend la seconde opérande, comme `minpd`.
    #[inline(always)]
    fn min(self, other: Self) -> Self {
        Self::select(self.lt(other), self, other)
    }

    /// Voie par voie : `self` si le masque est vrai, `other` sinon.
    #[inline(always)]
    fn select(mask: Mask, if_true: Self, if_false: Self) -> Self {
        let neon = mask.simd.neon;
        if_true.with(
            neon.vbslq_f64(mask.low, if_true.low, if_false.low),
            neon.vbslq_f64(mask.high, if_true.high, if_false.high),
        )
    }

    /// Voies finies (ni `NaN` ni infinie).
    #[inline(always)]
    fn is_finite(self) -> Mask {
        // Fini : `|x| <= f64::MAX`, faux pour `NaN` et pour l'infini.
        let largest = self.splat_like(f64::MAX);
        self.abs().le(largest)
    }

    compare!(eq, vceqq_f64, "`self == other`, false if a lane is `NaN`.");
    compare!(lt, vcltq_f64, "`self < other`, false if a lane is `NaN`.");
    compare!(le, vcleq_f64, "`self <= other`, false if a lane is `NaN`.");
    compare!(gt, vcgtq_f64, "`self > other`, false if a lane is `NaN`.");
    compare!(ge, vcgeq_f64, "`self >= other`, false if a lane is `NaN`.");
}

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $intrinsic:ident) => {
        impl $trait for V4 {
            type Output = V4;

            #[inline(always)]
            fn $method(self, rhs: V4) -> V4 {
                self.with(
                    self.simd.neon.$intrinsic(self.low, rhs.low),
                    self.simd.neon.$intrinsic(self.high, rhs.high),
                )
            }
        }
    };
}

arithmetic!(Add, add, vaddq_f64);
arithmetic!(Sub, sub, vsubq_f64);
arithmetic!(Mul, mul, vmulq_f64);
arithmetic!(Div, div, vdivq_f64);

impl Neg for V4 {
    type Output = V4;

    #[inline(always)]
    fn neg(self) -> V4 {
        self.with(
            self.simd.neon.vnegq_f64(self.low),
            self.simd.neon.vnegq_f64(self.high),
        )
    }
}

impl Mask {
    #[inline(always)]
    fn with(self, low: uint64x2_t, high: uint64x2_t) -> Self {
        Self {
            low,
            high,
            simd: self.simd,
        }
    }
}

impl super::vec::Mask for Mask {
    /// Voies vraies des deux masques.
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.with(
            self.simd.neon.vandq_u64(self.low, other.low),
            self.simd.neon.vandq_u64(self.high, other.high),
        )
    }

    /// Voies vraies d'au moins un des deux masques.
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.with(
            self.simd.neon.vorrq_u64(self.low, other.low),
            self.simd.neon.vorrq_u64(self.high, other.high),
        )
    }

    /// Les voies que le masque ne retient pas.
    #[inline(always)]
    fn not(self) -> Self {
        let neon = self.simd.neon;
        let ones = neon.vdupq_n_u64(u64::MAX);
        self.with(
            neon.veorq_u64(self.low, ones),
            neon.veorq_u64(self.high, ones),
        )
    }

    /// Un bit par voie, le bit 0 pour la première : les voies vraies.
    #[inline(always)]
    fn bits(self) -> u32 {
        let neon = self.simd.neon;
        // Le bit de signe de chaque voie est son bit : on le ramène en position 0.
        let low = neon.vshrq_n_u64::<63>(self.low);
        let high = neon.vshrq_n_u64::<63>(self.high);
        let bits = neon.vgetq_lane_u64::<0>(low)
            | (neon.vgetq_lane_u64::<1>(low) << 1)
            | (neon.vgetq_lane_u64::<0>(high) << 2)
            | (neon.vgetq_lane_u64::<1>(high) << 3);
        u32::try_from(bits).unwrap_or(0)
    }

    /// Au moins une voie est vraie.
    #[inline(always)]
    fn any(self) -> bool {
        let neon = self.simd.neon;
        let either = neon.vorrq_u64(self.low, self.high);
        (neon.vgetq_lane_u64::<0>(either) | neon.vgetq_lane_u64::<1>(either)) != 0
    }

    /// Les quatre voies sont vraies.
    #[inline(always)]
    fn all(self) -> bool {
        let neon = self.simd.neon;
        let both = neon.vandq_u64(self.low, self.high);
        neon.vgetq_lane_u64::<0>(both) == u64::MAX && neon.vgetq_lane_u64::<1>(both) == u64::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simd::vec::Mask as _;

    const SAMPLES: [f64; 24] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.5,
        -2.5,
        3.5,
        1e-300,
        -1e-300,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        0.1,
        -0.7,
        1.0 + f64::EPSILON,
        123_456.789,
        -9.87e-5,
    ];

    fn token() -> Neon {
        Neon::detect().expect("NEON is part of every aarch64 target")
    }

    /// Les quatre voies à partir de `offset` dans `SAMPLES`, en bouclant.
    fn quad(offset: usize) -> [f64; 4] {
        std::array::from_fn(|lane| SAMPLES[(offset + lane) % SAMPLES.len()])
    }

    fn same_bits(got: [f64; 4], want: [f64; 4], what: &str) {
        for lane in 0..4 {
            assert_eq!(
                got[lane].to_bits(),
                want[lane].to_bits(),
                "{what}, lane {lane}: {} vs {}",
                got[lane],
                want[lane]
            );
        }
    }

    /// Une opération voie par voie : `V4` rend les bits de la même opération sur des `f64`.
    fn check(what: &str, vector: impl Fn(V4, V4) -> V4, scalar: impl Fn(f64, f64) -> f64) {
        let tok = token();
        for offset in 0..SAMPLES.len() {
            for shift in [0, 5, 11] {
                let (a, b) = (quad(offset), quad(offset + shift));
                let mut got = [0.0; 4];
                vector(V4::load(tok, &a), V4::load(tok, &b)).store(&mut got);
                let want: [f64; 4] = std::array::from_fn(|lane| scalar(a[lane], b[lane]));
                // Un `NaN` a plusieurs écritures : seul le fait d'en être un compte.
                for lane in 0..4 {
                    if want[lane].is_nan() {
                        assert!(got[lane].is_nan(), "{what}, lane {lane}: {}", got[lane]);
                    } else {
                        assert_eq!(
                            got[lane].to_bits(),
                            want[lane].to_bits(),
                            "{what}, lane {lane}: {} vs {}",
                            got[lane],
                            want[lane]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn arithmetic_is_ieee() {
        check("add", |a, b| a + b, |a, b| a + b);
        check("sub", |a, b| a - b, |a, b| a - b);
        check("mul", |a, b| a * b, |a, b| a * b);
        check("div", |a, b| a / b, |a, b| a / b);
    }

    #[test]
    fn unary_operations_match_f64() {
        check("neg", |a, _| -a, |a, _| -a);
        check("abs", |a, _| a.abs(), |a, _| a.abs());
        check("sqrt", |a, _| a.sqrt(), |a, _| a.sqrt());
        check("floor", |a, _| a.floor(), |a, _| a.floor());
        check("round", |a, _| a.round(), |a, _| a.round_ties_even());
        check("copysign", |a, b| a.copysign(b), |a, b| a.copysign(b));
    }

    /// `max` et `min` rendent la seconde opérande sur un `NaN` ou une égalité, comme `maxpd` et
    /// `minpd` : `-0.0` contre `0.0` rend la seconde, et `NaN` contre `1.0` aussi.
    #[test]
    fn max_and_min_follow_the_x86_rule() {
        check("max", |a, b| a.max(b), |a, b| if a > b { a } else { b });
        check("min", |a, b| a.min(b), |a, b| if a < b { a } else { b });
        let tok = token();
        let mut got = [0.0; 4];
        V4::load(tok, &[f64::NAN, 1.0, -0.0, 0.0])
            .max(V4::load(tok, &[1.0, f64::NAN, 0.0, -0.0]))
            .store(&mut got);
        assert_eq!(got[0], 1.0, "NaN then 1");
        assert!(got[1].is_nan(), "1 then NaN gives the second");
        assert!(got[2].is_sign_positive() && got[3].is_sign_negative());
    }

    /// `mul_add` est un vrai FMA : un seul arrondi, celui de `f64::mul_add`.
    #[test]
    fn mul_add_rounds_once() {
        let tok = token();
        let x = 1.0 + 2.0_f64.powi(-30);
        let exact = 2.0_f64.powi(-29) + 2.0_f64.powi(-60);
        assert_ne!(x * x - 1.0, exact, "the test value does tell the two apart");
        let lanes = [x, 0.1, -3.7, 1e150];
        let others = [x, 0.3, 2.25, 1e10];
        let adds = [-1.0, 0.7, 1e-5, -1e160];
        let mut got = [0.0; 4];
        V4::load(tok, &lanes)
            .mul_add(V4::load(tok, &others), V4::load(tok, &adds))
            .store(&mut got);
        let want: [f64; 4] =
            std::array::from_fn(|lane| lanes[lane].mul_add(others[lane], adds[lane]));
        same_bits(got, want, "mul_add");
        assert_eq!(got[0], exact);
    }

    /// Les comparaisons sont fausses sur un `NaN`, et les masques se lisent comme sur AVX2.
    #[test]
    fn comparisons_and_masks() {
        let tok = token();
        for offset in 0..SAMPLES.len() {
            let (a, b) = (quad(offset), quad(offset + 7));
            let (va, vb) = (V4::load(tok, &a), V4::load(tok, &b));
            let cases = [
                (
                    "eq",
                    va.eq(vb),
                    std::array::from_fn::<bool, 4, _>(|l| a[l] == b[l]),
                ),
                (
                    "lt",
                    va.lt(vb),
                    std::array::from_fn::<bool, 4, _>(|l| a[l] < b[l]),
                ),
                (
                    "le",
                    va.le(vb),
                    std::array::from_fn::<bool, 4, _>(|l| a[l] <= b[l]),
                ),
                (
                    "gt",
                    va.gt(vb),
                    std::array::from_fn::<bool, 4, _>(|l| a[l] > b[l]),
                ),
                (
                    "ge",
                    va.ge(vb),
                    std::array::from_fn::<bool, 4, _>(|l| a[l] >= b[l]),
                ),
                (
                    "finite",
                    va.is_finite(),
                    std::array::from_fn::<bool, 4, _>(|l| a[l].is_finite()),
                ),
                (
                    "sign",
                    va.sign_mask(),
                    std::array::from_fn::<bool, 4, _>(|l| a[l].is_sign_negative()),
                ),
            ];
            for (what, mask, want) in cases {
                let want_bits: u32 = (0..4).map(|l| u32::from(want[l]) << l).sum();
                assert_eq!(mask.bits(), want_bits, "{what} bits at {offset}");
                assert_eq!(mask.any(), want_bits != 0, "{what} any");
                assert_eq!(mask.all(), want_bits == 0b1111, "{what} all");
                assert_eq!(mask.not().bits(), !want_bits & 0b1111, "{what} not");
                let picked = V4::select(mask, V4::splat(tok, 1.0), V4::splat(tok, 2.0)).to_array();
                for lane in 0..4 {
                    assert_eq!(
                        picked[lane],
                        if want[lane] { 1.0 } else { 2.0 },
                        "{what} select"
                    );
                }
            }
            let (m, n) = (va.lt(vb), va.gt(vb));
            assert_eq!(m.and(n).bits(), 0, "lt and gt");
            assert_eq!(m.or(n).bits(), m.bits() | n.bits(), "or");
        }
    }

    /// `scale_pow2`, `split_exponent` et `integer_bit_mask` : les briques de `math.rs`.
    #[test]
    fn exponent_helpers() {
        let tok = token();
        let values = [0.75, 1.0, 3.0e-5, 6.02e23, 1.0e-200, 12.5, 9.999, 2.0];
        for pair in values.chunks_exact(4) {
            let lanes: [f64; 4] = std::array::from_fn(|l| pair[l]);
            let (mantissa, exponent) = V4::load(tok, &lanes).split_exponent();
            let (m, e) = (mantissa.to_array(), exponent.to_array());
            for lane in 0..4 {
                let rebuilt = m[lane] * 2.0_f64.powf(e[lane]);
                assert!((1.0..2.0).contains(&m[lane]), "mantissa {}", m[lane]);
                assert_eq!(rebuilt, lanes[lane], "lane {lane}");
            }
        }
        let powers = [-1000_i32, -3, 0, 1000];
        let ks = powers.map(f64::from);
        let xs = [1.5, -2.25, 7.0, 1.0e-300];
        let mut got = [0.0; 4];
        V4::load(tok, &xs)
            .scale_pow2(V4::load(tok, &ks))
            .store(&mut got);
        // Une multiplication par une puissance de deux est exacte : le découpage ne change rien.
        let want: [f64; 4] = std::array::from_fn(|l| {
            xs[l] * 2.0_f64.powi(powers[l] / 2) * 2.0_f64.powi(powers[l] - powers[l] / 2)
        });
        same_bits(got, want, "scale_pow2");
        let ints = [0.0, 1.0, 2.0, 3.0];
        assert_eq!(V4::load(tok, &ints).integer_bit_mask::<63>().bits(), 0b1010);
        assert_eq!(V4::load(tok, &ints).integer_bit_mask::<62>().bits(), 0b1100);
        let negatives = [-1.0, -2.0, -3.0, -4.0];
        // -1 = ...1111, -2 = ...1110, -3 = ...1101, -4 = ...1100 : le bit 0 est à un pour -1 et
        // -3, le bit 1 pour -1 et -2.
        assert_eq!(
            V4::load(tok, &negatives).integer_bit_mask::<63>().bits(),
            0b0101
        );
        assert_eq!(
            V4::load(tok, &negatives).integer_bit_mask::<62>().bits(),
            0b0011
        );
    }
}
