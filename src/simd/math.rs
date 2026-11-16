//! Fonctions transcendantes sur quatre voies.
//!
//! Sous Linux avec glibc, un appel `libmvec` calcule les quatre voies à la fois (précision de
//! quelques ULP, comme les versions scalaires de la glibc). Ailleurs, la fonction de la
//! bibliothèque C est appelée une voie à la fois : le reste du noyau reste vectoriel.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
use std::arch::x86_64::__m256d;

use super::V4;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod libmvec {
    #[link(name = "mvec")]
    extern "C" {
        pub(super) fn _ZGVdN4v_sin();
        pub(super) fn _ZGVdN4v_cos();
        pub(super) fn _ZGVdN4v_tan();
        pub(super) fn _ZGVdN4v_exp();
        pub(super) fn _ZGVdN4v_log();
        pub(super) fn _ZGVdN4v_atanh();
        pub(super) fn _ZGVdN4v_asinh();
        pub(super) fn _ZGVdN4v_sinh();
        pub(super) fn _ZGVdN4v_atan();
        pub(super) fn _ZGVdN4vv_atan2();
        pub(super) fn _ZGVdN4vv_hypot();
        pub(super) fn _ZGVdN4vv_pow();
    }
}

/// Une fonction à un argument. `asm!` sur des registres `ymm` exige la fonctionnalité `avx` de
/// la fonction qui le contient : `call` est donc une fonction `target_feature` à part, que la
/// méthode appelle sous la preuve [`super::Avx2`] que porte `self`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
macro_rules! unary {
    ($name:ident, $symbol:ident, $std:path, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self) -> V4 {
                /// # Safety
                ///
                /// Le processeur doit supporter AVX2 et FMA.
                #[target_feature(enable = "avx2,fma")]
                unsafe fn call(x: __m256d) -> __m256d {
                    let result: __m256d;
                    // SAFETY: symbole libmvec AVX2 : un vecteur de quatre `f64` en ymm0,
                    // résultat en ymm0. Les ymm listés sont les registres que l'appel peut
                    // écraser, avec le reste de l'ABI SysV.
                    unsafe {
                        core::arch::asm!(
                            "call {f}",
                            f = sym libmvec::$symbol,
                            in("ymm0") x,
                            lateout("ymm0") result,
                            out("ymm1") _, out("ymm2") _, out("ymm3") _, out("ymm4") _,
                            out("ymm5") _, out("ymm6") _, out("ymm7") _, out("ymm8") _,
                            out("ymm9") _, out("ymm10") _, out("ymm11") _, out("ymm12") _,
                            out("ymm13") _, out("ymm14") _, out("ymm15") _,
                            clobber_abi("sysv64"),
                        );
                    }
                    result
                }
                // SAFETY: `self` n'existe que si AVX2 et FMA sont supportés.
                V4(unsafe { call(self.0) })
            }
        }
    };
}

/// Une fonction à deux arguments, même découpage que `unary!`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
macro_rules! binary {
    ($name:ident, $symbol:ident, $std:path, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self, other: V4) -> V4 {
                /// # Safety
                ///
                /// Le processeur doit supporter AVX2 et FMA.
                #[target_feature(enable = "avx2,fma")]
                unsafe fn call(first: __m256d, second: __m256d) -> __m256d {
                    let result: __m256d;
                    // SAFETY: symbole libmvec AVX2 à deux arguments : le premier en ymm0, le
                    // second en ymm1, résultat en ymm0.
                    unsafe {
                        core::arch::asm!(
                            "call {f}",
                            f = sym libmvec::$symbol,
                            in("ymm0") first,
                            inout("ymm1") second => _,
                            lateout("ymm0") result,
                            out("ymm2") _, out("ymm3") _, out("ymm4") _, out("ymm5") _,
                            out("ymm6") _, out("ymm7") _, out("ymm8") _, out("ymm9") _,
                            out("ymm10") _, out("ymm11") _, out("ymm12") _, out("ymm13") _,
                            out("ymm14") _, out("ymm15") _,
                            clobber_abi("sysv64"),
                        );
                    }
                    result
                }
                // SAFETY: `self` n'existe que si AVX2 et FMA sont supportés.
                V4(unsafe { call(self.0, other.0) })
            }
        }
    };
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
macro_rules! unary {
    ($name:ident, $symbol:ident, $std:path, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self) -> V4 {
                lane_wise(self, self, |lane, _| $std(lane))
            }
        }
    };
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
macro_rules! binary {
    ($name:ident, $symbol:ident, $std:path, $doc:literal) => {
        impl V4 {
            #[doc = $doc]
            #[inline(always)]
            pub(crate) fn $name(self, other: V4) -> V4 {
                lane_wise(self, other, |lane, second| $std(lane, second))
            }
        }
    };
}

/// Applique `function` à chaque voie de `first`, avec la voie correspondante de `second`.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[inline(always)]
fn lane_wise(first: V4, second: V4, function: impl Fn(f64, f64) -> f64) -> V4 {
    use std::arch::x86_64::{_mm256_loadu_pd, _mm256_storeu_pd};

    let mut lanes = [0.0; 4];
    let mut others = [0.0; 4];
    // SAFETY: `first` n'existe que si AVX2 est supporté ; `lanes` désigne quatre `f64`.
    unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), first.0) };
    // SAFETY: idem pour `second`.
    unsafe { _mm256_storeu_pd(others.as_mut_ptr(), second.0) };
    for (lane, other) in lanes.iter_mut().zip(others) {
        *lane = function(*lane, other);
    }
    // SAFETY: `first` n'existe que si AVX2 est supporté ; `lanes` désigne quatre `f64`.
    V4(unsafe { _mm256_loadu_pd(lanes.as_ptr()) })
}

unary!(sin, _ZGVdN4v_sin, f64::sin, "Quatre sinus.");
unary!(cos, _ZGVdN4v_cos, f64::cos, "Quatre cosinus.");
unary!(tan, _ZGVdN4v_tan, f64::tan, "Quatre tangentes.");
unary!(exp, _ZGVdN4v_exp, f64::exp, "Quatre exponentielles.");
unary!(log, _ZGVdN4v_log, f64::ln, "Quatre logarithmes naturels.");
unary!(
    atanh,
    _ZGVdN4v_atanh,
    f64::atanh,
    "Quatre arguments tangente hyperbolique."
);
unary!(
    asinh,
    _ZGVdN4v_asinh,
    f64::asinh,
    "Quatre arguments sinus hyperbolique."
);
unary!(
    sinh,
    _ZGVdN4v_sinh,
    f64::sinh,
    "Quatre sinus hyperboliques."
);
unary!(atan, _ZGVdN4v_atan, f64::atan, "Quatre arcs tangente.");
binary!(
    atan2,
    _ZGVdN4vv_atan2,
    f64::atan2,
    "Quatre `self.atan2(other)`, même ordre que `f64::atan2`."
);
binary!(
    hypot,
    _ZGVdN4vv_hypot,
    f64::hypot,
    "Quatre `self.hypot(other)`."
);
binary!(powf, _ZGVdN4vv_pow, f64::powf, "Quatre `self.powf(other)`.");
