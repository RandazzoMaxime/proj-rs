# La géodésique de Karney en Rust (`src/geodesic.rs`)

Ce lot porte `geodesic.c` (Charles Karney, MIT/X11), la bibliothèque C que PROJ 9.9 embarque, et
supprime sa compilation : `build.rs` ne compile plus de C et la dépendance de construction `cc`
disparaît. `cargo build` n'a plus besoin d'un compilateur C que pour SQLite (`rusqlite`).

La géodésique sert aux outils `geod` et `invgeod` (`src/geod.rs`) et aux projections `aeqd` et
`gnom`. Les conversions courantes (UTM, Lambert, Web Mercator) n'y touchent pas.

## Ce qui est porté, ce qui ne l'est pas

| C (`geodesic.c`) | Rust (`src/geodesic.rs`) |
|---|---|
| `geod_init`, `A3coeff`, `C3coeff` | `Geodesic::new`, `a3_coefficients`, `c3_coefficients` |
| `geod_lineinit`, `geod_lineinit_int` | `GeodesicLine::new`, `GeodesicLine::start` |
| `geod_genposition`, `geod_position` | `GeodesicLine::genposition`, `position_azimuth`, `position` |
| `geod_gendirect`, `geod_direct` | `Geodesic::direct` |
| `geod_geninverse_int`, `geod_geninverse`, `geod_inverse` | `Geodesic::inverse_solution`, `inverse`, `inverse_full`, `inverse_scale` |
| `Lengths`, `Astroid`, `InverseStart`, `Lambda12` | `Geodesic::lengths`, `astroid`, `Geodesic::inverse_start`, `Geodesic::lambda12` |
| `SinCosSeries` (sinus) | `sin_series` |
| `A1m1f`, `A2m1f`, `C1f`, `C1pf`, `C2f`, `A3f`, `C3f` | les fonctions du même nom, `fourier` pour les trois `C` |
| `sumx`, `AngNormalize`, `AngDiff`, `AngRound`, `LatFix`, `sincosdx`, `sincosde`, `atan2dx` | les fonctions du même nom |

**Non porté**, parce que rien ne l'appelle : l'aire `S12` (`C4coeff`, `C4f`, la série cosinus de
`SinCosSeries`, les parties `GEOD_AREA` de `lineinit`, `genposition` et `geninverse`), les fonctions
de polygone (`geod_polygon_*`, `geod_polygonarea`), `geod_inverseline`, `geod_directline`. Le
rayon authalique `c2` qui ne sert qu'à l'aire n'est pas calculé non plus. Un appelant qui veut l'aire
la portera avec les tables de `C4coeff`.

## Les mêmes bits que le C

Le port garde l'ordre des opérations du C, instruction pour instruction : mêmes `hypot`, `sin`,
`cos`, `atan2`, `cbrt` de la bibliothèque mathématique du système (Rust les appelle, comme le C),
pas de contraction `a * b + c` (rustc n'en fait jamais, c'est ce que `-ffp-contract=off`
obtenait pour le C), et les `volatile` de `sumx` et `AngRound` n'ont pas d'équivalent à écrire :
Rust ne réassocie pas les flottants.

Deux fonctions du C n'existent pas dans la bibliothèque standard de Rust, `remainder` et `remquo`.
Elles sont exactes dans le C (le reste d'une division par 360 ou 90 est représentable) ; les
équivalents de `remainder_turn` et `remquo_quarter` le sont aussi : un `fmod` par 720 ou par 360
(exact), puis une soustraction entre opérandes à un facteur deux l'un de l'autre (exacte, lemme de
Sterbenz), avec le choix pair sur les égalités et le signe du zéro de la libm. Un NaN ou un infini
passent par l'arithmétique comme dans la libm : le signe du NaN que lit `signbit` plus loin est
celui de la libm.

**Vérifié avant de retirer le C** : le port a été comparé au C, appelé par FFI, bit à bit
(`f64::to_bits`, deux NaN égaux) :

- 10 000 000 de problèmes inverses (`inverse_full`, `inverse`, `inverse_scale`) et 10 000 000 de
  problèmes directs et de lignes (`direct`, `GeodesicLine::plain`, `from_azimuth`) tirés dans les
  régimes qui comptent (équateur, pôles, méridiens, antipodes et presque, points presque
  confondus, longitudes décalées d'un tour, longueurs jusqu'à 40 Mm), sur douze aplatissements
  de la sphère à `1/5` et de `-1/10` (allongé) ;
- toutes les combinaisons de 30 valeurs spéciales (NaN, `±inf`, `±0`, `5e-324`, `1e300`, latitudes
  au-delà du pôle, les cas difficiles cités dans les commentaires de Karney) pour les deux points,
  sur les douze aplatissements.

Aucun écart. Le banc de comparaison n'est pas dans le dépôt : il avait besoin du C, qui n'y est plus.

## Contre PROJ 9.9.0

`tests/geodesic_reference.rs` compare à ce que les vrais `invgeod` et `geod` 9.9.0 impriment
(`tools/gen-geodesic-fixtures.py`, `tests/geodesic/inverse.txt` et `direct.txt`) : 690 problèmes
inverses et 600 directs, sur six ellipsoïdes (WGS84, GRS80, Bessel, la sphère, deux aplatissements
élevés), avec les cas limites. Écart au plus 5,7e-13 degré à l'inverse et 2,8e-14 degré en direct,
soit les mêmes chiffres qu'avec le C. La règle de parité (1e-6 m, 1e-11 degré) tient.

Deux précisions sur la lecture du texte de PROJ :

- `geod` imprime l'**azimut retour** au second point, la direction du premier vu de là ; la
  bibliothèque donne l'azimut d'arrivée, à un demi-tour. Le test ajoute 180° ;
- les références viennent de PROJ sous glibc : la libm de Windows ou de macOS diffère d'un bit sur
  `sin`, `cos`, `atan2`, de l'ordre de `4e-9` m de position, et une ligne courte l'amplifie en angle.
  Le test borne l'azimut par `1e-11 + 5e-7 / distance` degré (`azimuth_tolerance`) : 5e-9 pour une
  ligne de 100 m, la règle de parité seule au-delà de quelques centaines de kilomètres. Windows a
  montré 1,5e-10 degré sur une ligne de 102 m ;
- l'azimut d'une ligne presque antipodale est mal conditionné quelle que soit sa longueur (les cas
  difficiles de Karney) : 7e-10 degré mesuré sous Windows et 1,4e-9 sous macOS contre le texte de
  glibc, pour un cas de 19 989 km sur Bessel. Au-delà de 98,5 % de `π b` le test accepte 1e-7
  degré sur l'azimut ; la distance, elle, reste à 1e-6 m. Le test rapporte tous ses écarts d'un
  coup ;
- l'outil lit les degrés, les tient en radians et les rend en degrés : le dernier bit bouge, et une
  ligne de quelques mètres le transforme en 5e-10 degré d'azimut. Le test refait ce voyage.

`geod` refuse un aplatissement négatif : les ellipsoïdes allongés n'ont pas de ligne dans le
corpus, et ne sont vérifiés que par la comparaison au C ci-dessus. Les tests unitaires de `gnom` et
`aeqd` et le corpus de `tests/parity/geod` couvrent les chemins du reste du code.

## Écart de plateforme

Sous Linux et glibc le port rend les bits du C. Sur macOS et Windows, le C était compilé par
`clang` ou `cl` et le port l'est par LLVM : la libm est la même, la contraction des flottants et
le choix de `sincos` sont ceux du compilateur. `tests/parity/geod/known-macos.tsv` et
`known-windows.tsv` listent les géodésiques mal conditionnées qui s'écartaient déjà de PROJ sur ces
systèmes. **Je n'ai pas mesuré ces deux systèmes** ; la CI dit si l'ensemble change (un écart
nouveau ou disparu y échoue), et ces fichiers se corrigent alors d'après son journal.

## Commandes

```bash
tools/build-reference.sh && tools/gen-geodesic-fixtures.py target/proj-reference/build/bin
cargo test --test geodesic_reference
cargo test --workspace --all-features
```
