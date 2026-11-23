//! Calcul par lots sur quatre `f64` à la fois : AVX2 sur x86-64, NEON sur aarch64.
//!
//! Le module n'a pas de `unsafe` : les intrinsèques et le passage dans le code compilé avec AVX2
//! passent par `pulp`, qui le porte (`docs/PORTING-simd.md`). Les projections n'écrivent que leur formule, en quatre voies, contre l'API sûre de [`V4`] ;
//! elles ne touchent ni intrinsèque ni détection du processeur. [`V4`] a deux réalisations, une
//! par architecture, qui ont la même API et rendent les mêmes bits : `v4.rs` (un registre AVX2)
//! et `neon.rs` (deux registres NEON).
//!
//! - [`Token`] prouve que le processeur sait le jeu d'instructions du chemin vectoriel (AVX2 et
//!   FMA, ou NEON). On n'en obtient un que par [`Token::detect`]. Tout le reste, [`V4`] compris,
//!   s'appuie sur cette preuve.
//! - [`V4`] est quatre `f64` : opérateurs, comparaisons par masque, `select`, et les fonctions
//!   transcendantes (`sin`, `exp`, `atan2`, …) de `math.rs`.
//! - [`kernels`] regroupe les briques que plusieurs projections partagent (`adjlon`, `tsfn`,
//!   `phi2`).
//! - [`Lanes`] et [`run`] sont le driver : il découpe les tranches par quatre, rappelle le
//!   chemin scalaire pour la queue et pour tout groupe qu'une voie sort du domaine rapide, et
//!   n'entre dans le code `target_feature` qu'une fois par appel. [`Lanes3`] et [`run3`] sont le
//!   même driver pour trois entrées et trois sorties (géocentrique, Helmert).
//!
//! Sur une autre architecture, ou sans AVX2, [`run`] boucle sur le chemin scalaire : les résultats
//! sont alors ceux de la crate, sans écart.

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

#[cfg(target_arch = "aarch64")]
pub(crate) use neon::{Mask, Neon as Token, V4};
#[cfg(target_arch = "x86_64")]
pub(crate) use v4::{Avx2 as Token, Mask, V4};

/// Un noyau par lots à deux entrées et deux sorties.
pub(crate) trait Lanes {
    /// Un point par le chemin scalaire. `NaN` pour un point que la projection refuse.
    fn point(&self, a: f64, b: f64) -> (f64, f64);

    /// Quatre points. `None` quand une voie sort du domaine que le noyau vectoriel traite
    /// (valeur non finie, pôle, hors domaine) : le groupe passe alors par [`Self::point`].
    #[cfg(simd_lanes)]
    fn lanes(&self, tok: Token, a: V4, b: V4) -> Option<(V4, V4)>;
}

/// Un noyau par lots à trois entrées et trois sorties (géocentrique, Helmert).
pub(crate) trait Lanes3 {
    /// Un point par le chemin scalaire.
    fn point(&self, a: f64, b: f64, c: f64) -> (f64, f64, f64);

    /// Quatre points. `None` quand une voie sort du domaine que le noyau vectoriel traite : le
    /// groupe passe alors par [`Self::point`].
    #[cfg(simd_lanes)]
    fn lanes(&self, tok: Token, a: V4, b: V4, c: V4) -> Option<(V4, V4, V4)>;
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
    if let Some(tok) = Token::detect() {
        drive::run3_avx2(tok, kernel, input, output);
        return;
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
    if let Some(tok) = Token::detect() {
        drive::run_avx2(tok, kernel, in_a, in_b, out_a, out_b);
        return;
    }
    scalar(kernel, in_a, in_b, out_a, out_b);
}

/// Le chemin sans vecteur : un point après l'autre.
fn scalar<K: Lanes>(kernel: &K, in_a: &[f64], in_b: &[f64], out_a: &mut [f64], out_b: &mut [f64]) {
    for (((a, b), oa), ob) in in_a.iter().zip(in_b).zip(out_a).zip(out_b) {
        (*oa, *ob) = kernel.point(*a, *b);
    }
}

/// Chemin pris par les calculs par lots sur cette machine : `avx2` ou `scalar`.
///
/// Le chemin vectoriel est le même sur toutes les plateformes d'une architecture : les fonctions
/// mathématiques sont celles de [`V4`] (`math.rs`), pas celles de la bibliothèque C. `avx2` sur
/// x86-64, `neon` sur aarch64, `scalar` sinon.
#[must_use]
pub(crate) fn batch_path() -> &'static str {
    #[cfg(simd_lanes)]
    if Token::detect().is_some() {
        #[cfg(target_arch = "x86_64")]
        return "avx2";
        #[cfg(target_arch = "aarch64")]
        return "neon";
    }
    "scalar"
}

/// Le chemin vectoriel est pris sur cet hôte.
#[must_use]
pub(crate) fn has_lanes() -> bool {
    batch_path() != "scalar"
}
