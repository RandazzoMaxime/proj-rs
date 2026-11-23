# Port — le chemin par lots de `sterea`, `somerc` et `laea`

Lot 7, seconde partie. Fichiers : `src/projections/unit_batch.rs` (`UnitLanes`, `UnitBatch`),
`sterea_batch.rs`, `somerc_batch.rs`, `laea_batch.rs` et leurs `*_tests.rs`, `src/simd/kernels.rs`
(`asin`, `ASIN_LIMIT`, `auxlat_convert`), `engine.rs` (`classic_step_batch`, `ProjectedBatch`
passé en `pub(crate)`).

## Ce qui change

Les trois projections sont des étapes `Classic` : la préparation (`fwd_prepare`) et la
finalisation (`fwd_finalize`) restent les scalaires de `classic_batch.rs`, seul le noyau tourne en
quatre voies. Ce qu'elles partagent est dans `unit_batch.rs` : un noyau n'écrit que sa formule
(`UnitLanes::forward_lanes` et `inverse_lanes`, en unités du demi-grand axe), `UnitBatch` fait le
reste (`a` et `1/a`, les degrés de sortie, le repli scalaire). Le noyau suit `Kernel::forward` et
`Kernel::inverse` opération par opération.

Un groupe de quatre qu'une voie sort du domaine du noyau passe par le scalaire : ses points valent
alors ceux de `Step::forward`. Sont hors domaine : une valeur non finie, une latitude hors de
±(π/2 + 1e-12), un dénominateur nul, le centre de la projection (`rho = 0`), un Newton qui ne
converge pas dans le nombre de tours du scalaire (20 pour `sterea`, 6 pour `somerc`), et un
argument d'`asin` de valeur absolue au-dessus de `ASIN_LIMIT`.

- **`sterea`** (`gauss.cpp`, `sterea.cpp`) : `tan`, deux `powf`, `atan`, `sin_cos` ; au retour,
  `hypot`, `atan2`, `asin`, puis la boucle de Newton de `Gauss::inverse` avec un masque de voies
  convergées.
- **`somerc`** : `log(tan)`, `exp`, `atan`, deux `asin` ; au retour, `exp`, `atan`, `asin` et la
  boucle de Newton à six tours.
- **`laea` ellipsoïdal, modes oblique et équatorial** : la série authalique de `projmath` en
  Clenshaw, avec ses coefficients calculés une fois à la création (le scalaire appelle
  `auxlat_coeffs` à chaque point). Les modes polaires (un argument d'`asin` qui tend vers 1 au
  pôle), la sphère et un `n` au-delà de la série (`SERIES_N`, 0,01) gardent `classic_step` : pas
  de noyau, donc rien qui change.

`asin` n'a pas de fonction dans `math.rs` : `kernels::asin` est `atan2(x, sqrt((1 − x)(1 + x)))`.
Son erreur croît comme `1 / sqrt(1 − x²)`. Premier essai à `ASIN_LIMIT = 0,9999` : un `somerc` sur
le globe sort du seuil (1,8e-6 m à l'aller, relatif 2,4e-13). Avec 0,999 : l'aller passe, le retour
sort à 86,6° de latitude (2e-13 rad, la longitude y gagne `1 / cos φ`). `0,97` passe sur le globe
entier ; l'aire d'usage reste très en deçà (Suisse, `sin β ≈ 0,73`).

## Mesure

Protocole de `docs/PORTING-batch-datum.md` (deux passages alternés, moyenne). Base :
`origin/main`.

| cas | avant ns/pt | après ns/pt | écart |
|---|---:|---:|---:|
| `+proj=laea` EPSG:3035 (chaîne PROJ, radians) | 69,3 | 17,9 | −74,2 % |
| `EPSG:4326 → EPSG:28992` (`sterea`, avec le datum du lot) | 267,9 | 66,4 | −75,2 % |
| `EPSG:4326 → EPSG:2056` (`somerc`, avec le datum du lot) | 300,5 | 85,7 | −71,5 % |
| `EPSG:2056 → EPSG:4326` (inverse, avec le datum du lot) | 331,0 | 107,3 | −67,6 % |
| `EPSG:28992 → EPSG:4326` (inverse, avec le datum du lot) | 577,6 | 198,8 | −65,6 % |

Part des noyaux seuls sur le même passage de mesure (une série par ligne) : `sterea`
111,9 → 69,5 ns/pt sur `EPSG:4326 → EPSG:28992` après la pile, `cart` et `helmert` (−38 %) ;
`somerc` 160,8 → 83,1 sur `EPSG:4326 → EPSG:2056` (−48 %). Tous dépassent les 10 % demandés ;
aucun noyau n'est écarté.

Les lignes « avec le datum du lot » mêlent les deux parties ; la part de chacune est donnée juste
au-dessus et dans `docs/PORTING-batch-datum.md`.

`EPSG:4326 → EPSG:3035` et `EPSG:3035 → EPSG:4326` ne changent pas (122,7 → 122,1 et
152,6 → 157,1 ns/pt, bruit) : le couple a plusieurs candidats et la façade les prend point par
point (`docs/PORTING-batch-api.md`). Le noyau `laea` sert une opération construite seule
(`Proj::new` d'une chaîne PROJ, `+proj=laea`).

## Parité

Écart maximal contre le scalaire sur 200 000 points de l'aire d'usage : `sterea` 8,4e-9 m (aller),
7,8e-14° (retour) ; `somerc` 7,9e-9 m, 9,2e-14° ; `laea` 2,8e-9 m. Seuil du dépôt : 1e-6 m ou
1e-13 relatif, 1e-11°.

Tests (`*_batch_tests.rs`, tolérance de `batch_check.rs` : mètres 1e-6 ou 1e-13 relatif, radians
1,75e-13) : trois boîtes par définition (l'aire d'usage, un hémisphère ou le globe), 3 000
points tirés plus les angles de la boîte, les pôles, ±π, les latitudes refusées, `NaN`, les
infinis ; au retour les images de ces points par le scalaire, l'origine, des valeurs à 5e6 et 1e300.
Définitions : `sterea` Bessel (EPSG:28992), sphère, `+units=km +lon_wrap`, origine australe ;
`somerc` EPSG:2056 et 21781, sphère, origine australe avec unités ; `laea` EPSG:3035, équatorial,
austral avec unités, et les modes sans noyau (pôles, sphère, ellipsoïde aplati), qui doivent être
au bit celles du scalaire. Une erreur de 1e-10 relatif écrite dans le noyau de `sterea` (retour,
aller) fait échouer le test (vérifié, puis annulé).

## Limites

- Les couples à plusieurs candidats ne passent pas par ces noyaux (voir plus haut).
- Hors AVX2 et NEON, `UnitBatch` rejoue `Kernel::forward` et `Kernel::inverse` : mêmes bits que
  le scalaire, sans gain. Sur aarch64, les noyaux de `laea`, `somerc` et `sterea` tournent avec
  NEON et donnent les mêmes bits que sur AVX2 (`docs/PORTING-simd.md`).
