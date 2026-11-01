# Port — `operation.transformation_*` : constructeurs de transformations (lot operation-3)

Source : PROJ 9.9.0, `test/unit/test_operation.cpp`, les `TEST()` de la classe
`operation.` qui construisent une `Transformation` (Helmert, Molodensky) ou
inversent une conversion. Côté crate : `tests/test_operation_transformations.rs`
(nouveau), `src/iso19111/buildop/transformations.rs` (nouveau : constructeurs et
inverse des transformations) et trois retouches de `src/iso19111/buildop.rs` et
`src/iso19111/wkt.rs` (section 3).

Ce lot porte **6 `TEST()`** et laisse **13 tests ouverts**, avec la cause de
chacun. Les chaînes attendues sont celles de PROJ, sans modification. Les
valeurs de paramètres sont comparées à l'égalité exacte, comme PROJ le fait
dans ces tests (aucune tolérance n'est utilisée).

## 0. Périmètre et base

- Base : `HEAD` = `0f9be90` (lot operation-2).
- Les blocs `#ifdef USE_APPROXIMATE_HELMERT_INVERSE` ne sont pas compilés par
  PROJ 9.9.0. Les tests de position vector et de coordinate frame portés ne
  contiennent que la branche `#else` : elle vérifie la PROJ string de
  l'inverse. Les vérifications WKT de ces blocs ne sont pas dans le port,
  comme PROJ ne les exécute pas.
- Le test C++ `coordinateOperationAccuracies().size()` est porté comme
  `accuracy()`. La crate ne garde qu'une précision (`Option<f64>`), pas une
  liste : un écart d'API assumé, sans conséquence pour ces tests.

## 1. Portés (6)

| PROJ 9.9.0 `operation.*` | PROJ-RS `tests/test_operation_transformations.rs` | Ce qui est vérifié |
|---|---|---|
| `transformation_createGeocentricTranslations` | `operation_transformation_create_geocentric_translations` | `getTOWGS84Parameters(true)` de la transformation et de son inverse, noms des CRS inversés, PROJ string `+proj=helmert` avec `cart` GRS80/WGS84 |
| `transformation_createGeocentricTranslations_between_geocentricCRS` | `operation_transformation_create_geocentric_translations_between_geocentric_crs` | trois PROJ strings, entre CRS géocentriques en mètres et en kilomètres (`unitconvert`) |
| `transformation_createGeocentricTranslations_null` | `operation_transformation_create_geocentric_translations_null` | l'inverse d'une translation nulle est `+proj=noop` |
| `transformation_createGeocentricTranslations_neg_zero` | `operation_transformation_create_geocentric_translations_neg_zero` | l'inverse de `-0.0` est écrit `+z=0` (pas `-0`), `+x=-1` |
| `transformation_createPositionVector` | `operation_transformation_create_position_vector` | précision 100, `getTOWGS84Parameters`, PROJ string directe et inverse (`+inv +proj=helmert … +convention=position_vector`) |
| `transformation_createCoordinateFrameRotation` | `operation_transformation_create_coordinate_frame_rotation` | rotations signées (`+rx=-4` en directe, `+convention=coordinate_frame`), aucune précision |

Le ledger `docs/UNPORTED-UNIT.md` retire ces 6 noms de la section
`test_operation.cpp`.

## 2. Tentés ou non portés, ouverts, avec la cause

### A. `transformation_createTimeDependentPositionVector`, `…CoordinateFrameRotation`

Les constructeurs `create_time_dependent_position_vector` et
`create_time_dependent_coordinate_frame_rotation` existent (EPSG 1054 et 1057,
sept valeurs, sept taux, époque). Le test n'est pas porté : son bloc WKT n'est
pas sous `#ifdef`, et attend `Transformation from WGS 84 to NAD83 (approx.
inversion)`. La crate donne à une transformation sans nom le nom de sa
méthode, et l'inverse d'une Helmert ne reçoit pas ce nom « approx. inversion ».
La PROJ string de ces deux tests n'a pas été comparée.

### B. `transformation_createMolodensky`, `transformation_createAbridgedMolodensky`

Les constructeurs `create_molodensky` et `create_abridged_molodensky` existent,
et la partie WKT (`METHOD["Molodensky",ID["EPSG",9604]]`) passe. La PROJ string
de l'inverse ne peut pas être produite : `src/iso19111/transform_export.rs`
n'écrit pas `+proj=molodensky` (le cas tombe dans `unimplemented`, « Unimplemented
Inverse of Molodensky »). Écrire cette étape (`+ellps` de la CRS source,
`+dx +dy +dz +da +df`, `+abridged`) est une fonction d'export à part, dans
`transform_export.rs`, déjà grand. Lot distinct.

### C. `transformation_to_wkt`, `transformation_inverse`

Ces deux tests attendent un identifiant sur la **méthode** de la transformation
(`ID["codeSpaceOperationMethod","codeOperationMethod"]`, et
`ID["INVERSE(my codeSpace)","my code"]` pour l'inverse). `Transformation::create`
ne prend que le nom de la méthode et ses paramètres ; la crate ne peut pas donner
d'identifiant à une méthode depuis ce constructeur. Une API qui prend les
propriétés de la méthode est nécessaire ; hors de ce lot.

### D. `concatenated_operation`, `transformation_successive_helmert_noop`, `…_non_trivial_1`, `…_non_trivial_2`

Il n'existe pas de constructeur public `ConcatenatedOperation::create` (la crate
n'a que `inverse`, `steps`, et les accesseurs). `concatenated_operation` exige en
plus l'écriture WKT `CONCATENATEDOPERATION[...]`, que `wkt.rs` refuse
explicitement (« A concatenated operation has no single-node spelling here »).
Les trois tests `successive_helmert` attendent `+proj=noop` pour un aller-retour
Helmert : la réduction du pipeline n'est pas vérifiée dans la crate.

### E. `transformation_createTOWGS84`

Le constructeur prend un CRS source, et PROJ utilise la constante
`GeographicCRS::EPSG_4326` pour la cible. La crate n'a pas d'objet WGS 84 intégré
pour cela. Les deux `EXPECT_THROW` (3 ou 7 valeurs requises, CRS vertical refusé)
ne peuvent pas être vérifiés sans cette cible.

### F. `createAxisOrderReversal`

`Conversion::createAxisOrderReversal` n'existe pas. Le test utilise en outre
`CoordinateOperationFactory::createOperation`, une fonction à résultat unique ;
la crate n'a que `create_operations` (liste). Hors de ce lot.

### G. `conversion_inverse`

La partie WKT passe après la retouche 3.2 : `METHOD["Inverse of Transverse
Mercator", ID["INVERSE(EPSG)",9807]]`. La PROJ string
`+proj=pipeline +step +inv +proj=tmerc +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5`
n'est pas produite : `buildop.rs` refuse l'export d'une conversion inversée
(« has no PROJ string in this crate »). Une tentative de produire ce `+inv`
a fait échouer le test existant `tests/derived_operation.rs::inverse_conversion_has_no_forward_proj_string`,
qui affirme cette même erreur. La retouche a été annulée : ce test ne
régresse pas. Lever le refus demande de changer l'attendu d'un test de la
crate, ce qui est une décision à prendre à part.

## 3. Changements dans `src`

Chaque changement répond à un test du lot ou à la cohérence de l'inverse.

1. **`src/iso19111/buildop/transformations.rs`** (nouveau, sous-module de
   `buildop`) :
   - `Transformation::create_geocentric_translations` (EPSG 9603 pour un CRS
     géographique, 1031 pour un CRS géocentrique), `create_position_vector`
     (9606 ou 1033), `create_coordinate_frame_rotation` (9607 ou 1034),
     `create_time_dependent_position_vector` (1054),
     `create_time_dependent_coordinate_frame_rotation` (1057),
     `create_molodensky` (9604), `create_abridged_molodensky` (9605). Les
     valeurs sont en mètres, secondes d'arc et parties par million, comme les
     paramètres EPSG.
   - `towgs84_parameters` : `getTOWGS84Parameters(true)`, public. Il appelle
     `towgs84_strict`, déjà présent dans `bound.rs` (pour les CRS liés).
   - `has_translation_inverse` et `negate_values` : voir 3.2.
   - `build` retire le domaine que `create_from_code` enregistre, comme PROJ
     ne donne pas d'usage à ces constructeurs.
2. **`src/iso19111/buildop.rs`** :
   - `mod transformations;`.
   - `Transformation::inverse` : pour les méthodes 1031, 9603, 9604 et 9605,
     l'inverse nie les valeurs et n'active pas le drapeau `inverted`. PROJ
     écrit ainsi l'inverse d'une translation (`+x=-1` et non `+inv`). Corrige
     `transformation_createGeocentricTranslations` et `…_neg_zero`. Les autres
     méthodes gardent `+inv`.
   - `Conversion::inverse` passe de `pub(crate)` à `pub`, comme PROJ
     `inverse()`. Aucun autre appelant ne change.
3. **`src/iso19111/wkt.rs`** :
   - `inverted_method` : en WKT2, la méthode d'une conversion inversée s'écrit
     `Inverse of <nom>` avec `ID["INVERSE(<espace de code>)", code]`, comme PROJ
     `InverseConversion`. Le nom de la conversion et les PROJ strings ne
     changent pas.

Aucune valeur attendue n'a été modifiée. Aucun `#[allow]` n'est ajouté.

## 4. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 6 lignes retirées de `## test_operation.cpp`
  (la ligne `conversion_inverse` reste, voir 2.G).
- `docs/UNIT-TRIAGE.md` et `docs/progress.svg` : régénérés
  (`UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`).
- `README.md` et `ROADMAP.md` : **non modifiés** (restaurés depuis `HEAD`), comme
  pour le lot operation-2 et le lot io-6. Le test
  `counts_in_docs_match_the_ledgers` (`tests/docs_in_sync.rs:288`) échoue donc
  jusqu'à leur régénération dans un lot de documentation à part.
- `docs/UNPORTED-CLI.md` : inchangé.

## 5. Vérification

Les codes de sortie sont ceux du shell. Les commandes ont été lancées avec
`CARGO_INCREMENTAL=0` et `CARGO_PROFILE_DEV_DEBUG=0` (le volume était plein
pendant le lot, voir la note en fin de section).

```
cargo fmt --check                                                # fmt=0
cargo clippy --all-targets --all-features -- -D warnings         # clippy=0
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg # rc=0
git restore --source=HEAD -- README.md ROADMAP.md                # restaurés
cargo test --all-features --no-fail-fast                         # rc=101 : seul
                                                                 #   counts_in_docs_match_the_ledgers échoue
```

`cargo test --all-features --no-fail-fast` : 34 binaires `test result: ok`, dont
`test_operation_transformations` (6 passed), `test_operation_exports` (19 passed),
`derived_operation` (4 passed) et `cli_yaml` (1 passed, pas de « Broken pipe »).
Le seul échec est `docs_in_sync::counts_in_docs_match_the_ledgers` (section 4).

Note : pendant le lot le volume partagé était plein. Le `target/` du worktree
(build regenerable) a été nettoyé avec `cargo clean`, et le dossier de travail
temporaire de la session a perdu sa copie de build (`base_target`). Aucun fichier
du dépôt n'a été supprimé.
