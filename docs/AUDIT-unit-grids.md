# Audit: unit-grids lot

Scope: the `TEST()`s of PROJ 9.9.0 `src/test/unit/` assigned to this lot, read
against the C++ source (`test_grids.cpp`, `test_defmodel.cpp`,
`test_polyhedral.cpp`, `test_coordinates.cpp`, `test_util.cpp`,
`proj_context_test.cpp`, `proj_errno_string_test.cpp`,
`proj_angular_io_test.cpp`, `pj_phi2_test.cpp`, `test_misc.cpp`,
`test_tinshift.cpp`, `test_tmerczoned.cpp`) and the five
`proj_trans_bounds*` tests of `test_c_api.cpp` listed as blocked.

Status values:

- **closed**: a Rust test asserts PROJ's values under the C++ name, with the
  C++ tolerances, and it passes.
- **partial**: a Rust test covers part of the C++ `TEST()`; the gap is named.
- **open**: no Rust test; the cause is named.

## Summary

| File | TEST() | closed | partial | open |
|---|---:|---:|---:|---:|
| `test_grids.cpp` | 10 | 10 | 0 | 0 |
| `test_polyhedral.cpp` | 5 | 5 | 0 | 0 |
| `test_c_api.cpp` (blocked `proj_trans_bounds`) | 5 | 5 | 0 | 0 |
| `test_tmerczoned.cpp` | 8 | 8 | 0 | 0 |
| `proj_angular_io_test.cpp` | 5 | 5 | 0 | 0 |
| `test_defmodel.cpp` | 13 | 0 | 9 | 4 |
| `test_coordinates.cpp` | 3 | 0 | 0 | 3 |
| `test_util.cpp` | 2 | 0 | 0 | 2 |
| `proj_context_test.cpp` | 5 | 0 | 2 | 3 |
| `proj_errno_string_test.cpp` | 3 | 0 | 3 | 0 |
| `pj_phi2_test.cpp` | 1 | 0 | 0 | 1 |
| `test_misc.cpp` | 1 | 0 | 1 | 0 |
| `test_tinshift.cpp` | 1 | 0 | 1 | 0 |
| **total** | **62** | **33** | **16** | **13** |

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

Deliberate differences, all recorded in `docs/PORTING-unit-grids.md`:

- The C++ `m_ctxt` argument is absent: a grid set owns its bytes.
- `reassign_context` has no counterpart. `reopen` is a re-read.
- C++ `bool` outputs are `Option` or enums (`Nodata`).
- `valueAt`'s `compensateNTConvention` flag is not exposed (see the porting sheet).

## test_polyhedral.cpp (5 TEST): closed

The Rust port had one function for the five `TEST()`s. It is split into
`tests/unit_polyhedral.rs`, with PROJ's tolerances (1e-13 and 1e-11) and
points.

| C++ | Rust test |
|---|---|
| `roundtrip` | `unit_polyhedral::polyhedral_roundtrip` |
| `lat0_lon0_at_origin` | `unit_polyhedral::polyhedral_lat0_lon0_at_origin` |
| `dsea_invalid_net_rejected` | `unit_polyhedral::polyhedral_dsea_invalid_net_rejected` |
| `dsea_known_nets_accepted` | `unit_polyhedral::polyhedral_dsea_known_nets_accepted` |
| `x0_y0_stacks_on_lat0_lon0` | `unit_polyhedral::polyhedral_x0_y0_stacks_on_lat0_lon0` |

## test_c_api.cpp: the 5 blocked `proj_trans_bounds` tests: closed

`docs/PORTING-c-api-unit.md` listed them as blocked by `OGC:`, `ESRI:`,
inline WKT and `EPSG:9707`. A probe showed those pairs now resolve, and the
five are ported with PROJ's expected values in `tests/unit_trans_bounds_blocked.rs`:

| C++ | Rust test |
|---|---|
| `proj_trans_bounds_ignore_inf` | `c_api_proj_trans_bounds_ignore_inf` |
| `proj_trans_bounds_ignore_inf_geographic` | `c_api_proj_trans_bounds_ignore_inf_geographic` |
| `proj_trans_bounds_to_compound_crs` | `c_api_proj_trans_bounds_to_compound_crs` |
| `proj_trans_bounds_world_geodetic_to_spilhaus` | `c_api_proj_trans_bounds_world_geodetic_to_spilhaus` |
| `proj_trans_bounds_3D_ignore_inf` | `c_api_proj_trans_bounds_3d_ignore_inf` |

## test_tmerczoned.cpp (8 TEST): closed

`tests/proj_unit.rs`, `tmerczoned_*`. The coordinates and expected values were
compared line by line with the C++. The inverse tolerance was 1e-8 in the
earlier port; PROJ uses 1e-10, and the port now uses 1e-10 too.

## proj_angular_io_test.cpp (5 TEST): closed

`tests/proj_unit.rs`, `angular_units_*`. The pipelines are PROJ's. The Rust
`angular_input(bool)` and `degree_output(bool)` calls stand for PROJ's
`PJ_FWD`/`PJ_INV` (`false` and `true` in the order used by the Rust tests).
The `Degrees` test maps `proj_degree_input`/`proj_degree_output` to
`degree_output(true/false)` as its comments say.

## test_defmodel.cpp (13 TEST): partial and open

Ported in `tests/proj_unit.rs`: `defmodel_basic_and_rejected_json`,
`defmodel_error_cases`, `defmodel_iso8601_and_time_functions`.

| C++ | Status | Cause |
|---|---|---|
| `basic` | partial | The rejected-JSON cases are ported. The positive `op_file` call discards its result (`let _ =`), so the success path is not asserted. |
| `full` | open | Not ported. |
| `error_cases` | partial | Several error models are rejected in `defmodel_error_cases`; the C++ list was not compared line by line. |
| `ISO8601ToDecimalYear` | partial | Only through `defmodel` time fields. The C++ calls the conversion directly; `decimal_year` is private to `src/projections/models/defmodel.rs`. |
| `evaluate_constant` | partial | Through `Operation::forward` with a constant model; 3 epochs. |
| `evaluate_velocity` | partial | Velocity epochs in the same test. |
| `evaluate_step` | partial | Same test; the C++ values were not all compared. |
| `evaluate_reverse_step` | partial | Same test. |
| `evaluate_piecewise` | partial | Piecewise values, `before_first`/`after_last` variants. |
| `evaluate_exponential` | partial | Exponential formula, checked at 5 times. |
| `evaluator_horizontal_unit_degree` | open | 340 lines of C++ with grids for a horizontal unit; not ported. |
| `evaluator_horizontal_unit_metre` | open | Same. |
| `evaluator_projected_crs` | open | Projected CRS evaluation; not ported. |

## test_coordinates.cpp (3 TEST): open

| C++ | Cause |
|---|---|
| `static_crs` | `CoordinateMetadata` exists (`src/iso19111/coordinate_metadata.rs`) but the C++ checks `isEquivalentTo`, `coordinateEpoch` and the WKT/JSON round trips through `proj_create`, which are not transcribed. Not ported. |
| `dynamic_crs` | The C++ builds a `DynamicGeodeticReferenceFrame` and calls `coordinateEpochAsDecimalYear`. The crate has no public constructor for a dynamic frame (`grep` of `src/iso19111` finds none), so the CRS cannot be built here. |
| `crs_with_point_motion_operation_and_promote_to_3D` | Uses `DatabaseContext`/`AuthorityFactory` and `promoteTo3D`. Not ported. |

## test_util.cpp (2 TEST): open

`NameFactory::createLocalName`, `createNameSpace` and `scope()` /
`toFullyQualifiedName()` are C++ name objects. The crate has no name-object
model (`grep` for `NameFactory`, `LocalName` and `NameSpace` in `src/` finds
nothing), so nothing observable corresponds to them.

## proj_context_test.cpp (5 TEST): partial and open

| C++ | Status | Cause |
|---|---|---|
| `proj_context_set_file_finder` | open | The crate has no file-finder hook (`grep` for `file_finder` finds nothing). |
| `proj_context_set_search_paths` | partial | `tests/proj_unit.rs` checks that a grid resolves on the added path. The C++ test uses an `+init=` dictionary (`temp_proj_dic2:MY_PIPELINE`), and `+init=` dictionaries are not read by the crate. |
| `proj_context_set_user_writable_directory` | partial | `tests/proj_unit.rs` checks that the directory is created. The C++ also checks the getter and that a null path restores the default. |
| `read_grid_from_user_writable_directory` | open | Needs `+init=` dictionaries and the `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` toggle. |
| `proj_context_set_ca_bundle_path` | open | No public setter. `proj_unit.rs` asserts the endpoint, which is not the C++ check. |

## proj_errno_string_test.cpp (3 TEST): partial

The crate has no errno. `tests/proj_unit.rs` checks that bad definitions give
`CreateError::UnknownProj` and messages naming the operation. The C++
`proj_errno_string(0) == nullptr` and the per-code string checks have no
counterpart. Partial, and the difference is structural.

## pj_phi2_test.cpp (1 TEST): open

`phi2` is `pub(crate)` in `src/projmath.rs` and returns `Result<f64, ()>`.
The C++ `pj_phi2` returns a `double` with sentinel values (`±π/2`, `0`) for
`ts = 0, ±1, ±inf`. Those values are not observable through the crate's
public API, so the test has no public counterpart. Closing it needs a decision
on how `Err` maps to the sentinels, which is not taken in this lot.

## test_misc.cpp (1 TEST): partial

`tests/proj_unit.rs`, `misc_version`, checks the library version. The C++
macros `PROJ_COMPUTE_VERSION`, `PROJ_VERSION_NUMBER` and
`PROJ_AT_LEAST_VERSION` are C header macros with no Rust counterpart. The
`at_least` closure in the Rust test is local to the test, so it does not check
any library function.

## test_tinshift.cpp (1 TEST): partial

`tests/proj_unit.rs`, `tinshift_basic_triangles`, covers the triangle values
and an outside point through `Operation`. The C++ calls the internal
`TINShiftJSONFile::parse` (throws for `foo`, `null`, `{}`) and the internal
evaluator (`forward`/`inverse` with `FALLBACK_NONE`). Those internals are not
public. The parse failures are reachable through `Operation::create`, and are
not yet asserted.
