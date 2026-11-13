# Lot : insertion d'objets (factory.objectInsertion)

Référence : PROJ 9.9.0, `src/test/unit/test_factory.cpp`, `TEST(factory, objectInsertion)`
(l. 4411-5252), et le chemin d'insertion de `src/iso19111/factory.cpp`
(`getInsertStatementsFor`, `identifyOrInsert`, `identifyOrInsertUsages`,
`startInsertStatementsSession`, `suggestsCodeFor`, l. ~1700-3300). Les valeurs
attendues sont celles du C++, copiées mot pour mot dans les `assert_eq!` du test
Rust. Les INSERT ne sont pas réécrits à la main : chaque bloc du C++ est un
`#[test]` Rust, dans le même ordre.

## Bilan

`factory.objectInsertion` est fermé : la ligne est retirée de
`docs/UNPORTED-UNIT.md`. Les 27 tests de `tests/unit_factory_object_insertion.rs`
passent.

| Bloc C++ (lignes) | Test Rust |
|---|---|
| session, nesting, sans stop, sans session (4411-4442) | `object_insertion_session_cannot_nest`, `..._session_tolerated_without_stop`, `..._stop_without_session_is_tolerated` |
| rien à faire pour un objet connu (4444-4451) | `object_insertion_nothing_to_do_for_a_known_object` |
| géographique 2D, code numérique (4453-4520) | `object_insertion_geographic_2d_crs` |
| géographique 3D, usages connus (4522-4560) | `object_insertion_geographic_3d_crs_with_known_usage` |
| géocentrique liée, nouvel usage (4562-4606) | `object_insertion_bound_geocentric_crs_with_new_usage` |
| datum inconnu, code numérique, ancre et époque (4608-4640) | `object_insertion_geographic_2d_unknown_datum_numeric_code` |
| datum inconnu, code alphabétique (4642-4669) | `object_insertion_geographic_2d_unknown_datum_alpha_code` |
| ellipsoïde inconnue, numérique / alphabétique (4671-4727) | `object_insertion_geographic_2d_unknown_ellipsoid_numeric_code`, `..._alpha_code` |
| méridien inconnu, numérique / alphabétique (4729-4783) | `object_insertion_geographic_2d_unknown_prime_meridian_numeric_code`, `..._alpha_code` |
| projetée, code numérique (4785-4829) | `object_insertion_projected_crs_numeric_code` |
| verticale, datum connu (4831-4864) | `object_insertion_vertical_crs_known_datum_numeric_code` |
| verticale, datum inconnu, avec et sans `ANCHOR_EPOCH` (4866-4932) | `object_insertion_vertical_crs_unknown_datum_alpha_code`, `..._unknown_datum_anchor_epoch` |
| composée (4934-4987) | `object_insertion_compound_crs` |
| datum dynamique géodésique, vertical (4989-5025) | `object_insertion_dynamic_geodetic_datum`, `object_insertion_dynamic_vertical_datum` |
| ensembles de datums : membres en ligne, membres référencés, vertical (5027-5088) | `object_insertion_geodetic_datum_ensemble_inline_members`, `..._reference_members`, `object_insertion_vertical_datum_ensemble` |
| méthode de projection hors EPSG (5090-5120) | `object_insertion_non_epsg_projection_method` |
| méthode sans identifiant, paramètres dans le désordre (5122-5180) | `object_insertion_missing_method_id_and_parameter_order` |
| erreurs : méthode inconnue, paramètre inconnu (5182-5250) | `object_insertion_error_unknown_projection_method`, `..._error_unknown_projection_parameter` |

## Ce qui manquait, et ce qui a été ajouté

### `src/iso19111/insert.rs`

- `ComponentCodes` (nouveau, exporté) remplace le `bool numericCode` de
  `getInsertStatementsFor`. `Numeric` : les pièces prennent le numéro suivant
  (`suggestsCodeFor(..., true)`). `FromParentCode` : préfixe et code de l'objet
  (`GEODETIC_DATUM_<code>`, `ELLPS_<code>`, `PM_<code>`, `CONVERSION_<code>`,
  `VERTICAL_DATUM_<code>`, `MEMBER_<n>_OF_<code>`, `COMPONENT_<code>_<n>`). Le C++
  n'utilise pas le drapeau pour les corps célestes, les unités, les systèmes de
  coordonnées ni le CRS de base d'une projection (`GEODETIC_CRS_<code>`) : le code
  reste identique dans les deux modes.
- `statements_for` et `insert_statements` prennent ce paramètre
  (`components`) avant la liste des autorités permises.
- Ajouts dans le chemin d'insertion : datum vertical, ensemble de datums
  (géodésique et vertical, avec leurs lignes `*_ensemble_member`), CRS verticale,
  CRS composée, CRS liée (on insère sa CRS de base, comme le C++), et l'ensemble
  de datums d'une CRS géodésique (auparavant une erreur).
- `same_kind`, `equivalent` et `object_type` reconnaissent le datum vertical.

### `src/iso19111/model.rs` (constructeurs, API publique)

- `GeographicCRS::with_domains`, `GeodeticCRS::with_domains` : les usages d'un
  objet construit (le C++ met `OBJECT_DOMAIN` dans les propriétés).
- `ObjectDomain::with_scope`.
- `GeodeticReferenceFrame::with_anchor_epoch`, `VerticalReferenceFrame::with_anchor_epoch`
  (`ANCHOR_EPOCH`).
- `PrimeMeridian::create(name, longitude)`.

### `src/iso19111/identify.rs` et `equiv.rs` (correction, hors insertion)

`VerticalCRS::identify` comparait l'objet trouvé avec `vertical_crs_equivalent`,
une égalité qui ne suit pas le critère EQUIVALENT de PROJ. PROJ appelle
`_isEquivalentTo(crs, EQUIVALENT, dbContext)`. La CRS composée du test a son
composant vertical `EPSG:6360` identifié avec confiance 25 au lieu de 100, donc
insérée à tort. Le test identifie désormais avec `is_equivalent_to_with(.., Equivalent)`,
comme le C++, et l'helper `vertical_crs_equivalent` (devenu inutilisé) est retiré
de `equiv.rs`.

### `src/iso19111/projstring.rs` (correction, hors insertion)

Une projection PROJ connue sans code EPSG (`+proj=sinu`) recevait l'identifiant
`EPSG:0` pour sa méthode et ses paramètres. `epsg_identifier` donne un
identifiant vide pour le code `0`, comme `buildop::mapped_method` le fait déjà.
Effet : `insert_statements` écrit `'PROJ','sinu','Sinusoidal'` au lieu de
`'EPSG','0'`, que la base refuse. Le chemin de lecture des CRS PROJ change pour
ces méthodes ; l'ancien résultat n'était pas valide.

### Appels existants

`src/bin/projinfo.rs`, `tests/unit_shape.rs` et `tests/unit_capi_context_2.rs`
passent `ComponentCodes::FromParentCode`. C'est le `numericCode = false` que
`projinfo_lib.cpp` passe à `getInsertStatementsFor`.

## Écarts volontaires

- **Pas de refus sans session.** `getInsertStatementsFor` sans
  `startInsertStatementsSession` lève une `FactoryException` en C++ (l. 4444-4451).
  En Rust, les INSERT sont produits par un `InsertSession` (`statements_for`), donc
  l'appel sans session ne compile pas. C'est la seule partie du test qui n'a pas de
  forme exécutable : elle est couverte par le type, pas par une erreur à l'exécution.
- **Booléen remplacé par une énumération** : `ComponentCodes` (CLAUDE.md : pas de
  paramètre `bool` dans une signature publique).
- **Session** : les lignes de la session vivent dans un savepoint de la connexion
  (voir `PORTING-capi-context-2.md`). Elles sont visibles par `identify` pendant la
  session et disparaissent à l'arrêt. Les tests vérifient les deux points, dont
  `createGeographicCRS("1234")` qui échoue après l'arrêt.
- **Conversion UTM des tests.** Le modèle Rust n'a pas `Conversion::createUTM`. Les
  tests lisent la conversion UTM 31N d'un `PROJCRS` WKT dont la méthode porte
  `ID["EPSG",9807]` et dont les unités portent `ID["EPSG",9122]`, ce qui donne les
  mêmes lignes `conversion` que le C++ (`9122`, et non `9102`, pour les angles).
- **Objets construits avec des usages.** Les CRS des blocs 3 et 4 reçoivent leurs
  usages par `with_domains` (le C++ passe `OBJECT_DOMAIN`). Le CRS de base du bloc 4
  est `EPSG:4978`, qui porte un ensemble de datums : `datum_ensemble()` est passé à la
  place de `datum()`.
- **Exactitude d'un ensemble** : elle est écrite avec six décimales (`%f`), comme le
  C++. Aucun test ne compare ce texte ; les tests d'ensemble vérifient la présence
  des lignes et l'équivalence relue dans `HOBU`.

## Hors lot

- `operation_towgs84_pm_3d` (`tests/unit_op_towgs84_pm_3d.rs`) échoue aussi sur
  l'arbre de base, sans aucun changement de ce lot : la sortie choisit `+ellps=WGS84`
  au lieu de `GRS80` pour la cible. Non traité ici.

- `docs/PORTING-shape.md`, `docs/PORTING-database-open.md`,
  `docs/PORTING-factory-tmpdb-2.md`, `docs/PORTING-capi-database-path.md` et
  `docs/PORTING-crs-factory-closing.md` disent encore que `objectInsertion` est ouvert.
  Ces fiches précèdent le lot et ne sont pas modifiées ici.
- `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md` et le graphique de progression
  ne sont pas régénérés : `docs_in_sync` et `progress_svg` échouent jusqu'à
  `UPDATE_DOCS=1` par le mainteneur, comme le demande le lot.

## Commandes lancées

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --test unit_factory_object_insertion
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=.../target/proj-reference/build/bin:$PATH PROJ_DATA=.../target/proj-reference/build/data \
  cargo test --all-features --no-fail-fast
```
