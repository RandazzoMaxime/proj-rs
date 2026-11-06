# Port — compound to compound, second lot (`createOperationsCompoundToCompound` tests)

Source: PROJ 9.9.0, fetched from the tag `9.9.0` of OSGeo/PROJ into a scratch directory:
`src/iso19111/operation/coordinateoperationfactory.cpp` (`createOperationsCompoundToCompound`
7214–7847, `createOperationsCompoundToGeog` 6454–7000 for the vertical branch it relies on,
`createHorizVerticalHorizPROJBased` 2795, `createHorizNullVerticalPROJBased` 2884) and
`test/unit/test_operationfactory.cpp`.

This lot is a test lot. The code of `compound.rs` already follows the C++ branches of
`createOperationsCompoundToCompound` (see the function map below). The tests added here
pass on that code, and no line of `src/` changed. The lot closes five unit tests.

Note on the earlier doc `PORTING-compound-to-compound.md`: its "not ported" list is out of date.
The geoid trigger (`has_geoid`), the three-pass selection with `best`, `through_crs`,
`two_steps` (`createOpsInTwoSteps`), the interpolation CRS of a vertical operation
(`horiz_vertical_horiz_checked`) and the vector interpolation (BD72 + Ostend) are in
`compound.rs` now. What is still approximated is listed under "Open" below.

## Function map

| PROJ 9.9.0 | Rust (`src/iso19111/compound.rs` unless noted) |
|---|---|
| `createOperationsCompoundToCompound` (7214) | `compound_to_compound` (286) |
| intermediate geographic CRS, `opsGeogSrcToGeogDst` (7226–7236) | `compound_to_compound`, `inter_src`, `inter_dst`, `geog_ops` |
| bound hub branch, `hub3D` (7290–7324) | `compound_to_compound`, the `[Bound, Bound]` pattern |
| geoid trigger, `bTryThroughIntermediateGeogCRS` (7326–7396) | `compound_to_compound`, `has_geoid`, `cgvd_transition` (669) |
| `createWithIntermediateCRS` (7398–7420) | `through_crs` (576) |
| three-pass selection, `bestAccuracy`, `bestStepCount` (7452–7516) | `compound_to_compound`, `best` (`improves`, 649) |
| `createOpsInTwoSteps` (7547–7620) | `two_steps` (596) |
| interpolation CRS of a vertical operation, `setInterpolationCRSName` (7660–7800) | `compound_to_compound`, `interpolation_of` (635), `horiz_vertical_horiz_checked` (792) |
| `createHorizVerticalHorizPROJBased` (7759) | `horiz_vertical_horiz_checked`, `proj_based_horiz_vertical_horiz` |
| BD72 + Ostend vector interpolation (7802–7835) | `compound_to_compound`, `vector_interpolation` loop |
| `createComputeMetadata` (`ConcatenatedOperation`) | `compute_metadata` (686) |
| `getAccuracy`, `getStepCount` | `get_accuracy` (`compound.rs`), `get_step_count` (783) |
| `createOperationsCompoundToGeog`, vertical branch (6741–6905) | `compound_to_geog3d` (38), `compound_vertical_pass` (150) |
| `createOperationsVertToVert`, ballpark change of unit | `vertical_to_vertical` (1547) |

The tests are in `tests/unit_op_compound_to_compound_2.rs`. Each carries its PROJ `TEST()` name in
its doc comment, and its expected names and PROJ strings are copied from
`test/unit/test_operationfactory.cpp` at 9.9.0.

## Closed (5 unit tests, passing)

| PROJ `TEST()` | Test |
|---|---|
| `compoundCRS_to_compoundCRS` | `operation_compound_to_compound_utm31_to_utm32_with_odn_height` (`EPSG:32631+5701` to `EPSG:32632+5701`, `isEquivalentTo` of the horizontal conversion). PROJ builds the in-memory CRS, the registry ones have the same definitions. |
| `compoundCRS_to_compoundCRS_issue_2232` | `operation_compound_to_compound_issue_2232` (WKT, same candidate count both ways) |
| `compoundCRS_to_compoundCRS_issue_3191_BD72_Ostend_height_to_WGS84_EGM96` | `operation_compound_to_compound_issue_3191_bd72_ostend_height_to_wgs84_egm96` (name and PROJ string, both ways) |
| `compoundCRS_to_compoundCRS_issue_4550` | `operation_compound_to_compound_issue_4550` (WKT NAD83(HARN) ftUS to UTM 10N + NAVD88) |
| `compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height` | `operation_compound_to_compound_wgs84_egm2008_to_rd_new_nap_height` (four candidates, names, PROJ string, accuracy). The accuracy is `accuracy()` as a number: PROJ's `coordinateOperationAccuracies()[0]->value()` is a string, and the concatenated accuracy is stored as an `f64` here. |

## Open (8 unit tests, `#[ignore]` with the cause in the reason)

The expected values are PROJ's. Each test is kept, so `cargo test` shows what is missing.

| PROJ `TEST()` | What the Rust gives | Cause |
|---|---|---|
| `compoundCRS_to_compoundCRS_WGS84_EGM96_to_WGS84_Belfast` | first candidate is the ballpark `Transformation from EGM96 height to Belfast height` | The geog 3D to `EPSG:4326+5732` operations hold only the ballpark Belfast change. PROJ's `ETRS89-IRE [ETRF2000] to Belfast height (2)` with the interpolation CRS is not built on this path (compound-to-geographic branch). Not traced. |
| `compoundCRS_to_compoundCRS_issue_3328` | ballpark `EGM96 height to CGVD28 height` | Same compound-to-geographic path. The NAD83(CSRS) route is missing. |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_ballpark_geog` | ballpark `CGVD28 to CGVD2013(CGG2013)` | Same path. PROJ chains the NAD83(CSRS)v6 offset. |
| `compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform` | ballpark `Kumul 34 to EGM2008` first | PROJ's only-vertical-ballpark branch (`createOperationsCompoundToGeog`, 6849–6905, with the intermediate geographic CRS of the horizontal transforms) is not ported. |
| `compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96` | `OSGB36 to WGS 84 (9)` in the chain | PROJ chains `OSGB36 to ETRS89-GBR [OSNet v2009] (2)` then `ETRS89-GBR to WGS 84 (1)`. The choice between the two is not traced. |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp` | one more pair in the chain: `Inverse of BD72 to ETRS89-BEL (3) + BD72 to ETRS89 (3)` | Not traced (the vertical loop or the vector interpolation). |
| `compoundCRS_to_compoundCRS_context` | no `NGVD29 height (ftUS) to NAVD88 height (3)` candidate; only ballpark vertical ones | The registry vertical candidate with the VERTCON interpolation is not built. |
| `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound` | no candidate | See below. |

### Two defects found on the way (not fixed in this lot)

1. **`Crs::is_equivalent_to_with` has no geographic arm** (`src/iso19111/equiv.rs`, around line 393).
   Under `Equivalence::Equivalent`, a geographic or geodetic pair falls through to the strict
   `is_equivalent_to`, which compares CS axis names and the CRS name. So a bound hub
   `WGS 84` (axes `Latitude`/`Longitude`) is not equivalent to the `WGS 84` of `EPSG:9518`
   (`Geodetic latitude`/`Geodetic longitude`), and the bound branch of `compound_to_compound`
   is skipped for issue_3152. The fix is a short arm for `Geographic`, `Geodetic` and
   `GeodeticCRS::Geographic` pairs that calls `is_equivalent_to_with(criterion)`. It was tried:
   the full suite stays green, but issue_3152 then gives a candidate that loses the LN02 height
   (defect 2). The fix was reverted and is not in this branch.
2. **`bound_compound_to_geog3d` drops the vertical part** (`compound.rs`, around line 1717).
   The compound CRS of the bound `CH1903 / LV03` and `LN02 height` to `EPSG:4979` gives only
   `Inverse of Map projection of CH1903 / LV03 + MyTransformation from CH1903 to WGS84`. This is
   already the case on the code before this lot (checked with the equivalence change reverted),
   so it is a silent wrong result for those pairs, not a new one.

## Deliberate differences and not ported

- `createOperationsCompoundToGeog`: the `SetSkipHorizontalTransform` and `geogCRSOfVertCRSStack`
  contexts, the NAD83(CSRS)v7 hack, the only-vertical-ballpark branch. `compound_vertical_pass`
  is an approximation of 6741–6905, not a line-by-line port. This is the cause of most open cases.
- Point motion (`createOperation_compound_to_compound_with_point_motion_operation`) needs epochs:
  not in this lot.
- Not run by PROJ itself, so not ported as tests:
  - `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` is inside
    `#if no_longer_work_since_epsg_13_001` (PROJ 6470–6524).
  - The second half of `..._and_ballpark_geog` is inside `#if 0` (PROJ 6798–6838). Its first
    half is ported as the open test above.

## Test mapping left open in the CLI ledger

`cs2cs` `#126` (`EPSG:4326+5773` to `EPSG:4326+5782`, default) is still open. Its candidate
`Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) ...` needs
the same compound-to-geographic path as the open cases above. `docs/UNPORTED-CLI.md` is unchanged.

## Commands run

- `git checkout -B lot-compound-to-compound-2 4de4e6e` (HEAD was already `4de4e6e`).
- Baseline: `cargo test --test test_operationfactory` (9 passed), `cargo test --test cli_yaml`
  (passed, `docs/UNPORTED-CLI.md` unchanged).
- `cargo test --test unit_op_compound_to_compound_2` (5 passed, 8 ignored).
- `UPDATE_DOCS=1 cargo test --test docs_in_sync`, then `cargo test --test docs_in_sync` (2 passed).
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test --all-features`: see the lot report.
- A full run with the equivalence arm applied (defect 1, not kept): all existing tests passed, and
  only the new file had failures.
