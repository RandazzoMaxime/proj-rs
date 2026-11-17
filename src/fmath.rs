//! Fonctions mathématiques scalaires (une voie), en Rust pur, pour les chemins chauds d'un point.
//!
//! Mêmes séries que `simd/math.rs` (qui en importe les coefficients : une seule source), sans FMA :
//! `f64::mul_add` est logiciel sans FMA matériel, et le code doit tourner à l'identique sur toutes
//! les cibles. Les fonctions ne peuvent pas être partagées, car `V4` est une API à jeton `Avx2`.
//!
//! Une valeur hors du domaine rapide (non finie, grande, hors série) revient à la fonction de la
//! bibliothèque standard : le résultat aux cas limites est celui de la libm. Précision et vitesse
//! mesurées contre elle : `docs/PORTING-simd.md`, section « Fonctions scalaires ».

/// `1/(2k+1)!`, `k` de 1 à 8 : le noyau de `sinh` pour un petit argument.
pub(crate) const SINH_COEFFS: [f64; 8] = [
    1.0 / 6.0,
    1.0 / 120.0,
    1.0 / 5_040.0,
    1.0 / 362_880.0,
    1.0 / 39_916_800.0,
    1.0 / 6_227_020_800.0,
    1.0 / 1_307_674_368_000.0,
    1.0 / 355_687_428_096_000.0,
];

/// `1/(2k+3)`, `k` de 0 à 11 : la queue de la série de `log` en double précision étendue, et celle
/// de `atanh(x) = x * (1 + z * Q(z))` pour `|x| < 0.15`.
pub(crate) const ATANH_TAIL: [f64; 12] = [
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

/// `asinh(x) = x * (1 + z * P(z))`, `z = x^2` : les coefficients de `P` (`(-1)^k C(2k,k) / (4^k (2k+1))`
/// pour `k` de 1 à 11), pour `|x| < 0.15`.
pub(crate) const ASINH_SERIES: [f64; 11] = [
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

/// Les séries scalaires s'arrêtent à `1e-17` relatif sous cette limite : `e * sin(phi)` vaut au
/// plus 0,082 sur un ellipsoïde terrestre, et `|tan Ce|` reste petit à moins de 5° du méridien
/// central.
const SCALAR_SERIES_LIMIT: f64 = 0.1;

/// Schéma d'Estrin : `c[0] + c[1] x + ...` par des paires `c[2i] + c[2i+1] x` combinées par
/// `x^2`, `x^4`, `x^8`. La chaîne de dépendances est de `log2 n` niveaux au lieu de `n - 1` pour
/// Horner, ce qui compte parce qu'un point est une suite de calculs dépendants, sans autre
/// instruction à recouvrir. Une fonction par nombre de coefficients : une récursion générique
/// n'est pas déroulée à la compilation (mesuré : trois fois plus d'instructions).
#[inline(always)]
fn estrin6(x: f64, c: &[f64]) -> f64 {
    let x2 = x * x;
    ((c[0] + c[1] * x) + x2 * (c[2] + c[3] * x)) + (x2 * x2) * (c[4] + c[5] * x)
}

#[inline(always)]
fn estrin9(x: f64, c: &[f64]) -> f64 {
    let x2 = x * x;
    let x4 = x2 * x2;
    let low = ((c[0] + c[1] * x) + x2 * (c[2] + c[3] * x))
        + x4 * ((c[4] + c[5] * x) + x2 * (c[6] + c[7] * x));
    low + (x4 * x4) * c[8]
}

/// `x.hypot(y)` : `sqrt(max^2 + min^2)`, sans dépassement entre `1e-150` et `1e150`.
#[inline]
pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    let a = x.abs();
    let b = y.abs();
    let big = a.max(b);
    let small = a.min(b);
    if a + b < 1e150 && (big > 1e-150 || big == 0.0) {
        (big * big + small * small).sqrt()
    } else {
        x.hypot(y)
    }
}

/// `atanh(x)` par sa série entière pour `|x| < 0.1` (neuf termes) ; `None` au-delà : l'appelant
/// choisit la fonction de repli (la libm C, comme PROJ).
#[inline]
pub(crate) fn atanh_small(x: f64) -> Option<f64> {
    if x.abs() < SCALAR_SERIES_LIMIT {
        let z = x * x;
        Some((x * z) * estrin9(z, &ATANH_TAIL[..9]) + x)
    } else {
        None
    }
}

/// `asinh(x)` par sa série entière pour `|x| < 0.1` (neuf termes) ; au-delà, la libm.
#[inline]
pub(crate) fn asinh(x: f64) -> f64 {
    if x.abs() < SCALAR_SERIES_LIMIT {
        let z = x * x;
        (x * z) * estrin9(z, &ASINH_SERIES[..9]) + x
    } else {
        x.asinh()
    }
}

/// `sinh(x)` par sa série entière pour `|x| < 0.1` (six termes) ; au-delà, la libm.
#[inline]
pub(crate) fn sinh(x: f64) -> f64 {
    if x.abs() < SCALAR_SERIES_LIMIT {
        let z = x * x;
        (x * z) * estrin6(z, &SINH_COEFFS[..6]) + x
    } else {
        x.sinh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        #[link_name = "atanh"]
        fn libm_atanh(x: f64) -> f64;
    }

    const SAMPLES: usize = if cfg!(debug_assertions) {
        200_000
    } else {
        5_000_000
    };

    fn ulps(a: f64, b: f64) -> u64 {
        if a.is_nan() && b.is_nan() || a == b {
            return 0;
        }
        if a.is_nan() || b.is_nan() {
            return u64::MAX;
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

    struct Rng(u64);

    impl Rng {
        fn unit(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            f64::from_bits(0x3FF0_0000_0000_0000 | (self.0 >> 12)) - 1.0
        }

        fn uniform(&mut self, low: f64, high: f64) -> f64 {
            low + self.unit() * (high - low)
        }

        fn signed_magnitude(&mut self, low: f64, high: f64) -> f64 {
            let value = (low.ln() + self.unit() * (high.ln() - low.ln())).exp();
            if self.unit() < 0.5 {
                -value
            } else {
                value
            }
        }
    }

    fn check1(
        name: &str,
        limit: u64,
        mut sample: impl FnMut() -> f64,
        f: fn(f64) -> f64,
        g: fn(f64) -> f64,
    ) {
        let mut worst = (0, 0.0);
        for _ in 0..SAMPLES {
            let x = sample();
            let gap = ulps(f(x), g(x));
            if gap > worst.0 {
                worst = (gap, x);
            }
        }
        eprintln!("ULP {name:24} max {} at {:e}", worst.0, worst.1);
        assert!(worst.0 <= limit, "{name}: {} ULP at {:e}", worst.0, worst.1);
    }

    fn check2(
        name: &str,
        limit: u64,
        mut sample: impl FnMut() -> (f64, f64),
        f: fn(f64, f64) -> f64,
        g: fn(f64, f64) -> f64,
    ) {
        let mut worst = (0, 0.0, 0.0);
        for _ in 0..SAMPLES {
            let (x, y) = sample();
            let gap = ulps(f(x, y), g(x, y));
            if gap > worst.0 {
                worst = (gap, x, y);
            }
        }
        eprintln!(
            "ULP {name:24} max {} at ({:e}, {:e})",
            worst.0, worst.1, worst.2
        );
        assert!(
            worst.0 <= limit,
            "{name}: {} ULP at ({:e}, {:e})",
            worst.0,
            worst.1,
            worst.2
        );
    }

    #[test]
    fn hypot_matches_the_libm() {
        let mut rng = Rng(9);
        check2(
            "hypot wide",
            1,
            || {
                (
                    rng.signed_magnitude(1e-100, 1e100),
                    rng.signed_magnitude(1e-100, 1e100),
                )
            },
            hypot,
            f64::hypot,
        );
        let mut rng = Rng(10);
        check2(
            "hypot [-1e7, 1e7]^2",
            1,
            || (rng.uniform(-1e7, 1e7), rng.uniform(-1e7, 1e7)),
            hypot,
            f64::hypot,
        );
    }

    #[test]
    fn series_match_the_libm() {
        let mut rng = Rng(13);
        check1(
            "sinh [-1, 1]",
            1,
            || rng.uniform(-1.0, 1.0),
            sinh,
            f64::sinh,
        );
        let mut rng = Rng(15);
        check1(
            "asinh [-0.1, 0.1]",
            2,
            || rng.uniform(-0.0999, 0.0999),
            asinh,
            f64::asinh,
        );
        let mut rng = Rng(14);
        check1(
            "atanh [-0.1, 0.1] / glibc",
            2,
            || rng.uniform(-0.0999, 0.0999),
            |x| atanh_small(x).expect("inside the series domain"),
            |x| {
                // SAFETY: `atanh` de la libm C ne prend qu'un `f64` et ne lit ni n'écrit de mémoire.
                unsafe { libm_atanh(x) }
            },
        );
    }

    #[test]
    fn special_values_match_the_libm() {
        let values = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MIN_POSITIVE,
            5e-324,
            -5e-324,
            1e-300,
            1e300,
            -1e300,
            2e6,
            1e22,
            f64::MAX,
        ];
        for &x in &values {
            assert!(ulps(sinh(x), x.sinh()) <= 3, "sinh {x:e}");
            assert!(ulps(asinh(x), x.asinh()) <= 3, "asinh {x:e}");
            for &y in &values {
                assert!(ulps(hypot(y, x), y.hypot(x)) <= 2, "hypot({y:e}, {x:e})");
            }
        }
    }
}
