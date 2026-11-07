# Port — grid names of the default context (lot `grid-names-default`)

Source: PROJ 9.9.0, tag `9.9.0` of OSGeo/PROJ. The PROJ lines cited below are from
`src/iso19111/operation/singleoperation.cpp` (`substitutePROJAlternativeGridNames`,
`_getHorizontalShiftGTIFFFilename`, `exportToPROJString` of a transformation) and
`src/iso19111/operation/coordinateoperationfactory.cpp` (`usePROJNames_`, `SortFunction`,
`FilterResults::sort`).

The lot has two steps, one commit each. Step 1 is the PROJ string of a NADCON-derived operation once
its grid names are substituted. Step 2 is the default of the context, which PROJ sets to the PROJ
names (`usePROJNames_ = true`).

## Method and reference

The reference is PROJ 9.9.0 `projinfo` (`scratchpad/reftest/build/bin/projinfo`, `PROJ_DATA` on the
oracle `proj.db`). The PROJ string of an operation is `projinfo EPSG:<code>` for a single operation,
and `projinfo -s <source> -t <target> --grid-check none --crs-extent-use none` for the candidate list,
whose operations are printed in order with their PROJ strings. The expected strings of the tests are
copied from that output, and the tests of PROJ 9.9.0 `test/unit/test_operationfactory.cpp` are the
ones of `tests/unit_op_compound_vertical_units.rs`.

## Function map

| PROJ 9.9.0 | Rust (`src/iso19111`) |
|---|---|
| `Transformation::substitutePROJAlternativeGridNames`, horizontal GTiff branch (singleoperation.cpp:2446-2476) | `proj_grid_substitution.rs`, `horizontal_grid`, `gtiff_horizontal` |
| `Transformation::_getHorizontalShiftGTIFFFilename` (singleoperation.cpp:2061-2090) | `transform_export.rs`, `horizontal_grid_shift`, the GTiff `file` lookup |
| `Transformation::_exportToPROJString`, grid branch (singleoperation.cpp:4315-4389) | `transform_export.rs`, `grid_shift`, `horizontal_grid_shift` |
| `SingleOperation::parameterValue(name, code)` (singleoperation.cpp:1329-1353) | `model.rs` / `transform_export.rs`, `file_by_code`, `file_by_name` |
| `CoordinateOperationContext::usePROJNames_`, default true (coordinateoperationfactory.cpp:175, 333-346) | `factory.rs`, `CoordinateOperationContext::grid_names`, `alternative_grid_names`, `set_alternative_grid_names` |
| `findOpsInRegistryDirect`, `usePROJAlternativeGridNames` (1927-1931), `SortFunction` / `gridsNeeded` (1480-1498) | `factory.rs` (`operations_between` callers), `select.rs`, `sort_candidates` |

## Commit 1 — the GTiff grid of a NADCON5 (3D) method is read from its ellipsoidal height file

### What was wrong

`substitutePROJAlternativeGridNames` replaces the grid of a NADCON5 (3D) transformation (EPSG:1075)
by a `GENERAL_SHIFT_GTIFF` method whose file parameter is named `Latitude, longitude and ellipsoidal
height difference file` and has no EPSG code (singleoperation.cpp:2462-2468). The export read the
file of a GTiff method only from the EPSG code 8656 or from the name `Latitude and longitude
difference file`. The 3D name was not read, so `horizontal_grid_shift` returned `Ok(false)`, every
other grid branch returned `Ok(false)` too, and the export fell to `Unimplemented <operation name>`.

With the official grid names the operation is exported from its EPSG parameters, and the export
works. With the PROJ names the substituted operation is the one of the candidates, and its inverse
and forward both failed. In `operation_compound_ft_to_geog_ft` and `operation_compound_m_to_geog_ft`
the failing candidate is the last of the list (it is not PROJ exportable, so the sort puts it last),
and the test exports `res.last()`: `Unimplemented NAD83(NSRS2007) to NAD83(2011) (1)` and
`(2)`.

### What is now ported

`horizontal_grid_shift`, for a GTiff method, reads the file of the first of the three parameters
PROJ reads, in its order (singleoperation.cpp:2072-2089): the parameter with code 8656, the name
`Latitude and longitude difference file`, then the name `Latitude, longitude and ellipsoidal height
difference file`. The `no_z_transform` flag and the `+inv` of the inverse are as before, and the
inverse of the substituted operation is written as `+inv +proj=gridshift +no_z_transform`, as the
reference does.

### Reference

`projinfo -s EPSG:6318 -t EPSG:4759 --grid-check none --crs-extent-use none`, operation No. 1
(`INVERSE(DERIVED_FROM(EPSG)):8559`), and `projinfo EPSG:8559` for the forward one. The port gives
the same strings for both, and for operation No. 2 (EPSG:8552, the Alaska grid).

### Tests

File `tests/unit_op_nadcon5_gtiff_export.rs`, a regression test of the export, not a PROJ `TEST()`.
The operation is built with `AlternativeGridNames::Proj`, so its grid is the substituted one.

| Rust test | Reference | Status |
|---|---|---|
| `operation_8559_nadcon5_3d_forward_proj_string` | `projinfo EPSG:8559` | passes; failed with `Unimplemented NAD83(NSRS2007) to NAD83(2011) (1)` before the fix |
| `operation_8559_nadcon5_3d_inverse_proj_string` | `projinfo -s EPSG:6318 -t EPSG:4759`, No. 1 | passes; failed with `Unimplemented Inverse of NAD83(NSRS2007) to NAD83(2011) (1)` before the fix |

The two compound tests of `tests/unit_op_compound_vertical_units.rs` pass on the base with the
official default. They are the guard of step 2.

## Commit 2 — the default context reads the registered operations with the PROJ names

### What was wrong

PROJ's `CoordinateOperationContext` starts with `usePROJNames_ = true` (coordinateoperationfactory.cpp:175).
The registered operations are then read with `usePROJAlternativeGridNames` set, in
`findOpsInRegistryDirect` (1930) and the other registry readers (2000, 2222, 2256, 4853), and
`gridsNeeded` in `SortFunction` reads the names those operations carry (1480). The port defaulted to
the database names (`Official`) and sorted on the PROJ names, so a default context and a context set
to `Proj` disagreed, and neither was PROJ's.

### What is now ported

- `CoordinateOperationContext` keeps one setting, `grid_names`, which starts at `Proj`. The registry
  readers (the intermediate searches, the registry search of `create_operations`) and the sort of the
  candidates read the same setting. `Official` (`set_alternative_grid_names(Official)`) gives the
  database names to both, as `setUsePROJAlternativeGridNames(false)` does.
- The two accessors `registry_grid_names` and `sort_grid_names` of the previous lots are merged into
  `alternative_grid_names`. Their results were the same except for the default, which was the cause.

The NADCON5 (3D) export that failed under `Proj` is commit 1 (`transform_export.rs`,
`horizontal_grid_shift`). With it, the candidates of the NSRS2007 pairs export, and the sort puts the
ballpark last, as PROJ does.

### Deviation closed

The previous lots kept `Official` as the default, because `Proj` made
`operation_compound_ft_to_geog_ft` and `operation_compound_m_to_geog_ft` fail. That deviation is closed
by commit 1 and by this commit: the default is PROJ's.

### Tests

| Rust test | PROJ `TEST()` | Status |
|---|---|---|
| `operation_vert_crs_to_geog_crs_context` (file `tests/unit_op_vert_crs_context.rs`) | `operation.vertCRS_to_geogCRS_context` | closed, commit 2 (block 1 `Official`, block 2 the default, blocks 3 to 5 the reverse, NGVD29 and NZVD2016, and the Baltic 1957 block with every authority) |

The compound and geographic tests that the earlier lots left open for the default pass under it:
`operation_compound_ft_to_geog_ft`, `operation_compound_m_to_geog_ft` (`tests/unit_op_compound_vertical_units.rs`),
and the five `tests/unit_op_geog_candidates.rs` tests, whose "default" block is now PROJ's.

### Full suite

`cargo test --all-features --no-fail-fast` with the reference PROJ 9.9.0 on `PATH`: 1680 passed,
0 failed, 31 ignored (179 `test result:` lines). `cli_yaml` passes under `ulimit -s 1024`. `cargo fmt
--check` and clippy with `-D warnings`: clean. `UPDATE_DOCS=1 cargo test --test docs_in_sync --test
progress_svg` regenerated the counts (`README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md`,
`docs/progress.svg`), and both pass without the update.

## Commands run

- `cargo test --test unit_op_compound_vertical_units` with the default switched to `Proj` (a local
  experiment, reverted): the two NADCON5 tests fail as described above.
- `projinfo -s EPSG:6318 -t EPSG:4759 -o PROJ --grid-check none --crs-extent-use none`, and
  `projinfo -s EPSG:4759 -t EPSG:6318 ...` (reference and port), to compare the PROJ strings.
- Step 2: `cargo test --test unit_op_vert_crs_context`, with the Baltic 1957 block of
  `test_operationfactory.cpp:1023-1049` added to the draft, which the draft did not have. Then the full
  bar listed above.
- After the fix: `unit_op_*`, `unit_capi_*`, `unit_crs_*` (86 `test result:` lines, all passing) and
  `cli_yaml` under `ulimit -s 1024` (1 passed). `cargo fmt --check` and clippy with `-D warnings`: clean.
