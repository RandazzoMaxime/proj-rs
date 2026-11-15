# Fiche de portage : lot capi-operation-factory-core

Source : PROJ 9.9.0, `test/unit/test_c_api.cpp` (28 cas `CApi.*`),
`test/unit/test_operation.cpp` (5 cas `operation.*`), `test/unit/test_factory.cpp`
(3 cas `factory.*`). Chaque fonction est un `TEST`, le nom C++ figure dans sa doc
(Rust : `tests/unit_capi_*.rs`, `tests/unit_op_*.rs`, `tests/unit_factory_*.rs`).

Bilan au 2026-11-15 : 35 cas portés, 1 partiel, 0 absent (36 au total).
Partiel : `operation.normalizeForVisualization` (bloc composé non porté, voir plus bas).

## Constat du 2026-11-15

Cette fiche donnait 31 cas ouverts. Au commit de départ (`8cfd059`), la plupart avaient
déjà un test Rust vert. Les statuts « ouvert » étaient restés d'un état antérieur. Les
trois lignes suivantes restaient vraiment ouvertes et ont été corrigées dans ce lot :

- `CApi.proj_coordoperation_get_grid_used` : l'URL de la grille n'était pas réécrite
  avec l'endpoint du contexte. Corrigé : `CoordinateOperation::grids_needed_at`.
- `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` et `_ballpark_filter` : les
  options `ACCURACY=` et `ALLOW_BALLPARK=` étaient refusées au lieu d'être appliquées.
  Corrigé : `Proj::create_crs_to_crs_from_pj` les lit et filtre les candidats.

Le test `tests/proj_unit.rs` assertait un refus de `ACCURACY=0.05` sur EPSG:4326 vers
EPSG:32631. PROJ garde cette opération (précision 0 m, `projinfo` 9.9.0). L'assertion
est passée à `is_ok()`.

## Inventaire : `test_c_api.cpp`

| `TEST` PROJ 9.9.0 | Test Rust | Statut | Cause ou écart |
|---|---|---|---|
| `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` | `tests/unit_capi_operation_factory_core.rs` | porté | Fixture remplacée par EPSG:32631. |
| `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name` | `tests/unit_capi_operation_factory_core.rs` | porté | Aucun. |
| `CApi.proj_create_operations` | `tests/unit_capi_operation_factory_core.rs` | porté | `EXPECT_GE(idx, -1)` devient `is_ok()` : pas de code -1 en Rust. |
| `CApi.proj_create_operations_allow_ballpark_transformations` | `tests/unit_capi_operation_factory_core.rs` | porté | Aucun. |
| `CApi.proj_trans_bounds_to_compound_crs` | `tests/unit_capi_operation_factory_core.rs` | porté | Boîte en longitude d'abord. |
| `CApi.proj_get_suggested_operation_for_NAD83_to_NAD83_HARN` | `tests/unit_capi_suggested_nad83_harn.rs` | porté | « NAD83 to NAD83(HARN) (47) », comme PROJ. |
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | `tests/unit_capi_operations_suggest.rs` | porté | Compte de grilles par `grids_needed` (1). |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | `tests/unit_capi_operations_suggest.rs` | porté | `EXPECT_GE(idx, 0)` devient `is_some()`. |
| `CApi.proj_create_operations_discard_superseded` | `tests/unit_capi_discard_superseded.rs` | porté | 4 opérations. |
| `CApi.proj_create_operations_dont_discard_superseded` | `tests/unit_capi_discard_superseded.rs` | porté | 5 opérations. |
| `CApi.proj_create_operations_with_pivot` | `tests/unit_capi_pivot_always.rs` | porté | Quatre contextes. |
| `CApi.proj_create_vertical_crs_ex` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Voir `docs/PORTING-capi-vertical-crs-ex.md`. |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Aller-retour PROJJSON inclus. |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Précision 0,15 à 1e-10 près. |
| `CApi.proj_coordoperation_get_grid_used` | `tests/unit_capi_grid_used.rs` (2 tests) | porté | Endpoint `https://example.com` appliqué à l'URL. L'index 1 hors liste renvoie `None` ; l'index -1 n'existe pas en Rust. |
| `CApi.proj_coordoperation_get_grid_used_fullname_caching` | `tests/unit_capi_grid_used_caching.rs` | porté | Le `TEST` C++ est sous `TIFF_ENABLED` ; le port tourne sans condition. |
| `CApi.proj_coordoperation_is_instantiable` | `tests/unit_capi_coordop_instantiable.rs` | porté | `is_proj_instantiable` avec grilles connues. |
| `CApi.proj_normalize_for_visualization_with_alternatives` | `tests/unit_capi_normalize_alternatives.rs` | porté | Pipeline et coordonnées à 1e-8 près. |
| `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | `tests/unit_capi_normalize_alternatives.rs` | porté | `degree_input` / `degree_output`. |
| `CApi.proj_coordoperation_create_inverse` | `tests/unit_capi_coordop_create_inverse.rs` | porté | `ProjStringLayout::Multi` : indentation et largeur réglables. |
| `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` | `tests/unit_capi_crs_to_crs_filters.rs` | porté | `ACCURACY=100` garde le couple, `ACCURACY=0.05` le refuse. |
| `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter` | `tests/unit_capi_crs_to_crs_filters.rs` | porté | `ALLOW_BALLPARK=YES` garde, `NO` refuse. |
| `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations` | `tests/unit_capi_crs_to_crs_filters.rs` | porté | Par `Proj::new_known_crs`, pas par `proj_create_crs_to_crs`. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762` | `tests/unit_capi_custom_compound_geoid.rs` | porté | Le composé est passé en WKT2, pas en objet. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762` | `tests/unit_capi_custom_compound_geoid.rs` | porté | Idem. Époque 2010 en entrée. |
| `CApi.proj_trans_bounds_ignore_inf` | `tests/unit_capi_trans_bounds_esri.rs` | porté | ESRI:102036. |
| `CApi.proj_trans_bounds_ignore_inf_geographic` | `tests/unit_capi_trans_bounds_esri.rs` | porté | |
| `CApi.proj_trans_bounds_3D_ignore_inf` | `tests/unit_capi_trans_bounds_esri.rs` | porté | |

Le `TEST` `CApi.proj_concatoperation_get_step`, qui appelle aussi `proj_create_operations`,
est porté dans `tests/unit_shape.rs` (lot `shape`, `docs/PORTING-shape.md`). Il n'est pas
compté ici.

## Inventaire : `test_operation.cpp` et `test_factory.cpp`

| `TEST` PROJ 9.9.0 | Test Rust | Statut | Cause ou écart |
|---|---|---|---|
| `operation.normalizeForVisualization` | `tests/unit_op_normalize_for_visualization.rs` | partiel | Le bloc composé (`CompoundCRS`, `BoundCRS::createFromTOWGS84`, aller-retour WKT2 par opération) n'est pas porté. Le port couvre les autres blocs. |
| `operation.PointMotionOperation_with_epochs` | `tests/unit_io_point_motion.rs` | porté | |
| `operation.operation_Position_Vector_geocen_and_geocen_translations_NEU_velocities_gtg_with_source_epoch` | `tests/unit_op_geocen_gtg.rs` | porté | |
| `operation.operation_geocen_translations_by_grid_and_geocen_translations_NEU_velocities_gtg` | `tests/unit_op_geocen_gtg.rs` | porté | |
| `operation.operation_geocen_translations_NEU_velocities_gtg` | `tests/unit_op_geocen_gtg.rs` | porté | |
| `factory.AuthorityFactory_createDynamicVerticalDatum` | `tests/unit_factory_dynamic_vdatum.rs` | porté | Le `TEST` est sous `#ifdef no_more_dynamic_vertical_datum`, donc non compilé en PROJ 9.9.0 ; le port fixe le comportement de la base 9.9. |
| `factory.AuthorityFactory_build_all_concatenated` | `tests/unit_factory_build_all_concatenated.rs` | porté | Les neuf codes échouent. Le message « Inconsistent chaining » n'est pas comparé. |
| `factory.ogc_timecrs` | `tests/unit_factory_ogc_timecrs.rs` | porté | |

## Écarts volontaires

- Pas de handles C. `proj_list_get(list, -1)` n'a pas d'équivalent : le test
  asserte seulement `list.get(len)` (`None`).
- `CApi.proj_create_operations` : `EXPECT_GE(idx, -1)` de PROJ devient
  `suggested_operation(...).is_ok()`. Il n'y a pas de code `-1` en Rust.
- `CApi.proj_coordoperation_get_grid_used` : l'endpoint du contexte C est un argument
  de `CoordinateOperation::grids_needed_at`. Il n'existe pas de contexte à lire.
- `Proj::create_crs_to_crs_from_pj` : `ACCURACY` non numérique est une erreur (PROJ lit 0
  par `pj_atof`). `AUTHORITY` et `ONLY_BEST` restent refusés.
- `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` : le fixture
  `createProjectedCRS()` de PROJ est remplacé par EPSG:32631 (projected CRS de la base).

## Modifications de `src/`

Lot initial (commits antérieurs) :

- `src/iso19111/suggest.rs` (nouveau) : `pj_create_prepared_operations` et
  `pj_get_suggested_operation`. Source géographique uniquement, degrés supposés.
- `src/iso19111/factory.rs` : `CoordinateOperationFactory::suggested_operation`.
- `src/proj/bounds.rs` : `is_noop` d'un pipeline ISO vaut quand le pipeline est `noop`.

Lot du 2026-11-15 :

- `src/iso19111/select.rs` : `facade_candidates` prend un `CandidateFilter` et refuse
  une liste vidée par le filtre (message `every operation ... is filtered out`, pas
  « no operation from », pour ne pas tomber sur le noop de même datum). Ajout de
  `CoordinateOperation::grids_needed_at` et de `endpoint_url`.
- `src/proj_iso.rs` : `iso_pipeline_at` prend le `CandidateFilter`.
- `src/proj.rs` : `crs_to_crs_options` lit `FORCE_OVER`, `ACCURACY`, `ALLOW_BALLPARK`.
  `Proj::grids_needed` passe l'endpoint du `Proj` à la résolution des URL.

## Cas ouverts

Dans `test_c_api.cpp`, aucun cas n'est ouvert. Dans `test_operation.cpp`, un cas est
partiel : `operation.normalizeForVisualization`. Son bloc composé demande
`CompoundCRS` et `BoundCRS::createFromTOWGS84`, et l'aller-retour WKT2 par opération.
Ce lot ne le porte pas.

## Commandes lancées et résultats

Lot initial (2026-11-15, 1re session) :

```
cargo test --test test_c_api -- --test-threads=1                  # test result: ok. 52 passed; 0 failed; 0 ignored
cargo test --test unit_capi_operation_factory_core                # test result: ok. 5 passed; 0 failed; 0 ignored
cargo test --test unit_capi_suggested_nad83_harn                  # test result: ok. 1 passed; 0 failed; 0 ignored
cargo test --test unit_capi_operations_suggest                    # test result: ok. 2 passed; 0 failed; 0 ignored
cargo test --test unit_capi_discard_superseded                    # test result: ok. 2 passed; 0 failed; 0 ignored
cargo test --test unit_capi_pivot_always                          # test result: ok. 1 passed; 0 failed; 0 ignored
cargo test --test unit_capi_vertical_crs_ex                       # test result: ok. 3 passed; 0 failed; 0 ignored
cargo test --test cli_yaml -- --test-threads=1                    # test result: ok. 3 passed; 0 failed; 0 ignored
cargo test --test unit_network_core -- --test-threads=1           # test result: ok. 14 passed; 0 failed; 0 ignored
cargo test --test test_operationfactory -- --test-threads=1       # test result: ok. 9 passed; 0 failed; 0 ignored
cargo test --test docs_in_sync -- --test-threads=1                # test result: ok. 2 passed; 0 failed; 0 ignored
$R/build/bin/projinfo -s EPSG:4267 -t EPSG:4269 -o PROJ --spatial-test intersects   # Candidate operations found: 10 ; 1re : NAD27 to NAD83 (4)
$R/build/bin/projinfo -s EPSG:4230 -t EPSG:4171 --summary        # Candidate operations found: 1 ; ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1]
$R/build/bin/projinfo -s EPSG:4203 -t EPSG:4326 --spatial-test intersects   # Candidate operations found: 4
$R/build/bin/projinfo -s EPSG:4326 -t EPSG:32631                  # EPSG:16031, UTM zone 31N, 0 m
```

2e session du 2026-11-15 (filtres `ACCURACY` / `ALLOW_BALLPARK`, endpoint de grille) :

```
avant (tests écrits d'abord, implémentation absente) :
  unit_capi_crs_to_crs_filters   test result: FAILED. 1 passed; 2 failed   (accuracy, ballpark : assertions positives)
après :
  unit_capi_crs_to_crs_filters   test result: ok. 3 passed; 0 failed; 0 ignored
  proj_unit                      test result: ok. 35 passed; 0 failed; 0 ignored   (avant : 34 passed, 1 failed, assertion de refus)
  unit_capi_grid_used            test result: ok. 2 passed; 0 failed; 0 ignored   (avant : 1 passed, sans endpoint)
  test_c_api                     test result: ok. 52 passed; 0 failed; 0 ignored
  cli_yaml                       test result: ok. 3 passed; 0 failed; 0 ignored
  test_operationfactory          test result: ok. 9 passed; 0 failed; 0 ignored
  parity (710 cas, petit corpus) test result: ok. 1 passed; 0 failed; 1 ignored (wide)
  + 33 autres binaires liés aux options CRS-to-CRS, au réseau et aux facades : tous `ok`, 0 failed
cargo +stable fmt --check                                          # code 0 après `cargo +stable fmt`
cargo +stable clippy --all-targets --all-features -- -D warnings   # code 0
```

Pour `grid_used`, l'implémentation a été écrite avant le test : l'ordre TDD n'a pas été
respecté sur ce point. Le test a été ajouté ensuite, et il passe.
