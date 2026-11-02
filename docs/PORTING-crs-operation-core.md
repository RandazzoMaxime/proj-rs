# Port — `crs-operation-core`

PROJ 9.9.0 `test/unit/test_operation.cpp` and `test/unit/test_crs.cpp`, the `core` class in `docs/UNIT-TRIAGE.md`. Tests live in `tests/unit_crs_operation_core.rs`. One Rust function per C++ `TEST()`, named `operation_<name>` or `crs_<name>`.

## Operation tests ported (31)

| C++ `TEST` | Rust |
|---|---|
| `transformation_to_wkt` | `operation_transformation_to_wkt` |
| `concatenated_operation` | `operation_concatenated_operation` |
| `transformation_createTimeDependentPositionVector` | `operation_transformation_create_time_dependent_position_vector` |
| `transformation_createTimeDependentCoordinateFrameRotation` | `operation_transformation_create_time_dependent_coordinate_frame_rotation` |
| `transformation_successive_helmert_noop` | `operation_transformation_successive_helmert_noop` |
| `transformation_successive_helmert_non_trivial_1` | `operation_transformation_successive_helmert_non_trivial_1` |
| `transformation_successive_helmert_non_trivial_2` | `operation_transformation_successive_helmert_non_trivial_2` |
| `transformation_createMolodensky` | `operation_transformation_create_molodensky` |
| `transformation_createAbridgedMolodensky` | `operation_transformation_create_abridged_molodensky` |
| `transformation_inverse` | `operation_transformation_inverse` |
| `equidistant_cylindrical_lat_0` | `operation_equidistant_cylindrical_lat_0` |
| `geostationary_satellite_sweep_x_export` | `operation_geostationary_satellite_sweep_x_export` |
| `webmerc_export` | `operation_webmerc_export` |
| `webmerc_import_from_GDAL_wkt1` | `operation_webmerc_import_from_gdal_wkt1` |
| `webmerc_import_from_GDAL_wkt1_with_EPSG_code` | `operation_webmerc_import_from_gdal_wkt1_with_epsg_code` |
| `webmerc_import_from_GDAL_wkt1_EPSG_3785_deprecated` | `operation_webmerc_import_from_gdal_wkt1_epsg_3785_deprecated` |
| `webmerc_import_from_WKT2_EPSG_3785_deprecated` | `operation_webmerc_import_from_wkt2_epsg_3785_deprecated` |
| `webmerc_import_from_broken_esri_WGS_84_Pseudo_Mercator` | `operation_webmerc_import_from_broken_esri_wgs_84_pseudo_mercator` |
| `mercator_spherical_import` | `operation_mercator_spherical_import` |
| `wkt1_import_polar_stereographic_variantA` | `operation_wkt1_import_polar_stereographic_variant_a` |
| `wkt1_import_polar_stereographic_variantB` | `operation_wkt1_import_polar_stereographic_variant_b` |
| `wkt1_import_polar_stereographic_ambiguous` | `operation_wkt1_import_polar_stereographic_ambiguous` |
| `wkt1_import_equivalent_parameters` | `operation_wkt1_import_equivalent_parameters` |
| `conversion_inverse` | `operation_conversion_inverse` |
| `laborde_oblique_mercator` | `operation_laborde_oblique_mercator` |
| `adams_ws2_export` | `operation_adams_ws2_export` |
| `hyperbolic_cassini_soldner` | `operation_hyperbolic_cassini_soldner` |
| `mercator_variant_A_to_variant_B_no_crs` | `operation_mercator_variant_a_to_variant_b_no_crs` |
| `three_param_equivalent_to_seven_param` | `operation_three_param_equivalent_to_seven_param` |
| `position_vector_equivalent_coordinate_frame` | `operation_position_vector_equivalent_coordinate_frame` |
| `conversion_missing_parameter` | `operation_conversion_missing_parameter` |

CRS `core` tests are still open. See the ledger.

## Behavior

- **Inverse conversion PROJ string.** `Conversion::export_to_proj_string` writes the forward string inside `+proj=pipeline +step +inv`, as `InverseCoordinateOperation` does.
- **WKT1 polar stereographic.** Scale 1 (including a missing scale) is variant B. A latitude within `1e-10` degrees of ±90 with any other scale is variant A. Anything else keeps the method name `Polar_Stereographic`.
- **WKT1 equivalent parameters.** A method parameter is chosen by WKT1 name first (`equivalent_name`), then by `areEquivalentParameters`. The WKT2 name is stored even when the EPSG code is 0.
- **`+proj=labrd`.** `+pm` is a datum key, not a method parameter, so Laborde with `+pm=paris` imports.
- **Adams Square II.** `Adams_Square_II` exports as `+proj=spilhaus` with `k_0 = sqrt(2) * Scale_Factor` (`Scale_Factor` defaults to 1).
- **Helmert equivalence.** A geocentric translation, a position vector, and a coordinate frame rotation match when their seven `+towgs84` values agree within `1e-10` relative tolerance. Coordinate-frame rotations are negated. Source, target, interpolation CRS, accuracy, and the inverse flag still have to match.
- **Missing conversion parameters.** Under `Equivalent`, a parameter present on only one side is accepted when a scale is 1 or any other measure is 0. The check runs both ways. A same-named parameter with a different value is not equivalent.
- **UTM rewrite.** A WKT1 transverse Mercator that is a UTM zone is still replaced by the stock UTM conversion, except when it carries a parameter the method does not define (`UNKNOWN` = 13).

## Left open

The remaining `core` names stay in `docs/UNPORTED-UNIT.md`. They need a PROJ-based single operation, axis-order operations from the coordinate-operation factory, grid and TIN transformations, or the CRS identify / bound / promote surface. No expected value was loosened.

## Verification

```
cargo test --offline --test unit_crs_operation_core
cargo test --offline --test test_io wkt_parse_wkt1_polar
cargo test --offline --test test_operation_conv operation_conversion_missing_parameter_scale
cargo test --offline --test test_operation
```

All of those passed (31, 5, 1, and 62 tests). `cargo check --offline --all-targets --all-features` is clean. A later relink, including `cargo clippy`, failed because `xcode-select` has no developer directory, so `cc` cannot find `MacOSX.sdk`.
