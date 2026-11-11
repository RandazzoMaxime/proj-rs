# Porting sheet: normalize for visualization (lot `unit-capi-visualization-alternatives`)

Branch `unit-capi-visualization-alternatives`, base `origin/main`. Four PROJ 9.9.0 tests about
`normalizeForVisualization` (longitude or easting first). One commit per cause.

Tests written, not run by the author: the coordinator runs them.

## PROJ functions read

| PROJ 9.9.0 | Lines read | Use |
|---|---|---|
| `test/unit/test_c_api.cpp` `TEST_F(CApi, proj_normalize_for_visualization*)` | 4363-4462 | the three CApi tests |
| `test/unit/test_operation.cpp` `TEST(operation, normalizeForVisualization)` | 5607-5768 | operation test |
| `test/unit/test_crs.cpp` `TEST(crs, normalizeForVisualization_derivedprojected_operation)`, `createDerivedProjectedCRSNorthingEasting` | 7681-7705, 4817-4829 | derived projected test |
| `src/iso19111/operation/concatenatedoperation.cpp` `ConcatenatedOperation::fixSteps` | 285-509 (rest not read) | commit 2 |
| `src/iso19111/crs.cpp` `mustAxisOrderBeSwitchedForVisualization` | 866-947 | commit 3 |
| `src/coordinates.cpp` `proj_degree_input`, `proj_degree_output` | 75-94 | commit 1 |
| `src/proj_internal.h` default `left` / `right` of a PJ | 596-600 | a CRS object reads no angular unit |
| `src/trans.cpp` selection of an alternative (`iCurCoordOp`) | 233-376 (grep, then read) | open part of commit 1 |
| `src/iso19111/operation/singleoperation.cpp` `normalizeForVisualization` | 418-445 (grep only) | not changed |

## Function map

| PROJ 9.9.0 | Rust | Status |
|---|---|---|
| `proj_degree_input(P, dir)` | `Proj::degree_input(Direction)` (`src/proj.rs`) | ported, public |
| `proj_degree_output(P, dir)` | `Proj::degree_output(Direction)` (`src/proj.rs`); the private copy in `src/proj/bounds.rs` is removed | ported, public |
| `ConcatenatedOperation::fixSteps`, the axis-order ends (302-330) | `wkt_import.rs` `fix_axis_order_ends` | ported |
| `ConcatenatedOperation::fixSteps`, intermediate conversion (461-513) | `wkt_import.rs` `fix_intermediate_conversion` | ported, the EPSG:7989 name check included |
| `ConcatenatedOperation::fixSteps`, transformation direction (332-360, 559-563) | `wkt_import.rs` `fix_transformation_direction` | unchanged, see deviations |
| `CRS::mustAxisOrderBeSwitchedForVisualization` (869-947) | `normalize.rs` `must_axis_order_be_switched` | the derived projected branch added; north/east only, see open |
| `CRS::applyAxisOrderReversal` (for the kinds above) | `normalize.rs` `reversed` | derived projected branch added |
| `proj_normalize_for_visualization` (`c_api.cpp`) | no public call on `Proj`: `Proj::new_known_crs` gives the normalized pair | see deviations |

## Commit 1: `degree_input` and `degree_output` on `Proj`

`proj_degree_output(P, dir)` is `proj_degree_input(P, opposite(dir))`; `proj_degree_input(P, FWD)` is
`pj_left(P) == DEGREES`. The facade reads the same rule: `Body::Native` through `Operation::degree_output`,
`Body::Iso` through `IsoPipeline::degree_output` (target of a forward, source of an inverse, geographic),
`Body::Single` (a CRS object, `left` and `right` are `PJ_IO_UNITS_WHATEVER`) gives `false`.

Tests (`tests/unit_capi_normalize_alternatives.rs`):

| Rust test | PROJ TEST | Status |
|---|---|---|
| `capi_proj_normalize_for_visualization` | `CApi.proj_normalize_for_visualization`, second block | expected to pass; unverified |
| `capi_proj_normalize_for_visualization_with_alternatives` | `CApi.proj_normalize_for_visualization_with_alternatives` | the coordinate and degree checks; expected to pass, unverified |
| `capi_proj_normalize_for_visualization_with_alternatives_reverse` | `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | expected to pass; unverified |

Deviations of the first test:

- The first block (`proj_create("+proj=utm ...")` normalized to a null object) has no counterpart: the facade
  normalizes CRS pairs at creation, and has no normalize call on one PROJ-string operation.
- `proj_as_proj_string` has no facade counterpart. The second block exports the normalized operation with
  `export_to_proj_string_with_database`, the same operation the facade transforms with.

Open, not in the tree (not written, to avoid a known-failing test):

- `proj_pj_info(Pnormalized).definition` after the forward `proj_trans` of `..._with_alternatives`
  (`test_c_api.cpp` 4414-4423). PROJ's `proj_trans` picks the alternative for the point and stores it in `P`
  (`trans.cpp` 233-348, `P->iCurCoordOp`), then `proj_pj_info` reports that candidate (`info.cpp` 169-176).
  `Proj::convert` and `Proj::def` take `&self` and store nothing, so `def()` keeps its deferred text,
  `unavailable until proj_trans is called`. Needed: a recorded selection on `Proj` (interior mutability), a
  decision for the coordinator, not a change of the test.

## Commit 2: `fixSteps` of a concatenated operation read from WKT2_2019

Cause, from reading the code (not measured): case 3, 4326 to EPSG:3040, source and target inverted. The
normalized operation is `[reversal(source), T, C, reversal(target)]`. The import of WKT2_2019 gives the inner
steps no CRS pair (the WKT of an inner conversion step carries none, as the existing `fix_steps` assumes), so
on import `C` (the UTM conversion) has no pair, and the last reversal `R2` gets the pair (ETRS89 base,
projected target) because the previous step has no target. The Rust `fix_steps` skipped the intermediate
conversions, so `C` kept no pair. The axis-order export of `R2` with a geographic source and a projected target
is the refusal the sheet of `unit-strings-misc` describes. The round trip (`checkThroughWKTRoundtrip`) is the
first assertion that reaches that export.

PROJ order, kept here: the axis-order reversals at the two ends are paired first (302-330), then the loop
reads the neighbours of each intermediate conversion (461-513). The loop does the same for the rest of the steps,
as before.

Changes in `src/iso19111/wkt_import.rs`: `fix_axis_order_ends` (before the loop), `is_bare_axis_order_reversal`
(method 9843 or 9844, no pair), `fix_intermediate_conversion` (projected-to-geographic inverts, otherwise the pair
is the neighbours; the name check of PROJ issue 4116 (EPSG:7989) included), `set_operation_crss`,
`is_geographic_crs`. `src/iso19111/normalize.rs`: `reversed` is `pub(crate)`.

Callers whose behaviour this can alter: `fix_steps` has one caller, the concatenated WKT import
(`wkt_import.rs`, near line 3554). Every concatenated WKT read that has an inner conversion or an
axis-order reversal at an end gets pairs. The PROJ string changes only where a pair is now used by the export
(a projection, a geographic offset, or an axis-order reversal with units to convert); before, those exports
were refused. Equivalence checks that read the pair of an inner conversion may also change: not measured.

Not ported (in the function, not in this commit): the height-depth branch of the last step (`crs` EPSG:7987,
`concatenatedoperation.cpp` 514-544), the derived-CRS branches of `i == 0` and `i == last` beyond
`fix_steps`' existing subset, the `prevOpTarget == nullptr` throw (`fix_transformation_direction` returns instead).

Test (`tests/unit_op_normalize_for_visualization.rs`): `operation_normalize_for_visualization` = the C++ TEST
blocks 1, 2, 3, 4 (the no-inversion block), 6, 7. Case 5 (compound source) is not in the file: its list
order is the open grid-order cause of `docs/PORTING-strings-misc.md` (`operation.vertCRS_to_geogCRS_context`),
which uses the same `setUsePROJAlternativeGridNames(false)` setup. Writing it now would make it fail on that
cause. Expected to pass: blocks 1 to 4 and 6 to 7 are unverified; the round trip of blocks 1 to 3 is
the part this commit targets.

## Commit 3: derived projected CRS in the operation reversal

Cause (`crs.normalizeForVisualization_derivedprojected_operation`): `must_axis_order_be_switched` and `reversed`
took only geographic and projected CRS. The derived projected CRS of the test (north, east in foot) is switched
in C++ (`mustAxisOrderBeSwitchedForVisualizationInternal` reads `DerivedProjectedCRS` too, crs.cpp 940-944), so
the target reversal was missing and `normalize_for_visualization` returned `None`.

Changes in `src/iso19111/normalize.rs` only: the `DerivedProjected` arm in both functions.

Callers: `normalize_for_visualization` (operation level) is used by `src/proj_export.rs` (`pipeline.chosen()`),
`src/iso19111/identify.rs` (two calls), and the test `unit_crs_core_2.rs` (EPSG:3035, a projected CRS, not affected).
Only an operation with a derived projected CRS on a north-east side changes. `reversed` has no other caller.

Test (`tests/unit_crs_normalize_derived_operation.rs`): the C++ TEST, both strings. Expected to pass; unverified.
The operation string before normalization depends on `create_operation` of a derived projected target, not checked.

## Open

- `proj_pj_info` after `proj_trans` (commit 1, see above).
- Case 5 of `operation.normalizeForVisualization` (compound source, grid order), not written.
- `must_axis_order_be_switched` (operation level, `normalize.rs`) tests north then east only. PROJ's
  `mustAxisOrderBeSwitchedForVisualizationInternal` (crs.cpp 878-912) also switches EPSG:32661, EPSG:32761 and
  EPSG:5482 by their meridians. The facade's version (`proj_iso.rs` `must_switch_for_visualization`) has them.
  Not in this lot: the operation level would need the same function, one change at a time.
- The facade boundary (`proj_iso.rs` `Side::of`) does not switch derived projected CRS either. Not in this lot.
- Test 1 of the CApi group, block 1 (see deviations): no counterpart.

## Existing callers at risk (summary)

- `fix_steps` (concatenated WKT import): commit 2.
- `Proj::degree_output`: the private copy in `src/proj/bounds.rs` is replaced by the public method, same body:
  `bounds()` calls it with the same arguments.
- `normalize_for_visualization` (operation level): `src/proj_export.rs`, `src/iso19111/identify.rs`: only with a derived
  projected CRS in the pair (commit 3).
