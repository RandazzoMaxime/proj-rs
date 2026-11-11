# Porting sheet: normalize for visualization (lot `unit-capi-visualization-alternatives`)

Branch `unit-capi-visualization-alternatives`, base `origin/main`. PROJ 9.9.0 tests about
`normalizeForVisualization` (longitude or easting first). One commit per cause.

Tests written, not run by the author: the coordinator runs them.

## Coordinator run, round 1

Output of the coordinator's run, not in the tree.

| Test | Result | Our output | Handling in this branch |
|---|---|---|---|
| `capi_proj_normalize_for_visualization` | ok | | kept |
| `capi_proj_normalize_for_visualization_with_alternatives` | ok | | kept |
| `capi_proj_normalize_for_visualization_with_alternatives_reverse` | ok | | kept |
| `operation_normalize_for_visualization` | FAILED, block 1 | the read-back string of the normalized operation has `+step +proj=axisswap +order=2,1` in front; the in-memory one does not | cause fixed (commit 75d9dad, below); blocks 5 and 6 removed (see below) |
| `crs_normalize_for_visualization_derivedprojected_operation` | FAILED | `PROJ: Format("the source CRS is not geographic")` at the export of the operation, before any normalization | test and its code removed, not closed (below) |

## PROJ functions read

| PROJ 9.9.0 | Lines read | Use |
|---|---|---|
| `test/unit/test_c_api.cpp` `TEST_F(CApi, proj_normalize_for_visualization*)` | 4363-4462 | the three CApi tests |
| `test/unit/test_operation.cpp` `TEST(operation, normalizeForVisualization)` | 5607-5768 | operation test |
| `test/unit/test_crs.cpp` `TEST(crs, normalizeForVisualization_derivedprojected_operation)`, `createDerivedProjectedCRSNorthingEasting` | 7681-7705, 4817-4829 | derived projected test (removed) |
| `src/iso19111/operation/concatenatedoperation.cpp` `ConcatenatedOperation::fixSteps` | 150-509 (rest not read) | commit 2 |
| `src/iso19111/io.cpp` WKT import of a concatenated operation | 3790-3857 | the import calls `fixSteps(..., true)` on the SOURCECRS and TARGETCRS read |
| `src/iso19111/crs.cpp` `CRS::applyAxisOrderReversal` | 1015-1125 | commit 75d9dad |
| `src/iso19111/crs.cpp` `mustAxisOrderBeSwitchedForVisualization` | 866-947 | checked, no change |
| `src/iso19111/operation/singleoperation.cpp` `CoordinateOperation::normalizeForVisualization` | 415-464 | checked, same rule as `normalize.rs` |
| `include/proj/internal/crs_internal.hpp` `AXIS_ORDER_REVERSED_SUFFIX_STR` | 39 | commit 75d9dad |
| `src/coordinates.cpp` `proj_degree_input`, `proj_degree_output` | 75-94 | commit 1 |
| `src/proj_internal.h` default `left` / `right` of a PJ | 596-600 | a CRS object reads no angular unit |
| `src/trans.cpp` selection of an alternative (`iCurCoordOp`) | 233-376 (grep, then read) | open part of commit 1 |
| `src/iso19111/io.cpp` `PROJStringFormatter` simplifications | 9300-9489 (grep, then read) | checked for block 1 |
| `src/iso19111/conversion.cpp` `Conversion::_exportToPROJString` | 3768-3937, 4395-4420 | checked for the derived case |

## Function map

| PROJ 9.9.0 | Rust | Status |
|---|---|---|
| `proj_degree_input(P, dir)` | `Proj::degree_input(Direction)` (`src/proj.rs`) | ported, public |
| `proj_degree_output(P, dir)` | `Proj::degree_output(Direction)` (`src/proj.rs`); the private copy in `src/proj/bounds.rs` is removed | ported, public |
| `ConcatenatedOperation::fixSteps`, the axis-order ends (302-330) | `wkt_import.rs` `fix_axis_order_ends` | ported |
| `ConcatenatedOperation::fixSteps`, intermediate conversion (461-513) | `wkt_import.rs` `fix_intermediate_conversion` | ported, the EPSG:7989 name check included |
| `ConcatenatedOperation::fixSteps`, transformation direction (332-360, 559-563) | `wkt_import.rs` `fix_transformation_direction` | unchanged, see deviations |
| `CRS::applyAxisOrderReversal` (1015-1125) | `normalize.rs` `reversed`, `reverse_identified` | ported for geographic and projected CRS |
| `CRS::mustAxisOrderBeSwitchedForVisualization` (869-947) | `normalize.rs` `must_axis_order_be_switched` | north/east only, see open |
| `proj_normalize_for_visualization` (`c_api.cpp`) | no public call on `Proj`: `Proj::new_known_crs` gives the normalized pair | see deviations |

## Commit 1: `degree_input` and `degree_output` on `Proj`

`proj_degree_output(P, dir)` is `proj_degree_input(P, opposite(dir))`; `proj_degree_input(P, FWD)` is
`pj_left(P) == DEGREES`. The facade reads the same rule: `Body::Native` through `Operation::degree_output`,
`Body::Iso` through `IsoPipeline::degree_output` (target of a forward, source of an inverse, geographic),
`Body::Single` (a CRS object, `left` and `right` are `PJ_IO_UNITS_WHATEVER`) gives `false`.

Tests (`tests/unit_capi_normalize_alternatives.rs`), all three passed in round 1:

| Rust test | PROJ TEST | Status |
|---|---|---|
| `capi_proj_normalize_for_visualization` | `CApi.proj_normalize_for_visualization`, second block | ok (round 1) |
| `capi_proj_normalize_for_visualization_with_alternatives` | `CApi.proj_normalize_for_visualization_with_alternatives` | ok (round 1) |
| `capi_proj_normalize_for_visualization_with_alternatives_reverse` | `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | ok (round 1) |

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

Cause (reading of the code, not measured): the normalized operation of a pair with an axis-order change is
`[reversal(source), steps..., reversal(target)]`. The import of WKT2_2019 gives the inner steps no CRS pair,
so an inner conversion (the UTM step of block 3, for instance) has none after import. The Rust `fix_steps`
skipped the intermediate conversions, so they kept none, and the export refuses them.

PROJ order, kept here: the axis-order reversals at the two ends are paired first (302-330), then the loop
reads the neighbours of each intermediate conversion (461-513).

Changes in `src/iso19111/wkt_import.rs`: `fix_axis_order_ends` (before the loop), `is_bare_axis_order_reversal`
(method 9843 or 9844, no pair), `fix_intermediate_conversion` (projected-to-geographic inverts, otherwise the pair
is the neighbours; the name check of PROJ issue 4116 (EPSG:7989) included), `set_operation_crss`,
`is_geographic_crs`.

Callers whose behaviour this can alter: `fix_steps` has one caller, the concatenated WKT import
(`wkt_import.rs`, near line 3554; C++ `io.cpp` 3855). A pair changes the PROJ string only where the export
uses it (a projection, a geographic offset, or an axis-order reversal with units to convert). Equivalence
checks that read the pair of an inner conversion may change: not measured.

Not ported (in the function, not in this commit): the height-depth branch of the last step (`crs` EPSG:7987,
`concatenatedoperation.cpp` 514-544), the derived-CRS branches of `i == 0` and `i == last` beyond the existing
subset, the `prevOpTarget == nullptr` throw (`fix_transformation_direction` returns instead).

## Commit 75d9dad: `applyAxisOrderReversal` swaps the axes whatever their order

Cause, from `concatenatedoperation.cpp` 311 and 326 (`applyAxisOrderReversal`) and `crs.cpp` 1015-1125: the
reversal the import makes of the first and last step is a true reversal. Our `reversed` called
`normalize_for_visualization`, which swaps only a north-first axis pair. On a read-back normalized source (already
east first) it returned the CRS unchanged, so the reversal step got a pair with the same axis order at both ends,
and the projection after it was exported as if its source were east first. The output lost one `axisswap` at the
front (the round-1 failure of block 1).

Port: `reversed` swaps the two horizontal axes of a geographic or projected CRS (any axis order, the third axis
kept), `reverse_identified` gives the name suffix toggle (`NORMALIZED`, or `" (with axis order reversed)"`
removed), the remarks of `createProperties` (crs.cpp 1044-1060), and no identifier.

Callers: `reversed` is used by `normalize_for_visualization` (only on a north-first side, so its result is the same
as before) and by the new read-back code of commit 2 (`fix_axis_order_ends`). No other caller.

## Test removed: `operation.normalizeForVisualization`, blocks 5 and 6

Case 5 (compound source, `setUsePROJAlternativeGridNames(false)`): not in the file. Its list order is the open
grid-order cause of `docs/PORTING-strings-misc.md` (`operation.vertCRS_to_geogCRS_context`), which uses the same
setup.

Case 6 (bound source, `BoundCRS::createFromTOWGS84(NAD83, 1..7)` to `EPSG:4326`): removed, not verified. Our
normalization follows C++ here: a bound source is not switched (`crs.cpp` 918-947 has no bound branch), the target
EPSG:4326 is. The expected PROJ string of the C++ test has no `axisswap` at the front, whereas a geographic
source of a transformation exported by `transform_export.rs` may write one. That export was not traced.

Cases 1 to 4 and 7 of the C++ test are in `tests/unit_op_normalize_for_visualization.rs`. Round 1 failed at case 1
(the round trip, fixed by 75d9dad). Cases 2, 3, 4 and 7 were not reached by the round-1 run: they are unverified.

## Test removed: `crs.normalizeForVisualization_derivedprojected_operation`

Round-1 output: `PROJ: Format("the source CRS is not geographic")`, raised at line 51 of the test, which is the
export of the operation before normalization (`export_to_proj_string_with_database` of
`create_operation(EPSG:4326, derived projected)`). PROJ's string for that export is test_crs.cpp 7687-7691.

Trace, partial. The message is raised by `projection_steps` (`projstring.rs` 580-584) when a conversion whose
target is projected has a source that is not geographic. The conversion is written by `export_projection`
(`conversion_export.rs` 141-168). The conversions of a derived target come from `derived.rs`
(`derived_candidates`, the inverse of the candidates from the derived CRS, `derived_parts` for the pair). The
step with the non-geographic source was not identified. In C++ the deriving conversion of a derived target is
exported with the target modifiers (`conversion.cpp` 4422-4450, the derived projected branch), and an unknown
method goes through `exportToPROJStringGeneric` (`conversion.cpp` 4415). The fix is in the factory's derived path
and the export of its pairs, larger than this lot. Not closed.

The derived projected branch added in the lot (`must_axis_order_be_switched` and `reversed` for
`Crs::DerivedProjected`, C++ crs.cpp 940-944) is removed with the test: it served only this test.

## Open

- `proj_pj_info` after `proj_trans` (commit 1, see above).
- Operation case 5 (compound source, grid order), and case 6 (bound source), not in the tree.
- Derived projected operation export (see above), not closed.
- `must_axis_order_be_switched` (operation level, `normalize.rs`) tests north then east only. PROJ's
  `mustAxisOrderBeSwitchedForVisualizationInternal` (crs.cpp 878-912) also switches EPSG:32661, EPSG:32761 and
  EPSG:5482 by their meridians. The facade's version (`proj_iso.rs` `must_switch_for_visualization`) has them.
  Not in this lot.
- The facade boundary (`proj_iso.rs` `Side::of`) does not switch derived projected CRS either. Not in this lot.

## Existing callers at risk (summary)

- `fix_steps` (concatenated WKT import): commit 2.
- `reversed` (`normalize.rs`): commit 75d9dad, see above.
- `Proj::degree_output`: the private copy in `src/proj/bounds.rs` is replaced by the public method, same body.
- `normalize_for_visualization` (operation level): `src/proj_export.rs`, `src/iso19111/identify.rs`. Unchanged
  for every pair in the tree since the derived branch was removed.
