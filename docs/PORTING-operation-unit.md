# Port — PROJ `test/unit/test_operation.cpp`, conversion constructors and exports

Sources, PROJ 9.9.0:

- `test/unit/test_operation.cpp`: the tests.
- `src/iso19111/conversions.cpp` / `operation/conversion.cpp`: `Conversion::create*` parameter lists, `Conversion::_exportToWKT` (the omitted zero parameters, line ~3453), `Conversion::_exportToPROJString` (the checks, line ~4030 to ~4100).
- `src/iso19111/io.cpp`: the WKT1 import of `Mercator_1SP` (the GDAL ticket 3026 hack, line ~4223) and `Mercator_2SP` (line ~4424).

## Tests ported (42)

Each test is one function in `tests/test_operation.rs`, named `operation_<C++ name>` in snake case. The expected text is copied from `test_operation.cpp`. A PROJ string, a WKT2 string (`WktFormatter::create(Wkt2_2019)`, the default of `WKTFormatter::create()`), or a WKT1 GDAL string, is compared exactly.

| C++ TEST | Rust test |
|---|---|
| `tped_export` | `operation_tped_export` (WKT2 with `simul_cur_node_has_id`) |
| `tmg_export` | `operation_tmg_export` (PROJ string is an error) |
| `aea_export` | `operation_aea_export` |
| `azimuthal_equidistant_export` | `operation_azimuthal_equidistant_export` |
| `guam_projection_export` | `operation_guam_projection_export` (WKT1 is an error) |
| `lambert_cylindrical_equal_area_spherical_export` | `operation_lambert_cylindrical_equal_area_spherical_export` |
| `lambert_cylindrical_equal_area_export` | `operation_lambert_cylindrical_equal_area_export` |
| `lcc1sp_variant_b_export` | `operation_lcc1sp_variant_b_export` |
| `lcc2sp_export` | `operation_lcc2sp_export` |
| `lcc2sp_michigan_export` | `operation_lcc2sp_michigan_export` (WKT1 is an error) |
| `lcc2sp_belgium_export` | `operation_lcc2sp_belgium_export` |
| `cassini_soldner_export` | `operation_cassini_soldner_export` |
| `equidistant_conic_export` | `operation_equidistant_conic_export` |
| `eckert_export` | `operation_eckert_export` (the six Eckert methods) |
| `createEquidistantCylindrical` | `operation_create_equidistant_cylindrical` |
| `createEquidistantCylindricalSpherical` | `operation_create_equidistant_cylindrical_spherical` |
| `gall_export` | `operation_gall_export` |
| `goode_homolosine_export` | `operation_goode_homolosine_export` |
| `interrupted_goode_homolosine_export` | `operation_interrupted_goode_homolosine_export` |
| `geostationary_satellite_sweep_y_export` | `operation_geostationary_satellite_sweep_y_export` |
| `gnomonic_export` | `operation_gnomonic_export` |
| `hotine_oblique_mercator_variant_A_export` | `operation_hotine_oblique_mercator_variant_a_export` |
| `hotine_oblique_mercator_variant_A_export_swiss_mercator` | `operation_hotine_oblique_mercator_variant_a_export_swiss_mercator` |
| `hotine_oblique_mercator_variant_B_export` | `operation_hotine_oblique_mercator_variant_b_export` |
| `hotine_oblique_mercator_variant_B_export_swiss_mercator` | `operation_hotine_oblique_mercator_variant_b_export_swiss_mercator` |
| `hotine_oblique_mercator_two_point_natural_origin_export` | `operation_hotine_oblique_mercator_two_point_natural_origin_export` |
| `laborde_oblique_mercator_export` | `operation_laborde_oblique_mercator_export` |
| `imw_polyconic_export` | `operation_imw_polyconic_export` |
| `krovak_north_oriented_export` | `operation_krovak_north_oriented_export` |
| `krovak_export` | `operation_krovak_export` |
| `lambert_azimuthal_equal_area_export` | `operation_lambert_azimuthal_equal_area_export` |
| `miller_cylindrical_export` | `operation_miller_cylindrical_export` |
| `wkt1_import_mercator_variant_A` | `operation_wkt1_import_mercator_variant_a` |
| `wkt1_import_mercator_variant_A_that_is_variant_B` | `operation_wkt1_import_mercator_variant_a_that_is_variant_b` |
| `mercator_variant_B_export` | `operation_mercator_variant_b_export` |
| `odd_mercator_1sp_with_non_null_latitude` | `operation_odd_mercator_1sp_with_non_null_latitude` |
| `odd_mercator_2sp_with_latitude_of_origin` | `operation_odd_mercator_2sp_with_latitude_of_origin` |
| `mercator_spherical_export` | `operation_mercator_spherical_export` |
| `mollweide_export` | `operation_mollweide_export` |
| `polar_stereographic_variant_A_export` | `operation_polar_stereographic_variant_a_export` |
| `polar_stereographic_variant_B_export_positive_lat` | `operation_polar_stereographic_variant_b_export_positive_lat` |
| `polar_stereographic_variant_B_export_negative_lat` | `operation_polar_stereographic_variant_b_export_negative_lat` |

The ledger `docs/UNPORTED-UNIT.md` drops these 42 names. After this lot the `test_operation.cpp` section has 102 names. Counted by C++ `TEST()` name, 60 of the 162 are ported (18 before, the 18 ported by earlier lots that the ledger did not list).

## API added

`src/iso19111/buildop.rs`, following the existing `create_*` constructors:

- `Conversion::create_hotine_oblique_mercator_two_point_natural_origin` (no EPSG method code, by name, as PROJ).
- `Conversion::create_laborde_oblique_mercator` (EPSG:9813).
- `Conversion::create_international_map_world_polyconic` (no EPSG method code, by name).

## Behavior fixed

- **Equidistant Cylindrical constructors.** `create_equidistant_cylindrical` and `create_equidistant_cylindrical_spherical` passed four values for a five-parameter method. PROJ `createEquidistantCylindrical` passes `createParams(latitudeFirstParallel, 0.0, longitudeNatOrigin, falseEasting, falseNorthing)`. The Rust constructors now do the same. Nothing else called them.
- **Zero latitude of origin omitted (Equidistant Cylindrical, EPSG 1028 and 1029).** PROJ leaves `Latitude of natural origin` out of WKT when it is 0, because EPSG has no such parameter for the method. The rule is `omitted_zero_parameter` in `src/iso19111/wkt.rs`, used by both the WKT2 and the WKT1 parameter loops. Before, WKT1 applied it to 1029 only. It now applies to 1028 too, as in PROJ.
- **Zero false easting and northing omitted (Vertical Perspective, EPSG 9838).** The same rule as PROJ `_exportToWKT`. No test in this lot covers it: the Vertical Perspective tests are still open.
- **Mercator variant B export (EPSG 9805).** `export_to_proj_string` now refuses a non-zero `Latitude of natural origin` (as PROJ does for 9804), and a `Scale factor at natural origin` other than 1, with `Unexpected presence of scale factor in Mercator (variant B)` (PROJ `_exportToPROJString`, the variant B branch).
- **WKT1 import of `Mercator_1SP` with a non-zero `latitude_of_origin`.** GDAL ticket 3026. PROJ `io.cpp` reads it as Mercator variant B: the method becomes `Mercator_2SP`, `latitude_of_origin` becomes `standard_parallel_1`, and a `scale_factor` of 1 is dropped. A `center_latitude` parameter disables the hack, as in PROJ. The Rust code is in `build_wkt1_projection` (`src/iso19111/wkt_import.rs`).

## Deliberate differences

- The Krovak azimuth `30.2881397527778` is written `30.288_139_752_777_8` in Rust (digit separators only).
- `geostationary_satellite_sweep_x_export` is not ported (see below). Its WKT1 check needs the `EXTENSION["PROJ4", ...]` node of a projected CRS, which the crate does not write.
- The 3026 hack drops only a `scale_factor` whose value is exactly 1. PROJ compares the text `"1"`, so `1.0` or `1.00` in a WKT would be kept by PROJ and dropped here. No test has such a value.

## Still open, and why

These 102 names stay in `docs/UNPORTED-UNIT.md`. Grouped by cause. Each cause is checked against the source tree (`grep` in `src/`), except the last group, which has not been attempted.

1. **`Transformation` constructors are missing** (17): `transformation_createGeocentricTranslations` (with `_between_geocentricCRS`, `_null`, `_neg_zero`), `createPositionVector`, `createCoordinateFrameRotation`, `createTimeDependentPositionVector`, `createTimeDependentCoordinateFrameRotation`, `createMolodensky`, `createAbridgedMolodensky`, `createTOWGS84`, `transformation_successive_helmert_noop`, `_non_trivial_1`, `_non_trivial_2`, `three_param_equivalent_to_seven_param`, `position_vector_equivalent_coordinate_frame`, `transformation_inverse`. The crate has only `Transformation::create` (explicit parameters). No `create_geocentric_translations` or `create_position_vector` exists.
2. **No `Transformation` or `ConcatenatedOperation` WKT export, no public `ConcatenatedOperation` constructor** (2): `transformation_to_wkt`, `concatenated_operation`. `export_to_wkt` exists for `Conversion`, `Crs`, datums and ellipsoids only.
3. **Projection constructors missing from `buildop.rs`** (16): `nzmg_export`, `oblique_stereographic_export`, `orthographic_export`, `local_orthographic_export`, `american_polyconic_export`, `robinson_export`, `sinusoidal_export`, `stereographic_export`, `vandergrinten_export`, `wagner_export`, `wagnerIII_export`, `qsc_export`, `sch_export`, `eqearth_export`, `vertical_perspective_export`, `vertical_perspective_export_no_topocentric_height_and_false_easting_northing`. The method names are in `parammap.rs`, so the constructors are the missing step.
4. **Depend on the operation factory or the authority factory** (13, owned by the factory lot): `webmerc_export` (`CoordinateOperationFactory`), `geographic_topocentric`, `geocentric_topocentric`, `normalizeForVisualization`, `createAxisOrderReversal` (`Conversion::createAxisOrderReversal` is also missing), `adams_ws2_export`, `spilhaus_esri_export`, `hyperbolic_cassini_soldner` (these three build a CRS from `AuthorityFactory`), `operation_Geog3D_to_Geog2D_GravityRelatedHeight_with_compoundCRS`, `operation_Geographic2D_Offsets_by_TIN_Interpolation_JSON`, `operation_Position_Vector_geocen_and_geocen_translations_NEU_velocities_gtg_with_source_epoch`, `operation_geocen_translations_by_grid_and_geocen_translations_NEU_velocities_gtg`, `operation_geocen_translations_NEU_velocities_gtg`.
5. **`Conversion::convertToOtherMethod` is missing** (21): `mercator_variant_A_to_variant_B` (and `_scale_1`, `_no_crs`, `_invalid_scale`, `_invalid_eccentricity`), `mercator_variant_B_to_variant_A` (and `_invalid_std1`, `_invalid_eccentricity`), `lcc2sp_to_lcc1sp` (and its `phi0_eq_phi1_eq_phi2`, `phi0_diff_phi1_and_phi1_eq_phi2`, `invalid_lat0`, `invalid_lat1`, `invalid_lat2`, `invalid_lat1_opposite_lat2`, `invalid_lat1_and_lat2_close_to_zero`, `invalid_eccentricity` cases), `lcc1sp_to_lcc2sp_invalid_eccentricity`, `_invalid_scale`, `_invalid_lat0`, `_null_lat0`. `grep` finds no `convert_to_other_method` in `src/`.
6. **`SingleOperation::createPROJBased` is missing** (3): `PROJ_based`, `PROJ_based_empty`, `PROJ_based_with_global_parameters`.
7. **WKT1 export of a projected CRS needs `EXTENSION["PROJ4", ...]`** (1): `geostationary_satellite_sweep_x_export`. PROJ writes it from `Conversion::addWKTExtensionNode` (`operation/conversion.cpp`, line 3707), whose branches for tmerc approx and Web Mercator are not ported. This is a CRS WKT lot.
8. **WKT2 imports of `Transformation`, `BoundCRS`, point motion and seismic bin grids** (10), not examined in detail: `export_of_Geographic3D_to_GravityRelatedHeight_gtx_unknown_grid`, `export_of_boundCRS_with_proj_string_method`, `PointMotionOperation_with_epochs`, `export_of_Cartesian_Grid_Offsets_with_EngineeringCRS`, `Geographic3DToGravityRelatedHeight`, `inverse_of_Geographic3DToGravityRelatedHeight`, `CoordinateFrameRotationFullMatrixGeog2D`, `helmert_between_geog3D_and_compound`, `export_of_seismic_bin_grid_I_eq_J_plus_90`, `export_of_seismic_bin_grid_I_eq_J_minus_90`. They parse `COORDINATEOPERATION` WKT2 (`WktObject::transformation` exists), and several use gravity, grid, or seismic methods whose support is not checked.
9. **Not attempted in this lot** (19): `tmerc_south_oriented_export`, `tmerc_south_oriented_export_esri_102470`, `bonne_export`, `equidistant_cylindrical_lat_0` (PROJ string parser), `webmerc_import_from_GDAL_wkt1`, `webmerc_import_from_GDAL_wkt1_with_EPSG_code`, `webmerc_import_from_GDAL_wkt1_EPSG_3785_deprecated`, `webmerc_import_from_WKT2_EPSG_3785_deprecated`, `webmerc_import_from_broken_esri_WGS_84_Pseudo_Mercator`, `mercator_spherical_import`, `wkt1_import_polar_stereographic_variantA`, `wkt1_import_polar_stereographic_variantB`, `wkt1_import_polar_stereographic_ambiguous`, `wkt1_import_equivalent_parameters`, `conversion_missing_parameter`, `conversion_missing_parameter_scale`, `laborde_oblique_mercator` (PROJ string parser), `hotine_oblique_mercator_variant_A_export_equivalent_modulo_360`, `conversion_inverse`. Their constructors exist or are close; the expected text has not been compared yet.

`operation_tmerc_south_oriented_export` already existed in `tests/test_operation.rs` before this lot and checks less than the C++ TEST. The ledger keeps `tmerc_south_oriented_export` open.

## Not changed on purpose

- The factory selection (`createOperations`, the sort) is not touched.
- `bench/` is unchanged. A conversion constructor and its WKT are not a per-point cost, as in `PORTING-transformation-export.md`.
- `docs/UNPORTED-CLI.md` is rewritten by the `cli_yaml` harness during the test run. It is restored with `git checkout` and not committed.

## Verification

Commands run on branch `lot-unit-operation`:

- `cargo test --offline --test test_operation` with `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`: 62 passed, 0 failed.
- `PATH=… cargo test --all-features --no-fail-fast`: every test binary passes (`proj_rs` lib 91, `test_crs` 80, `test_factory` 58, `test_io` 56, `test_operation` 62, `transform_export` 17, `cli_yaml` 1, and the others).
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt --check`: clean.
