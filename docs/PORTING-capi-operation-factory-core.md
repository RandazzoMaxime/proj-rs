# Fiche de portage : lot capi-operation-factory-core

Source : PROJ 9.9.0, `test/unit/test_c_api.cpp` (28 cas `CApi.*`),
`test/unit/test_operation.cpp` (5 cas `operation.*`), `test/unit/test_factory.cpp`
(3 cas `factory.*`). Tests : `tests/unit_capi_operation_factory_core.rs`. Chaque
fonction est un `TEST`, le nom C++ figure dans sa doc.

Bilan : 5 cas portés, 31 ouverts (36 au total).

## Cas portés

| PROJ 9.9.0 | Rust |
|---|---|
| `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` | `capi_proj_crs_create_bound_crs_to_wgs84_on_invalid_type` |
| `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name` | `capi_proj_create_from_database_grid_alternative_null_old_proj_grid_name` |
| `CApi.proj_create_operations` | `capi_proj_create_operations` |
| `CApi.proj_create_operations_allow_ballpark_transformations` | `capi_proj_create_operations_allow_ballpark_transformations` |
| `CApi.proj_trans_bounds_to_compound_crs` | `capi_proj_trans_bounds_to_compound_crs` |

## Cas ouverts

Chaque cas ouvert a une raison d'une ligne. Aucun n'est affaibli dans le
fichier de tests : un cas non porté n'a pas de fonction.

### CApi

| PROJ 9.9.0 | Raison |
|---|---|
| `CApi.proj_coordoperation_get_grid_used` | Pas de liste des grilles utilisées par une opération : `DatabaseContext::grid_info` ne renvoie que l'URL d'une grille par nom, sans réécrire l'URL avec l'endpoint du contexte. |
| `CApi.proj_coordoperation_get_grid_used_fullname_caching` | Même absence de liste des grilles utilisées ; le cas est aussi sous `TIFF_ENABLED`. |
| `CApi.proj_coordoperation_is_instantiable` | Pas d'API publique d'instanciabilité d'une opération. |
| `CApi.proj_get_suggested_operation_for_NAD83_to_NAD83_HARN` | La suggestion est portée (`CoordinateOperationFactory::suggested_operation`), mais la liste diffère : `BoundingBox::intersection` ne découpe pas l'extension de EPSG:4152, qui traverse l'antiméridien (144,58 → −64,51). « NAD83 to NAD83(HARN) (47) » est trié après « (33) », donc la suggestion donne « (33) ». |
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | Source projetée (EPSG:27562) : la suggestion n'est portée que pour un CRS géographique (erreur `Unsupported`), et le nombre de grilles utilisées n'est pas exposé. |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | Source géocentrique (EPSG:6317) : la suggestion n'est portée que pour un CRS géographique. |
| `CApi.proj_create_operations_discard_superseded` | `CoordinateOperationContext` n'a pas de drapeau « discard superseded » ; la recherche `RegistrySearch::Both` n'a pas de variante sans obsolètes (seule `Forward` en a une). |
| `CApi.proj_create_operations_dont_discard_superseded` | Même absence du drapeau ; le test asserte les deux comptes (4 et 5). |
| `CApi.proj_create_operations_with_pivot` | Le pivot automatique (sans `set_intermediate_crs`) n'est pas porté : PROJ donne « ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1] », le crate ne donne que le ballpark. |
| `CApi.proj_normalize_for_visualization_with_alternatives` | Les coordonnées normalisées sont reproduites (écart < 1e-8), mais le texte du pipeline attendu n'est pas exposé : `Proj::def` renvoie « unavailable until proj_trans is called ». |
| `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | `Proj` n'expose pas `proj_degree_input` / `proj_degree_output`, que le test asserte. |
| `CApi.proj_coordoperation_create_inverse` | `proj_as_proj_string` avec `MULTILINE`, `INDENTATION_WIDTH=4`, `MAX_LINE_LENGTH=40` : `ProjLines::Multi` a une largeur fixe de 80 colonnes et une indentation de 2. |
| `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` | `Proj::create_crs_to_crs_from_pj` rejette toute option non vide (`ACCURACY=0.05`) au lieu de filtrer par précision. |
| `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter` | Même rejet des options (`ALLOW_BALLPARK=NO`). |
| `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations` | EPSG:25831+5773 n'est pas construit par `Proj::new_known_crs` (constat de `docs/PORTING-capi-factory-core.md`, non re-sondé dans ce lot). |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762` | Le CRS composé avec vertical lié (`proj_crs_create_bound_vertical_crs`, `egm96_15.gtx`) n'est pas un objet que `Proj` accepte ; la chaîne ballpark n'est pas portée. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762` | Même raison. |
| `CApi.proj_create_vertical_crs_ex` | Constructeur `proj_create_vertical_crs_ex` (grille de géoïde, `ACCURACY`) absent. |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | Même absence ; la surcharge avec un CRS géographique n'existe pas. |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | Même absence ; la précision implicite du géoïde n'est pas dérivée. |
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

Bench : `CONTRIBUTING.md` demande d'ajouter les cas du lot à `src/bin/bench.rs`
et une ligne à `bench/history.jsonl`. Ce bench mesure le débit par point des
projections (ns par point). Il n'a pas été étendu à ces cas de fabrique et
d'opérations : non fait dans ce lot.
