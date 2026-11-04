# Port — lot C deformation and velocity cases

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the deformation and velocity cases of lot `opfac-c` that [`PORTING-opfac-core-c.md`](PORTING-opfac-core-c.md) lists as open. The expected PROJ strings and names are copied from that C++ file. The oracle is the PROJ 9.9.0 `projinfo` binary, run with `--spatial-test intersects --grid-check none` (the C++ tests use `PARTIAL_INTERSECTION` and `IGNORE_GRID_AVAILABILITY`).

Rust tests are in `tests/unit_opfac_core_c_deformation.rs`.

**Result: 4 cases ported (4 tests), 4 cases not ported, each with its reason below.**

## C++ read

- `singleoperation.cpp`
  - `SingleOperation::exportToPROJStringGeneric`: the branches of method 1114 (Geographic3D offset by velocity grid), 1113 (vertical offset by velocity grid), 1144 (geocentric translations NEU velocity grid, gtg) and 1143 (position vector geocentric and NEU velocities, gtg).
  - `_getGeographic3DOffsetByVelocityGridFilename`, `_getVerticalOffsetByVelocityGridFilename`: the file parameter EPSG:1050.
  - `PointMotionOperation::_exportToPROJString` (line 5774): read, not ported (see below).
- `transformation.cpp`: `Transformation::inverseAsTransformation` and `InverseTransformation`. An inverse is the forward operation exported between `startInversion` and `stopInversion`, which the Rust `inverted` flag already does.
- `coordinateoperationfactory.cpp`
  - `findOpsInRegistryDirect` (line 1889): the direct result is `FilterResults`-filtered before it is tested for emptiness.
  - `createOperationsWithDatumPivot` (line 3167): read for case 3. Only its first pass is ported, see below.
  - `SortFunction` (line 913): checked against the Rust `precedes`, which already matches.
  - `createHorizVerticalPROJBased` (line 2717): the null-horizontal case is an exportable `PROJBased` wrapper.
  - `createOperationsCompoundToCompound` (line 7214).

## Rust changes

### `src/iso19111/transform_export.rs`

- Methods 1114, 1113, 1144 and 1143 are written in `export_forward`, before the grid shifts. The functions are `geographic3d_velocity_grid`, `vertical_velocity_grid`, `geocentric_velocity_grid` and `position_vector_velocity_grid`, with helpers `geocentric_endpoints`, `geocentric_ellipsoid_steps`, `velocity_grid_file`, `target_epoch_years`, `epoch_years` and `anchor_years`.
- Epochs are read from the parameters 1068 (source epoch) and 1069 (target epoch) in years, and from the anchor epoch of the datum for the velocity grid methods, as `datumNonNull()->anchorEpoch()` does.
- The interpolation CRS is compared with `demoteTo2D` of the endpoints, as `_exportToPROJString` does.

### `src/iso19111/factory.rs`

- `grid_transformation` now reads the `param1` and `param2` columns, as `other_transformation` already does. Before, the extra parameters of a grid transformation were dropped. Two rows have them: EPSG:10809 (target epoch 2000, needed by case 2) and EPSG:10811 (source and target epochs).
- `create_candidates`: the direct registry result counts as empty when `filter_and_sort` (PROJ's `FilterResults`) removes all of it. The datum pivot runs then. Its result replaces the direct operations only when it is not empty, so the behaviour is unchanged when the pivot finds nothing. This is what gives case 3.

### `src/iso19111/compound.rs` (shared with lot `vert`, see the report)

- `compound_to_compound`: a vertical operation with a null horizontal part is wrapped in `proj_based_horiz_vertical`, the Rust form of `createHorizVerticalPROJBased`. Before, it was returned as a bare copy. The name is the same. The export now includes the horizontal `axisswap` and `unitconvert` around the vertical step, as PROJ writes it. This is needed for case 4 (vertical velocity).

## Cases ported

| C++ TEST | Rust test | Notes |
|---|---|---|
| `createOperation_Geographic3D_Offset_by_velocity_grid` | `operation_create_operation_geographic3d_offset_by_velocity_grid` | Method 1114, the inverse of EPSG:10534 (the first operation). |
| `createOperation_ITRF2014_to_ETRS89_DNK` | `operation_create_operation_itrf2014_to_etrs89_dnk` | EPSG:10894, a concatenation of 8880 (time-dependent Helmert), 10809 (method 1144) and 10893 (method 1143). |
| `createOperation_test_createOperationsWithDatumPivot_iter_1` | `operation_create_operation_test_create_operations_with_datum_pivot_iter_1` | The first operation is the datum pivot through the `(3)` operation. |
| `createOperation_Vrtical_Offset_by_velocity_grid` | `operation_create_operation_vrtical_offset_by_velocity_grid` | Method 1113 with `push`/`pop` of the horizontal position. |

The tests use `export_to_proj_string_with_database`, which writes the PROJ grid names of the database (`NAD83v70VG.gvb` becomes `ca_nrc_NAD83v70VG.tif`). PROJ's factory has already renamed the grids of the operations it returns, so the C++ test calls `exportToPROJString` on the renamed grids.

## Cases not ported

Each reason is a missing C++ logic, not a tolerance or an expectation changed.

| C++ TEST | What the crate returns | Missing logic |
|---|---|---|
| `createOperation_compound_to_compound_with_Geographic3D_Offset_by_velocity_grid` | no operation (checked with a simplified WKT of the NAD83(CSRS)v3 MTM 7 + CGVD28 compound, without the GEOIDMODEL of the C++ test) | `createOperationsCompoundToCompound` with differing horizontal geographic CRS (NAD83(CSRS)v3 to NAD83(CSRS)v7 velocity, then the projection change) and the geoid `vgridshift` chain. `compound_to_compound` only accepts equivalent horizontal parts. |
| `NAD83_to_projected_CRS_based_on_NAD83_2011` | one ballpark operation, PROJ gives two | Datum pivot to a projected target. The crate pivots between geodetic CRS only. PROJ builds `NAD83 to NAD83(HARN) (47) + ... + California Albers` through the NAD83(2011) base of the target. |
| `createOperation_ETRF2000_to_Amersfoort` | one ballpark operation | `createOperationsWithDatumPivot` through `ETRS89-NLD [AGRS2010]`. The middle leg is itself a chain of two operations, which the crate's pivot does not build (`datum_pivot_operations` takes one direct registry operation between the candidate CRS). |
| `createOperation_ETRS89_XXX_to_ETRS89_YYY_using_ETRF2000` | one ballpark operation | Same missing two-leg pivot through ETRF2000 (`ETRS89-PRT [1995] to ETRF2000 (2)` and `Inverse of ETRS89-ESP [ERGNSS] to ETRF2000 (1)`). |

Not ported either: `PointMotionOperation` (methods 1141 and 1139, the Canada velocity grid v6 to v8 and NKG Gravsoft). PROJ writes it with `set` steps for the coordinate epochs (`_exportToPROJString`, line 5774), and the crate has no coordinate epochs. Its export stays `Unimplemented`, as before.

## Deliberate differences

- None in the output of the ported cases. The grid names are the PROJ names of the database, as above.

## Commands and results

- `cargo test --test unit_opfac_core_c_deformation`: 4 passed, 0 failed.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: 1376 passed, 0 failed, 26 ignored (the 26 are pre-existing ignores in other files).
- Oracle lines (`projinfo`, `--spatial-test intersects --grid-check none`):
  - `EPSG:8254 -> EPSG:8239`: operation 1 is `Inverse of NAD83(CSRS)v3 to NAD83(CSRS)v7 (2)`, the velocity grid pipeline with `+dt=13`.
  - `EPSG:7789 -> EPSG:10890`: `EPSG:10894 ITRF2014 to ETRS89-DNK (1)`, with `+t_epoch=2010` and `+t_epoch=2000`.
  - `NAD83(CSRS)v7 + CGVD2013a(2002) height -> NAD83(CSRS)v7 + CGVD2013a(2010) height`: operation 1 is the inverse of `CGVD2013a(2010) height to CGVD2013a(2002) height (1)`, with `push` `dt=-8` `pop`.
