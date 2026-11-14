# Fiche de portage : gie, gie self-tests, projsync, réseau

Audit détaillé : `docs/AUDIT-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/apps/gie.cpp` | `src/gie.rs` (runner, bibliothèque `proj_rs::gie`), `src/bin/gie.rs` (options, usage, `-l`, résumé) |
| `test/unit/gie_self_tests.cpp` | `tests/gie_self_tests.rs` (15 fonctions, 14 tests C++) |
| `gie.info_functions` (`gie_self_tests.cpp`, l. 351-482) | `tests/gie_info_functions.rs` |
| `proj_dmstor`, `proj_rtodms2`, `proj_strtod` (`dmstor.cpp`, `rtodms.cpp`) | `src/dms.rs` (`proj_rs::dms`) ; `src/bin/support/dms.rs` garde les noms des commandes |
| `proj_info`, `proj_grid_info`, `proj_init_info` (`info.cpp`, `grids.cpp`) | `src/info.rs` (`proj_rs::info`) |
| `pj_get_def` pour une projection qui n'est pas une CRS (`proj_pj_info`) | `operation_definition`, branche de repli, `src/proj_info.rs` |
| `test/cli/test_gie.yaml` | `tests/gie_cli.rs` |
| `test/gie/*.gie` (ratchet) | `tests/gie_run.rs` |
| proj_roundtrip, proj_trans_array, proj_xy_dist, proj_list_* | `src/gie_api.rs` |
| fwd_prepare (latitude, `fwd.cpp`) | `latitude_gate` dans `src/engine.rs` |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | non porté |
| `test/unit/test_network.cpp` | non porté |

## Écarts

- `gie -o` : `gie.cpp` ouvre le fichier de sortie en lecture (`"rt"`), ce qui
  échoue pour un nouveau fichier. Le port écrit le rapport dans le fichier.
- `gie -vv` : pas de compteurs roundtrips / échecs séparés (la bibliothèque
  renvoie un total par fichier).
- `gie` : les erreurs de structure `<gie>` ne sont pas signalées.
- `proj_trans_generic` (tableaux strides) : porté comme `transform_array`
  sur des points, qui est la forme Rust équivalente.
- Changement de contexte (`proj_context_set`) : `Operation::set_context` et
  `Operation::context` (`src/engine.rs`). Le contexte est gardé, pas lu : les lectures
  de `proj.db` (`+init=epsg:`, NKG) passent par la base embarquée, donc aucun résultat
  ne dépend encore du contexte.
- Porte de latitude : une correction du moteur (voir `latitude_gate`) est
  nécessaire, la crate donnait `OutsideDomain` pour une latitude de 95° en UTM.
- `proj_info().release` : « 9.9.0 » ici, « Rel. 9.9.0 (hash), date » dans PROJ.
  Le test vérifie seulement qu'il est non vide, comme le C.
- `proj_info().searchpath` : les répertoires par défaut sont `PROJ_DATA` (ou
  `PROJ_LIB`, ou `data/grids`), sans le répertoire utilisateur inscriptible de
  PROJ. La fonction ne lit aucune variable de contrôle : le `putenv` du C n'a pas
  d'équivalent.
- `proj_rtodms2` : la taille du buffer C `sizeof_s` est passée comme `max_len =
  sizeof_s - 1` (caractères sans le NUL terminal). Le retrait des zéros de
  `rtodms.cpp` s'applique au texte tronqué, comme dans le C.
- `DmsSign` : le signe `pos != 0, neg == 0` du C (sans lettre pour les négatifs)
  n'a pas d'équivalent ; les commandes ne l'emploient pas. Une lettre NUL n'écrit
  rien, comme le zéro du C.
- `proj_dmstor` : prend `&[u8]`, pas `&str`, pour lire le degré ISO 8859-1 (`0xB0`).
- `proj_grid_info` : `None` au lieu de `format = "missing"` et `filename = ""`.
  `filename` vaut `None` si le nom ne se résout pas en fichier local (le C le
  laisse vide aussi).
- `proj_init_info` : `None` au lieu de chaînes vides. `epsg` et `IGNF` n'ont pas
  de fichier ici : `filename` vaut `None`, les métadonnées viennent de la base.
- Bloc TIFF de `proj_grid_info` : la référence de build (`target/proj-reference`)
  est compilée sans TIFF (`ENABLE_TIFF=OFF`), donc son test C n'exécute pas ce
  bloc. La crate lit les GeoTIFF : le bloc tourne ici, avec les littéraux du C
  comme seul oracle disponible (pas de libtiff pour recompiler la référence).
- `proj_pj_info().definition` d'une projection nue (`+proj=utm`, `+proj=august`) :
  `Proj::def` renvoie maintenant ses paramètres (avec `ellps=GRS80` par défaut,
  comme `pj_init`), au lieu d'une erreur. Les chaînes qui sont des CRS gardent
  leur reconstruction ; un datum inconnu reste une erreur.
- `proj_pj_info().definition` d'une paire : `Proj::def` et `Proj::proj_info` donnent
  le texte normalisé de l'objet que la crate exécute (longitude d'abord, sans
  `axisswap` de bout de pipeline, comme `proj_normalize_for_visualization`). Le C de
  `proj_create_crs_to_crs_PULKOVO42_ETRS89` lit l'objet non normalisé :
  `Proj::definition_in_authority_order` le donne. Vérifié à la main contre PROJ 9.9.0
  sur EPSG:4326→3857, →32661, →25832, EPSG:32661→4326, EPSG:4807→4326, EPSG:4258→25832,
  EPSG:2230→26946, et les noops (EPSG:4326→4326, →4979). Non mesuré : l'ordre des
  `proj=set v_4` (epoch) par rapport aux swaps ; la crate les met après.

## proj_create_crs_to_crs_PULKOVO42_ETRS89 (`TEST(gie, ...)`, `gie_self_tests.cpp` l. 1269-1347)

Porté de bout en bout, les 12 assertions. Correspondance :

- `proj_pj_info(P).definition` avant le premier point, « unavailable until
  proj_trans is called » : `Proj::proj_info().definition` (`info.cpp` 190-191).
- `proj_get_name(P) == NULL` : `Proj::name()` vaut `None` pour une paire à plusieurs
  candidats. PROJ garde un objet « dummy » sans `iso_obj` (`crs_to_crs.cpp:787`).
  Pour une paire à une opération, c'est son nom ; pour un candidat de
  `last_used_operation()`, c'est son nom ; un CRS seul et une chaîne PROJ donnent
  `None` (écart : un CRS n'est pas une transformation).
- `P->fwd`, `fwd3d`, `fwd4d` nuls : `last_used_operation()` vaut `None` tant qu'aucun
  candidat n'a été instancié par un point (même état).
- `proj_get_source_crs` / `proj_get_target_crs` sur P : `Proj::source_crs()` et
  `target_crs()`, le CRS du premier candidat (`c_api.cpp` 2630). Écart : `None` pour
  un candidat de `last_used_operation()` (`Item` ne garde pas ses CRS).
- `proj_trans` FWD puis INV, `proj_trans_generic` FWD : `convert_xyz`, `project` et
  `convert_array` (pas de pas dans la crate, un tableau d'un point).
- `proj_pj_info(P).definition` après le point de Roumanie et après celui de Pologne :
  `Proj::definition_in_authority_order()`. Les deux pipelines du C sont des littéraux
  de test, vérifiés contre `cs2cs`/`proj` 9.9.0 et un programme C de contrôle
  (`proj_create_crs_to_crs` + `proj_pj_info` + `proj_trans_get_last_used_operation`,
  hors dépôt) : le nom du candidat est « Pulkovo 1942(58) to ETRS89 (4) » pour la
  Roumanie, « ... (1) » pour la Pologne.

## cart_selftest (`TEST(gie, cart_selftest)`)

Porté de bout en bout, sauf trois assertions sans équivalent observable :

- `ASSERT_NE(ctx, pj_get_default_ctx())` : comparaison de pointeurs entre deux
  `PJ_CONTEXT`. Un `Context` est une valeur, il n'y a pas de singleton à comparer.
- `ASSERT_EQ(45, obs[0].lpz.z)` et `ASSERT_EQ(50, obs[1].lpz.z)` : `proj_trans_generic`
  avec `z` à `stride` nul lit et écrit un tampon `h` séparé. L'API point par point
  (`Xyzt`) n'a pas de tampon `z` séparé.

Forme retenue pour le contexte : `Operation::create` garde `Context::new()`
(`proj_create(PJ_DEFAULT_CTX, ...)`), `set_context` remplace la copie (un
`Context` est une valeur, donc `proj_context_destroy` est un `drop`). Le contexte
est gardé, pas lu : les lectures de `proj.db` (`+init=epsg:`, NKG) passent par la base
embarquée (`with_db`), pas par le contexte de l'opération.
`ASSERT_EQ(pj_get_default_ctx(), P->ctx)` devient une égalité de valeur avec un
`Context::new()` pris au début du test.

Autres appels sans assertion ou sans équivalent direct :

- `proj_errno_reset(P)` : pas d'état d'erreur par objet, les erreurs sont des
  valeurs de retour.
- `PJ_IDENT` : la coordonnée est renvoyée telle quelle, aucun appel Rust.
- `n == 2` de `proj_trans_generic` et `proj_trans_array == 0` : `Ok(())` de
  `transform_array` sur le tableau entier.
- `PROJ_ERR_COORD_TRANSFM_INVALID_COORD` et `PROJ_ERR_COORD_TRANSFM` : les
  variantes `BatchError::Uniform(TransformError::InvalidCoord)` et
  `BatchError::Mixed`. `BatchError` n'a pas de `code()` ; la variante `Mixed` est
  documentée avec son code.

Valeurs contrôlées avec `proj` et `cs2cs` 9.9.0 (`target/proj-reference/build/bin`),
identiques à la crate à la précision affichée : UTM zone 32 GRS80 de (12°, 55°) :
`691875.632139660767 6098907.825005011633` ; cart GRS80 de (12°, 55°, 100) :
`3586525.761058 762339.584111 5201465.438293`.

## proj_create_crs_to_crs_from_pj_force_over (`TEST(gie, ...)`, l. 1520-1759)

Porté avec ses 46 `ASSERT_`/`EXPECT_`, dans `tests/gie_self_tests.rs`. Le test
a été écrit avant l'implémentation : sa première compilation échouait (`ForceOver`
et `force_over()` absents). Après le correctif, il passe. Une mutation qui retire
`over` des paramètres du moteur le fait échouer sur `EXPECT_LT(output_over.xyz.x, 0)`
(`tests/gie_self_tests.rs`, assertion `x_over < 0.0`), puis le code est restauré.

- `FORCE_OVER=YES` : `ForceOver::Yes` est lu par `Proj::force_over()`. Le drapeau
  est passé à chaque projection : `Operation::create_over` ajoute `over` aux
  paramètres d'une projection, ou à l'en-tête d'un pipeline (les étapes héritent
  des paramètres d'en-tête, `build_pipeline`). C'est `ctx->forceOver` pendant
  `pj_init` (`init.cpp:603-605`).
- Candidats : `iso_pipeline_at` reconstruit chaque candidat avec le drapeau
  (`crs_to_crs.cpp:671`, `:743`). `Proj::alternatives_force_over()` donne un
  drapeau par candidat, tous égaux au drapeau du transform comme en PROJ.
  Limite : pour `EPSG:4326` vers `EPSG:27700`, le tmerc ne change pas avec
  `over`, donc le test ne voit pas l'effet sur les candidats, seulement leur
  drapeau.
- Options : seul `FORCE_OVER=` est lu, avec la règle de PROJ (`yes`, sans
  distinction de casse, active le drapeau ; une autre valeur le laisse
  éteint ; un `NO` ultérieur ne le réinitialise pas). Toute autre option est
  refusée avec l'erreur d'avant (`ACCURACY=0.05` reste refusé,
  `tests/proj_unit.rs`).
- Écarts de forme, sans effet sur les valeurs : `proj_normalize_for_visualization`
  est l'identité sur la façade, dont la paire est déjà en longitude, latitude, donc
  `Pnormalized` est `P` ; `t = HUGE_VAL` n'est pas passé (aucun temps lu ici) ; le
  contexte et `proj_destroy` sont des `Drop`.
- Ajout au C : `alternatives.len() > 1`, la phrase du commentaire C « more than one
  coordinate operation candidate » écrite en assertion.
- Contrôle contre le binaire de référence 9.9.0 (`target/proj-reference/build/bin`,
  `PROJ_DATA` et `LD_LIBRARY_PATH` de la référence) : `cs2cs +over` sur `-220 0`
  donne `-24490287.97`, et `cs2cs` sans `+over` sur `140 0` donne `15584728.71`, les
  valeurs du C++ à la précision affichée.

## proj_create_crs_to_crs (`TEST_F(gieTest, ...)`, `gie_self_tests.cpp` l. 264-299)

Porté de bout en bout, les 8 assertions. Le C ne vérifie que le nom du CRS source
(« ETRS89 / UTM zone 32N ») ; il n'y a pas d'assertion de nom cible, donc pas
d'assertion ajoutée.

- `EXPECT_NEAR` sur `(307788.8761171057, 5999669.3036037628)` à 1e-8 : `convert_xyz`.
  Valeurs relues avec `cs2cs -f %.10f` 9.9.0. `5_999_669.303_603_763` en Rust est le
  même `f64` que `5999669.3036037628` du C (vérifié : `float()` des deux donne la même
  valeur).
- `proj_get_source_crs` puis `proj_get_name` : `Proj::source_crs()` puis `Crs::name()`,
  lus après le point comme dans le C.
- Deux `ASSERT_TRUE(P != nullptr)` et deux `EXPECT_TRUE(... == nullptr)` : `is_ok()` et
  `is_err()` de `new_known_crs`. La chaîne PROJ `proj=utm +zone=32 +datum=WGS84` est
  acceptée comme côté de paire.

Cause de l'assertion absente jusque-là : `Proj::source_crs()` renvoyait `None` pour
EPSG:25832→25833. L'opération choisie est « Inverse of UTM zone 32N + UTM zone 33N »
(`projinfo`), une concaténation créée par `derived_to` (`src/iso19111/derived.rs`).
`concatenate` appelait `pivot_concatenation`, qui laisse `source` et `target` à `None`.
PROJ fixe les CRS de la concaténation à `operationsIn[0]->sourceCRS()` et
`operationsIn.back()->targetCRS()` (`concatenatedoperation.cpp:242-256`). Correctif : ces
deux CRS sont posés dans `concatenate`, seul appelant de ce chemin. Le reste de
`pivot_concatenation` (pivot de datum) n'est pas modifié. Conséquence visible : les
concaténations dérivées portent désormais leurs CRS, comme en PROJ.

## Commandes

```
cargo test --offline --test gie_cli --test gie_run --test gie_self_tests --test gie_info_functions
```
`gie.proj_trans_with_a_crs` : porté. `proj_trans` sur un CRS renvoie `Err(ProjError::NotATransform)` (C++ : `HUGE_VAL`, `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`).
Le code vient de `ProjError::code()` (`src/errno.rs`) ; la variante `Conversion` donnait `PROJ_ERR_COORD_TRANSFM` (2048), écart corrigé.
Test : `tests/gie_self_tests.rs::trans_with_a_crs_is_an_error`, écrit avant le correctif (échec : 2048 au lieu de 1027).
