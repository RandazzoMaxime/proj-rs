//! Calcul par lots en parallèle : quatre `f64` à la fois (AVX2 sur x86-64, NEON sur aarch64) et,
//! avec la feature `avx512`, huit à la fois (AVX-512 sur x86-64).
//!
//! Le module n'a pas de `unsafe` : les intrinsèques et le passage dans le code compilé avec les
//! extensions du jeu d'instructions passent par `pulp`, qui le porte
//! (`docs/PORTING-simd.md`). Les projections n'écrivent que leur formule, générique sur le nombre
//! de voies, contre l'API sûre de [`Vector`] ; elles ne touchent ni intrinsèque ni détection du
//! processeur. [`Vector`] a une réalisation par jeu d'instructions (`v4.rs`, `neon.rs`, `v8.rs`).
//!
//! - [`SimdToken`] prouve que le processeur sait le jeu d'instructions d'un chemin vectoriel.
//!   On n'en obtient un que par `detect` sur le type du jeton. Tout le reste s'appuie sur cette
//!   preuve.
//! - [`Vector`] est `N` `f64` : opérateurs, comparaisons par masque, `select`, et les fonctions
//!   transcendantes (`sin`, `exp`, `atan2`, …) de `math.rs`.
//! - [`kernels`] regroupe les briques que plusieurs projections partagent (`adjlon`, `tsfn`,
//!   `phi2`).
//! - [`run`] et [`run3`] sont le driver : ils choisissent le chemin le plus large que le
//!   processeur supporte (AVX-512, puis AVX2, puis NEON), découpent les tranches par groupes de
//!   `N`, rappellent le chemin scalaire pour la queue et pour tout groupe qu'une voie sort du
//!   domaine rapide, et n'entrent dans le code `target_feature` qu'une fois par appel.
//!
//! Sur une autre architecture, ou sans le jeu d'instructions, [`run`] boucle sur le chemin
//! scalaire : les résultats sont alors ceux de la crate, sans écart.

#[cfg(simd_lanes)]
mod drive;
#[cfg(simd_lanes)]
pub(crate) mod kernels;
#[cfg(simd_lanes)]
mod math;
#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(target_arch = "x86_64")]
mod v4;
#[cfg(all(target_arch = "x86_64", feature = "avx512"))]
mod v8;
#[cfg(simd_lanes)]
mod vec;

#[cfg(simd_lanes)]
pub(crate) use math::VectorMath;
#[cfg(simd_lanes)]
pub(crate) use vec::{Mask, SimdToken, Vector};

/// Un noyau par lots à deux entrées et deux sorties.
pub(crate) trait Lanes {
    /// Un point par le chemin scalaire. `NaN` pour un point que la projection refuse.
    fn point(&self, a: f64, b: f64) -> (f64, f64);

    /// `N` points. `None` quand une voie sort du domaine que le noyau vectoriel traite
    /// (valeur non finie, pôle, hors domaine) : le groupe passe alors par [`Self::point`].
    #[cfg(simd_lanes)]
    fn lanes<V: Vector<N>, const N: usize>(&self, tok: V::Token, a: V, b: V) -> Option<(V, V)>;
}

/// Un noyau par lots à trois entrées et trois sorties (géocentrique, Helmert).
pub(crate) trait Lanes3 {
    /// Un point par le chemin scalaire.
    fn point(&self, a: f64, b: f64, c: f64) -> (f64, f64, f64);

    /// `N` points. `None` quand une voie sort du domaine que le noyau vectoriel traite : le
    /// groupe passe alors par [`Self::point`].
    #[cfg(simd_lanes)]
    fn lanes<V: Vector<N>, const N: usize>(
        &self,
        tok: V::Token,
        a: V,
        b: V,
        c: V,
    ) -> Option<(V, V, V)>;
}

/// Applique `kernel` aux trois tranches d'entrée, qui ont la longueur des trois de sortie.
///
/// # Panics
///
/// Panique si les six tranches n'ont pas la même longueur.
pub(crate) fn run3<K: Lanes3>(kernel: &K, input: [&[f64]; 3], output: [&mut [f64]; 3]) {
    let n = input[0].len();
    assert!(
        input.iter().all(|lane| lane.len() == n) && output.iter().all(|lane| lane.len() == n),
        "the six slices must have {n} values"
    );
    #[cfg(simd_lanes)]
    {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(feature = "avx512")]
            if let Some(tok) = v8::Avx512::detect() {
                drive::run3_simd::<v8::V8, K, 8>(tok, kernel, input, output);
                return;
            }
            if let Some(tok) = v4::Avx2::detect() {
                drive::run3_simd::<v4::V4, K, 4>(tok, kernel, input, output);
                return;
            }
        }
        #[cfg(target_arch = "aarch64")]
        if let Some(tok) = neon::Neon::detect() {
            drive::run3_simd::<neon::V4, K, 4>(tok, kernel, input, output);
            return;
        }
    }
    scalar3(kernel, input, output);
}

/// Le chemin sans vecteur : un point après l'autre.
fn scalar3<K: Lanes3>(kernel: &K, input: [&[f64]; 3], output: [&mut [f64]; 3]) {
    let [oa, ob, oc] = output;
    for (slot, ((oa, ob), oc)) in oa
        .iter_mut()
        .zip(ob.iter_mut())
        .zip(oc.iter_mut())
        .enumerate()
    {
        (*oa, *ob, *oc) = kernel.point(input[0][slot], input[1][slot], input[2][slot]);
    }
}

/// Applique `kernel` aux quatre tranches, qui ont la même longueur.
///
/// # Panics
///
/// Panique si les quatre tranches n'ont pas la même longueur.
pub(crate) fn run<K: Lanes>(
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    let n = in_a.len();
    assert_eq!(in_b.len(), n, "second input length {} != {n}", in_b.len());
    assert_eq!(out_a.len(), n, "first output length {} != {n}", out_a.len());
    assert_eq!(
        out_b.len(),
        n,
        "second output length {} != {n}",
        out_b.len()
    );
    #[cfg(simd_lanes)]
    {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(feature = "avx512")]
            if let Some(tok) = v8::Avx512::detect() {
                drive::run_simd::<v8::V8, K, 8>(tok, kernel, in_a, in_b, out_a, out_b);
                return;
            }
            if let Some(tok) = v4::Avx2::detect() {
                drive::run_simd::<v4::V4, K, 4>(tok, kernel, in_a, in_b, out_a, out_b);
                return;
            }
        }
        #[cfg(target_arch = "aarch64")]
        if let Some(tok) = neon::Neon::detect() {
            drive::run_simd::<neon::V4, K, 4>(tok, kernel, in_a, in_b, out_a, out_b);
            return;
        }
    }
    scalar(kernel, in_a, in_b, out_a, out_b);
}

/// Le chemin sans vecteur : un point après l'autre.
fn scalar<K: Lanes>(kernel: &K, in_a: &[f64], in_b: &[f64], out_a: &mut [f64], out_b: &mut [f64]) {
    for (((a, b), oa), ob) in in_a.iter().zip(in_b).zip(out_a).zip(out_b) {
        (*oa, *ob) = kernel.point(*a, *b);
    }
}

/// Chemin pris par les calculs par lots sur cette machine : `avx512` (feature `avx512`, CPU
/// x86-64-v4), `avx2`, `neon`, ou `scalar`.
///
/// Le chemin vectoriel est le même sur toutes les plateformes d'une architecture : les fonctions
/// mathématiques sont celles de [`Vector`] (`math.rs`), pas celles de la bibliothèque C.
#[must_use]
pub(crate) fn batch_path() -> &'static str {
    #[cfg(simd_lanes)]
    {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(feature = "avx512")]
            if v8::Avx512::detect().is_some() {
                return "avx512";
            }
            if v4::Avx2::detect().is_some() {
                return "avx2";
            }
        }
        #[cfg(target_arch = "aarch64")]
        if neon::Neon::detect().is_some() {
            return "neon";
        }
    }
    "scalar"
}

/// Le chemin vectoriel est pris sur cet hôte.
#[must_use]
pub(crate) fn has_lanes() -> bool {
    batch_path() != "scalar"
}
