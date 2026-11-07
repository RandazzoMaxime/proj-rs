# Port — `createOperationsCompoundToGeog` whole (lot `compound-derived`)

Lot `compound-derived`, branch `unit-fix-compound-derived`, base `origin/unit-fix-geog-candidates-2`
(d559cd2). Source: PROJ 9.9.0, `src/iso19111/operation/coordinateoperationfactory.cpp` (the
factory), traced with the `PROJ_DEBUG=3` build of the scratch tree. Third attempt at this lot,
after `docs/PORTING-unit-compound-to-geog.md` (branch `unit-fix-compound-to-geog`) and
`docs/PORTING-unit-compound-to-compound.md` (branch `unit-fix-compound-to-compound`).

## The point motion question (settled first)

The earlier attempt regressed three point motion tests: the leg `createOperations(interp 3D,
target)` of the interpolation loop returned `Canada velocity grid v7`, a point motion operation with
no epochs, and the export failed. Traced with PROJ (`NAD83(CSRS)v3 / MTM zone 7 + CGVD28 height` to
`NAD83(CSRS)v7 / UTM zone 19N + CGVD2013a(2010) height`):

- PROJ also returns the point motion operation without epochs: `findOpsInRegistryDirect(EPSG:8254 ->
  EPSG:8254)` gives one row, the velocity grid. The registry does not filter it.
- But `createOperationsFromDatabase` returns false (no perfect accuracy), so
  `createOperationsGeodToGeod` runs after it (`createOperations`, the "both geodetic" case) and adds
  `Null geographic offset from NAD83(CSRS)v7 to NAD83(CSRS)v7`. `projinfo -s EPSG:8254 -t EPSG:8254`
  lists 2 candidates; the port listed only the velocity grid.
- With two candidates, `hasOnlyOneOp` is false and the pair loop rejects the pair
  `Null geographic offset (2D)` / `Canada velocity grid v7` (trace: "do not use consistent
  operations"): a `PointMotionOperation` is a `SingleOperation`, not a `Transformation`, so its datum
  set is empty.

Both rules were ported in commit 1. No point motion regression came back.

## Function map

| PROJ 9.9.0 (`coordinateoperationfactory.cpp`) | Rust (`src/iso19111/`) |
|---|---|
| `createOperations`, `createOperationsGeodToGeod` after a non-perfect registry result (same datum) | `factory.rs` `create_candidates_at`, geodetic block |
| `dynamic_cast<const Transformation *>` in `makeDatumSet`, `useCompatibleTransformationsForSameSourceTarget` | `compound.rs` `is_transformation_step` |
| `createOperationsCompoundToGeog`, 6454-7172 | `compound.rs` `compound_to_geog3d` |
| derived horizontal part, 6473-6502 | `derived_horizontal_to_geog` (`derived.rs` `derived_base`) |
| NAD83(CSRS)v7 epoch case, 6504-6566 | `nad83_csrs_v7_epoch_hack` |
| bound CRS of a derived CRS, 6539-6568 | `compound_to_geog3d` (bound branch) |
| bound-and-bound branch, 6570-6640 | not ported: `bound_compound_to_geog3d` (as before) |
| 2D target, 6642-6652 | `horizontal_only` (as before) |
| general path, 6654-7170 | `compound_horizontal_vertical` |
| `SetSkipHorizontalTransform`, `SetGeogCRSOfVertCRS` (`geogCRSOfVertCRSStack`) | `skip_horizontal_context`; the stack's datum passed to `vertical_to_geog_candidates` |
| `geogCRSTmp`, 6795-6827 | `compound_horizontal_vertical` (`promote_to_3d_with_axis`) |
| only-vertical-ballpark intermediate, 6837-6905 | `compound_horizontal_vertical` |
| interpolation pairs and cache, 6913-7140 | `compound_horizontal_vertical`, `interpolation_legs`, `is_gtiff_shift` |
| `getInterpolationGeogCRS`, 6392-6433 (`demoteTo2D` with the database) | `interpolation_geographic` |
| `createHorizVerticalPROJBased`, 2717-2793 | `combine` (proj-based `HorizVertical` unless the horizontal part is a null offset) |
| `createHorizVerticalHorizPROJBased`, 2795-2880 (`inverse()->_isEquivalentTo` for the `using` name) | `proj_based_horiz_vertical_horiz` |
| `CRS::promoteTo3D(name, db, axis)`, crs.cpp 1341-1459 | `identify.rs` `GeographicCRS::promote_to_3d_with_axis` |
| `Conversion::_exportToPROJString`, the `horiz` of a compound CRS (conversion.cpp 3886-3900, 4420-4428) | `conversion_export.rs` `export_projection` |
| `CRS::getResolvedCRS`, compound components, 8342-8356 | `factory.rs` `resolve_crs` |
| `findsOpsInRegistryWithIntermediate` for a vertical/geographic pair, 4449-4463 | `factory.rs` `create_candidates_at` (`vertical_geographic`) |
| `createOperationsCompoundToCompound`, point motion block epochs (`opsFirst`, `opsLast`) | `compound.rs` `compound_point_motion` |

`projected_compound_to_geog` and `chain` (a special case with no PROJ counterpart) are removed.

## Commits

### 1. A same-datum pair keeps its null offset next to a registry result

`factory.rs`: after a registry result without perfect accuracy, a same-datum geodetic pair gets the
operations of `createOperationsGeogToGeog` (null offset, axis order, vertical unit), as without a
registry result. `compound.rs`: `is_transformation_step` skips point motion operations. Test:
`unit_op_point_motion_epochs::geographic_3d_to_itself_without_epochs_keeps_the_null_offset`
(names from the reference `projinfo -s EPSG:8254 -t EPSG:8254`). No other test changed; unit set
292 passed / 9 failed (the uncommitted drafts) before and after; `cli_yaml` passes, ledger unchanged.

### 2. Compound CRS extent from its components

`resolve_crs`: a compound CRS without identifier and without its own extent takes the intersection
of the resolved extents of its components. The geoid test (`NAD83 / Pennsylvania South + NAVD88
height`, no ids) then has the Pennsylvania area, which filters `NAD83 to NAD83(2011) (1)` (Gulf of
Mexico) out of the interpolation leg, as in the PROJ trace ("filtering reduced from 1 to 0").

### 3. Registry intermediates for a vertical and a geographic CRS

`findsOpsInRegistryWithIntermediate` runs for a vertical/geographic pair too when nothing was found
(it was gated on two vertical CRS). `Kumul 34 height` to `WGS 84` (3D) now gives
`Inverse of EGM96 height to Kumul 34 height (1) + Inverse of WGS 84 to EGM96 height (1)`, as the
reference `projinfo` does; the only-vertical-ballpark branch of the PNG94 tests needs it.

### 4. `createOperationsCompoundToGeog` ported

The pieces of the function map above, in one commit, because the tests close only together
(each piece was measured on the draft tests while building it; commits 2 and 3 alone change
nothing measured):

- the derived branch replaces `projected_compound_to_geog`; its first step is the inverse
  projection with compound endpoints, so the conversion export now reads the first component of a
  compound endpoint, as PROJ does (this was the `issue_4550` export error of the second attempt);
- the NAD83(CSRS)v7 case needs the epochs: `compound_to_geog3d` and `geog_to_compound` take
  `Epochs`, the derived branch passes `(source, source)` then `(source, target)`, and the point
  motion block of compound-to-compound reads `opsFirst`/`opsLast` at the source and target epochs.
  Without it `..._special_case_cgvd2013a` chained the `NAD83(CSRS)v3 to NAD83(CSRS)v7 (2)` velocity
  operation without epochs;
- the horizontal operations go to the full target, the interpolation 3D CRS is
  `demoteTo2D(db)->promoteTo3D(db, target height axis)` (the registered CRS when its height axis is
  the target's), the GTiff filter of the interpolation-to-target legs tests the method the operation
  gets once PROJ's grid names are substituted (the candidates here keep the database method: an
  `Amersfoort to ETRS89-NLD (9)` NTv2 is a `HORIZONTAL_SHIFT_GTIFF` in PROJ);
- `createHorizVerticalPROJBased` is proj-based, written by the `HorizVertical` exportable, also
  for a null horizontal part (`horizTransformIsNoOp`, which keeps the exportable): the last `ftUS`
  candidate is one `unitconvert`, and `compoundCRS_with_non_meter_horiz_and_vertical_to_geog`
  (`unit_compound_axis_swap`) keeps its axis swap last;
- the `X using Y` name of `createHorizVerticalHorizPROJBased` needs the inverse of the last leg to be
  *equivalent* to the first (method and values), not the same name: a 2D ballpark offset and its 3D
  inverse differ by their method, and PROJ names the full chain then
  (`compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS`).

### 5. Bound CRS of a derived horizontal part

The branch of 6539-6568: a bound projected CRS goes through the compound of the bound CRS of its
base. It changes nothing measured (the NAD2011 test stays open, see below), and it regresses
nothing; it is kept because it is PROJ's order and the bound-and-bound path no longer sees a
projected base.

## Tests

| PROJ `TEST()` | Rust test | Status |
|---|---|---|
| `compoundCRS_ftUS_to_geogCRS_ft` | `unit_op_compound_derived` | closed |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change` | `unit_op_compound_derived` | closed |
| `compoundCRS_to_geogCRS_2D_promote_to_3D_context` | `unit_op_compound_derived` | closed (133 both ways) |
| `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` | `unit_op_compound_derived` | closed |
| `compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort` | `unit_op_compound_derived` | closed |
| `compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform` | `unit_op_compound_derived` | closed |
| `compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96` | `unit_op_compound_to_compound_2` | closed (un-ignored) |
| `compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform` | `unit_op_compound_to_compound_2` | closed (un-ignored) |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp` | `unit_op_compound_to_compound_2` | closed (un-ignored; it already passed on the base) |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` | draft | open: 17 vs 16. Ours has two extra ballpark-pair candidates (`NAD83 -> NAD83(2011)` and `-> NAD83(NSRS2007)`) and lacks `... NAD83(NSRS2007) to NAVD88 height (1) using NAD83 to WGS 84 (1) + Inverse of NAD83(NSRS2007) to WGS 84 (1)`: the `NAD83 -> NAD83(NSRS2007)` leg set (datum pivot), not this function |
| `compoundCRS_to_geogCRS_3D_context` | draft | open: blocks 1 now pass; block 2 (`NAD83(NSRS2007) + NAVD88` to `WGS 84`, default spatial criterion) is the top-level filter finding of the first sheet |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | draft | open: the first leg (bound projected + V to bound geographic + V) is a nested concatenation `Inverse of unknown + Transformation from NAD83 to WGS84` then `Inverse of Transformation from NAD83 to WGS84`; `createComputeMetadata` flattens one level only, so the `T` / `inverse T` pair is not removed and the name keeps it. The bound-to-bound chaining (`bound_ops.rs`) builds the nested step |
| `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound` | ignored | open, as before (bound-and-bound branch not ported) |
| `compoundCRS_to_compoundCRS_context` | ignored | open, as before (datum pivot candidate set) |

## Deviations (stated)

- **Bound-and-bound branch** (6570-6640) and the rest of the bound cases stay in
  `bound_compound_to_geog3d`.
- **Synthesized vertical ballpark.** `createOperationsVertToGeog` appends the ballpark change of unit
  inside PROJ's private `createOperations`; here `create_candidates` does not, so
  `vertical_to_geog_candidates` appends it where `createOperationsCompoundToGeog` calls
  `createOperations(vertical, geographic)`. The `geogCRSOfVertCRSStack` is that call's
  `horizontal_datum`, not a context stack.
- **GTiff filter.** The method is tested after the grid substitution PROJ applies by default
  (the candidates here keep the database method and names).
- **Registry filter.** PROJ's `findOpsInRegistryDirect` returns the filtered registry operations;
  the port keeps the unfiltered ones in `found` (only `direct_empty` reads the filter). Replacing
  `found` by the filtered list was tried and reopened `test_projinfo.yaml#11` (`NAD27` to `NAD83`,
  strict containment: 320 candidates instead of 1), so it was reverted; it is not this lot.
- **Time.** `unit_op_compound_to_compound_2::operation_compound_to_compound_issue_4550` takes about
  50 s (the file about 70 s, 24 s on the base): the derived branch and the vertical/geographic
  intermediate search run more sub-searches, as PROJ does, without PROJ's operation cache.

## Verification

- Baseline (base + the uncommitted drafts): `cargo test --all-features --no-fail-fast` with the
  reference binaries, `PROJ_DATA` on `oracle-db`, `PROJ_CS2CS_REQUIRED=1`: 1687 passed, 9 failed
  (the drafts), 31 ignored.
- Final tree (the five commits), same command: 1684 passed, 3 failed, 28 ignored before the docs
  regeneration; the 3 failures were `counts_in_docs_match_the_ledgers`, `progress_chart_is_current`
  and `unit_triage_is_current`, fixed by `UPDATE_DOCS=1` (last commit). `cli_yaml` passed in that
  run, `docs/UNPORTED-CLI.md` unchanged.
- After commit 1: unit regression set 292 passed / 9 failed (drafts), `cli_yaml` passes.
- Before commit 4 was split out, the unit set with the drafts gave 299 passed, 3 failed (the three
  open drafts). One regression found on the full run and fixed before committing:
  `compoundCRS_with_non_meter_horiz_and_vertical_to_geog` (null horizontal part, see commit 4).
  Commit 4's tree (without commit 5) was checked on the compound test files only.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `(ulimit -s 1024; cargo test --all-features --test cli_yaml)`: passes on the final tree.
