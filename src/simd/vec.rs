//! L'API commune aux réalisations vectorielles du chemin par lots : [`Vector`].
//!
//! [`Vector`] a une réalisation par jeu d'instructions, toutes avec la même API et les mêmes
//! bits par voie : `v4.rs` (quatre `f64` dans un registre AVX2, x86-64), `neon.rs` (quatre `f64`
//! dans deux registres NEON, aarch64) et `v8.rs` (huit `f64` dans un registre AVX-512, x86-64,
//! feature `avx512`). Les projections, [`kernels`](super::kernels) et [`math`](super::math)
//! n'écrivent que contre ce trait, en étant génériques sur le nombre de voies `N` : une formule
//! écrite une fois tourne sur toutes les réalisations.
//!
//! Le module n'a pas de `unsafe` : les intrinsèques et le passage dans le code compilé avec les
//! extensions du jeu d'instructions passent par `pulp` (voir [`SimdToken`]). Les fonctions
//! transcendantes (`exp`, `log`, `sin_cos`, …) sont l'extension [`VectorMath`](super::math::VectorMath),
//! écrite une fois dans `math.rs` contre cette API.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// Le résultat d'une comparaison : un bit par voie.
pub(crate) trait Mask: Copy {
    /// Voies vraies des deux masques.
    fn and(self, other: Self) -> Self;

    /// Voies vraies d'au moins un des deux masques.
    fn or(self, other: Self) -> Self;

    /// Les voies que le masque ne retient pas.
    fn not(self) -> Self;

    /// Un bit par voie, le bit 0 pour la première : les voies vraies.
    fn bits(self) -> u32;

    /// Au moins une voie est vraie.
    fn any(self) -> bool;

    /// Toutes les voies sont vraies.
    fn all(self) -> bool;
}

/// Un jeton prouvant le jeu d'instructions d'un chemin vectoriel, et donnant l'entrée dans le
/// code compilé avec ses extensions. C'est `pulp` qui porte le `unsafe` de ce passage ; ce module
/// n'en a pas.
pub(crate) trait SimdToken: Copy {
    /// Exécute `f` dans une fonction compilée avec les extensions du jeton : les méthodes
    /// `#[inline(always)]` de [`Vector`] y deviennent des instructions en ligne.
    fn vectorize<F: pulp::NullaryFnOnce>(self, f: F) -> F::Output;
}

/// `N` `f64` calculés en parallèle.
///
/// Les opérateurs usuels (`+`, `-`, `*`, `/`, `-x`) viennent des traits de la bibliothèque
/// standard. Le reste est déclaré ici ; les fonctions transcendantes (`exp`, `log`, `sin_cos`, …)
/// sont fournies, écrites une fois dans [`math`](super::math) contre cette API.
pub(crate) trait Vector<const N: usize>:
    Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// La preuve du jeu d'instructions : on n'obtient un `Vector` qu'avec un jeton.
    type Token: SimdToken;
    /// Le résultat des comparaisons.
    type Mask: Mask;

    /// `N` fois `value`.
    fn splat(tok: Self::Token, value: f64) -> Self;

    /// `N` zéros.
    fn zero(tok: Self::Token) -> Self;

    /// Charge `N` valeurs (sans alignement exigé).
    fn load(tok: Self::Token, lanes: &[f64; N]) -> Self;

    /// Écrit les `N` voies.
    fn store(self, lanes: &mut [f64; N]);

    /// Les `N` voies, pour un calcul voie par voie.
    fn to_array(self) -> [f64; N];

    /// `N` fois `value`, sans jeton : `self` en est la preuve.
    fn splat_like(self, value: f64) -> Self;

    /// Un registre de ces `N` valeurs, sans jeton : `self` en est la preuve.
    fn reload(self, lanes: &[f64; N]) -> Self;

    /// Voie par voie : `if_true` si le masque est vrai, `if_false` sinon.
    fn select(mask: Self::Mask, if_true: Self, if_false: Self) -> Self;

    /// `self == other`, faux si une voie est `NaN`.
    fn eq(self, other: Self) -> Self::Mask;

    /// `self < other`, faux si une voie est `NaN`.
    fn lt(self, other: Self) -> Self::Mask;

    /// `self <= other`, faux si une voie est `NaN`.
    fn le(self, other: Self) -> Self::Mask;

    /// `self > other`, faux si une voie est `NaN`.
    fn gt(self, other: Self) -> Self::Mask;

    /// `self >= other`, faux si une voie est `NaN`.
    fn ge(self, other: Self) -> Self::Mask;

    /// Valeur absolue.
    fn abs(self) -> Self;

    /// Racine carrée.
    fn sqrt(self) -> Self;

    /// Plus grand entier inférieur ou égal.
    fn floor(self) -> Self;

    /// Le plus grand des deux. Une voie `NaN` rend la seconde opérande.
    fn max(self, other: Self) -> Self;

    /// Le plus petit des deux. Une voie `NaN` rend la seconde opérande.
    fn min(self, other: Self) -> Self;

    /// L'entier le plus proche, au pair en cas d'égalité.
    fn round(self) -> Self;

    /// La grandeur de `self` avec le signe de `sign` (un zéro négatif compris).
    fn copysign(self, sign: Self) -> Self;

    /// `self * a + b` en une seule opération, arrondie une fois (FMA).
    fn mul_add(self, a: Self, b: Self) -> Self;

    /// Voies finies (ni `NaN` ni infinie).
    fn is_finite(self) -> Self::Mask;

    /// Les voies dont le bit de signe est à un : négatifs, zéro négatif, `NaN` de signe négatif.
    fn sign_mask(self) -> Self::Mask;

    /// `self * 2^k` pour des `k` entiers, `|k| <= 2000`, en deux multiplications pour que la
    /// puissance de deux reste représentable.
    fn scale_pow2(self, k: Self) -> Self;

    /// Le masque des voies dont l'entier (`self` est entier, `|self| < 2^51`) a à un le bit
    /// `63 - SHIFT` : `SHIFT = 63` pour le bit 0, `SHIFT = 62` pour le bit 1.
    fn integer_bit_mask<const SHIFT: i32>(self) -> Self::Mask;

    /// Mantisse et exposant d'un `f64` positif normal : `self = m * 2^e` avec `m` dans `[1, 2[`.
    /// `e` est rendu en `f64`. Pour une valeur nulle, négative, sous-normale, infinie ou `NaN`,
    /// le résultat n'a pas de sens : l'appelant écarte ces voies.
    fn split_exponent(self) -> (Self, Self);
}
