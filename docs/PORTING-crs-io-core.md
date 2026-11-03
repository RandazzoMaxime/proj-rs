# Port — PROJ `test/unit/test_crs.cpp` et `test/unit/test_io.cpp`, lot crs-io-core

Source : PROJ 9.9.0, `test/unit/test_crs.cpp` (14 cas) et `test/unit/test_io.cpp` (13 cas), soit 27 cas listés dans `/tmp/port-lots/crs-io.txt`. Les fichiers ont été téléchargés en brut depuis `raw.githubusercontent.com/OSGeo/PROJ/9.9.0` (les sources font 8 181 et 18 856 lignes, `web_fetch` tronque). Les chaînes d'entrée et attendues du JSON, du WKT et des pipelines sont extraites du C++ par script, pas recopiées à la main.

Résultat : **3 cas portés et verts, 24 cas ouverts.** Les cas ouverts ont une raison dans la section suivante ; aucun n'est affaibli, ignoré ou retiré du lot.

Le fichier `tests/unit_crs_io_core.rs` ne contient que les trois cas verts. Les cas qui échouent ne sont pas committés en test rouge ; ils restent ici, avec la sortie observée.

## Cas portés

| C++ `TEST()` | Fonction Rust |
|---|---|
| `crs.boundCRS_nadcon_to_PROJ_string_usePROJAlternativeGridNames_false` | `crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_false` |
| `crs.normalizeForVisualization_bound` | `crs_normalize_for_visualization_bound` |
| `json_import.bound_crs_with_source_crs_in_transformation` | `json_import_bound_crs_with_source_crs_in_transformation` |

## Cas ouverts

Les cas marqués « échec » ont été exécutés et échouent. Ceux marqués « sondé » ont leur premier bloc exécuté dans une sonde temporaire, supprimée ensuite. Ceux marqués « non exécuté » ne sont pas portés dans ce lot : la raison vient de la lecture du code et de la fiche précédente, et n'a pas été re-vérifiée par un test.

| Nom C++ | État | Raison |
|---|---|---|
| `crs.projectedCRS_Krovak_EPSG_5221_as_PROJ_string` | échec | Le pipeline de `createOperation(base, projetée)` n'écrit pas l'étape `+inv +proj=longlat +ellps=bessel +pm=ferro` ni `+pm=ferro` sur le pas Krovak : `projstring.rs` n'écrit `+pm` que si le méridien de la source diffère de celui de la base, or ici la source est la base. |
| `crs.projectedCRS_Krovak_with_approximate_alpha_as_PROJ_string` | échec | Même écart : l'étape `+inv +proj=longlat ... +pm=ferro` et le `+pm=ferro` final manquent. |
| `crs.projectedCRS_from_WKT1_ESRI_as_WKT1_ESRI_s_jtsk03_krovak_east_north` | non exécuté | Le Krovak ESRI avec `X_Scale`, `Y_Scale`, `XY_Plane_Rotation` est importé en Krovak EPSG:9819 au lieu de Krovak (North Orientated) EPSG:1041 (fiche `PORTING-crs-core-2.md`). |
| `crs.projectedCRS_identify_db` | sondé | `ProjectedCRS::identify_with` ne cherche en base que les zones UTM : EPSG:2172 par code renvoie 0 résultat, le C++ en attend 1 à confiance 100. |
| `crs.projectedCRS_identify_wrong_auth_name_case` | non exécuté | Même chemin identify ; un factory anonyme ne trouve pas de candidat pour `AUTHORITY["Esri",54034]`. |
| `crs.verticalCRS_identify_db` | sondé | Pas d'identify de CRS verticale en base : EPSG:7651 renvoie 0 résultat, le C++ en attend 1 à confiance 100. |
| `crs.compoundCRS_identify_db` | sondé | Pas d'identify de CRS composée en base : EPSG:8769 renvoie 0 résultat, le C++ en attend 1 à confiance 100. |
| `crs.boundCRS_crs_link` | non portable | Le C++ vérifie les compteurs `shared_ptr::use_count` et le lien `canonicalBoundCRS` ; les `Crs` Rust sont des clones possédés, sans lien de retour. |
| `crs.boundCRS_identify_db` | non exécuté | `Crs::identify_with` n'a pas de bras pour `Crs::Bound` (`identify.rs`, `_ => Vec::new()`). |
| `crs.crs_createBoundCRSToWGS84IfPossible` | non exécuté | Le corps couvre une quinzaine de blocs (composé, IGNF, vertical, `createFromUserInput("EPSG:4326+3855")`) ; le bloc EPSG:2936 reste le point connu (une seule Helmert liée là où PROJ laisse le CRS non lié). |
| `crs.norway_ntm` | non exécuté | Identify d'une projetée non UTM en base, non implémenté (même cause que `crs.projectedCRS_identify_db`). |
| `crs.ETRF2000_PL_CS92` | non exécuté | Identify d'une projetée non UTM en base ; `createFromUserInput(text, db, true)` a un troisième argument (`usePROJ4InitRules`) sans équivalent Rust. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_and_conversion` | échec | Le parseur WKT n'appelle pas `ConcatenatedOperation::fixSteps` (`io.cpp` `buildConcatenatedOperation`, `concatenatedoperation.cpp:285`) : les pas `CONVERSION` n'ont pas de paire de CRS, donc l'export échoue (« a conversion step of a concatenated operation has no PROJ string »). `build_concatenated` garde aussi `reverse: false`. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_conversion` | échec | Même cause (`fixSteps` absent, pas de paire de CRS sur les pas `CONVERSION`). |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_to_inverse_conversion` | échec | Même cause. |
| `wkt_parse.CONCATENATEDOPERATION_with_inverse_conversion_of_compound` | échec | Même cause, avec en plus le pas composé stéréo/grille/verticale. |
| `wkt_parse.wkt1_oracle` | sondé | Identify d'une LCC (dialecte Oracle) en base : factory anonyme, 0 résultat ; le C++ attend EPSG:2154 à confiance 90. |
| `wkt_parse.wkt1_oracle_albers_conical_equal_area` | non exécuté | Même chemin identify ; le C++ attend EPSG:3005 à confiance 100. |
| `io.projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` | non exécuté | Pas de `PROJStringFormatter` public (`addStep`, `addParam`, `startInversion`, `ingestPROJString`). `PipelineFormatter` est `pub(crate)`, et son en-tête dit que la réécriture push/pop des grilles n'est pas portée. |
| `io.projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc` | non exécuté | Même raison. |
| `io.projstringformatter_krovak_to_krovak_east_north` | non exécuté | Même raison, et la fusion `+axis=swu` de Krovak n'existe pas. |
| `io.projstringformatter_krovak_east_north_to_krovak` | non exécuté | Même raison. |
| `io.projparse_init` | non exécuté | Mode de compatibilité PROJ4 absent (`init=epsg:4326` en longitude/latitude, `EXPECT_THROW` hors compatibilité). `createFromUserInput(..., true)` demande un booléen, qui doit être une enum dans une signature publique. |
| `io.createFromUserInput` | non exécuté | `create_from_user_input` ne renvoie qu'un `Crs` ; le test attend aussi datums, ensembles, URN d'opérations et `CoordinateMetadata`. |

## Écarts volontaires et changements de `src/`

- `Crs::export_to_proj_string_with_database(&db)` (nouveau, `projstring.rs`) : équivalent public de `CoordinateOperation::export_to_proj_string_with_database`. Il réécrit le `+nadgrids` d'un `BoundCRS` (et des composés) avec `grid_alternatives` de la base. `export_bound` et `export_compound` reçoivent un `Option<&DatabaseContext>` interne, pas de paramètre `bool` public.
- `Crs::normalize_for_visualization` normalise la base d'un `BoundCRS` et garde le hub et la transformation (`crs.cpp:1165`, `BoundCRS::create(baseNorm, hub, transformation)`). Avant, le bras `Bound` était cloné tel quel.
- Lecteur JSON : arm `DerivedGeographicCRS` (`build_derived_crs`, via `DerivedCRS::create`), et lecture de `transformation.source_crs` dans un `BoundCRS`. Les autres types `Derived*` restent non lus.
- Le C++ `EXPECT_NE(crs_4807, bound)` compare des pointeurs : le test Rust vérifie `matches!(bound, Crs::Bound(_))`, c'est-à-dire qu'un nouveau CRS lié a été produit.
- L'export JSON du test passe une base de données (`export_to_json_with` la demande), alors que le C++ passe `nullptr` à l'import seulement.
- `CoordinateOperationFactory::create_operation_proj_string` refuse une cible `BoundCRS` (« this operation is not a projection of the source CRS ») ; le test utilise `create_operation` puis `export_to_proj_string_with_database`, comme le C++ utilise `createOperation` puis `exportToPROJString`.
- Non fait : l'étape 6 de `CONTRIBUTING.md` (cas dans `src/bin/bench.rs` et `bench/history.jsonl`). Ces cas ne sont pas des transformations de coordonnées chronométrables ; rien n'a été ajouté à l'historique.
- `docs/UNPORTED-UNIT.md`, `docs/UNIT-TRIAGE.md`, `README.md` et `ROADMAP.md` ne sont pas modifiés.

## Commandes lancées

```
cargo test --offline --test unit_crs_io_core            # 3 passed, 0 failed
cargo fmt --check                                       # sortie vide, code 0
cargo clippy --all-targets --all-features -- -D warnings  # code 0, aucun avertissement
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
                                                        # 1344 passed, 0 failed, 26 ignored (déjà dans HEAD)
```

Les 26 `ignored` sont des tests existants de la base. Le décompte des lignes `#[ignore` est de 29 dans `HEAD` et de 29 dans le worktree : ce lot n'en ajoute aucun.
