# Fiche de portage : lot capi-operation-factory-core

Source : PROJ 9.9.0, `test/unit/test_c_api.cpp` (28 cas `CApi.*`),
`test/unit/test_operation.cpp` (5 cas `operation.*`), `test/unit/test_factory.cpp`
(3 cas `factory.*`). Tests : `tests/unit_capi_operation_factory_core.rs`. Chaque
fonction est un `TEST`, le nom C++ figure dans sa doc.

Bilan : 14 cas portés, 22 ouverts (36 au total).

Constat de ce lot : les douze `TEST` de PROJ 9.9.0 qui appellent la façade des
opérations (`proj_create_operations`, `proj_operation_factory_context_*`,
`proj_get_suggested_operation`) étaient déjà portés et verts au commit de départ
(`8cfd059`). Les statuts « ouvert » de cette fiche et de `docs/PORTING-io-capi-core-2.md`
étaient en retard. Ce lot ne change donc pas `src/` : il ferme les neuf cas
ci-dessous dans la fiche, corrige trois en-têtes de test (`test_c_api_990.cpp`
n'existe pas dans PROJ 9.9.0 ; le `TEST` est dans `test_c_api.cpp`) et mesure.

## Inventaire des `TEST` de la façade des opérations

Source : `src/test/unit/test_c_api.cpp` de PROJ 9.9.0. Les 12 `TEST` qui appellent
`proj_create_operations` ou un réglage de `proj_operation_factory_context_*` :

| `TEST` PROJ 9.9.0 | Test Rust | Statut | Cause ou écart |
|---|---|---|---|
| `CApi.proj_create_operations` | `tests/unit_capi_operation_factory_core.rs` : `capi_proj_create_operations` | porté | `EXPECT_GE(idx, -1)` devient `is_ok()` : pas de code -1 en Rust. |
| `CApi.proj_get_suggested_operation_for_NAD83_to_NAD83_HARN` | `tests/unit_capi_suggested_nad83_harn.rs` | porté | Suggestion « NAD83 to NAD83(HARN) (47) », comme PROJ. |
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | `tests/unit_capi_operations_suggest.rs` | porté | Compte de grilles par `grids_needed` (1), à la place de `proj_coordoperation_get_grid_used_count`. |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | `tests/unit_capi_operations_suggest.rs` | porté | `suggested_operation_xyz` ; `EXPECT_GE(idx, 0)` devient `is_some()`. |
| `CApi.proj_create_operations_discard_superseded` | `tests/unit_capi_discard_superseded.rs` | porté | `SupersededOperations::Discard`, 4 opérations. |
| `CApi.proj_create_operations_dont_discard_superseded` | `tests/unit_capi_discard_superseded.rs` | porté | `SupersededOperations::Keep`, 5 opérations. |
| `CApi.proj_create_operations_with_pivot` | `tests/unit_capi_pivot_always.rs` | porté | Les quatre contextes du `TEST` : pivot automatique, `Never`, pivot ETRS89 imposé, pivot NAD27 avec `Always`. |
| `CApi.proj_create_operations_allow_ballpark_transformations` | `tests/unit_capi_operation_factory_core.rs` : `capi_proj_create_operations_allow_ballpark_transformations` | porté | Aucun. |
| `CApi.proj_create_vertical_crs_ex` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Voir `docs/PORTING-capi-vertical-crs-ex.md`. |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Aller-retour PROJJSON inclus. |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | `tests/unit_capi_vertical_crs_ex.rs` | porté | Précision 0,15 à 1e-10 près. |
| `CApi.proj_concatoperation_get_step` | `tests/unit_shape.rs` : `c_api_proj_concatoperation_get_step` | porté (lot `shape`, `docs/PORTING-shape.md`) | Sans `PROJ_GRID_AVAILABILITY_IGNORED` : la fabrique ne vérifie pas les grilles. |

Aucun des douze `TEST` n'est absent. Aucun n'est partiel au sens des assertions :
les comptes, les noms et les suggestions de PROJ sont tous vérifiés. Les écarts
restants sont des choix d'API (pas de handle C, pas de code -1), listés ci-dessus.

Le registre `docs/UNPORTED-UNIT.md` ne porte aucune ligne `test_c_api.cpp` : il n'y
a rien à retirer, et `docs_in_sync` reste vert.

## Cas portés

| PROJ 9.9.0 | Rust |
|---|---|
| `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` | `capi_proj_crs_create_bound_crs_to_wgs84_on_invalid_type` |
| `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name` | `capi_proj_create_from_database_grid_alternative_null_old_proj_grid_name` |
| `CApi.proj_create_operations` | `capi_proj_create_operations` |
| `CApi.proj_create_operations_allow_ballpark_transformations` | `capi_proj_create_operations_allow_ballpark_transformations` |
| `CApi.proj_trans_bounds_to_compound_crs` | `capi_proj_trans_bounds_to_compound_crs` |
| `CApi.proj_get_suggested_operation_for_NAD83_to_NAD83_HARN` | `capi_proj_get_suggested_operation_for_nad83_to_nad83_harn` |
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | `capi_proj_create_operations_prime_meridian_non_greenwich` |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | `capi_proj_get_suggested_operation_with_operations_without_area_of_use` |
| `CApi.proj_create_operations_discard_superseded` | `capi_proj_create_operations_discard_superseded` |
| `CApi.proj_create_operations_dont_discard_superseded` | `capi_proj_create_operations_dont_discard_superseded` |
| `CApi.proj_create_operations_with_pivot` | `capi_proj_create_operations_with_pivot` |
| `CApi.proj_create_vertical_crs_ex` | `capi_proj_create_vertical_crs_ex` |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | `capi_proj_create_vertical_crs_ex_with_geog_crs` |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | `capi_proj_create_vertical_crs_ex_implied_accuracy` |

## Cas ouverts

Chaque cas ouvert a une raison d'une ligne. Aucun n'est affaibli dans le
fichier de tests : un cas non porté n'a pas de fonction.

### CApi

| PROJ 9.9.0 | Raison |
|---|---|
| `CApi.proj_coordoperation_get_grid_used` | Pas de liste des grilles utilisées par une opération : `DatabaseContext::grid_info` ne renvoie que l'URL d'une grille par nom, sans réécrire l'URL avec l'endpoint du contexte. |
| `CApi.proj_coordoperation_get_grid_used_fullname_caching` | Même absence de liste des grilles utilisées ; le cas est aussi sous `TIFF_ENABLED`. |
| `CApi.proj_coordoperation_is_instantiable` | Pas d'API publique d'instanciabilité d'une opération. |
| `CApi.proj_normalize_for_visualization_with_alternatives` | Les coordonnées normalisées sont reproduites (écart < 1e-8), mais le texte du pipeline attendu n'est pas exposé : `Proj::def` renvoie « unavailable until proj_trans is called ». |
| `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | `Proj` n'expose pas `proj_degree_input` / `proj_degree_output`, que le test asserte. |
| `CApi.proj_coordoperation_create_inverse` | `proj_as_proj_string` avec `MULTILINE`, `INDENTATION_WIDTH=4`, `MAX_LINE_LENGTH=40` : `ProjLines::Multi` a une largeur fixe de 80 colonnes et une indentation de 2. |
| `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` | `Proj::create_crs_to_crs_from_pj` rejette toute option non vide (`ACCURACY=0.05`) au lieu de filtrer par précision. |
| `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter` | Même rejet des options (`ALLOW_BALLPARK=NO`). |
| `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations` | EPSG:25831+5773 n'est pas construit par `Proj::new_known_crs` (constat de `docs/PORTING-capi-factory-core.md`, non re-sondé dans ce lot). |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762` | Le CRS composé avec vertical lié (`proj_crs_create_bound_vertical_crs`, `egm96_15.gtx`) n'est pas un objet que `Proj` accepte ; la chaîne ballpark n'est pas portée. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762` | Même raison. |
| `CApi.proj_trans_bounds_ignore_inf` | `ESRI:102036` (gnomonique ellipsoïdale, lat_0 = −90) ne donne aucun point fini : `gnom_e_forward` renvoie « hors domaine » pour (180, −89), alors que les attendus C supposent des points finis dans la boîte. Défaut du port gnomonique, hors lot. |
| `CApi.proj_trans_bounds_ignore_inf_geographic` | Le `PROJCS[...]` WKT1 en ligne n'est pas lu par `Proj::new_known_crs` (« unknown CRS »). |
| `CApi.proj_trans_bounds_3D_ignore_inf` | Même cause que `proj_trans_bounds_ignore_inf`. |

### operation

| PROJ 9.9.0 | Raison |
|---|---|
| `operation.normalizeForVisualization` | Non porté dans ce lot (test lu en partie) : combine `CompoundCRS`, `BoundCRS::createFromTOWGS84` et un aller-retour WKT2 par opération. |
| `operation.PointMotionOperation_with_epochs` | Non porté dans ce lot. |
| `operation.operation_Position_Vector_geocen_and_geocen_translations_NEU_velocities_gtg_with_source_epoch` | Non porté dans ce lot. |
| `operation.operation_geocen_translations_by_grid_and_geocen_translations_NEU_velocities_gtg` | Non porté dans ce lot. |
| `operation.operation_geocen_translations_NEU_velocities_gtg` | Non porté dans ce lot. |

### factory

| PROJ 9.9.0 | Raison |
|---|---|
| `factory.AuthorityFactory_createDynamicVerticalDatum` | Le test est sous `#ifdef no_more_dynamic_vertical_datum` (`test_factory.cpp`, l. 320-330) : non compilé dans PROJ 9.9.0 ; la base 9.9 n'a pas de `frame_reference_epoch` pour EPSG:1096 (voir `docs/PORTING-factory-unit.md`). |
| `factory.AuthorityFactory_build_all_concatenated` | Neuf codes doivent échouer avec « Inconsistent chaining of CRS in operations » ; le crate n'a pas de contrôle de chaînage (voir `docs/PORTING-capi-factory-core.md`). |
| `factory.ogc_timecrs` | `AuthorityFactory` (autorité OGC) ne lit pas `AnsiDate`, `JulianDate`, `UnixTime` : erreur `NotFound` (vérifié). |

## Écarts volontaires

- Pas d'handles C. `proj_list_get(list, -1)` n'a pas d'équivalent : le test
  asserte seulement `list.get(len)` (`None`).
- `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` : le fixture
  `createProjectedCRS()` de PROJ est remplacé par EPSG:32631 (projected CRS de la
  base). L'assertion, un objet qui n'est pas un CRS, est portée telle quelle.
- `CApi.proj_trans_bounds_to_compound_crs` : `Proj::transform_bounds_in` prend la
  boîte en longitude d'abord. La boîte C `(40, -120, 64, -80)` en latitude d'abord
  est donc `(-120, 40, -80, 64)`. Le chemin no-op renvoie la boîte inchangée.
- `CApi.proj_create_operations` : `EXPECT_GE(idx, -1)` de PROJ devient
  `suggested_operation(...).is_ok()`. Il n'y a pas de code `-1` en Rust.

## Modifications de `src/`

Mise à jour du 2026-11-15 : aucune modification de `src/`. Ce qui suit date du lot initial.

- `src/iso19111/suggest.rs` (nouveau) : `pj_create_prepared_operations`
  (boîte d'aire d'utilisation, découpe à l'antiméridien, pseudo-aire, précision,
  drapeaux offshore, inconnu et prioritaire) et `pj_get_suggested_operation`
  (premier candidat qui couvre le point, puis précision plus basse, puis pseudo-aire
  plus petite). Source géographique uniquement, degrés supposés.
- `src/iso19111/factory.rs` : `CoordinateOperationFactory::suggested_operation`.
- `src/iso19111/mod.rs` : `mod suggest;`.
- `src/proj/bounds.rs` : `is_noop` d'un pipeline ISO vaut quand le pipeline est
  `noop` (PROJ : `pj_info.id == "noop"`), au lieu de comparer les noms de CRS.
  Effet : `proj_trans_bounds` sur un no-op renvoie la boîte sans contrôle de
  densification, comme PROJ. C'est un changement de comportement ; la suite complète
  passe.

## Commandes lancées et résultats

```
cargo fmt --check                       # code 0, sortie vide
cargo clippy --all-targets --all-features -- -D warnings   # code 0
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
  # 1346 passed, 0 failed, 26 ignored (ignorés préexistants, dans d'autres fichiers)
cargo test --test unit_capi_operation_factory_core   # 5 passed
```

Mise à jour du 2026-11-15 (sans modification de `src/`) :

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
cargo +stable fmt --check (3 fichiers de test modifiés)           # code 0
$R/build/bin/projinfo -s EPSG:4267 -t EPSG:4269 -o PROJ --spatial-test intersects   # Candidate operations found: 10 ; 1re : NAD27 to NAD83 (4)
$R/build/bin/projinfo -s EPSG:4230 -t EPSG:4171 --summary        # Candidate operations found: 1 ; ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1]
$R/build/bin/projinfo -s EPSG:4203 -t EPSG:4326 --spatial-test intersects   # Candidate operations found: 4
```

Le petit corpus de parité (`tests/parity.rs`) n'existe pas dans cette branche :
il vient d'une autre branche. Il n'a donc pas été lancé dans ce lot.

Bench : `CONTRIBUTING.md` demande d'ajouter les cas du lot à `src/bin/bench.rs`
et une ligne à `bench/history.jsonl`. Ce bench mesure le débit par point des
projections (ns par point). Il n'a pas été étendu à ces cas de fabrique et
d'opérations : non fait dans ce lot.
