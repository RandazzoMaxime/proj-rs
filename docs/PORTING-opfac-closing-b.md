# Port — fermeture des tests opérations restants (lot « closing B »)

Source : PROJ 9.9.0, `test/unit/test_operationfactory.cpp` (lecture seule dans
`target/proj-reference/src/test/unit/`), et le code `src/iso19111/operation/` du même tag pour
les branches citées. Les valeurs mesurées viennent du `projinfo` de référence
(`target/proj-reference/build/bin`, `PROJ_DATA=target/proj-reference/build/data`).

## Bilan

Quatre tests de `docs/UNPORTED-UNIT.md` étaient ouverts. Trois sont fermés, un reste ouvert.

| `TEST()` PROJ 9.9.0 | État | Où |
|---|---|---|
| `operation.compoundCRS_to_compoundCRS_with_vertical_transform` | fermé | `tests/unit_opfactory_closing_b.rs` |
| `operation.compoundCRS_to_compoundCRS_context` | fermé (ancien `#[ignore]` retiré) | `tests/unit_op_compound_to_compound_2.rs` |
| `operation.boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs` | fermé | `tests/unit_opfactory_closing_b.rs` |
| `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` | **ouvert** (noms) | `tests/unit_opfactory_closing_b.rs` |

Les trois lignes fermées sont retirées de `docs/UNPORTED-UNIT.md`. La quatrième reste.

Fichiers absents de l'arbre de travail et cités par la consigne : `tests/unit_op_vertical_bound_context.rs`,
`docs/PORTING-operationfactory-vertical-bound.md`, `docs/PORTING-derived-vertical-projected.md`.
Le test « vertical transform » a donc été écrit dans le nouveau `tests/unit_opfactory_closing_b.rs`.

## 1. `compoundCRS_to_compoundCRS_with_vertical_transform` (fermé)

Cause : `bound_candidates` (`src/iso19111/bound_ops.rs`) ne traitait pas le couple
`BoundCRS` → `VerticalCRS`. `create_candidates(bound, vc2)` rendait zéro opération, donc le
VERTCON du compound source disparaissait.

Correction, selon `createOperationsBoundToVert` (`coordinateoperationfactory.cpp`) :
- `Crs::Bound` vers `Crs::Vertical` : si la base et le hub sont verticaux et que le hub est
  équivalent à la cible, la transformation liée elle-même (`bound_to_vertical`) ; sinon la base
  vers la cible.
- Réciproque `Vertical` vers `Bound` : l'inverse de `Bound` vers `Vertical` (le `applyInverse`
  de PROJ pour `boundDst && vertSrc`).

Export `+approx` au niveau d'une opération (demande du lot) :
- `TmercExport { Exact, Approximate }` (`src/iso19111/pipeline.rs`, exporté dans `iso19111` et
  `lib.rs`). C'est l'enum qui remplace le `TmercApprox` privé de `buildop.rs`.
- `PipelineFormatter::with_tmerc` : `+approx` est inséré juste après `+proj=tmerc` ou `+proj=utm`
  à l'ingestion d'une étape, comme `PROJStringFormatter::setUseApproxTMerc`.
- API publique `CoordinateOperation::export_to_proj_string_with_tmerc(db, lines, TmercExport)`
  (`src/iso19111/transform_export.rs`). `export_to_proj_string_with_database` appelle la même
  fonction en `TmercExport::Exact`.

Test : `operation_compound_to_compound_with_vertical_transform`. Les CRS sont lus en WKT avec
`METHOD["VERTCON",ID["EPSG",9658]]` et `PARAMETERFILE` d'identifiant `EPSG:8732`, les codes que
`Transformation::createVERTCON` pose. Sans ces identifiants, `epsg_method_code` ne reconnaît pas
la méthode et l'export répond `Unimplemented`.

## 2. `compoundCRS_to_compoundCRS_context` (fermé)

Cause : le premier candidat est une opération PROJ-based (`proj_based`) que la crate écrit en
WKT comme une liste d'étapes. PROJ écrit une seule `COORDINATEOPERATION` dont la méthode est
`PROJ-based operation method: <chaîne PROJ>` (`PROJBasedOperation::_exportToWKT`,
`projbasedoperation.cpp:194`, `createPROJBased` `coordinateoperationfactory.cpp:2868`). Le
round trip WKT2 réordonnait donc les étapes (unité verticale et `vgridshift` avant
`axisswap`).

Correction :
- `ConcatenatedOperation::proj_based_transformation` (`transform_export.rs`) construit la
  transformation avec la méthode PROJ-based, les deux CRS, la précision et les identifiants.
- `export_concatenated` (`wkt.rs`) l'écrit par `export_transformation` quand la paire de CRS est
  présente.

Changement de comportement : le WKT d'une opération PROJ-based est désormais une
`COORDINATEOPERATION` et non une `CONCATENATEDOPERATION`. Le test de round trip importe donc une
transformation. L'ancien test ignoré (`#[ignore]` retiré dans `tests/unit_op_compound_to_compound_2.rs`)
couvre les 152 candidats et la forme `+approx`.

## 3. `boundCRS_to_derived_geog_with_transformation_..._base_crs_of_source_crs` (fermé)

PROJJSON d'entrée repris du C++. Le résultat donnait `+proj=geogoffset +dh=-10` sans `+inv`, à
la place de `+step +inv +proj=geogoffset +dlat=0 +dlon=0 +dh=10`, les deux étant équivalents.

Cause : `PipelineFormatter::normalize_inverse` (`src/iso19111/pipeline.rs`) réécrivait une étape
`geogoffset` inversée en changeant le signe de ses valeurs. PROJ ne le fait pas : son
`stopInversion` ne fait que basculer le drapeau `inv` (`io.cpp`, sans entrée pour `geogoffset`).
La branche `geogoffset` est supprimée. Vérifié : `unit_op_vert_pivot`, `unit_capi_factory_core`,
`unit_op_compound_derived` et la suite complète passent.

## 4. `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` (ouvert)

Ce test est compilé hors de PROJ 9.9.0 (`#if no_longer_work_since_epsg_13_001`, pull 4803). Le
port suit le comportement mesuré de 9.9.0, avec :

```
projinfo -s EPSG:8360 -t EPSG:7423 -o PROJ --grid-check none --spatial-test intersects
```

9.9.0 donne quatre candidats :
1. `Baltic 1957 height to EVRF2007 height (1)`, `vertoffset` (Tchéquie). Pipeline identique au C++.
2. `Inverse of 'ETRS89-SVK [SKTRF09] to Baltic 1957 height (...)' + 'ETRS89-SVK [SKTRF09] to EVRF2007 height (...)'`,
   pipeline identique au C++ (Slovaquie).
3. Un candidat Slovaquie-Tchéquie, avec grilles absentes.
4. Le ballpark, `+proj=noop`.

Le test fermé vérifie le nombre (quatre), le premier candidat (chaîne et nom), la chaîne du
deuxième. Le nom du deuxième candidat et le nom de son inverse sont dans le test ignoré
`operation_compound_to_compound_concatenated_two_vert_transformation_names`.

Écart non résolu : la deuxième étape est nommée `Inverse of 'Inverse of ...'` par la crate,
où PROJ 9.9.0 la nomme `'...'`. Le pipeline est le même, mais l'orientation de l'étape (drapeau
`reverse` ou `oriented_operation`) n'est pas tracée. Le nom de `list[1].inverse()` diffère pour la
même raison. Ce n'est pas un écart volontaire, donc la ligne reste dans `docs/UNPORTED-UNIT.md`.

## Commandes lancées

- `cargo test --test unit_op_vertical_bound_context` (fichier renommé ensuite en
  `unit_opfactory_closing_b`), puis `cargo test --test unit_opfactory_closing_b` : trois passent,
  un ignoré.
- `cargo test --test unit_op_compound_to_compound_2 operation_compound_to_compound_context -- --ignored`
  avant la correction : échec au round trip. Après : passe.
- `cargo fmt`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` : propres.
- Suite complète avec `PATH` et `PROJ_DATA` de référence, `PROJ_CS2CS_REQUIRED=1`,
  `--no-fail-fast` : tout passe, sauf `docs_in_sync` et `progress_svg`.
  Ces deux-là échouent parce que `docs/UNPORTED-UNIT.md` a perdu trois lignes et que
  `UPDATE_DOCS` n'a pas été lancé, comme demandé pour ce lot. Il revient au mainteneur de
  régénérer les chiffres.

## Fiches voisines

`docs/PORTING-compound-to-compound-2.md` classe encore `compoundCRS_to_compoundCRS_context` comme
ouvert. Ce n'est plus le cas, et la présente fiche prend le relais.
