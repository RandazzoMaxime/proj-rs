# Lot : noms verticaux LAS et ESRI, TOWGS84 d'une CRS liée, datum WKT1

Référence : PROJ 9.9.0. `test/unit/test_io.cpp` (`TEST(wkt_parse, BOUNDCRS_transformation_from_names)`,
`BOUNDCRS_transformation_from_codes`, `vertcrs_wkt1_las_*`, `compoundcrs_wkt1_las_*`) et
`src/iso19111/io.cpp` (consommation des paramètres, bloc LAS de `buildVerticalCRS`, l. ~5157-5206),
`src/iso19111/operation/singleoperation.cpp` (`OperationParameterValue::convertFromAbridged`, l. 1005-1055).

Ce lot ferme les sept tests marqués `#[ignore]` ci-dessous. Les attendus sont ceux du C++, sans
modification.

## Bilan

| Test (fichier) | Marqueur d'origine | Cause réelle constatée | Correctif |
|---|---|---|---|
| `wkt_parse_vertcs_wkt1_esri_context` (`test_io_wkt2.rs`) | alias `EGM2008_Geoid` en `ESRI_OLD`, recherche `ESRI` seule | déjà couvert : `official_from_alias` (`factory.rs`, l. ~970) ajoute `ESRI_OLD` quand la source est `ESRI`, comme `getOfficialNameFromAlias` | aucun, marqueur retiré |
| `wkt_parse_vertcrs_wkt1_las_ftus` | `NAVD88 - Geoid03 (Feet)` non mappé en EPSG 6360 | déjà porté : `las_geoid_name` et `apply_las_vertical` (`wkt_import.rs`) suivent `io.cpp` | aucun, marqueur retiré |
| `wkt_parse_vertcrs_wkt1_las_metre` | `NAVD88 via Geoid09` non mappé en EPSG 5703 | déjà porté, même règle | aucun, marqueur retiré |
| `wkt_parse_compoundcrs_wkt1_las_only_geoid_name` | `NAVD88 - Geoid03 (Feet)` sans ligne d'alias ni règle LAS | déjà porté : la règle LAS ne dépend pas de l'alias | aucun, marqueur retiré |
| `wkt_parse_boundcrs_transformation_from_names` (`test_io_wkt2.rs`) | « pas d'accesseur public `getTOWGS84Parameters` » | l'accesseur existe : `Transformation::towgs84_parameters()` (`buildop/transformations.rs`), équivalent de `getTOWGS84Parameters(true)`. La vraie cause : `ABRIDGEDTRANSFORMATION` ne reconnaît pas les paramètres par nom (`X-axis translation`, ...) et ne leur donne pas le code EPSG | `convert_from_abridged` (voir ci-dessous) |
| `wkt_parse_boundcrs_transformation_from_codes` | idem | la vraie cause : `rotx` (code 8608) lu sans unité, donc en radians au lieu d'arc-secondes ; `-4` devenait `825059.2` | même correctif |
| `wkt_parse_wkt1_geographic_old_datum_name_without_epsg_code` (`test_io_wkt1.rs`) | « pas de ligne d'alias ni de code AUTHORITY » | le test passe avec `proj.db` (`parse_db`) : le nom `System of the Unified Trigonometrical Cadastral Network (Ferro)` est produit. Le marqueur ne reproduit pas | aucun, marqueur retiré |

Cinq tests (lignes 1 à 4 et 7 du tableau) étaient déjà verts une fois le marqueur retiré : leur
`#[ignore]` ne décrivait plus le code. Seuls les deux tests `BOUNDCRS` échouaient réellement. Aucune
assertion n'a été changée.

## Correctif : `ABRIDGEDTRANSFORMATION` (`src/iso19111/wkt_import.rs`)

Port de `consumeParameters(isAbridged = true)` et de `convertFromAbridged` :

- `convert_from_abridged(name, value, epsg_code)` (nouvelle fonction privée) reconnaît les sept
  paramètres de Helmert par nom EPSG équivalent (`equiv::equivalent_name`) ou par code (8605 à
  8611). Il renvoie le code EPSG, l'unité imposée (mètre pour les translations, arc-seconde pour
  les rotations, ppm pour l'échelle) et la valeur convertie. Pour l'échelle, `(s - 1) * 1e6`.
- `parameters_of` prend un `ParameterSyntax` (`Wkt` ou `Abridged`, enum privée) au lieu de rien :
  quatre appels, trois en `Wkt`, celui de `build_transformation` en `Abridged`.
- En `Abridged`, l'unité lue ou devinée est remplacée par l'unité imposée, la valeur convertie, et
  l'identifiant du paramètre devient `EPSG:<code>`, comme `parameters.back() = ...` dans `io.cpp`.

Pour éviter une double conversion de l'échelle, la conversion « échelle sans UNIT → ppm » existante
ne s'applique plus qu'en `Wkt`. En `Abridged`, c'est `convert_from_abridged` qui convertit.

## Tests

- `tests/test_io_wkt2.rs` : les deux tests `BOUNDCRS` TOWGS84 assertent maintenant les sept valeurs
  `1, 2, 3, 4, 5, 6, 7` (`test_io.cpp` l. 5207-5212 et 5256-5261) via
  `Transformation::towgs84_parameters()`. Le commentaire « open part » est retiré.
- Marqueurs `#[ignore]` retirés : 7 tests, dont 6 dans `test_io_wkt2.rs` (`..._esri_context`,
  `..._las_ftus`, `..._las_metre`, `..._las_only_geoid_name`, `..._transformation_from_names`,
  `..._transformation_from_codes`) et 1 dans `test_io_wkt1.rs`
  (`..._old_datum_name_without_epsg_code`). Les autres `#[ignore]` de ces fichiers ne sont pas
  traités ici.

## Écarts

- Aucun écart volontaire vers PROJ dans ce lot.
- La conversion « échelle sans UNIT → ppm » pour les nœuds non abrégés existait avant ce lot. Elle
  n'a pas d'équivalent dans le `consumeParameters` du C++ ; elle est conservée telle quelle, hors
  de la voie abrégée.

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --test test_io_wkt2 --test test_io_wkt1
```
