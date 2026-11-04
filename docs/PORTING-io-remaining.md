# Lot io-remaining : cas ouverts de `test/unit/test_io.cpp`

Source : PROJ 9.9.0. Fichiers lus :

- `test/unit/test_io.cpp` (corps des 14 `TEST()` du lot) ;
- `src/iso19111/io.cpp` : `WKTParser::Private::build` (dispatch `PROJCS`, branche
  `EXTENSION["PROJ4"]` ob_tran), `WKTParser::Private::buildConcatenatedOperation` ;
- `src/iso19111/operation/concatenatedoperation.cpp` : `ConcatenatedOperation::fixSteps`,
  `ConcatenatedOperation::create`. Le chemin `src/operation/...` n'existe pas en 9.9.0.

Oracle : `projinfo -o PROJJSON` de PROJ 9.9.0, avec `PROJ_DATA` pointant sur le `share/proj`
de l'oracle, pour fixer la règle d'écriture des `id` PROJJSON.

## Bilan

| Issue | Cas |
|---|---|
| Portés (5) | `json_import.several_usages`, `json_import.bound_crs_with_name_and_usage`, `json_import.engineering_crs_affine_CS`, `json_import.multiple_ids`, `json_export.coordinate_system_id` |
| Non portés (9) | 4 × `wkt_parse.CONCATENATEDOPERATION_*`, `wkt_parse.DerivedGeographicCRS_GDAL_PROJ4_EXSTENSION_hack`, `wkt_parse.wkt1_oracle`, `wkt_parse.wkt1_oracle_albers_conical_equal_area`, `io.projparse_init`, `io.createFromUserInput` |

## Changements

`src/iso19111/json.rs` (PROJJSON export) :

- **Règle d'écriture de `id`.** Un objet écrit son `id` s'il est le sommet de l'export, si son
  parent le demande (`Allow::Asked`), ou si son **parent immédiat** n'a pas d'`id`. L'ancienne
  règle regardait tous les ancêtres. L'oracle donne, pour un `GeographicCRS` d'`id` 4326 avec un
  datum d'`id` 6326 et un ellipsoïde d'`id` 7030 : aucun `id` imbriqué (le datum et l'ellipsoïde
  sont supprimés) ; sans `id` au sommet, le datum est écrit et l'ellipsoïde ne l'est pas ; sous un
  `BoundCRS` d'`id`, la source sans `id` garde l'`id` de son datum et l'`id` de son ellipsoïde est
  écrit (l'oracle le fait pour un ellipsoïde sans datum d'`id`). `Chain` ne porte donc plus que
  l'état de l'objet lui-même (`chain_of(ident)`).
- `BoundCRS` : la source et la cible sont écrites avec `Allow::Asked` (l'`id` de la cible `4326`
  est écrit, vérifié à l'oracle).
- `write_id` écrit `id` pour un identifiant et un tableau `ids` pour plusieurs (oracle : `ids`
  pour un `Ellipsoid` à deux identifiants). Le helper `write_identifier` écrit un objet.
- `write_ensemble` et `write_ensemble_members` perdent le paramètre `parent`, devenu inutile.
- Nouveau `CoordinateSystem::export_to_json_with` : `type`, `subtype`, `axis`, puis `id` du CS.
  L'identifiant du CS vient déjà de `create_coordinate_system` (`Identified::with_id`).

`tests/unit_io_lot_remaining.rs` : 5 fonctions, une par cas porté, avec le `TEST(group, name)` C++
en doc comment. Les textes PROJJSON sont ceux de `test_io.cpp`, octet pour octet.

Le cas `engineering_crs_affine_CS` appelle `create_object_from_user_input` (équivalent de
`createFromUserInput(json, nullptr)` pour un CRS JSON). `create_object_from_json` ne lit pas
`EngineeringCRS` : c'est le point d'entrée, pas le lecteur, qui était en cause.

## Cas non portés

- **`wkt_parse.CONCATENATEDOPERATION_with_conversion_and_conversion`**,
  **`..._with_conversion_coordinateoperation_conversion`**,
  **`..._with_conversion_coordinateoperation_to_inverse_conversion`**,
  **`..._with_inverse_conversion_of_compound`** : il manque
  `ConcatenatedOperation::fixSteps` (`concatenatedoperation.cpp`), appelée par
  `buildConcatenatedOperation` (io.cpp, `fixDirectionAllowed = true`). Le parseur Rust
  (`wkt_import.rs`, `build_concatenated`) garde `reverse: false` et ne calcule aucune paire de CRS
  pour les pas `CONVERSION`. Il faut aussi les branches où `ProjectedCRS` est une CRS dérivée (la
  base géographique sert de CRS source ou cible du pas), la détection de `INVERSE(...)` sur la
  méthode, et la branche `setCRSsUpdateInverse`. Le modèle a déjà `Conversion::source/target` et
  `inverted`, donc le port est faisable, mais il est assez gros pour un lot à part.
- **`wkt_parse.DerivedGeographicCRS_GDAL_PROJ4_EXSTENSION_hack`** : tentative faite puis annulée
  (pas de demi-correction). Deux pièces manquent :
  1. `WKTParser::Private::build`, branche `PROJCS` avec `EXTENSION["PROJ4"]` commençant par
     `+proj=ob_tran +o_proj=longlat|lonlat|latlong|latlon` : construire la CRS depuis la chaîne
     PROJ (`+type=crs`). Le dispatch Rust (`wkt_import.rs`) peut le faire ; c'est fait dans la
     tentative annulée, et le résultat WKT est bien une CRS dérivée.
  2. `projstring.rs` : `+proj=ob_tran +o_proj=longlat ... +type=crs` produit aujourd'hui
     `Crs::Projected` (`proj_based_projected`). Il faut `Crs::Derived` (base géographique,
     conversion `ob_tran`). Dans la tentative, la CRS est dérivée et `isEquivalentTo` passe, mais :
  3. l'export PROJ de la conversion `ob_tran` PROJ-based échoue : « Unsupported conversion
     method: PROJ ob_tran o_proj=longlat » ;
  4. la fabrique renvoie une `Conversion` nue pour dérivée → base, là où C++ renvoie
     `+step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=ob_tran ... +step +proj=unitconvert ...`.
- **`wkt_parse.wkt1_oracle`**, **`wkt_parse.wkt1_oracle_albers_conical_equal_area`** : `identify`
  d'une projetée LCC (dialecte Oracle) ou Albers doit renvoyer EPSG:2154 à confiance 90 et
  EPSG:3005 à confiance 100. Le `identify` Rust ne traite que l'UTM ; l'algorithme
  `ProjectedCRS::identify` de PROJ (comparaison de méthode et de paramètres sur la base) n'est pas
  porté.
- **`io.projparse_init`** : mode de compatibilité PROJ4 (`usePROJ4InitRules`), avec `init=epsg:...`
  en longitude/latitude et `EXPECT_THROW` hors compatibilité. Le paramètre est un booléen, qui doit
  être une enum dans une signature publique. Non porté.
- **`io.createFromUserInput`** : le test attend aussi datums, ellipsoïdes, méridiens, ensembles,
  URN d'opérations et `CoordinateMetadata` (`ITRF2014@2025.1`). `create_from_user_input` ne renvoie
  qu'un `Crs`. Non porté.

## Commandes et résultats

```
cargo test --all-features --test unit_io_lot_remaining   # 5 passed, 0 failed
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
    # dernière exécution : 1380 passed, 0 failed (après 60450bb)
cargo fmt --check                                         # OK
cargo clippy --all-targets --all-features -- -D warnings  # OK
```

Commits : `b2c7e18` (json_import, 4 cas), `60450bb` (json_export.coordinate_system_id).
Aucun `UPDATE_DOCS`, aucune modification de `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md`,
`docs/UNPORTED-*.md`, `docs/progress.svg`, ni de `src/iso19111/pipeline.rs` ou
`pipeline/tests.rs`.
