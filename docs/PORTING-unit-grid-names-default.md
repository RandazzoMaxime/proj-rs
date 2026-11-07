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

## Commands run (step 1)

- `cargo test --test unit_op_compound_vertical_units` with the default switched to `Proj` (a local
  experiment, reverted): the two NADCON5 tests fail as described above.
- After the fix: `unit_op_*`, `unit_capi_*`, `unit_crs_*` (86 `test result:` lines, all passing) and
  `cli_yaml` under `ulimit -s 1024` (1 passed). `cargo fmt --check` and clippy with `-D warnings`: clean.
