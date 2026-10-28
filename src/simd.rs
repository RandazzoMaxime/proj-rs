//! Appels AVX2 à libmvec, isolés pour que le `unsafe` reste petit.
//!
//! Chaque fonction n'existe que pour un besoin mesuré : un sinus scalaire
//! domine le Transverse Mercator. libmvec calcule quatre valeurs par appel.

use std::arch::x86_64::{__m256d, _mm256_loadu_pd, _mm256_sqrt_pd, _mm256_storeu_pd};

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[link(name = "mvec")]
extern "C" {
    fn _ZGVdN4v_sin();
    fn _ZGVdN4v_cos();
    fn _ZGVdN4v_log();
    fn _ZGVdN4v_exp();
    fn _ZGVdN4v_atanh();
    fn _ZGVdN4vv_atan2();
}

/// Charge quatre `f64` à partir de `index`.
///
/// # Panics
///
/// Panique en debug si `index + 4` dépasse la tranche.
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2. `index + 4 <= data.len()`.
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn load4(data: &[f64], index: usize) -> __m256d {
    debug_assert!(index + 4 <= data.len());
    let ptr = data.as_ptr();
    // SAFETY: `index + 4 <= data.len()`, donc `add` reste dans l'allocation.
    let p = unsafe { ptr.add(index) };
    // SAFETY: `p` désigne quatre `f64` lisibles. `loadu` n'exige pas d'alignement 32.
    unsafe { _mm256_loadu_pd(p) }
}

/// Écrit quatre `f64` à partir de `index`.
///
/// # Panics
///
/// Panique en debug si `index + 4` dépasse la tranche.
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2. `index + 4 <= data.len()`.
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn store4(data: &mut [f64], index: usize, value: __m256d) {
    debug_assert!(index + 4 <= data.len());
    let ptr = data.as_mut_ptr();
    // SAFETY: `index + 4 <= data.len()`, donc `add` reste dans l'allocation.
    let p = unsafe { ptr.add(index) };
    // SAFETY: `p` désigne quatre `f64` inscriptibles. `storeu` n'exige pas d'alignement 32.
    unsafe { _mm256_storeu_pd(p, value) }
}

/// # Safety
///
/// Le fil d'exécution doit supporter AVX.
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn sqrt4(value: __m256d) -> __m256d {
    // sqrtpd est une instruction AVX, disponible sous la cible `avx2`.
    _mm256_sqrt_pd(value)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
macro_rules! libmvec1 {
    ($name:ident, $symbol:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// # Safety
        ///
        /// Le fil d'exécution doit supporter AVX2. `x` est quatre `f64` finis
        /// pour `log` et `atanh`.
        #[target_feature(enable = "avx2,fma")]
        pub(crate) unsafe fn $name(x: __m256d) -> __m256d {
            let y: __m256d;
            // SAFETY: symbole libmvec AVX2, un vecteur de 4 f64 en ymm0, résultat en ymm0.
            // Les ymm listés sont les registres caller-saved que l'appel peut écraser.
            unsafe {
                core::arch::asm!(
                    "call {f}",
                    f = sym $symbol,
                    in("ymm0") x,
                    lateout("ymm0") y,
                    out("ymm1") _,
                    out("ymm2") _,
                    out("ymm3") _,
                    out("ymm4") _,
                    out("ymm5") _,
                    out("ymm6") _,
                    out("ymm7") _,
                    out("ymm8") _,
                    out("ymm9") _,
                    out("ymm10") _,
                    out("ymm11") _,
                    out("ymm12") _,
                    out("ymm13") _,
                    out("ymm14") _,
                    out("ymm15") _,
                    clobber_abi("sysv64"),
                );
            }
            y
        }
    };
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
libmvec1!(sin4, _ZGVdN4v_sin, "Quatre sinus.");
#[cfg(all(target_os = "linux", target_env = "gnu"))]
libmvec1!(cos4, _ZGVdN4v_cos, "Quatre cosinus.");
#[cfg(all(target_os = "linux", target_env = "gnu"))]
libmvec1!(log4, _ZGVdN4v_log, "Quatre logarithmes naturels.");
#[cfg(all(target_os = "linux", target_env = "gnu"))]
libmvec1!(exp4, _ZGVdN4v_exp, "Quatre exponentielles.");
#[cfg(all(target_os = "linux", target_env = "gnu"))]
libmvec1!(atanh4, _ZGVdN4v_atanh, "Quatre arctanh.");

/// Quatre `atan2(y, x)`, même ordre que `f64::atan2`.
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn atan2_4(y: __m256d, x: __m256d) -> __m256d {
    let r: __m256d;
    // SAFETY: `_ZGVdN4vv_atan2` prend y en ymm0 et x en ymm1, et renvoie ymm0.
    unsafe {
        core::arch::asm!(
            "call {f}",
            f = sym _ZGVdN4vv_atan2,
            in("ymm0") y,
            inout("ymm1") x => _,
            lateout("ymm0") r,
            out("ymm2") _,
            out("ymm3") _,
            out("ymm4") _,
            out("ymm5") _,
            out("ymm6") _,
            out("ymm7") _,
            out("ymm8") _,
            out("ymm9") _,
            out("ymm10") _,
            out("ymm11") _,
            out("ymm12") _,
            out("ymm13") _,
            out("ymm14") _,
            out("ymm15") _,
            clobber_abi("sysv64"),
        );
    }
    r
}

/// Transcendantales du CRT, une voie à la fois. Le reste du noyau reste en AVX2.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
unsafe fn map1(value: __m256d, function: fn(f64) -> f64) -> __m256d {
    let mut lanes = [0.0; 4];
    // SAFETY: quatre f64, et cette fonction exige AVX2.
    unsafe { store4(&mut lanes, 0, value) };
    for lane in &mut lanes {
        *lane = function(*lane);
    }
    // SAFETY: `lanes` a quatre éléments.
    unsafe { load4(&lanes, 0) }
}

/// Quatre sinus du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn sin4(value: __m256d) -> __m256d {
    // SAFETY: même cible AVX2 que `map1`.
    unsafe { map1(value, f64::sin) }
}

/// Quatre cosinus du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn cos4(value: __m256d) -> __m256d {
    // SAFETY: même cible AVX2 que `map1`.
    unsafe { map1(value, f64::cos) }
}

/// Quatre logarithmes naturels du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2. Chaque voie est strictement positive.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn log4(value: __m256d) -> __m256d {
    // SAFETY: même cible AVX2 que `map1`.
    unsafe { map1(value, f64::ln) }
}

/// Quatre exponentielles du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn exp4(value: __m256d) -> __m256d {
    // SAFETY: même cible AVX2 que `map1`.
    unsafe { map1(value, f64::exp) }
}

/// Quatre arctanh du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2. Chaque voie est dans (-1, 1).
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn atanh4(value: __m256d) -> __m256d {
    // SAFETY: même cible AVX2 que `map1`.
    unsafe { map1(value, f64::atanh) }
}

/// Quatre `atan2(y, x)` du CRT.
///
/// # Safety
///
/// Le fil d'exécution doit supporter AVX2.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn atan2_4(y: __m256d, x: __m256d) -> __m256d {
    let mut ys = [0.0; 4];
    let mut xs = [0.0; 4];
    // SAFETY: quatre f64, et cette fonction exige AVX2.
    unsafe { store4(&mut ys, 0, y) };
    // SAFETY: quatre f64, et cette fonction exige AVX2.
    unsafe { store4(&mut xs, 0, x) };
    for index in 0..4 {
        ys[index] = ys[index].atan2(xs[index]);
    }
    // SAFETY: `ys` a quatre éléments.
    unsafe { load4(&ys, 0) }
}
