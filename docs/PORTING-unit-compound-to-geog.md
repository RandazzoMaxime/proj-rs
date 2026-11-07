# Status — compound CRS to geographic CRS (lot `compound-to-geog`)

Source: PROJ 9.9.0, tag `9.9.0` of OSGeo/PROJ. The PROJ lines below are from
`src/iso19111/operation/coordinateoperationfactory.cpp` (the factory) and
`test/unit/test_operationfactory.cpp` (the tests).

This lot is a status sheet. Its one code cause, `createOperationsCompoundToGeog`, was not closed in
two attempts: the first attempt closed two tests and regressed three, and the second did not repair
them. The code was reverted. What is committed is one test that passes on the base (a tests-only
commit), and this sheet. The next attempt starts from the "Next step" section.

## Function map (PROJ -> Rust, `src/iso19111`)

| PROJ 9.9.0 (`coordinateoperationfactory.cpp`) | Rust |
|---|---|
| `createOperationsCompoundToGeog`, 6470-7170 | `compound.rs` `compound_to_geog3d` (the vertical branch only), `compound_vertical_pass` |
| `getInterpolationGeogCRS`, 6392-6433 | `compound.rs` `interpolation_geographic` (name-based ballpark check, see deviations) |
| `createHorizVerticalPROJBased`, 2717-2793 | `compound.rs` `combine` (a plain `Concatenated`, not a proj-based operation) |
| `createHorizVerticalHorizPROJBased`, 2795-2880 | `compound.rs` `horiz_vertical_horiz_checked`, `proj_based_horiz_vertical_horiz` |
| `MyPROJStringExportableHorizVertical(HorizVerticalHoriz)PROJBased`, 2384-2561 | `transform_export.rs` `export_proj_based`, `model.rs` `ProjBasedKind` |
| `createOperationsCompoundToGeog`, projected horizontal (no PROJ counterpart) | `compound.rs` `projected_compound_to_geog` (a special case, see findings) |
| `SetSkipHorizontalTransform`, 6745-6784 | `compound.rs` `skip_horizontal_context` |

## Tests

Draft file kept out of the tree (its failing tests are not committed). Scratch copy:
`scratchpad/full_geog.rs` (the compound draft), `scratchpad/full_misc.rs` (the non-metre drafts).

| Test (PROJ `TEST()`) | Status on the base | Cause |
|---|---|---|
| `WGS84_G1762_to_compoundCRS_with_bound_vertCRS` | passes, closed (tests-only commit) | none |
| `compoundCRS_ftUS_to_geogCRS_ft` | open | the last candidate (ballpark vertical change with xy terms over a same-datum height change) is missing for a foot target: the horizontal part is searched against the 2D target |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change` | open (48 vs 49) | same as above, the ftUS candidate |
| `compoundCRS_to_geogCRS_2D_promote_to_3D_context` | open (210 vs 133) | filtering of the 2D promotion candidates; not traced |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` | open (1 vs 16) | projected horizontal: the special case `projected_compound_to_geog` answers, PROJ's general path is not reached |
| `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` | open | ballpark geographic offsets around the geoid vertical are missing (`Ballpark geographic offset from NAD83 to NAD83(2011)` and back), the special case answers instead |
| `compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort` | open | the "... using Amersfoort to ETRS89-NLD [AGRS2010] (8)" naming comes from the interpolation path, which the special case bypasses |
| `compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform` | open | KNOWN_AVAILABLE: PROJ's registry EGM96 chain (`Inverse of EGM96 height to Kumul 34 height (1) + ...`) is not built; ours is the ballpark |
| `compoundCRS_to_geogCRS_3D_context` (first block) | open: first block passes after attempt 1, second block open | second block: see the top-level filter finding |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | open ("the named pipeline" missing) | bound geographic + geoid to a geocentric target: `bound_compound_to_geog3d` path, not traced |
| `compoundCRS_to_proj_string_with_non_metre_height` | open | the target `+vunits=us-ft` is not written: our pipeline ends `unitconvert +xy_in=rad +xy_out=deg`, PROJ adds `+z_in=m +z_out=us-ft` through `vgridshift +grids=us_noaa_g2018u0.tif` |
| `compoundCRS_to_PROJJSON_with_non_metre_height` | open | the geoid is `us_noaa_g2018u0.tif`, PROJ picks `us_noaa_g2012bu0.tif` (the choice comes from the PROJJSON source) and the vertical unit steps differ |

## Attempt 1: `createOperationsCompoundToGeog` ported whole (reverted)

Changes (diff kept in `scratchpad/lot-attempt1-compound.diff`, 599 lines):

- the horizontal operations of the 3D target are searched against the full target
  (`horizTransforms = createOperations(componentsSrc[0], targetCRS)`, 6699-6704), not the 2D one;
- the vertical operations use `SetSkipHorizontalTransform` (6745-6784); the same-geographic case
  `geogCRSTmp` (6814-6835) is ported;
- `onlyVerticalBallparkTransformation` (6844-6919) is ported with its intermediate geographic CRS
  candidates, sorted by name;
- the interpolation loop (6935-7140) is ported: a cache by the interpolation CRS identifier, the
  `datum_names_used` filter when source and target share a geographic CRS, the GTiff filter of the
  interpolation-to-target legs;
- the fallback `createHorizVerticalPROJBased` (7152-7170) is kept, now proj-based (`HorizVertical`),
  with the empty-intersection check;
- the projected special case `projected_compound_to_geog` was deleted (no PROJ counterpart).

Measured on the tests (`scratchpad/lot-att1.log`, against the base):

- closed: `compoundCRS_ftUS_to_geogCRS_ft`, `compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change`;
- `compoundCRS_to_geogCRS_3D_context`: the first block (`... ftUS ...`, the axis swap after the unit
  conversion) now passes; the second block (NAD83(NSRS2007)) fails, see the next finding;
- **regressed (3)**: `unit_op_compound_misc::operation_create_operation_compound_to_compound_with_geographic_3d_offset_by_velocity_grid`,
  `unit_op_geog_point_motion_ctx::operation_compound_to_compound_with_point_motion_operation` and
  `..._special_case_cgvd2013a`. Each fails at the PROJ-string export with
  `CoordinateOperationNNPtr::_exportToPROJString() unimplemented when source coordinate epoch is missing`.

Root of the regression (traced with a temporary print, removed): the projected horizontal of the
source (for example `NAD83(CSRS)v7 / MTM zone 7`) now reaches the general path. Its interpolation
legs contain `Inverse of MTM zone 7 + Inverse of NAD83(CSRS)v7 to CGVD28 height (1) + Canada velocity
grid v7`: the last leg, `createOperations(interpolation 3D, target)`, is a point-motion operation
without epochs. The base never reached this leg, because `projected_compound_to_geog` returned
first. PROJ does not list that candidate in these tests, and the reason is not yet traced (see the
next step).

## Attempt 2: three variants of the attempt-1 code (reverted)

Each variant ran the three affected test files.

1. Horizontal operations against the 2D target (`demote_to_2d`, the base's search): the point-motion
   regression stays; `compoundCRS_ftUS_to_geogCRS_ft` fails again. Not kept.
2. Horizontal operations with the filtered `create_operations`: the regression stays, and the
   complex-horizontal test fails again. Not kept.
3. The interpolation-to-target 3D CRS promoted through the database (`promote_to_3d_with_db`) instead
   of `promote_with_axis` (the height axis of the target): the regression stays, and the ftUS test
   fails. Not kept.

The two attempts of the rule are used. The code is reverted to the base (`git checkout`).

## Findings

1. **Top-level filter, not this lot.** `compoundCRS_to_geogCRS_3D_context`, second block
   (`NAD83(NSRS2007) + NAVD88 height` to `WGS 84`, `list[1]`): with `PartialIntersection`, the
   candidate `Inverse of NAD83(NSRS2007) to NAVD88 height (1) + NAD83(NSRS2007) to WGS 84 (1)` is
   second, as PROJ expects. With the default criterion (PROJ's test uses `create(authFactory, nullptr,
   0.0)`, and PROJ keeps the candidate under its strict containment) `filter_and_sort` reduces 230
   candidates to 4 (`factory.rs`, `select_candidates`). The suspect is the extent of the compound source
   (`sort_extents` / `crs_extent` of a compound), not the compound code.
2. **`projected_compound_to_geog` is a special case that PROJ does not have.** It answers for a
   projected horizontal of the same datum, and it blocks the ballpark offsets and the interpolation
   naming of the geoid and Amersfoort tests. Removing it exposes the point-motion leg (finding above).
3. **The ballpark geographic offsets** of `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` are the
   legs `createOperations(componentsSrc[0], interpolation)` and `createOperations(interpolation3D,
   targetCRS)` of the interpolation path. With the port they come out as registered offsets
   (`NAD83 to NAD83(2011) (1)`), not as the ballpark offsets PROJ shows, so the leg lookup differs.
4. **The NAD83(CSRS)v7 hack** (`createOperationsCompoundToGeog`, 6500-6560) needs the source and
   target epochs; the port has no epochs in the compound path. Not ported.
5. **Not ported** in attempt 1, kept as in the base: the derived horizontal branch (6480-6510),
   `geogCRSOfVertCRSStack` (6758-6770), the bound-horizontal branches (kept in
   `bound_compound_to_geog3d`), the null-horizontal wrapper of `createHorizVerticalPROJBased`
   (`horizTransformIsNoOp` returns the vertical operation under the compound source, as in the base).
6. **Deviation kept**: `interpolation_geographic` also rejects a transformation whose name contains
   `ballpark`; PROJ only tests `hasBallparkTransformation`.

## Next step

1. Trace, with `PROJ_DEBUG=3` on `scratchpad/lot/projtrace-build/bin/projinfo`, the candidate list of
   `NAD83(CSRS)v7 / MTM zone 7 + CGVD28 height` to `NAD83(CSRS)v7` (3D). Find why PROJ's
   `createOperations(interpolation 3D, targetCRS)` gives no point-motion leg without epochs, and
   port that rule (it is probably in `createOperationsFromDatabase` / `findOpsInRegistryDirect`, not in
   the compound code).
2. Then retry attempt 1 from `scratchpad/lot-attempt1-compound.diff`, with the projected special case
   removed. The point-motion regression should then be gone; check the two non-metre tests again.
3. Finding 1 (the top-level filter for a compound source) is a separate lot.

## Commands run

- `cargo test --all-features --no-fail-fast` on the base with the drafts: 1676 passed, 9 failed (the
  drafts), 31 ignored.
- After attempt 1: 1675 passed, 10 failed (2 closed, 3 regressions, and the draft file).
- Final, on the committed state (base code, one test added): `cargo test --all-features --no-fail-fast`
  1667 passed, 0 failed, 31 ignored. `ulimit -s 1024; cargo test --all-features --test cli_yaml`: passes.
  `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`: clean.
  `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`, then both checks pass.
