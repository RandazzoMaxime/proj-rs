# Port — `operation.*_export` : exports de projections (lot operation-2)

Source : PROJ 9.9.0, `test/unit/test_operation.cpp`, les `TEST()` d'export
de projection de la classe `operation.`. Côté crate :
`tests/test_operation_exports.rs` (nouveau), `src/iso19111/buildop/projections.rs`
(nouveau : constructeurs de conversion), et trois retouches de
`src/iso19111/buildop.rs`, `src/iso19111/wkt_import.rs` et
`src/iso19111/parammap.rs` (section 3).

Ce lot porte **19 `TEST()`** et laisse **3 tests** ouverts, avec la cause de
chacun. Les chaînes attendues sont celles de PROJ, sans modification. Ce lot ne
contient pas de comparaison numérique à tolérance.

## 0. Périmètre et base

- Base : `HEAD` = `f4ea07b`. `docs/UNIT-TRIAGE.md` et
  `tests/docs_in_sync.rs` y sont présents.
- Les chaînes attendues sont recopiées de `test_operation.cpp` (9.9.0). Les
  blocs WKT multi-lignes sont écrits avec `concat!` et `\n`, pas avec une
  chaîne brute, pour ne pas dépendre des fins de ligne de Windows.
- `tests/test_operation.rs` n'est pas modifié. Il contient déjà
  `operation_tmerc_south_oriented_export`, version plus faible du TEST()
  PROJ ; le nouveau test du même nom, dans le nouveau fichier, couvre le
  TEST() complet.

## 1. Portés (19)

| PROJ 9.9.0 `operation.*` | PROJ-RS `tests/test_operation_exports.rs` | Ce qui est vérifié |
|---|---|---|
| `tmerc_south_oriented_export` | `operation_tmerc_south_oriented_export` | WKT2 et WKT1 ; PROJ string en erreur ; CRS « Lo29 » en `+axis=wsu +k=1` |
| `tmerc_south_oriented_export_esri_102470` | `operation_tmerc_south_oriented_export_esri_102470` | WKT1 ESRI avec `Scale_Factor = -1` : méthode sud, `+k=1` |
| `bonne_export` | `operation_bonne_export` | WKT2, WKT1, CRS WKT1 `standard_parallel_1` → `+lat_1=1` |
| `nzmg_export` | `operation_nzmg_export` | WKT2, WKT1, PROJ string |
| `oblique_stereographic_export` | `operation_oblique_stereographic_export` | WKT2, WKT1, `+proj=sterea` |
| `orthographic_export` | `operation_orthographic_export` | WKT2, WKT1, `+proj=ortho` |
| `local_orthographic_export` | `operation_local_orthographic_export` | WKT2 (1130), WKT1, `+alpha` et `+k` |
| `american_polyconic_export` | `operation_american_polyconic_export` | WKT2, WKT1 `Polyconic`, `+proj=poly` |
| `robinson_export` | `operation_robinson_export` | WKT2, WKT1, `+proj=robin` |
| `sinusoidal_export` | `operation_sinusoidal_export` | WKT2, WKT1, `+proj=sinu` |
| `stereographic_export` | `operation_stereographic_export` | WKT2, WKT1, `+proj=stere` |
| `vandergrinten_export` | `operation_vandergrinten_export` | WKT2, WKT1 `VanDerGrinten`, `+R_A` |
| `wagner_export` | `operation_wagner_export` | Wagner I, II, IV, V, VI, VII (III a son test) : WKT2 avec `simulCurNodeHasId`, WKT1, PROJ string |
| `wagnerIII_export` | `operation_wagner_iii_export` | `+proj=wag3 +lat_ts`, WKT2 et WKT1 |
| `qsc_export` | `operation_qsc_export` | WKT2, WKT1, `+proj=qsc` |
| `sch_export` | `operation_sch_export` | WKT2 (peg point), WKT1 `peg_point_*`, `+proj=sch` |
| `eqearth_export` | `operation_eqearth_export` | WKT2 (1078), `+proj=eqearth` |
| `vertical_perspective_export` | `operation_vertical_perspective_export` | WKT2 (9838, 8834 à 8840), `+proj=nsper +h` |
| `hotine_oblique_mercator_variant_A_export_equivalent_modulo_360` | `operation_hotine_oblique_mercator_variant_a_export_equivalent_modulo_360` | Deux conversions Hotine dont les angles diffèrent d'un tour sont équivalentes (`Equivalent`) |

Le ledger `docs/UNPORTED-UNIT.md` retire ces 19 noms de la section
`test_operation.cpp`.

## 2. Tentés, ouverts, avec la cause

### A. `geostationary_satellite_sweep_x_export`

Les deux premières assertions (conversion WKT2 et PROJ string
`+proj=geos +sweep=x`) passent : le constructeur
`create_geostationary_satellite_sweep_x` existait déjà. La troisième ne
passe pas : le test lit le WKT1 du `ProjectedCRS` et cherche
`EXTENSION["PROJ4", "+proj=geos +sweep=x ... +datum=WGS84 +units=m +no_defs"]`.
`src/iso19111` ne contient aucun écrivain de `EXTENSION["PROJ4", …]` (les
seules occurrences de `PROJ4` sont des noms de grilles et de l'import). Le
test reste ouvert pour cette raison, qui relève d'un lot WKT de CRS
(`Conversion::addWKTExtensionNode` dans PROJ, `operation/conversion.cpp`).

### B. `webmerc_export`

Les constructions et la chaîne WKT2 de la conversion « Popular
Visualisation Pseudo Mercator » (EPSG:1024) seraient identiques. Le test
continue au-delà : le WKT1 du CRS « Pseudo-Mercator » contient
`EXTENSION["PROJ4", "+proj=merc ... +nadgrids=@null +wktext +no_defs"]`, qui
n'est pas écrit (même cause qu'en A) ; la PROJ string du CRS se termine par
`+nadgrids=@null +wktext`, jeton que la crate n'émet pas ; et la
`createOperation(EPSG:4326, projCRS)` attendue (un pipeline
`axisswap`, `unitconvert`, `webmerc`) relève de la fabrique d'opérations, lot
distinct. Un port partiel ne serait pas le TEST() de PROJ ; il reste ouvert.

### C. `adams_ws2_export`

Le CRS `ESRI:54098` (WGS_1984_Adams_Square_II) se lit bien par la fabrique
ESRI, mais son export PROJ échoue : `Unsupported conversion method:
Adams_Square_II`. `src/iso19111` ne contient ni `Adams` ni `Spilhaus` : la
méthode ESRI et la projection `+proj=spilhaus` ne sont pas mappées. C'est le
même manque que `spilhaus_esri_export` (hors de ce lot, dans la liste
des tests ouverts de `docs/UNPORTED-UNIT.md`). Un mapping de la méthode ESRI
et des paramètres `azi`, `k_0`, `rot` est un lot à part.

### Hors liste, non porté

`vertical_perspective_export_no_topocentric_height_and_false_easting_northing`
(le second TEST sur 9838) n'est pas dans la liste du lot. Il contrôle
l'omission des zéros de faux est et faux nord déjà implémentée dans
`wkt.rs` (`omitted_zero_parameter`), mais ce comportement reste sans test
dans ce lot.

## 3. Changements dans `src`

Chaque changement répond à un test du lot.

1. **`src/iso19111/buildop/projections.rs`** (nouveau, sous-module de
   `buildop`) : vingt constructeurs `Conversion::create_*` qui manquaient
   pour les projections de la liste : `new_zealand_mapping_grid`,
   `oblique_stereographic`, `stereographic`, `orthographic`,
   `local_orthographic`, `american_polyconic`, `robinson`, `sinusoidal`,
   `van_der_grinten`, `wagner_i`, `wagner_ii`, `wagner_iv`, `wagner_v`,
   `wagner_vi`, `wagner_vii`, `wagner_iii`, `quadrilateralized_spherical_cube`,
   `spherical_cross_track_height`, `equal_earth`, `vertical_perspective`.
   Chacun appelle `from_name` avec les noms de méthode de `parammap.rs`, comme
   les constructeurs existants. Les noms et l'ordre des paramètres suivent
   PROJ `Conversion::create*`.
   `buildop.rs` ne fait qu'ajouter `mod projections;`.
2. **`src/iso19111/buildop.rs`**, `measures_equivalent` : sous le critère
   `Equivalent` (pas `Strict`), deux angles qui diffèrent d'un nombre entier de
   tours (360°) sont équivalents. Avant, ils ne l'étaient pas, et le test
   Hotine échouait. Le critère reste de 2e-10 relatif. Ce changement vaut pour
   toute mesure angulaire de paramètre : c'est la règle voulue, mais elle
   s'applique à toutes les comparaisons `Equivalent` d'angles.
3. **`src/iso19111/wkt_import.rs`**, `build_wkt1_projection` :
   - **Noms WKT1 de paramètres selon la méthode.** Un nom WKT1 se résolvait
     d'abord par la table générique, puis la méthode était ignorée. Pour Bonne,
     `standard_parallel_1` devenait EPSG 8823 au lieu de 8801 de la méthode :
     le PROJ string sortait `+lat_1=0`. Désormais, le nom WKT1 est cherché dans
     les paramètres de la méthode (code de la méthode, ou son nom WKT1 si la
     méthode n'a pas de code dans `wkt1_method`), puis la table générique est le
     repli. Corrige `bonne_export`.
   - **Transverse Mercator sud en ESRI.** Un `Transverse_Mercator` ESRI dont le
     `scale_factor` vaut -1 devient `Transverse Mercator (South Orientated)`
     avec `scale_factor = 1`, comme PROJ `io.cpp` pour le dialecte ESRI. Corrige
     `tmerc_south_oriented_export_esri_102470`. La règle n'est active que pour
     le dialecte ESRI (`self.esri`).
4. **`src/iso19111/parammap.rs`** : `method_by_wkt1_name`, une recherche de
   méthode par nom WKT1 (sans tenir compte de la casse), utilisée seulement
   par le point 3.

Aucune valeur attendue n'a été modifiée. Aucun `#[allow]` n'est ajouté.

## 4. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 19 lignes retirées de `## test_operation.cpp`.
- `docs/UNIT-TRIAGE.md` et `docs/progress.svg` : régénérés
  (`UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`).
- `README.md` et `ROADMAP.md` : **non modifiés** (restaurés depuis `HEAD`).
  Leurs compteurs ne correspondent plus au ledger. Le test
  `counts_in_docs_match_the_ledgers` (`tests/docs_in_sync.rs:288`) échoue donc
  jusqu'à leur régénération dans un lot de documentation à part, comme pour
  le lot io-6.
- `docs/UNPORTED-CLI.md` : inchangé.

## 5. Vérification

Les commandes ci-dessous ont été lancées dans le worktree. Les codes de sortie
sont ceux du shell.

```
cargo fmt --check                                                 # fmt=0 (après cargo fmt sur les fichiers du lot)
cargo clippy --all-targets --all-features -- -D warnings          # clippy=0
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg  # rc=0 (2 + 1 tests)
git restore --source=HEAD -- README.md ROADMAP.md                 # restaurés
cargo test --all-features --no-fail-fast                          # rc=101, voir ci-dessous
```

`cargo test --all-features --no-fail-fast` : tous les binaires passent, sauf
`docs_in_sync::counts_in_docs_match_the_ledgers` (section 4). Résultats par
binaire, dont `test_operation_exports` : 19 passed, 0 failed. `cli_yaml` passe
(pas de « Broken pipe »).

Les commandes ont été lancées avec `CARGO_INCREMENTAL=0` et
`CARGO_PROFILE_DEV_DEBUG=0` : le volume était plein pendant le lot (pièces
de build, pas de données du dépôt). Ces variables ne changent que la taille
des binaires de test, pas leur comportement.
