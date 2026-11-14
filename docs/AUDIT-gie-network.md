# Audit du lot gie-network

Référence : PROJ 9.9.0, `test/unit/gie_self_tests.cpp` (20 TEST/TEST_F),
`test/unit/test_network.cpp` (26 TEST), `test/cli/test_gie.yaml` (4 cas),
`test/cli/test_projsync.sh`, `src/apps/gie.cpp`, `src/apps/projsync.cpp`.

Légende : **porté** = assertions C++ reprises avec les valeurs de PROJ ;
**partiel** = une partie des assertions est portée, le reste est listé ;
**écart** = pas de test, cause exacte donnée.

## gie CLI (`test/cli/test_gie.yaml`)

| Cas C++ | Test Rust | Statut |
|---|---|---|
| Invalid argument (`--invalid`) | `tests/gie_cli.rs::invalid_argument_is_reported_on_stderr` | porté |
| Version (`--version`) | `tests/gie_cli.rs::version_argument_prints_the_release` | porté |
| Help (`-h`) | `tests/gie_cli.rs::help_argument_prints_the_usage` | porté |
| Fichier inexistant | `tests/gie_cli.rs::missing_input_file_is_reported_with_the_delimiter` | porté |

Hors yaml, options de `gie.cpp` : `tests/gie_cli.rs` couvre `argc == 1`, `-q`
sans fichier (retour 255), `-v` sans fichier, `-l`, `-o`, et le résumé d'un
fichier passant. Non portés : les compteurs `-vv` (roundtrips et échecs
séparés, la bibliothèque ne les distingue pas) et les erreurs de structure
`<gie>` (« Missing '</gie>' cmnd »).

Le ratchet `tests/gie_run.rs::vendored_gie_ratchet` passe toujours, avec le
runner déplacé dans `proj_rs::gie` (`src/gie.rs`).

## gie_self_tests.cpp (20 tests)

| Test C++ | Test Rust | Statut |
|---|---|---|
| `gie.cart_selftest` | `gie_self_tests.rs::cart_selftest` | partiel : UTM, cart 3D, pôles, `transform_array`. Les changements de contexte (`proj_context_set`, `P->ctx`) ne sont pas portés : un objet Rust ne porte pas de contexte. |
| `gie.info_functions` | — | écart : `proj_init_info` (métadonnées d'un fichier `+init`) n'a pas d'API ; `proj_grid_info` n'est pas exposé ; `proj_rtodms2`/`proj_dmstor` sont dans `src/bin/support/dms.rs`, non exposés à la bibliothèque. |
| `gie.proj_factors` | `gie_self_tests.rs::proj_factors` | partiel : Mercator et UTM portés (`Operation::factors`, `factors.cpp` + `pj_deriv`). Non portés : les blocs EPSG:3395, EPSG:2222, EPSG:3044, EPSG:27571, EPSG:5972, EPSG:4326 (CRS, `proj_factors` sur un objet CRS non implémenté) et le bloc `+units=ft` (la sortie n'est pas convertie en mètres). |
| `gie.list_functions` | `gie_self_tests.rs::list_functions` | porté (`proj_list_operations` : 190 ids de `proj -l`) |
| `gie.io_predicates` | `gie_self_tests.rs::io_predicates` | porté ; `P->inverted = 1` est traduit par la direction `inverse = true` de la même opération. |
| `gie.unitconvert_selftest` | `gie_self_tests.rs::unitconvert_selftest` | porté |
| `gie.unitconvert_selftest_date` | `gie_self_tests.rs::unitconvert_selftest_date` | porté |
| `gie.horner_selftest` | `gie_self_tests.rs::horner_selftest` | porté (`src/projections/models/horner.rs`, `horner.cpp`) |
| `gie.horner_only_fwd_selftest` | `gie_self_tests.rs::horner_only_fwd_selftest` | porté |
| `gie.proj_create_crs_to_crs_PULKOVO42_ETRS89` | `gie_self_tests.rs::proj_create_crs_to_crs_pulkovo42_etrs89` | partiel : valeurs et aller-retour portés. Non portés : le texte `proj_pj_info().definition` (la crate renvoie `EPSG:4179 -> EPSG:4258`, pas le pipeline PROJ), les noms de CRS source et cible (pas de `proj_get_name` sur un transform), et le texte « unavailable until proj_trans is called » (artefact de construction paresseuse, sans équivalent). |
| `gie.proj_create_crs_to_crs_WGS84_EGM08_to_WGS84` | — | écart : `EPSG:4326+3855 -> EPSG:4979` donne « Inverse of WGS 84 to EGM2008 height (1) » (grille `us_nga_egm08_25.tif` absente). PROJ donne « ... ballpark ». Cause : `Proj::new_known_crs` utilise `OnlyBest::Off` (façade, `src/iso19111/select.rs`), qui ne relance pas la recherche avec écartement des grilles absentes comme `proj_create_crs_to_crs` (`crs_to_crs.cpp:668-735`). La liste des trois candidats est identique à PROJ (`projinfo`). Correction non faite : elle touche les tests de parité de la façade. |
| `gie.proj_create_crs_to_crs_outside_area_of_use` | `gie_self_tests.rs::proj_create_crs_to_crs_outside_area_of_use` | porté |
| `gie.proj_create_crs_to_crs_with_area_large` | `gie_self_tests.rs::proj_create_crs_to_crs_with_area_large` | porté (l'aire est stockée sans influer sur la sélection ; les valeurs coïncident ici) |
| `gie.proj_create_crs_to_crs_with_longitude_outside_minus_180_180` | `gie_self_tests.rs::proj_create_crs_to_crs_with_longitude_outside_minus_180_180` | porté. L'axe normalisé est le seul de la crate : `P` et `Pnormalized` du C++ donnent le même appel. |
| `gie.proj_trans_generic` | `gie_self_tests.rs::proj_trans_generic` | porté via `transform_array` (les tableaux strides de `proj_trans_generic` ne sont pas une API Rust). |
| `gie.proj_trans_with_a_crs` | `gie_self_tests.rs::trans_with_a_crs_is_an_error` | partiel : l'erreur est vérifiée, pas le code `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`. |
| `gie.proj_create_crs_to_crs_from_pj_force_over` | — | écart : l'option `FORCE_OVER` et le drapeau `P->over` ne sont pas implémentés. |
| `gieTest.proj_create_crs_to_crs` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs` | partiel : valeurs portées, noms de CRS non (voir PULKOVO). |
| `gieTest.proj_create_crs_to_crs_EPSG_4326` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs_epsg_4326` | porté (entrée en longitude, latitude : ordre de la crate). |
| `gieTest.proj_create_crs_to_crs_proj_longlat` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs_proj_longlat` | porté |

Compte sur les 20 tests C++ : 17 ont un test Rust (12 complets, 5 partiels :
`cart_selftest`, `PULKOVO42`, `trans_with_a_crs`, `proj_create_crs_to_crs`,
`proj_factors`), et 3 sont des écarts (`info_functions`,
`proj_create_crs_to_crs_WGS84_EGM08`, `proj_create_crs_to_crs_from_pj_force_over`). `tests/gie_self_tests.rs`
contient 15 fonctions : les 14 ci-dessus et `nan_input_gives_no_coordinate`,
tirée de `io_predicates`.

Correction apportée au moteur, nécessaire au test `cart_selftest` : la porte de
latitude de `fwd_prepare` (`fwd.cpp`) n'était pas appliquée aux projections
natives (chemin `NativeProjected`, utm). Une latitude de 95° donnait
`OutsideDomain` au lieu de `InvalidCoord`. Voir `latitude_gate` dans
`src/engine.rs`.

## test_network.cpp (26 tests) : écart total

Aucun test de ce fichier n'est porté. Cause : la couche réseau
(`src/network.rs`) est `pub(crate)`. Les tests appellent
`proj_context_set_enable_network`, `proj_grid_cache_*` (set_enable, clear,
set_ttl, set_max_size, set_filename), `proj_context_set_network_callbacks`,
`proj_context_set_fileapi`, `proj_download_file`, `proj_is_download_needed`.
`Proj` expose `enable_network` et `grid_cache_enable` mais pas les callbacks
réseau ni l'API de fichiers (`ProjError::RemoteCallbacks` : « not built »).
Il faut d'abord une API publique et un serveur HTTP de test local
(`std::net::TcpListener`), non commencé.

| Test C++ | Statut |
|---|---|
| initial_check, basic, curl_invalid_resource, custom, getfilesize, simul_open_error, simul_read_range_error, simul_file_change_while_opened, curl_hgridshift, curl_vgridshift, curl_vgridshift_vertcon, network_endpoint_env_variable, network_endpoint_api_and_not_reachable_{gridshift,xyzgridshift,hgridshift,vgridshift}, cache_basic, proj_grid_cache_clear, cache_saturation, cache_ttl, cache_lock, download_whole_files, file_api, proj_coordoperation_get_grid_used, pyproj_issue_1192, do_not_attempt_network_access_known_available_network_on | écart (cause ci-dessus) |

## projsync (`src/apps/projsync.cpp`, `test/cli/test_projsync.sh`) : écart total

Non commencé. `projsync.cpp` lit un index JSON du CDN : il faut un
analyseur JSON (pas de nouvelle dépendance sans justification) et le
téléchargement via le même mini serveur HTTP que le lot réseau. Le binaire
`projsync` n'existe pas dans `src/bin`.

## Commandes

```
cargo test --offline --test gie_cli --test gie_run --test gie_self_tests
```
