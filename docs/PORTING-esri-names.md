# Port — noms de méthodes et de paramètres ESRI (WKT1 ESRI)

Source : PROJ 9.9.0, `src/iso19111/operation/esriparammappings.cpp` et `.hpp`
(table générée par `scripts/build_esri_projection_mapping.py`), et ce qui s'en
sert dans `src/iso19111/operation/conversion.cpp` (`getESRIMethodNameAndParams`,
branche ESRI de `_exportToWKT`, `convertToOtherMethod`), `io.cpp`
(`WKTFormatter::add`) et `datum.cpp` (`Ellipsoid::squaredEccentricity`).

Périmètre : l'export WKT1 ESRI d'une conversion (nom de méthode et paramètres).
L'import WKT1 ESRI (`getMappingsFromESRI`, `WKTParser::Private::getESRIMapping`)
n'est pas porté : `wkt_import.rs` garde ses propres règles.

## Correspondance d'API

| PROJ 9.9.0 | PROJ-RS |
|---|---|
| `ESRIParamMapping` | `EsriParam` : `Value` (valeur du paramètre WKT2) ou `Fixed` (texte brut). `is_fixed_value` n'est pas porté : seul l'import le lit |
| `ESRIMethodMapping` | `EsriMethod` |
| `esriMappings`, `getEsriMappings` (82 lignes) | `ESRI_METHODS` |
| `paramsESRI_*` (81 tableaux) | `PARAMS_*` (même ordre) |
| `getESRIMapping` | `first_mapping` (privée) |
| `getESRIMethodNameAndParams` | `esri_method`, qui renvoie un `EsriChoice` |
| `SingleOperation::parameterValue(nom, code)` | `find_parameter` : par code EPSG, puis par nom |
| `parameterValueNumericAsSI`, `parameterValueNumeric` | `si_value`, `numeric` (0 si absent, comme C++) |
| `Conversion::convertToOtherMethod` (9804 vers 9805) | `mercator_variant_b` |
| `Ellipsoid::squaredEccentricity` | `squared_eccentricity` |
| branche ESRI de `_exportToWKT` | `export_esri_method` et `write_esri_value` (`wkt.rs`) |
| `getMappingsFromESRI` (import) | non porté |

La table est générée depuis le source C++ et `src/proj_constants.h`, puis
relue : 82 méthodes, 81 tableaux, 349 lignes `Value` et 17 lignes `Fixed`.
Comme en C++, la première ligne qui correspond gagne (par code EPSG s'il est
non nul, sinon par nom WKT2 sans tenir compte de la casse).

## Correspondance de tests

| Test C++ (`test/unit/test_crs.cpp`) | Test Rust (`tests/test_crs.rs`) |
|---|---|
| `crs.mercator_1SP_as_WKT1_ESRI` | `crs_mercator_1sp_as_wkt1_esri` |
| `crs.Plate_Carree_as_WKT1_ESRI` | `crs_plate_carree_as_wkt1_esri` |
| `crs.Equidistant_Cylindrical_as_WKT1_ESRI` | `crs_equidistant_cylindrical_as_wkt1_esri` |
| `crs.Hotine_Oblique_Mercator_Azimuth_Natural_Origin_as_WKT1_ESRI` | `crs_hotine_oblique_mercator_azimuth_natural_origin_as_wkt1_esri` |
| `crs.Rectified_Skew_Orthomorphic_Natural_Origin_as_WKT1_ESRI` | `crs_rectified_skew_orthomorphic_natural_origin_as_wkt1_esri` |
| `crs.Hotine_Oblique_Mercator_Azimuth_Center_as_WKT1_ESRI` | `crs_hotine_oblique_mercator_azimuth_center_as_wkt1_esri` |
| `crs.Rectified_Skew_Orthomorphic_Center_as_WKT1_ESRI` | `crs_rectified_skew_orthomorphic_center_as_wkt1_esri` |
| `crs.Gauss_Kruger_as_WKT1_ESRI` | `crs_gauss_kruger_as_wkt1_esri` |
| `crs.Stereographic_North_Pole_as_WKT1_ESRI` | `crs_stereographic_north_pole_as_wkt1_esri` |
| `crs.Stereographic_South_Pole_as_WKT1_ESRI` | `crs_stereographic_south_pole_as_wkt1_esri` |
| `crs.Krovak_North_Orientated_as_WKT1_ESRI` | `crs_krovak_north_orientated_as_wkt1_esri` |
| `crs.Krovak_as_WKT1_ESRI` | `crs_krovak_as_wkt1_esri` |
| `crs.LCC_2SP_as_WKT1_ESRI` | `crs_lcc_2sp_as_wkt1_esri` |

Ces 13 tests étaient marqués `#[ignore]` ; le marqueur est retiré. Les 21 autres
tests `*_wkt1_esri*` de `tests/test_crs.rs` passent sans changement.

## Comportement conservé

- **Krovak** (EPSG:1041) prend la ligne « Krovak (North Orientated) » ; EPSG:9819
  prend la ligne « Krovak » avec `X_Scale 1.0`, `Y_Scale 1.0`, `XY_Plane_Rotation 0.0`.
- **LCC 2SP** (EPSG:9802) prend la première ligne, sans `Scale_Factor`. La ligne
  alt3, qui en a un, n'est jamais choisie. La ligne Michigan (EPSG:1051) a son
  propre code et garde le sien.
- **Surcharges de PROJ** : Polar Stereographic A (EPSG:9810) ne prend le nom
  `Polar_Stereographic_Variant_A` que pour les CRS UPS (EPSG:5041, 5042). Ailleurs,
  le nom devient `Stereographic`, avec les paramètres de Variant A. Behrmann
  (latitude 30°) garde les paramètres de `Cylindrical_Equal_Area`.
- **Stereographic North/South Pole** : le nom dépend du signe de la latitude
  standard, les paramètres sont ceux de la première ligne.
- **Angles** : écrits dans l'unité d'axe, et ramenés dans ±180° en degrés
  (azimut 295° devient `-65.0`).
- **Valeurs fixes** écrites telles quelles : `-1.0`, `90.0`, `0.0`, sans guillemets.
- **Paramètre absent** : `1.0` si le nom ESRI contient « scale », sinon `0.0`.
- **Mercator 1SP** (EPSG:9804) est exporté en Mercator 2SP (`Mercator`, avec
  `Standard_Parallel_1`) quand la latitude d'origine est nulle et `k0` dans (0, 1].
  Sinon il est exporté en `Mercator_Variant_A`.
- **Mercator sphérique** (EPSG:1026, sphère) : suit la lecture du C++ (écrit
  comme `Mercator_Variant_A`). Non vérifié contre la référence : aucun CRS de la
  base sphérique testé ne produit ce cas, et le parseur de chaînes PROJ ignore `+R`
  (voir plus bas).

## Écarts volontaires

- **Pseudo-Mercator** (EPSG:1024) s'exporte en `Mercator_Auxiliary_Sphere` (PROJ).
  Avant, le port écrivait `Mercator_1SP` (vérifié sur `EPSG:3857`).
- **Branches ad hoc supprimées** : Transverse Mercator (9807) et LCC 1SP (9801)
  ont été retirées de `wkt.rs`. La table donne les mêmes noms et les mêmes valeurs.
- **Mercator 9804 à latitude d'origine non nulle** : le port ne lève plus d'erreur
  en ESRI, comme PROJ (lecture du C++, pas de cas de test).
- **`is_fixed_value`** et **l'import** ne sont pas portés.

## Écarts résiduels avec `projinfo` 9.9.0

Comparaison sur 65 codes EPSG (`projinfo EPSG:n -o WKT1_ESRI`, ligne `PROJCS`
ou `GEOGCS` seulement) :

- 33 codes changent avec ce lot. 30 sont alors identiques à la référence.
- 0 régression : aucun code qui était identique à la référence ne l'est plus.
- 3 codes changent mais restent différents, pour des raisons hors table :
  EPSG:5221 (nom du datum, et le 16ᵉ chiffre sur `Azimuth` et `PRIMEM`),
  EPSG:5514 (le 16ᵉ chiffre seul : la référence CLI écrit 16 chiffres
  significatifs, les tests C++ attendent 15), EPSG:3416 (alias du nom GCS
  `GCS_ETRS89-AUT_2002`).
- Chaînes PROJ Mercator (`+lat_0`, `+lat_ts`, `+k`) : identiques à la référence.
- Hors lot, déjà présent sur la base : le parseur de chaînes PROJ ignore `+R=`
  (le CRS garde `SPHEROID["WGS_1984"]`). Un CRS sphérique écrit en chaîne PROJ
  sort donc avec un autre ellipsoïde que PROJ.
- 4 codes étaient déjà différents avant ce lot, et ne changent pas : EPSG:3408
  et 3409 (LAEA sphérique, méthode 1027 absente de la table), EPSG:2053 et 2963
  (méthodes absentes de la table). Le repli du port écrit alors des noms de
  paramètres en minuscules ou en WKT1 (`latitude_of_center`, `Latitude_Of_Origin`),
  là où PROJ écrit les noms EPSG (`Latitude of natural origin`). À traiter dans un
  lot séparé : c'est le repli générique, pas la table.

## Commandes lancées

- `cargo fmt --check` : propre.
- `cargo clippy --all-targets --all-features -- -D warnings` : propre.
- `cargo test --all-features --test test_crs -- wkt1_esri` : 34 passed, 0 failed.
- Suite complète, avec `PATH` sur la référence PROJ 9.9.0 et
  `PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES` :
  `cargo test --all-features --no-fail-fast`, code de sortie 0. `test_crs` :
  105 passed, 0 ignored (base : 92 passed, 13 ignored).
- `grep -rn '#\[ignore' tests/` : 13 avant, 0 après.
- Le harnais `cli_yaml` réécrit `docs/UNPORTED-CLI.md`. Le même changement
  (trois lignes retirées) apparaît sur `origin/main`, donc il ne vient pas de ce
  lot. Le fichier n'est pas commité.
- README : compteurs « Unit tests » mis à jour à partir de `docs/UNPORTED-UNIT.md`
  (portés 350 à 363, non portés 1 028 à 1 015 ; `test_crs.cpp` 90 à 103 et 107 à 94).
- `src/bin/bench.rs` : aucun cas ajouté. Ce lot touche l'export WKT, pas les
  opérations de coordonnées que le bench mesure.
