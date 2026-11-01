# Port — export WKT et PROJJSON de la façade (`Proj::as_wkt`, `Proj::to_projjson`)

Sources, PROJ 9.9.0 :

- `src/iso19111/c_api.cpp` : `proj_as_wkt` et `proj_as_projjson`. Les options
  `MULTILINE`, `INDENTATION_WIDTH` et `SCHEMA` ne s'appliquent que si elles sont données.
- `src/iso19111/io.cpp` : `JSONFormatter` (`setMultiLine`, `setIndentationWidth`,
  `setSchema`, `ObjectContext`, `outputUsage`). Le schéma par défaut est PROJJSON v0.7.
- `src/proj_json_streaming_writer.cpp` : `": "` en mode multi-ligne, `":"` sinon. Une
  virgule sans espace en mode une ligne, deux espaces d'indentation par défaut.
- `src/crs.cpp` : `GeodeticCRS::_exportToJSONInternal`, `ProjectedCRS`, `VerticalCRS`,
  `CompoundCRS`, `BoundCRS`.
- `src/iso19111/datum.cpp` : `DatumEnsemble`, `GeodeticReferenceFrame`,
  `VerticalReferenceFrame`, `Ellipsoid`, `PrimeMeridian`.
- `src/iso19111/common.cpp` : `ObjectDomain`, `ObjectUsage::baseExportToJSON`, `UnitOfMeasure`.
- `src/iso19111/coordinatesystem.cpp` : axes et `CoordinateSystem`.
- `src/iso19111/operation/conversion.cpp`, `singleop.cpp` (`OperationMethod`,
  `OperationParameterValue`), `transformation.cpp` (transformation abrégée).
- `test/unit/test_io.cpp` (`json_import.*`, dont les textes attendus exacts) et
  `tests/cli_yaml/test_projinfo.yaml` (`projinfo -o ALL EPSG:4326`, sortie PROJ réelle).

## Le problème

`Proj::as_wkt` renvoyait le WKT stocké dans `proj.db` ou le texte donné à `Proj::new`.
`Proj::to_projjson` faisait de même. Un CRS sans texte stocké était refusé. libproj ne
renvoie jamais le texte d'entrée : il écrit l'objet depuis son modèle, dans le dialecte
demandé.

## Ce que fait PROJ

- `proj_as_wkt(obj, version)` : `WKTFormatter` avec la convention demandée. Multi-ligne par
  défaut, sauf WKT1 Esri ; indentation de 4 espaces.
- `proj_as_projjson(obj)` : `JSONFormatter`, multi-ligne, deux espaces, `$schema` v0.7.
  Un objet de niveau supérieur écrit son `id` ; un objet fils ne l'écrit que si aucun
  ancêtre n'en a un (`outputId`), sauf si le parent le demande
  (`setAllowIDInImmediateChild`). L'usage (`scope`, `area`, `bbox`) n'est écrit que pour
  l'objet de niveau supérieur (`outputUsage`).
- Une paire de `new_known_crs` est une opération de coordonnées : libproj l'écrit comme
  opération. Ce port ne le fait pas (voir « Ce qui reste ouvert »).

## Ce qui a changé

| Fichier | Changement |
|---|---|
| `src/proj_export.rs` (nouveau) | `as_wkt` (options honorées : multi-ligne, indentation, axes, strict, hauteur ellipsoïdique, LINUNIT), `to_projjson` (multi-ligne, indentation, schéma), `crs_of` (CRS d'un `Proj` : `Single`, ou `Native` avec `+type=crs`). Tests unitaires : une ligne égale au multi-ligne joint, indentation, WKT1 Esri une ligne. |
| `src/proj.rs` | Déclaration `#[path = "proj_export.rs"] mod proj_export;`, enfant de `proj`, pour lire les champs privés de `WktOptions`. Corps de `as_wkt` et `to_projjson` seulement. `WktOptions` : suppression de `#[expect(dead_code)]` (devenu inutile, le lint échouerait) et doc à jour. Doc de `WktVersion` à jour. |
| `src/iso19111/json.rs` | Écrivain PROJJSON refondu. `JsonFormat` (`JsonLines`, indentation, schéma), `export_to_json_with`. Kinds : géographique (ids selon la règle PROJ), géocentrique, projeté (base, conversion, CS), vertical (datum, ensemble, géoïde), composé, lié (transformation abrégée). `Place`, `Ident`, `Allow`, `Chain` portent la règle des `id`. |
| `src/iso19111/mod.rs` | Ré-export de `JsonFormat`, `JsonLines`. |
| `src/iso19111/wkt_import.rs` | Un `BOUNDCRS` n'a pas de nom : le premier fils (`SOURCECRS`) était pris pour un nom. Correction de `build_bound`, qui calculait déjà les propriétés sans nom mais les ignorait. |
| `src/crs.rs`, `src/crs_text.rs` | Suppression du code mort : `stored_wkt`, `stored_projjson`, `stored_markup`, `read_stored`, `stored_in_table`, `column_names`, `column_cell`, `is_wkt`. |
| `src/lib.rs` | Doc du module `wkt` à jour. |
| `tests/proj_export.rs` (nouveau) | 12 tests : WKT2:2015 et WKT2:2019 de `EPSG:4326` (textes `projinfo`), PROJJSON de `EPSG:4326` (texte `projinfo`), PROJJSON projeté, géocentrique, vertical, composé, lié (textes de `test_io.cpp`), options, paire sans export. Les textes attendus sont extraits par script (`scratchpad`), jamais recopiés à la main. |
| `tests/proj_facade.rs` | Trois assertions de l'ancien stub mises à jour : WKT de `EPSG:4326` disponible ; PROJJSON géographique disponible ; PROJJSON projeté en entrée refusé (import non construit). |
| `tests/georust_parity.rs` | `test_wkt` : `#[ignore]` retiré, il passe. `test_projjson` : raison de l'ignore précisée. |

## Résultats contre PROJ

| Cas | Source du texte attendu | Résultat |
|---|---|---|
| WKT2:2019 de `EPSG:4326`, multi-ligne | `projinfo -o ALL` (yaml) | égal, caractère pour caractère |
| WKT2:2015 de `EPSG:4326`, multi-ligne | `projinfo -o ALL` (yaml) | égal |
| PROJJSON de `EPSG:4326` | `projinfo -o ALL` (yaml) | égal |
| PROJJSON projeté (UTM 31N, base avec `id`) | `json_import.projected_crs` | égal |
| PROJJSON géocentrique | `json_import.geocentric_crs` | égal |
| PROJJSON vertical | `json_import.vertical_crs` | égal |
| PROJJSON composé (géographique + vertical, ids) | `json_import.compound_crs` | égal |
| PROJJSON lié (NTv2, paramètre fichier, source omise) | `json_import.bound_crs` | égal |
| `test_wkt` (georust) | expected de georust, WKT2:2019 une ligne | passe |

Les entrées des cas projetés, géocentriques, verticaux, composés et liés sont des WKT
écrits à la main, dont les identifiants EPSG reprennent ceux du test PROJ. Le texte
attendu, lui, vient de PROJ.

## Écarts volontaires

- **Version par défaut** : sans `version`, `as_wkt` écrit WKT2:2019. Le choix est
  documenté dans la doc publique ; la valeur par défaut de georust n'a pas été vérifiée
  contre la source de georust.
- **Options non exposées** : `WktOptions` n'a que `Default` en public, comme chez georust.
  Les options internes sont testées dans le module (`proj_export::tests`).
- **Opérations natives** : un `Proj::new` d'une chaîne PROJ sans `+type=crs` (une
  projection, un pipeline) n'est pas un CRS. PROJ écrit alors une conversion ; ce port
  renvoie une erreur (`ExportToWkt` / `ExportToJson`).
- **Attributs de l'ellipsoïde** : une sphère s'écrit avec `radius` (PROJ) ; un demi-grand
  axe qui n'est pas en mètres s'écrit en objet `{value, unit}` (PROJ).

## Ce qui reste ouvert

- **Opération d'une paire** (`new_known_crs`, `create_crs_to_crs_from_pj`). PROJ écrit la
  conversion ou la transformation qui relie les deux CRS. Pour `EPSG:2230` vers
  `EPSG:26946`, c'est une `ConcatenatedOperation` de deux conversions. Le crate n'expose
  pas l'objet d'opération de la sélection (`best_operation` ne rend que des chaînes de
  pipeline), et il n'y a pas d'écrivain PROJJSON d'opération. `as_wkt` et `to_projjson`
  renvoient donc une erreur pour une paire. `test_projjson` reste ignoré.
- **Import PROJJSON des autres kinds** : `create_from_user_input` ne lit que
  `GeographicCRS`. Un `Proj::new` d'un PROJJSON projeté a un CRS « hérité » côté
  `Crs::parse`, mais pas d'objet ISO : `to_projjson` et `as_wkt` renvoient une erreur.
  Le test `proj_facade` l'exprime.
- **Éléments non écrits, refusés par une erreur** : `anchor_epoch`,
  `deformation_models` (datum dynamique ou modèle de déformation), `meridian` d'un axe.
- **Éléments non écrits, sans erreur** (écart silencieux) :
  - l'étendue verticale et temporelle (`vertical_extent`, `temporal_extent`) : le modèle
    ne les porte pas ;
  - `interpolation_crs` d'un modèle de géoïde ;
  - un second identifiant : seul le premier est écrit (PROJ écrit `ids`) ;
  - l'`id` d'un système de coordonnées : jamais écrit.
  Ces quatre points sont dans le périmètre de ce document. Ils ne sont pas traités.
- **Usages multiples** : écrits en tableau `usages`, comme PROJ. Une étendue verticale
  dans un usage reste absente (voir ci-dessus).
- **Message d'erreur** : `ProjError::ExportToWkt` et `ProjError::ExportToJson` gardent leur
  texte d'origine (« WKT export is not built », « The definition could not be represented
  in the requested JSON format »). Ce texte est dans `src/proj.rs`, hors des corps de
  méthodes que ce lot modifie. Le mainteneur le mettra à jour avec les autres erreurs.

## Commandes et codes de sortie

| Commande | Code | Note |
|---|---|---|
| `cargo test --all-features --test proj_export` | 0 | 12 passés |
| `cargo test --all-features --lib proj_export` | 0 | 4 passés (tests unitaires du module) |
| `cargo test --all-features --test georust_parity` | 0 | 19 passés, 6 ignorés (`test_projjson` reste ignoré) |
| `cargo test --all-features --test proj_facade` | 0 | 9 passés |
| `cargo test --all-features --test test_io --test test_crs --test test_io_bound` | 0 | 110, 116, 10 passés |
| `UPDATE_DOCS=1 cargo test --all-features --test docs_in_sync --test progress_svg` | 0 | Régénère README (19 sur 25 pour georust) et ROADMAP. `docs/UNIT-TRIAGE.md` et `docs/progress.svg` ne changent pas. |
| `cargo fmt --check` | 0 | |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | |
| `cargo test --all-features --no-fail-fast` | 0 | 817 passés, 0 échec, 10 ignorés. Avant : 800 passés, 11 ignorés. |
| `cargo test --all-features --no-fail-fast` (avant la régénération des documents) | 101 | Seul échec : `counts_in_docs_match_the_ledgers`, README désaligné après le passage de `test_wkt`. Corrigé par la ligne ci-dessus. |
