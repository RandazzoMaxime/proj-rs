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
| `gie.cart_selftest` | `gie_self_tests.rs::cart_selftest` | porté, sauf 3 assertions sans équivalent observable : `ASSERT_NE(ctx, pj_get_default_ctx())` (identité de pointeurs entre contextes) et les deux `ASSERT_EQ` sur le tampon `z` de `obs[]` (45 et 50, tampon à `stride` nul, absent de l'API point par point). Le changement de contexte (`proj_context_set`, `P->ctx`) est porté par `Operation::context` / `Operation::set_context`. Détail dans `docs/PORTING-gie-network.md`. |
| `gie.info_functions` | `tests/gie_info_functions.rs::info_functions` (et `init_info_reads_the_metadata_section_of_a_file`) | porté, toutes les assertions : `proj_info` (`proj_rs::info::library_info`), `proj_pj_info` (`Proj::proj_info`), `proj_grid_info` (`info::grid_info`), `proj_init_info` (`info::init_info`), `proj_rtodms2` et `proj_dmstor` (`proj_rs::dms`). Écarts de forme seulement : `release` vaut « 9.9.0 » ici (« Rel. 9.9.0 (…) » dans PROJ), le test vérifie qu'il est non vide comme le C ; le bloc TIFF tourne ici, la référence de build est sans TIFF, et ses valeurs sont les littéraux du C. |
| `gie.proj_factors` | `gie_self_tests.rs::proj_factors` | partiel : Mercator et UTM portés (`Operation::factors`, `factors.cpp` + `pj_deriv`). Non portés : les blocs EPSG:3395, EPSG:2222, EPSG:3044, EPSG:27571, EPSG:5972, EPSG:4326 (CRS, `proj_factors` sur un objet CRS non implémenté) et le bloc `+units=ft` (la sortie n'est pas convertie en mètres). |
| `gie.list_functions` | `gie_self_tests.rs::list_functions` | porté (`proj_list_operations` : 190 ids de `proj -l`) |
| `gie.io_predicates` | `gie_self_tests.rs::io_predicates` | porté ; `P->inverted = 1` est traduit par la direction `inverse = true` de la même opération. |
| `gie.unitconvert_selftest` | `gie_self_tests.rs::unitconvert_selftest` | porté |
| `gie.unitconvert_selftest_date` | `gie_self_tests.rs::unitconvert_selftest_date` | porté |
| `gie.horner_selftest` | `gie_self_tests.rs::horner_selftest` | porté (`src/projections/models/horner.rs`, `horner.cpp`) |
| `gie.horner_only_fwd_selftest` | `gie_self_tests.rs::horner_only_fwd_selftest` | porté |
| `gie.proj_create_crs_to_crs_PULKOVO42_ETRS89` | `gie_self_tests.rs::proj_create_crs_to_crs_pulkovo42_etrs89` | porté, toutes les 12 assertions. `proj_pj_info().definition` (« unavailable until proj_trans is called », puis les deux pipelines) : `Proj::definition_in_authority_order()`, dans l'ordre de `proj_create_crs_to_crs` avec les `axisswap` de bout de pipeline ; `Proj::proj_info().definition` garde le texte normalisé de la crate. `proj_get_name` nul : `Proj::name()`. Noms de CRS : `Proj::source_crs()` et `target_crs()`. `P->fwd` nul avant un point : `last_used_operation()` vide. `proj_trans_generic` : `convert_array`. Détail dans `docs/PORTING-gie-network.md`. |
| `gie.proj_create_crs_to_crs_WGS84_EGM08_to_WGS84` | — | écart : `EPSG:4326+3855 -> EPSG:4979` donne « Inverse of WGS 84 to EGM2008 height (1) » (grille `us_nga_egm08_25.tif` absente). PROJ donne « ... ballpark ». Cause : `Proj::new_known_crs` utilise `OnlyBest::Off` (façade, `src/iso19111/select.rs`), qui ne relance pas la recherche avec écartement des grilles absentes comme `proj_create_crs_to_crs` (`crs_to_crs.cpp:668-735`). La liste des trois candidats est identique à PROJ (`projinfo`). Correction non faite : elle touche les tests de parité de la façade. |
| `gie.proj_create_crs_to_crs_outside_area_of_use` | `gie_self_tests.rs::proj_create_crs_to_crs_outside_area_of_use` | porté |
| `gie.proj_create_crs_to_crs_with_area_large` | `gie_self_tests.rs::proj_create_crs_to_crs_with_area_large` | porté (l'aire est stockée sans influer sur la sélection ; les valeurs coïncident ici) |
| `gie.proj_create_crs_to_crs_with_longitude_outside_minus_180_180` | `gie_self_tests.rs::proj_create_crs_to_crs_with_longitude_outside_minus_180_180` | porté. L'axe normalisé est le seul de la crate : `P` et `Pnormalized` du C++ donnent le même appel. |
| `gie.proj_trans_generic` | `gie_self_tests.rs::proj_trans_generic` | porté via `transform_array` (les tableaux strides de `proj_trans_generic` ne sont pas une API Rust). |
| `gie.proj_trans_with_a_crs` | `gie_self_tests.rs::trans_with_a_crs_is_an_error` | porté : `HUGE_VAL` est `Err(ProjError::NotATransform)`, dont `code()` vaut `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE` (`src/errno.rs`). |
| `gie.proj_create_crs_to_crs_from_pj_force_over` | `gie_self_tests.rs::proj_create_crs_to_crs_from_pj_force_over` | porté, les 46 `ASSERT_`/`EXPECT_` du C++ (la boucle sur les alternatives compte pour une). `FORCE_OVER=YES` donne le drapeau `P->over` (`ForceOver::Yes`, lu par `force_over()`), que reçoivent chaque projection du pipeline et chaque candidat (`Operation::create_over`). Écarts de forme : `Pnormalized` est `P` (la façade est déjà en longitude, latitude) ; `t = HUGE_VAL` n'est pas passé ; le contexte et `proj_destroy` sont des `Drop`. Détail dans `docs/PORTING-gie-network.md`. |
| `gieTest.proj_create_crs_to_crs` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs` | porté, les 8 assertions. Le nom du CRS source vient de `Proj::source_crs()` et `Crs::name()`, lu après le point ; le C n'a pas d'assertion de nom cible. Il a fallu poser les CRS d'une concaténation dérivée (`src/iso19111/derived.rs`). Détail dans `docs/PORTING-gie-network.md`. |
| `gieTest.proj_create_crs_to_crs_EPSG_4326` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs_epsg_4326` | porté (entrée en longitude, latitude : ordre de la crate). |
| `gieTest.proj_create_crs_to_crs_proj_longlat` (TEST_F) | `gie_self_tests.rs::proj_create_crs_to_crs_proj_longlat` | porté |

Compte sur les 20 tests C++ : 19 ont un test Rust (18 complets, dont `cart_selftest`
avec 3 assertions sans équivalent ; 1 partiel : `proj_factors`), et 1 est un écart
(`proj_create_crs_to_crs_WGS84_EGM08`). `tests/gie_self_tests.rs`
contient 19 fonctions : les 18 tests C++ de ce fichier et `nan_input_gives_no_coordinate`,
tirée de `io_predicates`. `tests/gie_info_functions.rs` porte `info_functions`
et une fonction de contrôle sur un fichier `+init` (`tests/gie/test_init_metadata`).

Correction de comportement liée à ce test : `Proj::def` et `Proj::proj_info`
donnent maintenant les paramètres d'une projection qui n'est pas une CRS
(`+proj=utm; +zone=32; +ellps=GRS80` donne `proj=utm zone=32 ellps=GRS80`,
comme `proj_pj_info`). Avant, ils renvoyaient `None` / une erreur. Voir
`operation_definition` dans `src/proj_info.rs`.

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
cargo test --offline --test gie_cli --test gie_run --test gie_self_tests --test gie_info_functions
```
