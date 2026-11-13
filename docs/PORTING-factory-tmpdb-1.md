# Port — `FactoryWithTmpDatabase` de `test_factory.cpp` (lot 1, base temporaire)

Source : PROJ 9.9.0, `test/unit/test_factory.cpp`, fixture `FactoryWithTmpDatabase`
(`createStructure`, `populateWithFakeEPSG`, `createSourceTargetPivotCRS`,
`createTransformationForPivotTesting`, `checkSourceToOther`).

Dix `TEST_F` sont fermés. Ils tournent sur un `proj.db` écrit dans un fichier
temporaire, puis ouvert par `DatabaseContext::open`.

## Aide partagée

`tests/common/tmp_db.rs` (`TmpDb`, `FAKE_EPSG`) :

- `create_structure` : toutes les instructions de `DatabaseContext::create().database_structure()`
  (tables, vues, déclencheurs, lignes de version du schéma), puis `PRAGMA foreign_keys = 1`,
  comme `createStructure`.
- `populate_with_fake_epsg` : les 44 instructions de `populateWithFakeEPSG`, extraites
  mécaniquement du C++ (compte vérifié : 44 `execute(` = 44 `INSERT`).
- `execute`, `execute_all` : chaque instruction doit réussir, comme `ASSERT_TRUE(execute(...))`.
- `database` : `DatabaseContext::open` après les écritures.

`DatabaseContext::open` est en lecture seule : les lignes sont écrites d'abord, puis
un contexte est ouvert après chaque écriture, comme le C++ relit
`DatabaseContext::create(m_ctxt)` après ses `INSERT`.

Les fichiers vont dans `CARGO_TARGET_TMPDIR` (dossier de `target/` que cargo fournit aux
tests d'intégration), pas dans `/tmp`, et sont supprimés à la fin de chaque test.

## Correspondance

| `test_factory.cpp` (`FactoryWithTmpDatabase.*`) | `tests/unit_factory_tmp_db.rs` |
|---|---|
| `AuthorityFactory_test_with_fake_EPSG_database` | `factory_with_tmp_database_authority_factory_test_with_fake_epsg_database` |
| `AuthorityFactory_test_inversion_first_and_last_steps_of_concat_op` | `factory_with_tmp_database_authority_factory_test_inversion_first_and_last_steps_of_concat_op` |
| `AuthorityFactory_test_with_fake_EPSG_and_OTHER_database` | `factory_with_tmp_database_authority_factory_test_with_fake_epsg_and_other_database` |
| `AuthorityFactory_test_sorting_of_coordinate_operations` | `factory_with_tmp_database_authority_factory_test_sorting_of_coordinate_operations` |
| `AuthorityFactory_createFromCRSCodesWithIntermediates_source_equals_target` | `..._create_from_crs_codes_with_intermediates_source_equals_target` |
| `AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_target_pivot` | `..._case_source_pivot_target_pivot` |
| `AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_pivot_target` | `..._case_source_pivot_pivot_target` |
| `AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_pivot_target` | `..._case_pivot_source_pivot_target` |
| `AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_target_pivot` | `..._case_pivot_source_target_pivot` |
| `getAuthorities` | `factory_with_tmp_database_get_authorities` |

Les noms Rust sont préfixés `factory_with_tmp_database_` ; le reste est le nom C++ en
minuscules, comme dans `tests/test_factory.rs`.

Chaque test reprend toutes les assertions du C++ :

- `test_with_fake_EPSG_database` : les 15 objets (unité, étendue, méridien, ellipsoïde,
  deux datums avec leur ancrage `my anchor`, 4326, 3855, conversion, 32631, composé,
  trois transformations, concaténée) ;
- `checkSourceToOther` (quatre cas) : les cinq appels `createFromCRSCodesWithIntermediates`
  de l'authority `OTHER`, le même appel dans l'authority générale, puis les quatre
  `createOperations` (contexte sans pivot, pivot `NS_PIVOT/PIVOT`, `NEVER`, pivot inexistant).

## Nouvelle API dans `src/`

Le lot a demandé de porter `createFromCRSCodesWithIntermediates`. Il n'existait qu'une
fonction privée, `crs_codes_intermediate_operations`, appelée par `createOperations`, sans
la liste explicite de pivots (`getIntermediateCRS`, notée « non portée »).

- `AuthorityFactory::create_from_crs_codes_with_intermediates(source, target, intermediates)`
  (`src/iso19111/factory.rs`) : l'appel C++ à neuf arguments, avec les valeurs par défaut
  des autres arguments. `source == target` rend vide avant toute recherche de CRS, comme le C++.
- `PivotScope` : `Operations` (la recherche de `createOperations`, comportement inchangé)
  et `Listed(&[(auth, code)])` (la nouvelle API). Les deux appels existants passent
  `PivotScope::Operations`. Leur contexte a toujours `intermediates` vide (garde
  `context.intermediates.is_empty()` déjà présente), donc leur comportement ne change pas.
- Filtre de la liste : `(v1.<rôle>_crs_auth_name = ? AND v1.<rôle>_crs_code = ?)`, en `OR`,
  comme `buildIntermediateWhere`. C++ filtre aussi `v2`. Ce n'est pas nécessaire :
  la jointure égalise déjà la colonne de `v2` à X.

## Écarts

1. Pas d'arguments `bool` dans la nouvelle API publique (règle de `CLAUDE.md`). Les quatre
   drapeaux du C++ (`usePROJAlternativeGridNames`, `discardIfMissingGrid`,
   `considerKnownGridsAsAvailable`, `discardSuperseded`) ne sont pas exposés : l'appel
   applique leur valeur `false`, c'est-à-dire noms de grille officiels, pas de rejet pour
   grille absente, opérations superseded conservées.
2. Non portés dans cette API : `allowedAuthorities`, `intersectingExtent1/2` et
   `skipIntermediateExtentIntersection`. Avec leurs valeurs par défaut du C++, ils n'ont
   aucun effet. `allowedIntermediateObjectType` est lui aussi fixé à sa valeur par défaut
   `ObjectType::CRS`, qui n'applique pas la restriction aux CRS géographiques.
3. Le C++ attache la base à un handle SQLite en mémoire. Ici la base est un fichier
   temporaire, ouvert en lecture seule après écriture. Le résultat des factories est le même.
4. Les déclencheurs sont gardés (`createStructure` les exécute). L'aide `create_empty_structure`
   de `tests/unit_database_open.rs` les omet, comme le test C++ d'`attachExtraDatabases`. Elle
   n'est pas modifiée ici.

## Hors lot

Restent ouverts dans `docs/UNPORTED-UNIT.md` (section `test_factory.cpp`, 10 lignes retirées) :

- `AuthorityFactory_proj_based_transformation`, `AuthorityFactory_wkt_based_transformation`,
  `..._not_wkt`, `..._not_co_wkt` ;
- `lookForGridInfo`, `lookForGridInfo_from_old_name_with_new_grid_available` ;
- `custom_geodetic_crs`, `custom_projected_crs`, `custom_derived_projected_crs` ;
- `CoordinateMetadata`, `createOperations_exact_transform_not_whole_area`,
  `check_fixup_direction_concatenated_inverse_map_projection`.

## Commandes

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
  PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --test unit_factory_tmp_db
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Pour la suite complète, sauf `docs_in_sync` et `progress_svg` (les compteurs de
`README.md` et `ROADMAP.md` ne sont pas régénérés dans ce lot) :
`cargo test --all-features`, avec le même `PATH` et `PROJ_DATA`.
