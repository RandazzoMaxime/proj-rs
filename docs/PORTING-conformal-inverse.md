# Inverse de krovak et de sterea par la série conforme

Écart volontaire vers PROJ 9.9.0. Aucune API publique ne change.

## Ce que fait PROJ

- `krovak.cpp`, inverse : point fixe sur la latitude, 100 tours au plus, arrêt à `|Δφ| < 1e-15`.
  Chaque tour calcule un `sin`, un `pow` et un `atan`.
- `gauss.cpp` (`pj_inv_gauss`, utilisé par `sterea`) : même boucle, 20 tours au plus, arrêt à
  `|Δφ| < 1e-14`.

Dans les deux cas, la quantité qui sert de point fixe est `tan(χ/2 + π/4)`, où χ est la latitude
**conforme**. Elle est connue en forme fermée avant la boucle (`num_base` pour krovak,
`num` pour gauss).

## Ce que fait proj-rs

La latitude conforme `χ = 2·atan(tan(χ/2 + π/4)) − π/2` passe en latitude géodésique par la série
d'ordre 6 `auxlat_convert` (`projmath.rs`, coefficients `Conformal → Geographic`), la même que
`Tmerc`. L'erreur de troncature est en `n⁷` (≈ 4·10⁻²⁰ pour Bessel, ≈ 10⁻²⁰ pour GRS80), sous
l'epsilon machine. Plus de boucle.

Le chemin vectoriel de `sterea` (`sterea_batch.rs`) garde sa boucle Newton et `DEL_TOL`.

## Écart mesuré

311 points par opération (grille du banc, extrêmes `(0, ±89,9)`, antiméridien, pôle, `NaN`),
ancien code contre nouveau code, aller puis retour :

| opération | écart max de l'inverse | statuts d'erreur différents |
|---|---:|---:|
| `krovak` | 4,4·10⁻¹⁶ rad | 0 |
| `mod_krovak` | 4,4·10⁻¹⁶ rad | 0 |
| `sterea` | 4,4·10⁻¹⁶ rad | 0 |

Soit environ 3 nm. Le forward n'est pas modifié (bits identiques). Les 187 autres opérations du
catalogue sont identiques au bit près.

## Références de bits régénérées

| fichier | ce qui change |
|---|---|
| `tests/pipeline_stack/golden.txt` | section « EPSG:28992 » inverse, 5 lignes, de 1 à 2 ulp |
| `tests/simd_bits.rs` | `inverse EPSG:4326 EPSG:28992`, dans `GOLDEN` et `GOLDEN_AVX512` |

Seul EPSG:28992 (Amersfoort / RD New, `sterea`) est touché. Le hash était identique dans les deux
tables avant le changement. Il n'a été recalculé que sur la voie AVX-512 : la voie AVX2 n'a pas pu
être exécutée sur cette machine.

## Vitesse (1 cœur, 128 points, meilleur de 7)

| opération | inverse avant | inverse après |
|---|---:|---:|
| `krovak` | 559 ns | 218 ns |
| `mod_krovak` | 590 ns | 240 ns |
| `sterea` | 544 ns | 150 ns |

`krovak` forward : constantes de projection (`sin`/`cos` de `ad`, `rho0·tan(S0/2+π/4)^n`)
calculées une fois à la création, 14 % d'instructions en moins, mêmes bits.

## Limite connue

PROJ rend une erreur quand sa boucle ne converge pas. La série ne diverge jamais : elle ne rend
une erreur que pour un résultat non fini. Aucun point de la grille, extrêmes compris, n'a donné
un statut différent, mais un point pathologique non testé pourrait rendre un nombre là où PROJ
échoue.
