# Port — `test_operationfactory.cpp`, core class, first 114 names

Lot `opfactory-core-a`. PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the first 114 names under `### test_operationfactory.cpp (228)` in `docs/UNIT-TRIAGE.md`, from `operation.geogCRS_to_geogCRS` through `operation.transformation_vertical_offset_to_PROJ_string`.

Rust tests are `tests/unit_opfactory_core_a.rs`. Each function is `operation_<C++ name>` in snake case. Expected names and PROJ strings are copied from the C++ `TEST()`.

## Tests written (39)

| C++ TEST | Rust |
|---|---|
| `geogCRS_to_geogCRS` | `operation_geog_crs_to_geog_crs` |
| `geogCRS_to_geogCRS_context_ntv1_ntv2_ctable2` | `operation_geog_crs_to_geog_crs_context_ntv1_ntv2_ctable2` |
| `geogCRS_to_geogCRS_context_NAD27_to_WGS84` | `operation_geog_crs_to_geog_crs_context_nad27_to_wgs84` |
| `geogCRS_to_geogCRS_context_helmert_geog3D_crs` | `operation_geog_crs_to_geog_crs_context_helmert_geog3d_crs` |
| `geogCRS_to_geogCRS_context_helmert_geocentric_3D` | `operation_geog_crs_to_geog_crs_context_helmert_geocentric_3d` |
| `geogCRS_to_geogCRS_context_helmert_geog3D_to_geocentirc` | `operation_geog_crs_to_geog_crs_context_helmert_geog3d_to_geocentirc` |
| `geogCRS_to_geogCRS_context_same_grid_name` | `operation_geog_crs_to_geog_crs_context_same_grid_name` |
| `geogCRS_to_geogCRS_geographic_offset_context` | `operation_geog_crs_to_geog_crs_geographic_offset_context` |
| `geogCRS_to_geogCRS_noop` | `operation_geog_crs_to_geog_crs_noop` |
| `geogCRS_to_geogCRS_longitude_rotation` | `operation_geog_crs_to_geog_crs_longitude_rotation` |
| `geogCRS_to_geogCRS_3D` | `operation_geog_crs_to_geog_crs_3d` |
| `geogCRS_3D_lat_long_non_metre_to_geogCRS_longlat` | `operation_geog_crs_3d_lat_long_non_metre_to_geog_crs_longlat` |
| `geocentricCRS_to_geogCRS_same_datum` | `operation_geocentric_crs_to_geog_crs_same_datum` |
| `geocentricCRS_to_geogCRS_different_datum` | `operation_geocentric_crs_to_geog_crs_different_datum` |
| `geogCRS_to_geocentricCRS_different_datum` | `operation_geog_crs_to_geocentric_crs_different_datum` |
| `geocentricCRS_to_geocentricCRS_same_noop` | `operation_geocentric_crs_to_geocentric_crs_same_noop` |
| `geocentricCRS_to_geocentricCRS_different_ballpark` | `operation_geocentric_crs_to_geocentric_crs_different_ballpark` |
| `geogCRS_to_projCRS` | `operation_geog_crs_to_proj_crs` |
| `geogCRS_longlat_to_geogCS_latlong` | `operation_geog_crs_longlat_to_geog_cs_latlong` |
| `geogCRS_longlat_to_geogCS_latlong_database` | `operation_geog_crs_longlat_to_geog_cs_latlong_database` |
| `geogCRS_longlat_to_projCRS` | `operation_geog_crs_longlat_to_proj_crs` |
| `geogCRS_different_from_baseCRS_to_projCRS` | `operation_geog_crs_different_from_base_crs_to_proj_crs` |
| `geocentricCRS_to_projCRS` | `operation_geocentric_crs_to_proj_crs` |
| `projCRS_to_geogCRS` | `operation_proj_crs_to_geog_crs` |
| `compoundCRS_to_geogCRS` | `operation_compound_crs_to_geog_crs` |
| `transformation_longitude_rotation_to_PROJ_string` | `operation_transformation_longitude_rotation_to_proj_string` |
| `transformation_Geographic2D_offsets_to_PROJ_string` | `operation_transformation_geographic2d_offsets_to_proj_string` |
| `transformation_Geographic3D_offsets_to_PROJ_string` | `operation_transformation_geographic3d_offsets_to_proj_string` |
| `transformation_Geographic2D_with_height_offsets_to_PROJ_string` | `operation_transformation_geographic2d_with_height_offsets_to_proj_string` |
| `transformation_vertical_offset_to_PROJ_string` | `operation_transformation_vertical_offset_to_proj_string` |
| `transformation_ntv2_to_PROJ_string` | `operation_transformation_ntv2_to_proj_string` |
| `transformation_VERTCON_to_PROJ_string` | `operation_transformation_vertcon_to_proj_string` |
| `transformation_NZLVD_to_PROJ_string` | `operation_transformation_nzlvd_to_proj_string` |
| `transformation_BEV_AT_to_PROJ_string` | `operation_transformation_bev_at_to_proj_string` |
| `geocentricCRS_to_geogCRS_same_datum_context` | `operation_geocentric_crs_to_geog_crs_same_datum_context` |
| `geocentricCRS_to_geogCRS_same_datum_context_all_auth` | `operation_geocentric_crs_to_geog_crs_same_datum_context_all_auth` |
| `geogCRS_geocentricCRS_same_datum_to_context` | `operation_geog_crs_geocentric_crs_same_datum_to_context` |
| `projCRS_to_projCRS` | `operation_proj_crs_to_proj_crs` |
| `transformation_Geographic3D_to_GravityRelatedHeight_gtx` | `operation_transformation_geographic3d_to_gravity_related_height_gtx` |

`docs/UNPORTED-UNIT.md` is unchanged. A name stays on the ledger until `cargo test` runs it.

## Behavior

- `create_operation` is `createOperations` with `RegistryUse::None`, first candidate.
- No registered operation between two geodetic CRS falls through to `geodetic_operations` (`createOperationsGeodToGeod`), including a same-datum geographic/geocentric conversion. A non-empty registry list still only adds the ballpark.
- A projected target with no registry uses its deriving conversion when the geographic source has the same datum, the same angular unit, and the same dimension (`same_geographic_base`). The caller's axis order is the one exported. The database path still requires the same authority code.
- Longitude rotation (9601), geographic offsets (9619, 9660, 9618), vertical offset (9616), and Cartesian grid offsets (9656) invert by negating parameter values. Change of vertical unit stays on the inverted-flag path.
- Vertical offset export is `geogoffset +dh` between the linear units of two vertical CRS.
- WKT parameters with no identifier take the EPSG code of the method parameter of the same name. Method 9665 is `Geographic3D to GravityRelatedHeight (US .gtx)`, file parameter 8666.
- A name that starts with `Conversion from ` inverts as a conversion, including method 9602 stored as a transformation.

## Deliberate differences

- `geogCRS_longlat_to_geogCS_latlong` compares the operation method by name and EPSG code. `OperationMethod` has no WKT export and no `isEquivalentTo`.
- `transformation_Geographic3D_to_GravityRelatedHeight_gtx` asserts the PROJ string only. `gridsNeeded` is not a public API.
- `transformation_height_to_PROJ_string` is not written: it also asserts `gridsNeeded`, and `createGravityRelatedHeightToGeographic3D` is not ported.

## Left open

Diagnosed by running the factory before the linker broke, or by reading the factory. The test was removed rather than loosened.

- `geogCRS_to_geogCRS_context_inverse_needed`: a direct operation exists, so the same-datum path through RGF93 v1 (EPSG:4171, `fr_ign_gr3df97a.tif`) is not searched.
- `geogCRS_to_geogCRS_context_lonlat_vs_latlon_crs`: the datum pivot treats two geographic CRS as an identity leg, so it does not wrap axis-order changes and does not search WGS 84 as a third CRS.
- `geogCRS_to_geogCRS_context_ED50_to_WGS72_no_NTF_intermediate`: no datum-pivot search through a non-NTF intermediate. The only candidate is the ballpark.
- `geogCRS_to_geogCRS_context_NAD27_to_WGS84_G1762`: no ensemble hub through WGS 84 when the target is G1762 rather than EPSG:4326.
- `geogCRS_to_geogCRS_context_match_by_name` and `geogCRS_to_geogCRS_context_datum_ensemble`: registry lookup uses the CRS identifier, not the datum name.
- Bound CRS tests (`boundCRS_*`, `nadgrids_with_pm`, `towgs84_pm_3d`, `WGS84_G1762_to_compoundCRS_with_bound_vertCRS`, `geogCRS_with_towgs84_to_geocentric`): `createOperations` has no BoundCRS branch.
- The remaining context tests in this lot (concatenated NTF and Egypt, CH1903, IGNF, projected-CRS context, 3D promotion, ITRF intermediates) need that pivot or bound branch. They are not given a weakened assertion.

## Verification

This machine has no Command Line Tools and no MacOSX SDK (`xcode-select -p` fails, `/usr/bin/cc` is the xcode-select stub). `cargo test` stops at the link. Nothing in this file is reported as passed.

- `cargo check --offline --test unit_opfactory_core_a`: finished. The only warning is the untracked `tests/zz_scratch.rs`, which is not part of the lot.
- `cargo clippy --offline --test unit_opfactory_core_a --no-deps -- -D warnings`: finished, no warnings.
- `cargo fmt` on the touched Rust files.

An earlier session, before the inverse-negation and projected-CRS edits, ran `cargo test --offline --test unit_opfactory_core_a` and saw the first 17 tests pass. That run is not repeated here.
