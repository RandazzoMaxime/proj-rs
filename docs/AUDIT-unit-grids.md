# Audit: unit-grids lot

Scope: the `TEST()`s of PROJ 9.9.0 `src/test/unit/` assigned to this lot, read
against the C++ source, and the five `proj_trans_bounds*` tests of
`test_c_api.cpp` that were listed as blocked.

Status values:

- **closed**: a Rust test asserts PROJ's values under the C++ name, with the
  C++ tolerances, and passes.
- **partial**: a Rust test covers part of the C++ `TEST()`; the gap is named.
- **open**: no Rust test yet; the cause is named.

## Summary

| File | TEST() | closed | partial | open |
|---|---:|---:|---:|---:|
| `test_grids.cpp` | 10 | 10 | 0 | 0 |
| `test_polyhedral.cpp` | 5 | 5 | 0 | 0 |
| `test_c_api.cpp` (blocked `proj_trans_bounds`) | 5 | 5 | 0 | 0 |
| `test_tmerczoned.cpp` | 8 | 8 | 0 | 0 |
| `proj_angular_io_test.cpp` | 5 | 5 | 0 | 0 |
| `pj_phi2_test.cpp` | 1 | 1 | 0 | 0 |
| `proj_errno_string_test.cpp` | 3 | 3 | 0 | 0 |
| `test_util.cpp` | 2 | 2 | 0 | 0 |
| `proj_context_test.cpp` | 5 | 1 | 2 | 2 |
| `test_defmodel.cpp` | 13 | 0 | 9 | 4 |
| `test_coordinates.cpp` | 3 | 0 | 0 | 3 |
| `test_tinshift.cpp` | 1 | 0 | 1 | 0 |
| `test_misc.cpp` | 1 | 0 | 1 | 0 |
| **total** | **62** | **40** | **13** | **9** |

## test_grids.cpp (10 TEST_F): closed

| C++ | Rust test |
|---|---|
| `VerticalShiftGridSet_null` | `unit_grids_null_gtx::vertical_shift_grid_set_null` |
| `VerticalShiftGridSet_gtx` | `unit_grids_null_gtx::vertical_shift_grid_set_gtx` |
| `HorizontalShiftGridSet_null` | `unit_grids_null_gtx::horizontal_shift_grid_set_null` |
| `GenericShiftGridSet_null` | `unit_grids_null_gtx::generic_shift_grid_set_null` |
| `HorizontalShiftGridSet_gtiff` | `unit_grids_gtiff::horizontal_shift_grid_set_gtiff` |
| `GenericShiftGridSet_gtiff` | `unit_grids_gtiff::generic_shift_grid_set_gtiff` |
| `GenericShiftGridSet_gtiff_valuesAt_tiled_optim` | `unit_grids_gtiff::generic_shift_grid_set_gtiff_valuesat_tiled_optim` |
| `GenericShiftGridSet_gtiff_with_subgrid` | `unit_grids_gtiff::generic_shift_grid_set_gtiff_with_subgrid` |
| `GenericShiftGridSet_gtiff_with_two_level_of_subgrids_no_grid_name` | `unit_grids_gtiff::generic_shift_grid_set_gtiff_with_two_level_of_subgrids_no_grid_name` |
| `GenericShiftGridSet_gtiff_projected` | `unit_grids_gtiff::generic_shift_grid_set_gtiff_projected` |

The NTv2 sign convention of `valueAt` (`compensateNTConvention`) is exposed as
`NtConvention` and tested in `tests/unit_grids_ntv2_convention.rs`. It has no
C++ `TEST()` of its own; the test checks the sign rule of `grids.cpp`.

## test_polyhedral.cpp (5 TEST): closed

`tests/unit_polyhedral.rs`, one test per C++ `TEST()`: `polyhedral_roundtrip`,
`polyhedral_lat0_lon0_at_origin`, `polyhedral_dsea_invalid_net_rejected`,
`polyhedral_dsea_known_nets_accepted`, `polyhedral_x0_y0_stacks_on_lat0_lon0`.

## test_c_api.cpp: the 5 blocked `proj_trans_bounds` tests: closed

`tests/unit_trans_bounds_blocked.rs`. The CRS pairs now resolve (`OGC:CRS84`,
`ESRI:`, inline WKT, `EPSG:9707`), so the expected values of PROJ are asserted
directly.

## test_tmerczoned.cpp (8 TEST): closed

`tests/proj_unit.rs`, `tmerczoned_*`. Coordinates and expected values were
compared with the C++. The inverse tolerance is PROJ's 1e-10.

## proj_angular_io_test.cpp (5 TEST): closed

`tests/proj_unit.rs`, `angular_units_*`. The `bool` arguments of the Rust
`angular_input` and `degree_output` stand for `PJ_FWD`/`PJ_INV` in the order
the tests document.

## pj_phi2_test.cpp (1 TEST): closed

`tests/unit_math_phi2.rs`, `pj_phi2_basic`, through `proj_rs::math::phi2`. The
special values (`±0`, `±1`, `±inf`, NaN) and the 4-ULP comparisons are PROJ's.
`phi2` returns `Result<f64, Phi2Error>`; the error is PROJ's non-convergence.

## proj_errno_string_test.cpp (3 TEST): closed

`tests/unit_errno_string.rs`, through `proj_rs::errno::error_string`. The table
is `src/strerrno.cpp`, the codes are `proj.h`. Each typed error has a `code()`
(`ProjError`, `ProjCreateError`, `TransformError`) with the crate's own mapping,
documented per variant in `src/errno.rs`. PROJ does not expose that mapping.
The C++ context argument has no counterpart: the string does not depend on a
context.

## test_util.cpp (2 TEST): closed

`tests/unit_util_names.rs`, through `proj_rs::names` (`LocalName`, `NameSpace`).
This is a minimal name model: global name space, nested name spaces with a
separator, and fully qualified names. It does not port the rest of
`NameFactory`.

## proj_context_test.cpp (5 TEST): 1 closed, 2 partial, 2 open

| C++ | Status | Rust test / cause |
|---|---|---|
| `proj_context_set_ca_bundle_path` | closed | `tests/unit_context_paths.rs`. `ProjBuilder::set_ca_bundle_path` and `ca_bundle_path`. |
| `proj_context_set_search_paths` | partial | `tests/unit_context_paths.rs` checks the getter `search_paths`. The C++ test then resolves `+init=temp_proj_dic2:MY_PIPELINE` from the directory. `+init=` dictionary lookup is not implemented. |
| `proj_context_set_user_writable_directory` | partial | `tests/unit_context_paths.rs` checks the setter and getter and that the directory is created. Not ported: the default-path getter and the reset by a null path (the Rust setter takes a path, not an optional one). |
| `read_grid_from_user_writable_directory` | open | Needs `+init=` dictionary lookup and the `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` toggle at run time. |
| `proj_context_set_file_finder` | open | The crate has no file-finder hook. Adding one means a callback consulted by the `+init=` and grid resolution in `src/gridpath.rs`. |

## test_defmodel.cpp (13 TEST): 9 partial, 4 open

Ported in `tests/proj_unit.rs`: `defmodel_basic_and_rejected_json`,
`defmodel_error_cases`, `defmodel_iso8601_and_time_functions`.

| C++ | Status | Cause |
|---|---|---|
| `basic` | partial | Rejected JSON is covered. The positive `op_file` call discards its result (`let _ =`). |
| `full` | open | Not ported. |
| `error_cases` | partial | Several error models are rejected; the C++ list was not compared line by line. |
| `ISO8601ToDecimalYear` | partial | Covered only through the time fields of a model. `decimal_year` is private to `src/projections/models/defmodel.rs`. |
| `evaluate_constant` | partial | Through `Operation::forward`; 3 epochs. |
| `evaluate_velocity` | partial | Velocity epochs in the same test. |
| `evaluate_step` | partial | Same test; not every C++ value compared. |
| `evaluate_reverse_step` | partial | Same test. |
| `evaluate_piecewise` | partial | Piecewise values and `before_first`/`after_last` variants. |
| `evaluate_exponential` | partial | Checked at 5 times. |
| `evaluator_horizontal_unit_degree` | open | 340 lines of C++ with a horizontal unit grid; not ported. |
| `evaluator_horizontal_unit_metre` | open | Same. |
| `evaluator_projected_crs` | open | Projected CRS evaluation; not ported. |

Cause for the open ones: the defmodel evaluator is not exposed (the C++ calls
it directly), so each C++ case has to be reproduced through `Operation`, one
value at a time.

## test_coordinates.cpp (3 TEST): open

| C++ | Cause |
|---|---|
| `static_crs` | `CoordinateMetadata` exists (`src/iso19111/coordinate_metadata.rs`). The C++ also checks `isEquivalentTo`, the WKT and JSON round trips through `proj_create`, and the epoch absence. Not ported. |
| `dynamic_crs` | The C++ builds a `DynamicGeodeticReferenceFrame` and reads `coordinateEpochAsDecimalYear`. The crate has no public constructor for a dynamic frame (`grep` of `src/iso19111` finds none). Not ported. |
| `crs_with_point_motion_operation_and_promote_to_3D` | Uses `DatabaseContext`, `AuthorityFactory` and `promoteTo3D`. Not ported. |

## test_tinshift.cpp (1 TEST): partial

`tests/unit_tinshift_parse.rs` ports the three parse errors (`foo`, `null`,
`{}`) and the horizontal triangle values with the z carried through, through
`+proj=tinshift` operations. Not ported: the vertical component
(`transformed_components` with vertical values, z = 1000.1 and 1000.325), the
`FALLBACK_NONE` checks that the C++ reads through `fallbackStrategy()`, and the
accessors `fileType`/`formatVersion`/`inputCRS`/`outputCRS`. The C++ evaluator
is internal; a public evaluator is not added.

## test_misc.cpp (1 TEST): partial

`tests/proj_unit.rs`, `misc_version`, checks the library version. The C macros
`PROJ_COMPUTE_VERSION`, `PROJ_VERSION_NUMBER` and `PROJ_AT_LEAST_VERSION` have
no Rust counterpart, and the `at_least` closure of the Rust test is local, so
it does not check any library function.
