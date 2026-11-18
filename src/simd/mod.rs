//! Calcul par lots sur quatre `f64` à la fois (AVX2).
//!
//! Le module est la seule frontière du `unsafe` de la crate pour le calcul vectoriel. Les
//! projections n'écrivent que leur formule, en quatre voies, contre l'API sûre de [`V4`] ;
//! elles ne touchent ni intrinsèque ni détection du processeur.
//!
//! - [`Avx2`] prouve que le processeur sait AVX2 et FMA. On n'en obtient un que par
//!   [`Avx2::detect`]. Tout le reste, [`V4`] compris, s'appuie sur cette preuve.
//! - [`V4`] est quatre `f64` : opérateurs, comparaisons par masque, `select`, et les fonctions
//!   transcendantes (`sin`, `exp`, `atan2`, …) de `math.rs`.
//! - [`kernels`] regroupe les briques que plusieurs projections partagent (`adjlon`, `tsfn`,
//!   `phi2`).
//! - [`Lanes`] et [`run`] sont le driver : il découpe les tranches par quatre, rappelle le
//!   chemin scalaire pour la queue et pour tout groupe qu'une voie sort du domaine rapide, et
//!   n'entre dans le code `target_feature` qu'une fois par appel.
//!
//! Hors x86-64, ou sans AVX2, [`run`] boucle sur le chemin scalaire : les résultats sont alors
//! ceux de la crate, sans écart.

#[cfg(target_arch = "x86_64")]
mod drive;
#[cfg(target_arch = "x86_64")]
pub(crate) mod kernels;
#[cfg(target_arch = "x86_64")]
mod math;
#[cfg(target_arch = "x86_64")]
mod v4;

#[cfg(target_arch = "x86_64")]
pub(crate) use v4::{Avx2, Mask, V4};

/// Un noyau par lots à deux entrées et deux sorties.
pub(crate) trait Lanes {
    /// Un point par le chemin scalaire. `NaN` pour un point que la projection refuse.
    fn point(&self, a: f64, b: f64) -> (f64, f64);

    /// Quatre points. `None` quand une voie sort du domaine que le noyau vectoriel traite
    /// (valeur non finie, pôle, hors domaine) : le groupe passe alors par [`Self::point`].
    #[cfg(target_arch = "x86_64")]
    fn lanes(&self, tok: Avx2, a: V4, b: V4) -> Option<(V4, V4)>;
}

/// Un noyau par lots à trois entrées et trois sorties (géocentrique, Helmert).
pub(crate) trait Lanes3 {
    /// Un point par le chemin scalaire.
    fn point(&self, a: f64, b: f64, c: f64) -> (f64, f64, f64);

    /// Quatre points. `None` quand une voie sort du domaine que le noyau vectoriel traite : le
    /// groupe passe alors par [`Self::point`].
    #[cfg(target_arch = "x86_64")]
    fn lanes(&self, tok: Avx2, a: V4, b: V4, c: V4) -> Option<(V4, V4, V4)>;
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
    #[cfg(target_arch = "x86_64")]
    if let Some(tok) = Avx2::detect() {
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
    #[cfg(target_arch = "x86_64")]
    if let Some(tok) = Avx2::detect() {
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
/// Le chemin `avx2` est le même sur toutes les plateformes x86-64 : les fonctions mathématiques
/// sont celles de [`V4`] (`math.rs`), pas celles de la bibliothèque C.
#[must_use]
pub(crate) fn batch_path() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    if Avx2::detect().is_some() {
        return "avx2";
    }
    "scalar"
}
