# API v1 : trois façades, une surface stable

Statut : **lot 1 implémenté** (workspace, façade native, masquage des internes, crate georust,
squelette du crate C, MSRV et semver-checks en CI). **Lot 2 en partie implémenté** : l'ABI de
`proj.h` est complète (292 fonctions exportées, structures vérifiées), 196 fonctions font leur
travail, 96 échouent comme PROJ échoue (liste dans « Écarts du lot 2 »). Le lot 3 (publication)
reste à faire. Ce document fixe ce que la v1 promet, et ce qu'il ne promet pas. Les écarts entre le
cahier des charges et le code sont dans « Écarts du lot 1 » et « Écarts du lot 2 », en fin de page.

## Le problème

Aujourd'hui, la racine du crate exporte tout : le modèle ISO 19111 (environ 120 types), le moteur
(`Operation`, `Xyzt`), les outils de test (`gie`), le CLI (`projinfo`, `projsync`) et la façade
georust (`Proj`, `ProjBuilder`). Tout ce qui est `pub` est couvert par semver. Publier la v1 en
l'état figerait des centaines de types internes, et la phase 2 (SIMD, GPU) ne pourrait plus rien
changer sans une v2.

## Les trois façades

| Façade | Pour qui | Où | Contrat |
|---|---|---|---|
| Native | les projets Rust neufs | racine de `proj-rs` | API Rust idiomatique, petite, stable |
| georust | les utilisateurs de `proj` 0.31 | crate `proj-rs-georust` (`lib name = "proj"`) | mêmes noms et signatures que `proj` 0.31 : changer une ligne de `Cargo.toml` suffit |
| C | les programmes hors Rust qui lient libproj | crate `proj-rs-capi` (`cdylib` + `staticlib`, `libproj`) | `proj.h` de PROJ 9.9.0, ABI comprise : relier sans recompiler |

Les trois appellent le même moteur. Aucune ne réimplémente une transformation.

### Native (racine de `proj-rs`)

```rust
let ctx = proj_rs::Context::default();             // réseau, chemins, cache : désactivés par défaut
let src = proj_rs::Crs::from_user_input(&ctx, "EPSG:4326")?;
let dst = proj_rs::Crs::from_user_input(&ctx, "EPSG:2154")?;
let t = proj_rs::Transformer::builder(&src, &dst)
    .normalize_axis_order()                         // longitude, latitude comme le SIG
    .build()?;
let p = t.forward(proj_rs::Point::new(2.35, 48.85))?;
t.forward_slice(&mut points)?;                      // lot : SIMD quand il existe
```

- `Context` : réseau, endpoint, chemins de recherche, répertoire utilisateur, cache. Clonable,
  `Send + Sync`.
- `Crs` : `from_user_input` (code, WKT, PROJJSON, chaîne PROJ), `to_wkt(WktVersion)`,
  `to_projjson()`, `to_proj_string()`, `name()`, `kind()`, `area_of_use()`.
- `Transformer` : `builder(&src, &dst)` puis `area_of_interest`, `only_best`, `allow_ballpark`,
  `normalize_axis_order`, `backend`, `build()`. Méthodes `forward`, `inverse`, `forward_slice`,
  `inverse_slice`, `operations()` (les candidats, comme `projinfo`), `last_used_operation()`.
- **Ordre des axes = celui de l'autorité**, comme PROJ (EPSG:4326 est latitude, longitude).
  `normalize_axis_order()` donne l'ordre SIG, comme `proj_normalize_for_visualization`.
- `Point { x, y, z, t }` : `f64`, `z` et `t` à 0 et `+inf` par défaut, comme `PJ_COORD`.
- `Backend` : `#[non_exhaustive] enum Backend { Auto, Scalar }`. `Auto` prend le meilleur chemin
  disponible (aujourd'hui AVX2 si présent). Les variantes `Simd` et `Gpu` s'ajouteront sans casser
  l'API, puisque l'enum est `#[non_exhaustive]`. Le trait des noyaux reste privé.
- `Error` : `#[non_exhaustive]`, avec un code d'erreur équivalent à `proj_errno`.
- Garantie : le résultat est celui de PROJ 9.9.0, à la tolérance de `docs/PARITY.md`. Le chemin
  par lots peut différer du scalaire de 11 nm au plus (`docs/PORTING-simd.md`).

### georust (`proj-rs-georust`)

Ré-exporte la façade qui existe aujourd'hui (`src/proj.rs` : `Proj`, `ProjBuilder`, `Coord`,
`Transform`, `ProjError`, `Area`, `wkt`, la fonctionnalité `geo-types`). Le crate s'appelle
`proj-rs-georust` et sa bibliothèque `proj` : un projet passe de `proj = "0.31"` à
`proj = { package = "proj-rs-georust", version = "1" }` sans toucher au code. Sa surface est
vérifiée contre celle de `proj` 0.31 par un test qui compile les exemples de la documentation de
`proj` 0.31.

### C (`proj-rs-capi`)

- Produit `libproj.so` / `libproj.dylib` / `proj.dll` et `libproj.a`, avec le `proj.h` de PROJ
  9.9.0 copié tel quel. Les noms et les signatures sont identiques ; la structure `PJ_COORD` et les
  constantes sont les mêmes.
- Couverture : les fonctions de `proj.h` (292 fonctions et la variable `pj_release`). Une fonction
  que le port ne sait pas honorer renvoie l'erreur de PROJ (`NULL` ou 0 et `proj_errno`), jamais
  un résultat faux.
- Preuve : les tests C de PROJ compilés contre notre bibliothèque. Ce sont `test_c_api.cpp` (152
  TEST, en ne gardant que ceux qui n'appellent que `proj.h`), `proj_context_test.cpp`,
  `proj_errno_string_test.cpp`, `proj_angular_io_test.cpp`, `include_proj_h_from_c.c` et
  `test_fork.c`. Leur liste filtrée est dans `capi/tests/README.md`, avec la raison de chaque
  exclusion.

## Ce qui n'est pas stable

Ces modules restent accessibles, parce que les binaires et les tests en dépendent, mais ils sont
`#[doc(hidden)]` et hors semver : `iso19111`, `gie`, `projinfo`, `projsync`, `listing`, `info`,
`errno`, `names`, `math`, le moteur (`Operation`, `Xyzt`, `CrsPipeline`) et les projections
nommées (`Tmerc`, `Lcc`, `Mercator`). La fonctionnalité `unstable` les rend visibles dans la
documentation, sous `proj_rs::unstable`, pour qui accepte qu'ils changent.

Tout autre item que l'ancienne racine exportait suit la même règle : `dms`, `geod`, `version`,
`defmodel`, `tinshift`, `grids`, `wkt`, `batch_path`, `VERSION`, `Helmert`, `Ellipsoid`,
`Geographic`, `Projected`, les types du réseau et des fichiers (`NetworkCallbacks`, `FileApi`),
et la façade georust (`Proj`, `ProjBuilder`, `Coord`, `Transform`, `Area`, `ProjError`…). La v1
ne promet que la façade native, `proj-rs-georust` et le C : la racine ne montre rien d'autre.

## Outils de stabilité

- `cargo-semver-checks` en CI sur `proj-rs` et `proj-rs-georust`, contre la dernière version
  publiée (à partir de la v1).
- MSRV déclarée (`rust-version`) et vérifiée par un job CI.
- `#![deny(missing_docs)]` sur la surface stable ; chaque item public a un exemple compilé.

## Ordre des lots

1. Workspace, façade native, masquage des internes, crate georust, semver-checks et MSRV en CI.
2. Crate C : l'ABI de `proj.h`, par groupes de fonctions (contexte et erreurs, création et
   transformation, objets ISO 19111, fabriques et opérations, grilles et réseau), chaque groupe
   prouvé par les tests C de PROJ qui le couvrent.
3. Publication : versions `1.0.0`, `publish = true`, noms vérifiés libres sur crates.io.

## Écarts du lot 1

Ce que le code fait autrement que ce document, ou en plus, et pourquoi.

- **Types d'appui sur la racine.** Les méthodes de la façade prennent des types que ce document ne
  nomme pas : `ContextBuilder`, `TransformerBuilder`, `BBox` (west, south, east, north, en degrés),
  `CrsKind`, `WktVersion` (`Wkt2015`, `Wkt2015Simplified`, `Wkt2019`, `Wkt2019Simplified`,
  `Wkt1Gdal`, `Wkt1Esri`), `OnlyBest` (`Default`, `Yes`, `No`), `Ballpark` (`Allowed`,
  `Forbidden`), `Network` et `Cache` (`Disabled`, `Enabled`), `OperationInfo`. Aucun paramètre
  public n'est un `bool` : ce sont des enums.
- **Les noms `Context` et `Crs` changent de sens à la racine.** Les anciens items (`Context`,
  `ContextBuilder`, `iso19111::Crs`) sont sous `proj_rs::unstable`. Les 49 fichiers de test et les
  deux fichiers de `src/bin` qui les importaient de la racine ont changé d'import, et rien d'autre.
  Les autres chemins (`proj_rs::Proj`, `proj_rs::Operation`…) sont inchangés, cachés.
- **`Context::default()` ignore `PROJ_NETWORK`, `PROJ_NETWORK_ENDPOINT` et `proj.ini`**, comme le
  veut « désactivés par défaut ». `Context::from_environment()` les lit comme PROJ. Le contexte
  porte le réseau, le point d'accès, les chemins de recherche, le répertoire utilisateur et le
  cache ; il ne porte ni le chemin de `proj.db` ni les règles `init=`, que le contexte interne garde.
- **`Crs::area_of_use()` rend un `Option<BBox>`**, et le nom de la zone est `Crs::area_name()`.
- **`last_used_operation()` rend un `OperationInfo`**, pas un transformateur. Avec plusieurs
  candidats, c'est celui du dernier point ; avec une seule opération, c'est elle, sans nom : la
  sélection la rend sous forme de chaîne PROJ.
- **`Transformer` est `Send` mais pas `Sync`** : l'ensemble de candidats garde le candidat du
  dernier point dans une `Cell`, comme `P->iCurCoordOp`. `Context`, `Crs` et `Point` sont
  `Send + Sync`.
- **`Backend::Auto` ne vectorise que les transformateurs à une seule opération.** Un ensemble de
  candidats choisit l'opération point par point (`proj_trans`), donc `forward_slice` y passe un
  point à la fois. Le résultat est celui de `forward`.
- **Coordonnées non finies refusées.** `x`, `y` ou `z` infini ou `NaN` donne `Error::InvalidCoordinate`
  au lieu d'un `NaN` en sortie. `t = +inf` veut dire « sans date » et passe au moteur comme 0, ce que
  fait déjà la façade georust.
- **`forward_slice` laisse la tranche intacte en cas d'erreur** et rend l'erreur du premier point
  en échec, sans son indice.
- **`area_of_interest`, `only_best` et `allow_ballpark` agissent sur la sélection** (celle de
  `cs2cs`, `select_alternatives_for_crs`). Limite connue : avec `OnlyBest::No`, la sélection lit
  l'environnement (`PROJ_NETWORK`) pour savoir si le réseau est ouvert, pas le `Context`.
- **`operations()` liste les candidats de `projinfo --spatial-test intersects`**, ceux entre
  lesquels le transformateur choisit. Le test compare leurs noms et leur ordre à ceux de `projinfo`
  9.9.0 pour `EPSG:4326` vers `EPSG:2154` et `EPSG:4267` vers `EPSG:4269`.
- **Crate C : `proj_rs_capi`, pas `proj`.** Le squelette s'appelle `proj_rs_capi` (artefacts
  `libproj_rs_capi.so` et `.a`). Une bibliothèque `proj` dans le workspace entrerait en collision
  avec la bibliothèque `proj` du crate georust (`target/doc/proj`, fichiers uplifted). Le lot 2
  renomme les artefacts `libproj` à l'installation.
- **Binaires : `doc = false`.** Le binaire `proj` et la bibliothèque `proj` de georust écrivaient la
  même page dans `target/doc`.
- **Exemples de la documentation de `proj` 0.31.** Les 30 blocs de code des commentaires de
  `lib.rs`, `proj.rs`, `geo_types.rs`, `transform.rs` et du `README.md` sont copiés tels quels dans
  `georust/tests/proj_031_docs.rs` (voir `georust/tests/README.md` pour la liste). Deux
  retouches, toutes dans les exemples du README (que `proj` ne teste pas) : la coquille `epsiolon`,
  qui ne compile pas, et quatre `assert_relative_eq!` à la précision machine contre des valeurs de
  libproj 9.6, que le port (PROJ 9.9) manque de 3e-9 m ; ils prennent `epsilon = 1e-6`.
- **MSRV mesurée : 1.83.** `cargo +1.83.0 check --workspace --lib --bins --all-features` passe.
  Avec 1.82.0 (`--ignore-rust-version`), les binaires `cct` et `cs2cs` échouent : `ExitCode`
  n'est comparable avec `!=` que depuis 1.83. Les dépendances exigent au moins 1.81 (`half`).
- **`cargo-semver-checks`** est dans la CI avec `if: false` : aucune version n'est publiée. Il
  s'active au lot 3, quand `1.0.0` existe sur crates.io.

## Écarts du lot 2

Ce que le crate C fait autrement que ce document, ou pas encore, et pourquoi.

- **Nombre de fonctions : 292, pas 294.** `proj.h` déclare 292 fonctions et une variable
  (`pj_release`, exportée). Les « 294 » du cahier des charges venaient d'un décompte par
  `grep PROJ_DLL` qui compte aussi `__declspec` et `__attribute__`.
- **Noms des artefacts.** Le crate garde le nom `proj_rs_capi` : une bibliothèque `proj` dans
  l'espace de travail donnerait `proj.pdb` deux fois sous Windows (le binaire `proj` du crate
  racine) et `target/doc/proj` deux fois (georust). `capi/tests/dist.sh` installe
  `libproj.so.25` (soname fixé par `build.rs`), `libproj.so` et `libproj.a` sous Linux,
  `libproj.25.dylib`, `libproj.dylib` et `libproj.a` sous macOS. **Windows n'est pas fait** :
  `proj.dll` et `proj.lib` demandent de renommer la DLL et de refaire la bibliothèque d'import,
  ce qui n'a pas pu être vérifié ici. Seul Linux est testé.
- **En-têtes.** `capi/include/` contient `proj.h`, `proj_constants.h`, `proj_experimental.h` et
  `proj_symbol_rename.h` de PROJ 9.9.0, octet pour octet (`run.sh` compare `proj.h` à celui de
  l'arbre de référence). `proj_experimental.h` ne déclare plus rien : tout est dans `proj.h`.
- **ABI des structures.** `capi/src/ffi.rs` est généré par bindgen à partir de `proj.h` (outil à
  usage unique, non requis au build). Il porte, pour chacune des 27 structures, des assertions de
  compilation sur la taille, l'alignement et le décalage de chaque champ (valeurs de clang sur
  l'en-tête). `capi/tests/abi_layout.c`, généré à partir de ces assertions, les redit en C : il
  compile contre `capi/include/proj.h` et contre l'en-tête de PROJ. Rust et C s'accordent donc
  sur les mêmes nombres. `PJ_COORD` fait 32 octets, `PJ_INFO` 56, `PJ_GRID_INFO` 360.
- **Fonctions non implémentées : 96 sur 292.** Chacune est exportée (`unsupported.rs`, généré par
  `capi/tools/gen_unsupported.py`), renvoie `NULL` ou 0 et pose `PROJ_ERR_OTHER` avec le message
  « *nom* : not implemented in proj-rs » (`PROJ_RS_CAPI_TRACE_UNSUPPORTED=1` les imprime). La
  liste exacte :
  - réseau et fichiers (2) : `proj_context_set_fileapi`, `proj_context_set_network_callbacks`
    (les traits `FileApi` et `NetworkCallbacks` du moteur existent, l'adaptation C n'est pas
    écrite) ;
  - listes statiques (5) : `proj_list_operations`, `proj_list_ellps`, `proj_list_units`,
    `proj_list_angular_units`, `proj_list_prime_meridians` (il manque les descriptions de
    `pj_list.h` et le pointeur de constructeur de `PJ_OPERATIONS`) ;
  - constructeurs d'objets (18) : `proj_create_cs`, `proj_create_geographic_crs_from_datum`,
    `proj_create_geocentric_crs_from_datum`, `proj_query_geodetic_crs_from_datum`,
    `proj_create_derived_geographic_crs`, `proj_create_engineering_crs`,
    `proj_create_vertical_crs`, `proj_create_vertical_crs_ex`, `proj_create_compound_crs`,
    `proj_create_conversion`, `proj_create_linear_affine_parametric_conversion`,
    `proj_create_linear_3D_affine_parametric_conversion`, `proj_create_transformation`,
    `proj_convert_conversion_to_other_method`, `proj_create_projected_crs`,
    `proj_create_derived_projected_crs`, `proj_crs_add_horizontal_derived_conversion`,
    `proj_crs_create_bound_vertical_crs` ;
  - les 71 `proj_create_conversion_*` (UTM, Mercator, Lambert, Albers, stéréographiques, Robinson,
    Wagner, Eckert, pôle tournant...). Le moteur a les constructeurs (`Conversion::create_*`,
    `buildop.rs`) : il reste à convertir les unités d'angle et de longueur de chaque appel C.
  Les constructeurs sont le gros du reste : 20 des 26 cas en attente de `test_c_api.cpp` en
  dépendent.
- **Preuve.** `capi/tests/run.sh` compile et lance, contre la bibliothèque, `test_c_api.cpp`
  (152 `TEST`), `proj_context_test.cpp`, `proj_errno_string_test.cpp`,
  `proj_angular_io_test.cpp`, `include_proj_h_from_c.c` et `test_fork.c`, plus la vérification des
  symboles et de l'ABI. Tout passe sauf ce qui est écrit dans `capi/tests/excluded.txt` (33 cas qui
  ne peuvent pas passer : 32 appellent l'API C++ de PROJ, 1 lit `pj_ctx::ca_bundle_path`) et
  `capi/tests/pending.txt` (26 cas qui ne passent pas encore, chacun avec sa raison : 20 par
  constructeur absent, 6 par écart du moteur). Résultat : 94 cas de `test_c_api.cpp`
  passent. `test_network`, `test_defmodel`, `test_tinshift` et les autres exécutables de
  `test/unit` utilisent l'API C++ et ne sont pas visés.
- **Écarts du moteur vus par les tests (à traiter dans `src/iso19111/`).**
  `proj_create_from_database` : le moteur n'écrit pas de chaîne PROJ pour la conversion nue
  EPSG:16031, donc `proj_pj_info` n'a pas d'`id` (`CApi.proj_create_from_database`).
  `proj_as_proj_string` d'un CRS ignore `USE_APPROX_TMERC` (`CApi.proj_as_proj_string_approx_tmerc_option_yes`).
  Un CRS dynamique lu par son nom (`ITRF2014@2025.0`) n'entre pas dans `Proj::new`
  (`CApi.proj_create_crs_to_crs_coordinate_metadata_in_src` et `_in_target`).
  `EPSG:4284` vers `EPSG:4284` donne un ensemble de candidats, pas le `noop` de PROJ
  (`CApi.proj_trans_bounds_noop_geographic`). Une conversion donnée par chaîne PROJ répond
  « sans temps » même avec `+t_epoch` (`CApi.proj_coordoperation_requires_per_coordinate_input_time`).
- **Ordre des axes.** La façade `Proj` transforme toujours dans l'ordre longitude, latitude.
  `proj_create_crs_to_crs` rend l'ordre du CRS, comme PROJ : la couche C permute les deux premiers
  axes de chaque côté d'après `Proj::pair_axis_swaps`, jusqu'à `proj_normalize_for_visualization`.
  Les 35 tests `proj_trans_bounds*` passent ainsi, boîtes comprises.
- **`proj_context_get_database_path`.** La base est celle du crate. La fonction rend le `proj.db`
  de `PROJ_DATA` s'il existe (les tests le copient), sinon `NULL`. Redonner ce chemin à
  `proj_context_set_database_path` sélectionne de nouveau la base embarquée.
- **Fils d'exécution.** Les fonctions de `proj.h` reçoivent des pointeurs de
  contexte et d'objet : comme PROJ, le crate ne protège pas un même contexte ou objet contre deux
  fils. Chaque point d'entrée passe par `catch_unwind` : un panic donne l'échec de la fonction.
- **Tests externes.** gtest vient de `libgtest-dev` (1.14), PROJ prend 1.15.2 : les tests n'en
  dépendent pas. `TIFF_ENABLED` est défini, comme le fait CMake, ce qui ajoute deux blocs de
  `test_c_api.cpp`.
- **Changements dans le moteur** (tous cachés, hors semver) : `Proj::{is_transform,
  native_operation, pair_axis_swaps, trans_coord, trans_batch}`, `ProjBuilder::{replace_search_paths,
  set_user_directory_path, create_user_directory}`, `Factors` garde les quatre dérivées de
  `PJ_FACTORS`, `iso19111::create_user_object` et `proj_based_operation_of_text` deviennent `pub`.
