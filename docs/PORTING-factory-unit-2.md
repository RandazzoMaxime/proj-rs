# Port — tests unitaires `test_factory.cpp` (lot 3, opérations de coordonnées)

Source : PROJ 9.9.0, `test/unit/test_factory.cpp`. Lot ciblé sur les `TEST()`
qui se vérifient avec ce que la crate sait déjà exporter : chaîne PROJ d'une
opération EPSG (`+proj=helmert`, `+proj=affine`), et équivalence des étapes
d'une opération concaténée. Les 10 tests portés sont dans
`tests/test_factory.rs`.

## Correspondance

| PROJ 9.9.0 `factory.*` | PROJ-RS `tests/test_factory.rs` |
|---|---|
| `AuthorityFactory_createCoordinateOperation_helmert_3` | `factory_authority_factory_create_coordinate_operation_helmert_3` (inclut `-1` absent) |
| `AuthorityFactory_createCoordinateOperation_helmert_7_CF` | `factory_authority_factory_create_coordinate_operation_helmert_7_cf` |
| `AuthorityFactory_createCoordinateOperation_helmert_7_PV` | `factory_authority_factory_create_coordinate_operation_helmert_7_pv` |
| `AuthorityFactory_createCoordinateOperation_helmert_15_PV` | `factory_authority_factory_create_coordinate_operation_helmert_15_pv` |
| `AuthorityFactory_createCoordinateOperation_helmert_15_PV_rounding_of_drz` | `factory_authority_factory_create_coordinate_operation_helmert_15_pv_rounding_of_drz` |
| `AuthorityFactory_affine_parametric_transform` | `factory_authority_factory_affine_parametric_transform` |
| `AuthorityFactory_createCoordinateOperation_conversion` | `factory_authority_factory_create_coordinate_operation_conversion` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation` | `factory_authority_factory_create_coordinate_operation_concatenated_operation` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_three_steps` | `factory_authority_factory_create_coordinate_operation_concatenated_operation_three_steps` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_step1_is_conversion` | `factory_authority_factory_create_coordinate_operation_concatenated_operation_step1_is_conversion` |

## Comparaisons

- Chaînes PROJ : égalité exacte avec `ProjLines::Single`, comme `EXPECT_EQ`
  dans PROJ. Seul `helmert_7_PV` utilise une recherche de sous-chaîne, comme le
  `find` de PROJ.
- `isEquivalentTo` : `Transformation::is_equivalent_to` et
  `Conversion::is_equivalent_to`, pour chaque étape d'une concaténée. La
  comparaison ne porte ni sur le nom ni sur le code.
- Les tests ne changent aucune valeur attendue. Les codes EPSG sont ceux de PROJ.

## Restent ouverts dans `docs/UNPORTED-UNIT.md` (section `test_factory.cpp`, 48 entrées)

Tentés dans ce lot, non portés :

- `AuthorityFactory_createCoordinateOperation_CF_full_matrix_geog3D` (`10675`) :
  l'export échoue avec `Unimplemented` (« Saba to Saba height (1) »). La cause
  n'est pas examinée ; la chaîne attendue est un `+proj=helmert +exact` géog3D.
- `AuthorityFactory_10566_issue_4212` (`10566`, grilles PROJ) : l'export échoue
  avec `Unimplemented` (« GLLMSL(2022) height to GVR2016 height (1) »), malgré
  `AlternativeGridNames::Proj`. Cause non examinée.
- `AuthorityFactory_createCoordinateOperation_molodensky_badekas_PV` (`1066`) :
  `validateParameters()` n'est pas exposé sur `Transformation`. La partie
  export n'a pas été vérifiée isolément.
- `AuthorityFactory_createCoordinateOperation_concatenated_operation_inverse_step1` (`8443`)
  et `..._inverse_step2` (`7811`) : `step_direction` est NULL dans
  `concatenated_operation_step` (sens direct). PROJ déduit le sens de chaque
  étape à partir de la chaîne des CRS ; la crate garde l'étape en sens direct.
  Le test ne passe pas sans changer la construction des concaténées. Non
  corrigé ici.
- `AuthorityFactory_createCoordinateOperation_concatenated_operation_epsg_9103` :
  la crate donne 4 étapes, PROJ en donne 5 (PROJ ajoute une étape explicite
  géographique vers géocentrique). Le nom de l'étape inverse diffère aussi :
  « ITRF2008 to NAD83(2011) (1) » au lieu de « Inverse of … ».
- `AuthorityFactory_build_all_concatenated` : `EPSG:11005` se construit dans la
  crate, alors que PROJ attend une exception (liste de 9 codes : 8422, 8481,
  8482, 8565, 8566, 8572, 11005, 11398, 11206). Écart réel à examiner ; le test
  est retiré tant qu'il échoue.

Non portés, sans tentative ou avec une API absente :

- `AuthorityFactory_createCoordinateOperation_helmert_8_CF`,
  `helmert_15_CF`, `grid_transformation_one_parameter`,
  `grid_transformation_two_parameter`, `other_transformation`,
  `point_motion_operation` : assertions WKT. La crate n'écrit pas de WKT.
- `AuthorityFactory_createCoordinateOperation_concatenated_operation_step_2_and_3_are_conversion`
  (`7987`) : l'export `geogoffset` n'a pas été vérifié.
- `AuthorityFactory_createFromCoordinateReferenceSystemCodes` et
  `..._anonymous_authority` : `create_from_coordinate_reference_system_codes`
  ne prend que deux codes. Le test demande le filtre des obsolètes et un
  ordre précis (`4179` → `4258` : trois résultats).
- `AuthorityFactory_getAuthorityCodes` (environ 190 lignes d'assertions) et
  `AuthorityFactory_IAU_2015` : non examinés dans ce lot.
- `AuthorityFactory_createDynamicVerticalDatum` : `#ifdef no_more_dynamic_vertical_datum`
  dans PROJ 9.9.0 (voir `docs/PORTING-factory-unit.md`).
- `FactoryWithTmpDatabase.*` : base temporaire et création d'objets non exposées.

## Commandes

```sh
cargo fmt --check                                # ok
cargo clippy --all-targets -- -D warnings        # ok, aucune sortie
cargo test --test test_factory                   # 68 passed (58 avant), 0 failed
```

Recompte : `test_factory.cpp` 58 → 48 entrées ouvertes dans
`docs/UNPORTED-UNIT.md`. Les compteurs de `README.md` et `ROADMAP.md` ne sont
pas modifiés, parce que `CONTRIBUTING.md` ne demande pas de les recompter pour
un lot de tests unitaires. `src/bin/bench.rs` n'est pas modifié non plus : les
lots de tests unitaires précédents ne l'ont pas touché non plus.
