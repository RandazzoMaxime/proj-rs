//! Fonctions transcendantes sur quatre voies, en Rust, sans bibliothèque externe.
//!
//! Les algorithmes sont ceux de fdlibm (la libm de Sun, reprise par FreeBSD et musl) pour `sin`,
//! `cos`, `exp` et la queue de `log` en double précision étendue, et ceux de Cephes (S. L. Moshier),
//! tels que les reprend la bibliothèque vectorielle d'Agner Fog (Apache-2.0), pour `atan` et
//! `log` : une fraction rationnelle, moins coûteuse qu'un polynôme de degré 22. Tous sont écrits
//! contre l'API de [`V4`] : réduction d'argument de Cody et Waite, sélections par masque à la
//! place des branches, polynômes évalués par le schéma d'Estrin (des chaînes de dépendances plus
//! courtes que Horner). Ils tiennent le niveau de la libm scalaire, de 1 à 3 ULP selon la
//! fonction (le banc de ce fichier en donne la mesure), donc la même précision au nanomètre sur
//! une coordonnée que les versions de la bibliothèque C.
//!
//! Aucune dépendance : ni `libmvec` (glibc), ni l'assembleur en ligne, ni appel à la libm du
//! système. Le code est le même sur Linux, Windows et macOS.
//!
//! Une voie hors du domaine rapide d'une fonction (valeur non finie, argument très grand, très
//! petit ou hors de l'ensemble de définition) est recalculée par la fonction de la bibliothèque
//! standard : le résultat aux cas limites est celui du scalaire, `NaN` et infinis compris. Ces
//! voies sont rares et passent par une fonction froide.

use super::{Mask, V4};

/// `ln 2`, partie haute : 32 bits significatifs, donc `k * LN2_HI` est exact pour `|k| < 2^21`.
const LN2_HI: f64 = 6.931_471_803_691_238_164_90e-1;
/// `ln 2`, partie basse : `ln 2 - LN2_HI`.
const LN2_LO: f64 = 1.908_214_929_270_587_700_02e-10;
/// `1 / ln 2`.
const INV_LN2: f64 = 1.442_695_040_888_963_387_00;
/// `sqrt(2)`.
const SQRT_2: f64 = 1.414_213_562_373_095_1;

/// `pi`, partie haute et basse (`pi - PI_HI`).
const PI_HI: f64 = 3.141_592_653_589_793_116_0;
const PI_LO: f64 = 1.224_646_799_147_353_177_2e-16;

/// `pi / 2` en trois morceaux : la réduction `x - n * pi/2` garde alors la précision relative du
/// reste même près d'un multiple de `pi / 2`.
const PIO2_HI: f64 = 1.570_796_326_794_896_558_0;
const PIO2_MID: f64 = 6.123_233_995_736_766_035_9e-17;
const PIO2_LO: f64 = -1.497_384_904_859_169_8e-33;
/// `2 / pi`.
const TWO_OVER_PI: f64 = 6.366_197_723_675_813_824_3e-1;
/// Au-delà, la réduction par trois morceaux perd de la précision : la voie passe par la libm.
const REDUCTION_LIMIT: f64 = 1_048_576.0;

/// Coefficients du noyau de `sin` sur `[-pi/4, pi/4]` (fdlibm `__kernel_sin`).
const S1: f64 = -1.666_666_666_666_663_243_48e-1;
const S2: f64 = 8.333_333_333_322_489_461_24e-3;
const S3: f64 = -1.984_126_982_985_794_931_34e-4;
const S4: f64 = 2.755_731_370_707_006_767_89e-6;
const S5: f64 = -2.505_076_025_340_686_341_95e-8;
const S6: f64 = 1.589_690_995_211_550_102_21e-10;

/// Coefficients du noyau de `cos` (fdlibm `__kernel_cos`).
const C1: f64 = 4.166_666_666_666_660_190_37e-2;
const C2: f64 = -1.388_888_888_887_410_957_49e-3;
const C3: f64 = 2.480_158_728_947_672_941_78e-5;
const C4: f64 = -2.755_731_435_139_066_330_35e-7;
const C5: f64 = 2.087_572_321_298_174_827_90e-9;
const C6: f64 = -1.135_964_755_778_819_482_65e-11;

/// `atan(z) = z + z * z^2 * P(z^2) / Q(z^2)` pour `|z| <= 0.66` : la fraction rationnelle de
/// Cephes (S. L. Moshier), telle que la reprend la bibliothèque vectorielle d'Agner Fog
/// (Apache-2.0). `ATAN_P[i]` est le coefficient de `(z^2)^i` ; `Q` est unitaire de degré 5.
const ATAN_P: [f64; 5] = [
    -6.485_021_904_942_025_371_773e1,
    -1.228_866_684_490_136_173_410e2,
    -7.500_855_792_314_704_667_340e1,
    -1.615_753_718_733_365_076_637e1,
    -8.750_608_600_031_904_122_785e-1,
];
const ATAN_Q: [f64; 5] = [
    1.945_506_571_482_613_964_425e2,
    4.853_903_996_359_136_964_868e2,
    4.328_810_604_912_902_668_951e2,
    1.650_270_098_316_988_542_046e2,
    2.485_846_490_142_306_297_962e1,
];
/// `sqrt(2) + 1 = tan(3 pi / 8)`, la limite haute de la plage intermédiaire de `atan`.
const TAN_3PI_8: f64 = 2.414_213_562_373_095;
/// Au-dessous, `atan` n'a pas de réduction.
const ATAN_SMALL: f64 = 0.66;

/// `log(1 + f) = f - f^2/2 + f^3 * P(f) / Q(f)` pour `f` dans `[sqrt(1/2) - 1, sqrt(2) - 1]` :
/// la fraction rationnelle de Cephes pour `log`. `LOG_P[i]` est le coefficient de `f^i` (degré 5) ;
/// `Q` est unitaire de degré 5.
const LOG_P: [f64; 6] = [
    7.708_387_337_558_853_916_66,
    1.793_686_785_078_198_163_13e1,
    1.449_892_253_416_109_308_46e1,
    4.705_791_198_788_817_258_54,
    4.974_949_949_767_470_014_25e-1,
    1.018_756_638_045_809_317_96e-4,
];
const LOG_Q: [f64; 5] = [
    2.312_516_201_267_653_405_83e1,
    7.115_447_506_185_638_944_66e1,
    8.298_752_669_127_766_032_11e1,
    4.522_791_458_375_322_211_05e1,
    1.128_735_871_891_674_505_90e1,
];

/// `1/n!`, `n` de 2 à 13 : le noyau de `exp` sur `[-ln2/2, ln2/2]`.
const EXP_COEFFS: [f64; 12] = [
    1.0 / 2.0,
    1.0 / 6.0,
    1.0 / 24.0,
    1.0 / 120.0,
    1.0 / 720.0,
    1.0 / 5_040.0,
    1.0 / 40_320.0,
    1.0 / 362_880.0,
    1.0 / 3_628_800.0,
    1.0 / 39_916_800.0,
    1.0 / 479_001_600.0,
    1.0 / 6_227_020_800.0,
];

/// `1/(2k+1)!`, `k` de 1 à 8 : le noyau de `sinh` pour un petit argument.
const SINH_COEFFS: [f64; 8] = [
    1.0 / 6.0,
    1.0 / 120.0,
    1.0 / 5_040.0,
    1.0 / 362_880.0,
    1.0 / 39_916_800.0,
    1.0 / 6_227_020_800.0,
    1.0 / 1_307_674_368_000.0,
    1.0 / 355_687_428_096_000.0,
];

/// `asinh(x) = x * (1 + z * P(z))`, `z = x^2` : les coefficients de `P` (`(-1)^k C(2k,k) / (4^k (2k+1))`
/// pour `k` de 1 à 11), pour `|x| < 0.15`.
const ASINH_SERIES: [f64; 11] = [
    -0.16666666666666666,
    0.075,
    -0.044642857142857144,
    0.030381944444444444,
    -0.022372159090909092,
    0.017352764423076924,
    -0.01396484375,
    0.011551800896139705,
    -0.009761609529194078,
    0.008390335809616815,
    -0.0073125258735988454,
];

/// Au-dessous de cette valeur absolue, `atanh` et `asinh` s'évaluent par leur série entière.
const SERIES_LIMIT: f64 = 0.15;

/// `1/(2k+3)`, `k` de 0 à 11 : la queue de la série de `log` en double précision étendue, et celle
/// de `atanh(x) = x * (1 + z * Q(z))` pour `|x| < 0.15`.
const ATANH_TAIL: [f64; 12] = [
    1.0 / 3.0,
    1.0 / 5.0,
    1.0 / 7.0,
    1.0 / 9.0,
    1.0 / 11.0,
    1.0 / 13.0,
    1.0 / 15.0,
    1.0 / 17.0,
    1.0 / 19.0,
    1.0 / 21.0,
    1.0 / 23.0,
    1.0 / 25.0,
];

/// Remplace les voies `bad` de `result` par `function(input)`, voie par voie.
#[inline(always)]
fn patch(input: V4, result: V4, bad: Mask, function: fn(f64) -> f64) -> V4 {
    if bad.any() {
        patch_lanes(input, result, bad, function)
    } else {
        result
    }
}

#[cold]
#[inline(never)]
fn patch_lanes(input: V4, result: V4, bad: Mask, function: fn(f64) -> f64) -> V4 {
    let bits = bad.bits();
    let inputs = input.to_array();
    let mut out = result.to_array();
    for (lane, value) in out.iter_mut().enumerate() {
        if (bits >> lane) & 1 == 1 {
            *value = function(inputs[lane]);
        }
    }
    result.reload(&out)
}

/// Comme [`patch`], pour une fonction à deux arguments.
#[inline(always)]
fn patch2(first: V4, second: V4, result: V4, bad: Mask, function: fn(f64, f64) -> f64) -> V4 {
    if bad.any() {
        patch2_lanes(first, second, result, bad, function)
    } else {
        result
    }
}

#[cold]
#[inline(never)]
fn patch2_lanes(first: V4, second: V4, result: V4, bad: Mask, function: fn(f64, f64) -> f64) -> V4 {
    let bits = bad.bits();
    let firsts = first.to_array();
    let seconds = second.to_array();
    let mut out = result.to_array();
    for (lane, value) in out.iter_mut().enumerate() {
        if (bits >> lane) & 1 == 1 {
            *value = function(firsts[lane], seconds[lane]);
        }
    }
    result.reload(&out)
}

/// `exp(r)` pour `|r| <= ln 2 / 2` : `1 + r + r^2 * P(r)`, `P` le développement de Taylor.
#[inline(always)]
fn exp_reduced(r: V4) -> V4 {
    let p = estrin12(r, &EXP_COEFFS);
    (r * r).mul_add(p, r) + r.splat_like(1.0)
}

/// `(m, e)` avec `x = m * 2^e` et `m` dans `[sqrt(1/2), sqrt(2)[`, pour un `x` positif normal.
#[inline(always)]
fn centered_mantissa(x: V4) -> (V4, V4) {
    let k = |value| x.splat_like(value);
    let (m, e) = x.split_exponent();
    let high = m.gt(k(SQRT_2));
    (
        V4::select(high, m * k(0.5), m),
        V4::select(high, e + k(1.0), e),
    )
}

/// `x` est un positif normal fini : le domaine rapide de `log`.
#[inline(always)]
fn positive_normal(x: V4) -> Mask {
    let k = |value| x.splat_like(value);
    x.ge(k(f64::MIN_POSITIVE)).and(x.le(k(f64::MAX)))
}

/// `log(1 + u)` à partir de `arg = 1 + u` (arrondi) et de `log(arg)` : la correction de
/// Goldberg `log(arg) * u / (arg - 1)` rend la précision relative pour un petit `u`.
#[inline(always)]
fn log1p_from(u: V4, arg: V4, log_arg: V4) -> V4 {
    let one = u.splat_like(1.0);
    let unchanged = arg.eq(one);
    V4::select(unchanged, u, log_arg * (u / (arg - one)))
}

/// `sinh(a)` pour un petit `a` : `a * (1 + a^2/3! + a^4/5! + ...)`, sans la soustraction de
/// l'exponentielle qui perdrait des bits.
///
/// Ce sont des fonctions et non des fermetures : une fermeture n'hérite pas du `target_feature` de
/// la fonction qui l'appelle, et n'est pas toujours insérée en ligne : ses intrinsèques deviendraient
/// des appels (voir `v4.rs`).
#[inline(always)]
fn sinh_series(a: V4) -> V4 {
    let k = |value| a.splat_like(value);
    let z = a * a;
    let mut p = k(SINH_COEFFS[7]);
    for coefficient in SINH_COEFFS[..7].iter().rev() {
        p = z.mul_add(p, k(*coefficient));
    }
    (a * z).mul_add(p, a)
}

/// `sinh(a) = (e^a - e^-a) / 2`, pour un `a` qui n'est pas petit.
#[inline(always)]
fn sinh_exponential(a: V4) -> V4 {
    let k = |value| a.splat_like(value);
    let e = a.exp();
    k(0.5) * (e - k(1.0) / e)
}

/// `c[0] + c[1] x + ... + c[5] x^5` par le schéma d'Estrin : trois niveaux de FMA au lieu de
/// cinq pour Horner, donc une chaîne de dépendances plus courte (les trois paires et les deux
/// puissances de `x` se calculent en parallèle).
#[inline(always)]
fn estrin5(x: V4, c: &[f64; 6]) -> V4 {
    let k = |value| x.splat_like(value);
    let x2 = x * x;
    let x4 = x2 * x2;
    let low = x.mul_add(k(c[1]), k(c[0]));
    let mid = x.mul_add(k(c[3]), k(c[2]));
    let high = x.mul_add(k(c[5]), k(c[4]));
    x4.mul_add(high, x2.mul_add(mid, low))
}

/// Comme [`estrin5`], pour le degré 5 d'un polynôme **unitaire** : `c[0] + ... + c[4] x^4 + x^5`.
#[inline(always)]
fn estrin5_monic(x: V4, c: &[f64; 5]) -> V4 {
    let k = |value| x.splat_like(value);
    let x2 = x * x;
    let x4 = x2 * x2;
    let low = x.mul_add(k(c[1]), k(c[0]));
    let mid = x.mul_add(k(c[3]), k(c[2]));
    // c[4] x^4 + x^5 = x^4 (c[4] + x)
    let high = x + k(c[4]);
    x4.mul_add(high, x2.mul_add(mid, low))
}

/// Comme [`estrin5`], pour le degré 4 : `c[0] + ... + c[4] x^4`.
#[inline(always)]
fn estrin4(x: V4, c: &[f64; 5]) -> V4 {
    let k = |value| x.splat_like(value);
    let x2 = x * x;
    let low = x.mul_add(k(c[1]), k(c[0]));
    let mid = x.mul_add(k(c[3]), k(c[2]));
    x2.mul_add(x2.mul_add(k(c[4]), mid), low)
}

/// `c[0] + c[1] x + ... + c[11] x^11` par le schéma d'Estrin : quatre niveaux de FMA au lieu de
/// onze pour Horner.
#[inline(always)]
fn estrin12(x: V4, c: &[f64; 12]) -> V4 {
    let k = |value| x.splat_like(value);
    let x2 = x * x;
    let x4 = x2 * x2;
    let x8 = x4 * x4;
    let a0 = x.mul_add(k(c[1]), k(c[0]));
    let a1 = x.mul_add(k(c[3]), k(c[2]));
    let a2 = x.mul_add(k(c[5]), k(c[4]));
    let a3 = x.mul_add(k(c[7]), k(c[6]));
    let a4 = x.mul_add(k(c[9]), k(c[8]));
    let a5 = x.mul_add(k(c[11]), k(c[10]));
    let b0 = x2.mul_add(a1, a0);
    let b1 = x2.mul_add(a3, a2);
    let b2 = x2.mul_add(a5, a4);
    x8.mul_add(b2, x4.mul_add(b1, b0))
}

/// `atan(z)` pour `|z| <= 0.66` : `z + z * z^2 * P(z^2) / Q(z^2)` (Cephes, voir [`ATAN_P`]).
#[inline(always)]
fn atan_rational(z: V4) -> V4 {
    let zz = z * z;
    let p = estrin4(zz, &ATAN_P);
    let q = estrin5_monic(zz, &ATAN_Q);
    (p / q).mul_add(z * zz, z)
}

impl V4 {
    /// Quatre exponentielles.
    #[inline(always)]
    pub(crate) fn exp(self) -> V4 {
        let k = |value| self.splat_like(value);
        let fast = self.abs().le(k(700.0));
        let x = V4::select(fast, self, k(0.0));
        let n = (x * k(INV_LN2)).round();
        let r = (-n).mul_add(k(LN2_HI), x);
        let r = (-n).mul_add(k(LN2_LO), r);
        patch(self, exp_reduced(r).scale_pow2(n), fast.not(), f64::exp)
    }

    /// Quatre logarithmes naturels.
    ///
    /// Fraction rationnelle de Cephes (voir [`LOG_P`]) sur la mantisse centrée en 1, avec `ln 2`
    /// en deux morceaux de fdlibm.
    #[inline(always)]
    pub(crate) fn log(self) -> V4 {
        let k = |value| self.splat_like(value);
        let fast = positive_normal(self);
        let x = V4::select(fast, self, k(1.0));
        let (m, e) = centered_mantissa(x);
        let f = m - k(1.0);
        let p = estrin5(f, &LOG_P);
        let q = estrin5_monic(f, &LOG_Q);
        let f2 = f * f;
        let ratio = (f2 * f) * p / q;
        // f - f^2/2 + ratio, avec `e * ln 2` en deux morceaux ajoutés aux deux extrémités.
        let result = e.mul_add(k(LN2_LO), ratio);
        let result = result + (-f2).mul_add(k(0.5), f);
        let result = e.mul_add(k(LN2_HI), result);
        patch(self, result, fast.not(), f64::ln)
    }

    /// Sinus et cosinus de la même valeur : une seule réduction d'argument.
    #[inline(always)]
    pub(crate) fn sin_cos(self) -> (V4, V4) {
        let k = |value| self.splat_like(value);
        let fast = self.abs().lt(k(REDUCTION_LIMIT));
        let x = V4::select(fast, self, k(0.0));
        // `n` est l'entier le plus proche de `x * 2/pi` ; `r = x - n * pi/2`, `|r| <= pi/4`.
        let n = (x * k(TWO_OVER_PI)).round();
        let r = (-n).mul_add(k(PIO2_HI), x);
        let r = (-n).mul_add(k(PIO2_MID), r);
        let r = (-n).mul_add(k(PIO2_LO), r);
        let z = r * r;
        // fdlibm `__kernel_sin` : r + r^3 * (S1 + z * (S2 + z * (S3 + ...))).
        let p = estrin4(z, &[S2, S3, S4, S5, S6]);
        let sin_r = (z * r).mul_add(z.mul_add(p, k(S1)), r);
        // fdlibm `__kernel_cos` : 1 - z/2 + z^2 * (C1 + z * (C2 + ...)), avec la compensation
        // `(1 - w) - hz` qui rend les bits perdus par `1 - z/2`.
        let w2 = z * z;
        let q1 = z.mul_add(z.mul_add(k(C3), k(C2)), k(C1));
        let q2 = z.mul_add(z.mul_add(k(C6), k(C5)), k(C4));
        let poly = (z * q1).mul_add(k(1.0), w2 * w2 * q2);
        let hz = k(0.5) * z;
        let w = k(1.0) - hz;
        let cos_r = w + (((k(1.0) - w) - hz) + z * poly);
        // Le quadrant `n mod 4` choisit et signe : sin = [s, c, -s, -c], cos = [c, -s, -c, s].
        let odd = n.integer_bit_mask::<63>();
        let negative_sin = n.integer_bit_mask::<62>();
        let negative_cos = (n + k(1.0)).integer_bit_mask::<62>();
        let sin = V4::select(odd, cos_r, sin_r);
        let sin = V4::select(negative_sin, -sin, sin);
        let cos = V4::select(odd, sin_r, cos_r);
        let cos = V4::select(negative_cos, -cos, cos);
        // Un petit argument garde son signe : `sin(-0) = -0`.
        let tiny = x.abs().lt(k(7.450_580_596_923_828e-9));
        let sin = V4::select(tiny, x, sin);
        let bad = fast.not();
        (
            patch(self, sin, bad, f64::sin),
            patch(self, cos, bad, f64::cos),
        )
    }

    /// Quatre tangentes.
    #[inline(always)]
    pub(crate) fn tan(self) -> V4 {
        let (sin, cos) = self.sin_cos();
        sin / cos
    }

    /// Quatre arcs tangente.
    ///
    /// Cephes, comme la bibliothèque d'Agner Fog : trois plages, une fraction rationnelle
    /// ([`ATAN_P`], [`ATAN_Q`]).
    #[inline(always)]
    pub(crate) fn atan(self) -> V4 {
        let k = |value| self.splat_like(value);
        let one = k(1.0);
        let t = self.abs();
        // Toutes les voies petites : pas de réduction, ni de division de plage. La fonction est
        // impaire, le signe vient tout seul. Un `NaN` ne passe pas `lt` : il va au chemin général.
        if t.lt(k(ATAN_SMALL)).all() {
            return atan_rational(self);
        }
        // Petit : z = t, intermédiaire (t <= tan 3pi/8) : z = (t - 1) / (t + 1) + pi/4,
        // grand : z = -1 / t + pi/2.
        let not_big = t.le(k(TAN_3PI_8));
        let not_small = t.ge(k(ATAN_SMALL));
        let num = V4::select(not_small, V4::select(not_big, t - one, -one), t);
        let den = V4::select(not_small, V4::select(not_big, t + one, t), one);
        let base = V4::select(
            not_big,
            k(std::f64::consts::FRAC_PI_4),
            k(std::f64::consts::FRAC_PI_2),
        );
        let base = V4::select(not_small, base, k(0.0));
        // Le complément de `pi/4` ou `pi/2` en double précision, ajouté d'abord.
        let low = V4::select(not_big, k(PIO2_MID * 0.5), k(PIO2_MID));
        let low = V4::select(not_small, low, k(0.0));
        ((atan_rational(num / den) + low) + base).copysign(self)
    }

    /// Quatre `self.atan2(x)`, `self` étant `y`, dans l'ordre de `f64::atan2`.
    ///
    /// On se ramène au premier octant (`t = min/max` dans `[0, 1]`) puis `atan(t)` par la fraction
    /// de [`atan_rational`], et on remet l'octant et le quadrant par une base (`0`, `pi/2` ou `pi`)
    /// et un signe.
    #[inline(always)]
    pub(crate) fn atan2(self, x: V4) -> V4 {
        let k = |value| self.splat_like(value);
        let fast = self.is_finite().and(x.is_finite());
        let ay = self.abs();
        let ax = x.abs();
        let swap = ay.gt(ax);
        let num = V4::select(swap, ax, ay);
        let den = V4::select(swap, ay, ax);
        // Les deux nuls donnent `0 / 0` : l'angle est 0 (puis `pi` si `x` est `-0`).
        let both_zero = ay.eq(k(0.0)).and(ax.eq(k(0.0)));
        let t = V4::select(both_zero, k(0.0), num / den);
        // t < 0.66 : z = t ; sinon z = (t - 1) / (t + 1), et `pi/4` s'ajoute.
        let not_small = t.ge(k(ATAN_SMALL));
        let z = V4::select(not_small, t - k(1.0), t) / V4::select(not_small, t + k(1.0), k(1.0));
        let low = V4::select(not_small, k(PIO2_MID * 0.5), k(0.0));
        let high = V4::select(not_small, k(std::f64::consts::FRAC_PI_4), k(0.0));
        let angle = (atan_rational(z) + low) + high;
        // octant (swap) et quadrant (x < 0) : base + signe * angle.
        let x_negative = x.sign_mask();
        let negated = -angle;
        let signed = V4::select(
            swap,
            V4::select(x_negative, angle, negated),
            V4::select(x_negative, negated, angle),
        );
        let base_high = V4::select(swap, k(PIO2_HI), V4::select(x_negative, k(PI_HI), k(0.0)));
        let base_low = V4::select(swap, k(PIO2_MID), V4::select(x_negative, k(PI_LO), k(0.0)));
        let result = (signed + base_low) + base_high;
        patch2(self, x, result.copysign(self), fast.not(), f64::atan2)
    }

    /// Quatre `self.hypot(other)`.
    #[inline(always)]
    pub(crate) fn hypot(self, other: V4) -> V4 {
        let k = |value| self.splat_like(value);
        let a = self.abs();
        let b = other.abs();
        let big = a.max(b);
        let fast = a
            .is_finite()
            .and(b.is_finite())
            .and(big.lt(k(1e150)))
            .and(big.gt(k(1e-150)).or(big.eq(k(0.0))));
        let small = a.min(b);
        patch2(
            self,
            other,
            big.mul_add(big, small * small).sqrt(),
            fast.not(),
            f64::hypot,
        )
    }

    /// Quatre arguments sinus hyperbolique.
    #[inline(always)]
    pub(crate) fn asinh(self) -> V4 {
        let k = |value| self.splat_like(value);
        let one = k(1.0);
        // Tous les arguments petits : la série entière, impaire, sans logarithme.
        if self.abs().lt(k(SERIES_LIMIT)).all() {
            let z = self * self;
            let mut p = k(ASINH_SERIES[10]);
            for coefficient in ASINH_SERIES[..10].iter().rev() {
                p = z.mul_add(p, k(*coefficient));
            }
            return (self * z).mul_add(p, self);
        }
        let ax = self.abs();
        let fast = ax.lt(k(1e150));
        let a = V4::select(fast, ax, one);
        // `a < 2` : log1p(a + t / (1 + sqrt(1 + t))), `t = a^2` ; sinon log(2a + 1/(sqrt(a^2+1) + a)).
        let small = a.lt(k(2.0));
        let t = a * a;
        let u = a + t / (one + t.add_one().sqrt());
        let wide = k(2.0) * a + one / (t.add_one().sqrt() + a);
        let arg = V4::select(small, one + u, wide);
        let log = arg.log();
        let value = V4::select(small, log1p_from(u, arg, log), log);
        // Un très petit argument est sa propre image.
        let tiny = a.lt(k(3.725_290_298_461_914e-9));
        let value = V4::select(tiny, a, value);
        patch(self, value.copysign(self), fast.not(), f64::asinh)
    }

    /// Quatre arguments tangente hyperbolique.
    #[inline(always)]
    pub(crate) fn atanh(self) -> V4 {
        let k = |value| self.splat_like(value);
        let one = k(1.0);
        // Tous les arguments petits : la série entière, impaire, sans logarithme ni division.
        if self.abs().lt(k(SERIES_LIMIT)).all() {
            let z = self * self;
            let mut q = k(ATANH_TAIL[11]);
            for coefficient in ATANH_TAIL[..11].iter().rev() {
                q = z.mul_add(q, k(*coefficient));
            }
            return (self * z).mul_add(q, self);
        }
        let ax = self.abs();
        let fast = ax.lt(one);
        let a = V4::select(fast, ax, k(0.5));
        let u = (a + a) / (one - a);
        let arg = one + u;
        let value = k(0.5) * log1p_from(u, arg, arg.log());
        let tiny = a.lt(k(3.725_290_298_461_914e-9));
        let value = V4::select(tiny, a, value);
        patch(self, value.copysign(self), fast.not(), f64::atanh)
    }

    /// Quatre sinus hyperboliques.
    #[inline(always)]
    pub(crate) fn sinh(self) -> V4 {
        let k = |value| self.splat_like(value);
        let ax = self.abs();
        let fast = ax.lt(k(700.0));
        let a = V4::select(fast, ax, k(1.0));
        let small = a.lt(k(0.5));
        // On ne calcule que ce qu'une voie demande : la série si toutes sont petites, l'exponentielle
        // si aucune ne l'est, les deux sinon.
        let value = if small.all() {
            sinh_series(a)
        } else if !small.any() {
            sinh_exponential(a)
        } else {
            V4::select(small, sinh_series(a), sinh_exponential(a))
        };
        patch(self, value.copysign(self), fast.not(), f64::sinh)
    }

    /// Quatre `self.powf(y)`, pour `self > 0`. Le logarithme est calculé en double précision
    /// étendue (deux `f64`), puis l'exponentielle de `y * log(self)` : l'erreur de `log` ne
    /// grossit pas avec `|y * log(self)|`.
    #[inline(always)]
    pub(crate) fn powf(self, y: V4) -> V4 {
        let k = |value| self.splat_like(value);
        let positive = positive_normal(self);
        let x = V4::select(positive, self, k(1.0));
        let (log_hi, log_lo) = log_dd(x);
        let z_hi = y * log_hi;
        let z_lo = y.mul_add(log_hi, -z_hi) + y * log_lo;
        let fast = positive.and(y.is_finite()).and(z_hi.abs().le(k(700.0)));
        let z_hi = V4::select(fast, z_hi, k(0.0));
        let n = (z_hi * k(INV_LN2)).round();
        let r = (-n).mul_add(k(LN2_HI), z_hi);
        let r = (-n).mul_add(k(LN2_LO), r) + V4::select(fast, z_lo, k(0.0));
        patch2(self, y, exp_reduced(r).scale_pow2(n), fast.not(), f64::powf)
    }

    /// `self + 1`.
    #[inline(always)]
    fn add_one(self) -> V4 {
        self + self.splat_like(1.0)
    }
}

/// `log(x)` en deux `f64` (`hi + lo`, erreur de l'ordre de `2^-65` relatif), `x` positif normal.
#[inline(always)]
fn log_dd(x: V4) -> (V4, V4) {
    let k = |value| x.splat_like(value);
    let (m, e) = centered_mantissa(x);
    // log(m) = 2 atanh(s), s = f / (2 + f) = s0 + s_lo : `2 + f` n'est pas exact, son défaut
    // `d_lo` corrige le quotient.
    let f = m - k(1.0);
    let d_hi = k(2.0) + f;
    let d_lo = f - (d_hi - k(2.0));
    let s0 = f / d_hi;
    let residual = (-s0).mul_add(d_hi, f) - s0 * d_lo;
    let s_lo = residual / d_hi;
    let z = s0 * s0;
    let mut q = k(ATANH_TAIL[11]);
    for coefficient in ATANH_TAIL[..11].iter().rev() {
        q = z.mul_add(q, k(*coefficient));
    }
    let tail = (k(2.0) * s0 * z) * q;
    // somme exacte (TwoSum) de `e * LN2_HI` et `2 * s0`, puis des termes bas.
    let high_part = e * k(LN2_HI);
    let double_s = s0 + s0;
    let sum = high_part + double_s;
    let virtual_part = sum - high_part;
    let error = (high_part - (sum - virtual_part)) + (double_s - virtual_part);
    let low = error + (s_lo + s_lo + tail) + e * k(LN2_LO);
    let hi = sum + low;
    let lo = low - (hi - sum);
    (hi, lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simd::Avx2;

    /// Groupes de quatre points par cas. Un build de débogage n'insère aucun intrinsèque en ligne :
    /// il est plus de cent fois plus lent, d'où moins de tirages (la CI lance les tests ainsi).
    const SAMPLES: usize = if cfg!(debug_assertions) {
        3_000
    } else {
        250_000
    };

    /// Une fonction vectorielle à un argument, sa référence scalaire.
    type UnaryCase = (&'static str, fn(V4) -> V4, fn(f64) -> f64);
    /// Idem pour deux arguments.
    type BinaryCase = (&'static str, fn(V4, V4) -> V4, fn(f64, f64) -> f64);

    /// La distance en ULP de deux `f64` : le nombre de valeurs représentables entre eux.
    fn ulps(a: f64, b: f64) -> u64 {
        if a.is_nan() && b.is_nan() {
            return 0;
        }
        if a.is_nan() || b.is_nan() {
            return u64::MAX;
        }
        if a == b {
            return 0;
        }
        let key = |x: f64| {
            let bits = i64::from_ne_bytes(x.to_bits().to_ne_bytes());
            if bits < 0 {
                i64::MIN.wrapping_sub(bits)
            } else {
                bits
            }
        };
        key(a).wrapping_sub(key(b)).unsigned_abs()
    }

    /// Un générateur congruentiel : des tirages déterministes dans `[0, 1[`.
    struct Rng(u64);

    impl Rng {
        fn unit(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            // Les 52 bits de poids fort remplissent la mantisse d'un nombre de `[1, 2[`.
            f64::from_bits(0x3FF0_0000_0000_0000 | (self.0 >> 12)) - 1.0
        }

        /// Uniforme dans `[low, high]`.
        fn uniform(&mut self, low: f64, high: f64) -> f64 {
            low + self.unit() * (high - low)
        }

        /// Log-uniforme sur les grandeurs `[low, high]`, positives.
        fn magnitude(&mut self, low: f64, high: f64) -> f64 {
            (low.ln() + self.unit() * (high.ln() - low.ln())).exp()
        }

        /// Comme `magnitude`, de signe aléatoire.
        fn signed_magnitude(&mut self, low: f64, high: f64) -> f64 {
            let value = self.magnitude(low, high);
            if self.unit() < 0.5 {
                -value
            } else {
                value
            }
        }
    }

    /// Évalue `vector` sur quatre voies et rend la pire erreur contre `reference`, avec l'entrée.
    fn worst1(
        tok: Avx2,
        mut sample: impl FnMut() -> f64,
        vector: impl Fn(V4) -> V4,
        reference: impl Fn(f64) -> f64,
    ) -> (u64, f64) {
        let mut worst = (0, 0.0);
        for _ in 0..SAMPLES {
            let xs = [sample(), sample(), sample(), sample()];
            let mut out = [0.0; 4];
            vector(V4::load(tok, &xs)).store(&mut out);
            for lane in 0..4 {
                let want = reference(xs[lane]);
                let gap = ulps(out[lane], want);
                if gap > worst.0 {
                    worst = (gap, xs[lane]);
                }
            }
        }
        worst
    }

    fn worst2(
        tok: Avx2,
        mut sample: impl FnMut() -> (f64, f64),
        vector: impl Fn(V4, V4) -> V4,
        reference: impl Fn(f64, f64) -> f64,
    ) -> (u64, f64, f64) {
        let mut worst = (0, 0.0, 0.0);
        for _ in 0..SAMPLES {
            let points = [sample(), sample(), sample(), sample()];
            let xs = [points[0].0, points[1].0, points[2].0, points[3].0];
            let ys = [points[0].1, points[1].1, points[2].1, points[3].1];
            let mut out = [0.0; 4];
            vector(V4::load(tok, &xs), V4::load(tok, &ys)).store(&mut out);
            for lane in 0..4 {
                let want = reference(xs[lane], ys[lane]);
                let gap = ulps(out[lane], want);
                if gap > worst.0 {
                    worst = (gap, xs[lane], ys[lane]);
                }
            }
        }
        worst
    }

    /// `f64::atanh` calcule `0.5 * ln_1p(2x / (1 - x))`, qui perd presque tous ses chiffres pour `x`
    /// proche de -1 (`1 + u` vaut alors 3e-6). La fonction est impaire : la référence travaille sur `|x|`.
    fn atanh_reference(x: f64) -> f64 {
        x.abs().atanh().copysign(x)
    }

    macro_rules! unary_case {
        ($tok:expr, $name:literal, $domain:literal, $sample:expr, $vector:expr, $reference:expr, $limit:expr) => {{
            let (gap, at) = worst1($tok, $sample, $vector, $reference);
            eprintln!("ULP {:6} {:28} max {gap} ULP at {at:e}", $name, $domain);
            assert!(
                gap <= $limit,
                "{} on {}: {gap} ULP at {at:e}",
                $name,
                $domain
            );
        }};
    }

    macro_rules! binary_case {
        ($tok:expr, $name:literal, $domain:literal, $sample:expr, $vector:expr, $reference:expr, $limit:expr) => {{
            let (gap, x, y) = worst2($tok, $sample, $vector, $reference);
            eprintln!(
                "ULP {:6} {:28} max {gap} ULP at ({x:e}, {y:e})",
                $name, $domain
            );
            assert!(
                gap <= $limit,
                "{} on {}: {gap} ULP at ({x:e}, {y:e})",
                $name,
                $domain
            );
        }};
    }

    /// L'erreur mesurée contre la libm de la bibliothèque standard, sur de larges domaines : 1 ULP
    /// pour `sin`, `cos`, `exp`, `log`, `atan`, `atan2`, `hypot` et `powf`, 2 pour `asinh`,
    /// `atanh` et `sinh`, 3 pour `tan` (quotient de deux fonctions à 1 ULP). Les bornes ci-dessous
    /// sont ces mesures plus un. `libmvec` annonce jusqu'à 4 ULP.
    #[test]
    fn accuracy_matches_the_libm() {
        let Some(tok) = Avx2::detect() else {
            return;
        };
        let mut rng = Rng(1);
        let pi = std::f64::consts::PI;
        unary_case!(
            tok,
            "sin",
            "[-100, 100]",
            || rng.uniform(-100.0, 100.0),
            |v| v.sin_cos().0,
            f64::sin,
            2
        );
        unary_case!(
            tok,
            "sin",
            "[-1e5, 1e5]",
            || rng.uniform(-1e5, 1e5),
            |v| v.sin_cos().0,
            f64::sin,
            2
        );
        unary_case!(
            tok,
            "sin",
            "|x| in [1e-9, 1]",
            || rng.signed_magnitude(1e-9, 1.0),
            |v| v.sin_cos().0,
            f64::sin,
            2
        );
        unary_case!(
            tok,
            "cos",
            "[-100, 100]",
            || rng.uniform(-100.0, 100.0),
            |v| v.sin_cos().1,
            f64::cos,
            2
        );
        unary_case!(
            tok,
            "cos",
            "[-1e5, 1e5]",
            || rng.uniform(-1e5, 1e5),
            |v| v.sin_cos().1,
            f64::cos,
            2
        );
        unary_case!(
            tok,
            "sin",
            "near k * pi/2",
            || {
                let n = (rng.unit() * 2000.0).floor() - 1000.0;
                n * pi / 2.0 + rng.signed_magnitude(1e-12, 1e-2)
            },
            |v| v.sin_cos().0,
            f64::sin,
            2
        );
        unary_case!(
            tok,
            "tan",
            "[-1.5, 1.5]",
            || rng.uniform(-1.5, 1.5),
            V4::tan,
            f64::tan,
            4
        );
        unary_case!(
            tok,
            "exp",
            "[-700, 700]",
            || rng.uniform(-700.0, 700.0),
            V4::exp,
            f64::exp,
            2
        );
        unary_case!(
            tok,
            "exp",
            "[-2, 2]",
            || rng.uniform(-2.0, 2.0),
            V4::exp,
            f64::exp,
            2
        );
        unary_case!(
            tok,
            "log",
            "x in [1e-300, 1e300]",
            || rng.magnitude(1e-300, 1e300),
            V4::log,
            f64::ln,
            2
        );
        unary_case!(
            tok,
            "log",
            "[0.5, 2]",
            || rng.uniform(0.5, 2.0),
            V4::log,
            f64::ln,
            2
        );
        unary_case!(
            tok,
            "log",
            "[0.999, 1.001]",
            || rng.uniform(0.999, 1.001),
            V4::log,
            f64::ln,
            2
        );
        unary_case!(
            tok,
            "atan",
            "[-1000, 1000]",
            || rng.uniform(-1000.0, 1000.0),
            V4::atan,
            f64::atan,
            2
        );
        unary_case!(
            tok,
            "atan",
            "|x| in [1e-10, 1e10]",
            || rng.signed_magnitude(1e-10, 1e10),
            V4::atan,
            f64::atan,
            2
        );
        unary_case!(
            tok,
            "asinh",
            "[-10, 10]",
            || rng.uniform(-10.0, 10.0),
            V4::asinh,
            f64::asinh,
            3
        );
        unary_case!(
            tok,
            "asinh",
            "[-0.14, 0.14] (series)",
            || rng.uniform(-0.14, 0.14),
            V4::asinh,
            f64::asinh,
            3
        );
        unary_case!(
            tok,
            "atan",
            "[-0.4, 0.4] (no reduction)",
            || rng.uniform(-0.4, 0.4),
            V4::atan,
            f64::atan,
            2
        );
        unary_case!(
            tok,
            "sinh",
            "[-0.45, 0.45] (series)",
            || rng.uniform(-0.45, 0.45),
            V4::sinh,
            f64::sinh,
            3
        );
        unary_case!(
            tok,
            "sinh",
            "[0.6, 20] (exponential)",
            || rng.uniform(0.6, 20.0),
            V4::sinh,
            f64::sinh,
            3
        );
        unary_case!(
            tok,
            "asinh",
            "|x| in [1e-10, 1e10]",
            || rng.signed_magnitude(1e-10, 1e10),
            V4::asinh,
            f64::asinh,
            3
        );
        unary_case!(
            tok,
            "atanh",
            "(-1, 1)",
            || rng.uniform(-0.999_999, 0.999_999),
            V4::atanh,
            atanh_reference,
            3
        );
        unary_case!(
            tok,
            "atanh",
            "|x| in [1e-10, 0.1]",
            || rng.signed_magnitude(1e-10, 0.1),
            V4::atanh,
            atanh_reference,
            3
        );
        unary_case!(
            tok,
            "sinh",
            "[-10, 10]",
            || rng.uniform(-10.0, 10.0),
            V4::sinh,
            f64::sinh,
            3
        );
        unary_case!(
            tok,
            "sinh",
            "|x| in [1e-10, 1]",
            || rng.signed_magnitude(1e-10, 1.0),
            V4::sinh,
            f64::sinh,
            3
        );
        unary_case!(
            tok,
            "sinh",
            "[-700, 700]",
            || rng.uniform(-700.0, 700.0),
            V4::sinh,
            f64::sinh,
            3
        );
        binary_case!(
            tok,
            "atan2",
            "[-10, 10]^2",
            || (rng.uniform(-10.0, 10.0), rng.uniform(-10.0, 10.0)),
            |a, b| a.atan2(b),
            f64::atan2,
            2
        );
        binary_case!(
            tok,
            "atan2",
            "wide magnitudes",
            || (
                rng.signed_magnitude(1e-100, 1e100),
                rng.signed_magnitude(1e-100, 1e100)
            ),
            |a, b| a.atan2(b),
            f64::atan2,
            2
        );
        binary_case!(
            tok,
            "hypot",
            "wide magnitudes",
            || (rng.magnitude(1e-100, 1e100), rng.magnitude(1e-100, 1e100)),
            |a, b| a.hypot(b),
            f64::hypot,
            2
        );
        binary_case!(
            tok,
            "powf",
            "x [0.01, 100], y [-5, 5]",
            || (rng.uniform(0.01, 100.0), rng.uniform(-5.0, 5.0)),
            |a, b| a.powf(b),
            f64::powf,
            2
        );
        binary_case!(
            tok,
            "powf",
            "x [0.5, 2], y [-30, 30]",
            || (rng.uniform(0.5, 2.0), rng.uniform(-30.0, 30.0)),
            |a, b| a.powf(b),
            f64::powf,
            2
        );
    }

    /// Les valeurs spéciales donnent ce que donne la libm : `NaN`, infinis, zéros signés,
    /// négatifs, sous-normaux, dépassements, arguments très grands.
    #[test]
    fn special_values_match_the_libm() {
        let Some(tok) = Avx2::detect() else {
            return;
        };
        let specials = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MIN_POSITIVE,
            5e-324,
            -5e-324,
            f64::MAX,
            f64::MIN,
            1e-300,
            1e300,
            709.0,
            710.0,
            -745.0,
            -746.0,
            1e6,
            1e15,
            std::f64::consts::PI,
            std::f64::consts::FRAC_PI_2,
        ];
        let functions: [UnaryCase; 9] = [
            ("sin", |v| v.sin_cos().0, f64::sin),
            ("cos", |v| v.sin_cos().1, f64::cos),
            ("tan", V4::tan, f64::tan),
            ("exp", V4::exp, f64::exp),
            ("log", V4::log, f64::ln),
            ("atan", V4::atan, f64::atan),
            ("asinh", V4::asinh, f64::asinh),
            ("atanh", V4::atanh, f64::atanh),
            ("sinh", V4::sinh, f64::sinh),
        ];
        for (name, vector, reference) in functions {
            for chunk in specials.chunks(4) {
                let mut xs = [1.0; 4];
                xs[..chunk.len()].copy_from_slice(chunk);
                let mut out = [0.0; 4];
                vector(V4::load(tok, &xs)).store(&mut out);
                for lane in 0..chunk.len() {
                    let want = reference(xs[lane]);
                    let gap = ulps(out[lane], want);
                    // `atanh` de `std` est faux près de -1 (voir `atanh_reference`) : on compare
                    // la version symétrique.
                    let want = if name == "atanh" {
                        atanh_reference(xs[lane])
                    } else {
                        want
                    };
                    let gap = if name == "atanh" {
                        ulps(out[lane], want)
                    } else {
                        gap
                    };
                    assert!(
                        gap <= 4,
                        "{name}({:e}) = {:e}, libm {want:e} ({gap} ULP)",
                        xs[lane],
                        out[lane]
                    );
                }
            }
        }
        // Les deux arguments : atan2, hypot, powf.
        let pairs: [BinaryCase; 3] = [
            ("atan2", |a, b| a.atan2(b), f64::atan2),
            ("hypot", |a, b| a.hypot(b), f64::hypot),
            ("powf", |a, b| a.powf(b), f64::powf),
        ];
        for (name, vector, reference) in pairs {
            for first in specials.chunks(4) {
                for second in specials.chunks(4) {
                    let mut xs = [1.0; 4];
                    let mut ys = [1.0; 4];
                    xs[..first.len()].copy_from_slice(first);
                    ys[..second.len()].copy_from_slice(second);
                    // Chaque voie de `xs` rencontre chaque rotation de `ys`.
                    for shift in 0..4 {
                        let rotated = [
                            ys[shift % 4],
                            ys[(shift + 1) % 4],
                            ys[(shift + 2) % 4],
                            ys[(shift + 3) % 4],
                        ];
                        let mut out = [0.0; 4];
                        vector(V4::load(tok, &xs), V4::load(tok, &rotated)).store(&mut out);
                        for lane in 0..4 {
                            let want = reference(xs[lane], rotated[lane]);
                            let gap = ulps(out[lane], want);
                            assert!(
                                gap <= 4,
                                "{name}({:e}, {:e}) = {:e}, libm {want:e} ({gap} ULP)",
                                xs[lane],
                                rotated[lane],
                                out[lane]
                            );
                        }
                    }
                }
            }
        }
    }
}
