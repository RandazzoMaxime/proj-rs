# Porting sheet: capi-factory-core

PROJ 9.9.0 `test/unit/test_c_api.cpp` and `test/unit/test_factory.cpp`, the `core`
rows in `docs/UNIT-TRIAGE.md`. Rust tests are `tests/unit_capi_factory_core.rs`.
Each function is one `TEST()`, with the C++ name in a comment. Snake case drops
the capital `D` that clippy rejects in identifiers (`promote_to_3d`).

## Mapping

| PROJ `TEST()` | Rust |
|---|---|
| `AuthorityFactory_createCoordinateOperation_helmert_8_CF` | `factory_authority_factory_create_coordinate_operation_helmert_8_cf` |
| `AuthorityFactory_createCoordinateOperation_helmert_15_CF` | `factory_authority_factory_create_coordinate_operation_helmert_15_cf` |
| `AuthorityFactory_createCoordinateOperation_CF_full_matrix_geog3D` | `factory_authority_factory_create_coordinate_operation_cf_full_matrix_geog3d` |
| `AuthorityFactory_createCoordinateOperation_molodensky_badekas_PV` | `factory_authority_factory_create_coordinate_operation_molodensky_badekas_pv` |
| `AuthorityFactory_createCoordinateOperation_grid_transformation_one_parameter` | `factory_authority_factory_create_coordinate_operation_grid_transformation_one_parameter` |
| `AuthorityFactory_createCoordinateOperation_grid_transformation_two_parameter` | `factory_authority_factory_create_coordinate_operation_grid_transformation_two_parameter` |
| `AuthorityFactory_createCoordinateOperation_other_transformation` | `factory_authority_factory_create_coordinate_operation_other_transformation` |
| `AuthorityFactory_10566_issue_4212` | `factory_authority_factory_10566_issue_4212` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_inverse_step1` | `..._inverse_step1` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_inverse_step2` | `..._inverse_step2` |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_step_2_and_3_are_conversion` | `..._step_2_and_3_are_conversion` (EPSG:7987) |
| `AuthorityFactory_createCoordinateOperation_concatenated_operation_epsg_9103` | `..._epsg_9103` |
| `AuthorityFactory_createFromCoordinateReferenceSystemCodes` | `factory_authority_factory_create_from_coordinate_reference_system_codes` |
| `AuthorityFactory_createFromCoordinateReferenceSystemCodes_anonymous_authority` | `..._anonymous_authority` |
| `createObjectsFromName` | `factory_create_objects_from_name` |
| `getPointMotionOperationsFor` | `factory_get_point_motion_operations_for` |
| `proj_crs_create_bound_crs_to_WGS84` | `c_api_proj_crs_create_bound_crs_to_wgs84` |
| `proj_create_from_name` | `c_api_proj_create_from_name` |
| `proj_identify` | `c_api_proj_identify` |
| `proj_create_geographic_crs` | `c_api_proj_create_geographic_crs` |
| `proj_create_geocentric_crs` | `c_api_proj_create_geocentric_crs` |
| `proj_create_projected_crs` | `c_api_proj_create_projected_crs` |
| `proj_create_transformation` | `c_api_proj_create_transformation` |
| `proj_create_compound_crs` | `c_api_proj_create_compound_crs` |
| `proj_convert_conversion_to_other_method` | `c_api_proj_convert_conversion_to_other_method` |
| `proj_get_non_deprecated` | `c_api_proj_get_non_deprecated` |
| `proj_query_geodetic_crs_from_datum` | `c_api_proj_query_geodetic_crs_from_datum` |
| `proj_uom_get_info_from_database` | `c_api_proj_uom_get_info_from_database` |
| `proj_grid_get_info_from_database` | `c_api_proj_grid_get_info_from_database` |
| `proj_get_crs_info_list_from_database` | `c_api_proj_get_crs_info_list_from_database` |
| `proj_get_units_from_database` | `c_api_proj_get_units_from_database` |
| `proj_normalize_for_visualization_on_crs` | `c_api_proj_normalize_for_visualization_on_crs` |
| `proj_as_projjson` | `c_api_proj_as_projjson` |
| `proj_crs_promote_to_3D` | `c_api_proj_crs_promote_to_3d` |
| `proj_crs_demote_to_2D` | `c_api_proj_crs_demote_to_2d` |
| `proj_crs_create_projected_3D_crs_from_2D` | `c_api_proj_crs_create_projected_3d_crs_from_2d` |
| `proj_crs_create_bound_vertical_crs` | `c_api_proj_crs_create_bound_vertical_crs` |
| `datum_ensemble` | `c_api_datum_ensemble` |
| `proj_get_geoid_models_from_database` | `c_api_proj_get_geoid_models_from_database` |
| `proj_crs_is_dynamic` | `c_api_proj_crs_is_dynamic` |

`Crs::is_dynamic` has no boolean: WGS 84 is not dynamic (`considerWGS84AsDynamic` is false).
`DatabaseContext::grid_info` is `lookForGridInfo` with known grids not treated as available.
`Crs::non_deprecated` is `CRS::getNonDeprecated`.

## Behaviour added for those tests

- Concatenated operations with a null step direction run `fix_unspecified_step_directions`: the first step is inverted when its target is the concatenated source, the last step is the mirror, a geographic/geocentric link is inserted when the names match and the kinds differ, and a conversion with no CRS receives the neighbouring CRS (method 1068 synthesizes the depth CRS). The last conversion's target is the concatenated target.
- A synthetic geographic↔geocentric step is a transformation of method 9602. Its name is `Conversion from {name} (geog2D|geog3D) to {name} (geocentric)` when the CRS names match. That is the PROJ name. `docs/PORTING-cs2cs-epochs.md` recorded the unqualified `Conversion from {name} to {name}`.
- Vertical offset (EPSG:9616) and conversions 1068 and 1104 export as pipeline steps. An inverted geogoffset negates `+dh`.
- `create_bound_crs_to_wgs84_if_possible` binds through the Helmert step of a concatenated longitude-rotation plus Helmert (EPSG:4807).
- `DatabaseContext::create_geographic_crs` / `create_geocentric_crs` follow `proj_create_geographic_crs` and `proj_create_geocentric_crs`. `WGS_1984` becomes `World Geodetic System 1984`. A datum name containing `_` is looked up with an approximate name search, then `official_name_from_alias` with equivalent spelling, because the approximate search does not fold accents (the Ferro WKT1 datum name). `metre`/`meter` and `degree`/`grad` are stored as the canonical units so the stricter axis-unit-name check matches EPSG:4978 and EPSG:4326. A name ending in ` (deprecated)` is stored without the suffix and marked deprecated.
- `Conversion::create`, `VerticalCRS::create`, and `VerticalCS::create_gravity_related_height` are the constructors `proj_create_projected_crs`, `proj_create_compound_crs`, and the bound vertical CRS use.
- `Crs::identify_with` dispatches a projected CRS. A longitude/latitude base is identified as latitude/longitude. A UTM zone whose base is not WGS 84, NAD27, or NAD83 is looked up with `AuthorityFactory::utm_projected_codes` (method 9807, central meridian of the zone). IGNF `ETRS89UTM28` is EPSG:25828 at confidence 70. EPSG:3040 is the same zone with northing/easting and scores 50, so it is dropped.
- The ellipsoid `proj_identify` returns null in C because an ellipsoid is not a CRS. The Rust test does not call `identify` on `Ellipsoid`.
- `proj_convert_conversion_to_other_method` is `DerivedConversion::convert_to_other_method` with the EPSG method code. The C++ name path (`Mercator (variant B)` / `Mercator (variant A)`) calls the same function with codes 9805 and 9804. There is no name-to-code lookup. The C++ null result for a non-conversion is not represented: the Rust test starts from `ProjectedCRS::deriving_operation`.
- `proj_get_crs_info_list_from_database` post-filters `AuthorityFactory::get_crs_info_list` in the test the way `c_api.cpp` does (deprecated, type, bbox contains versus intersects). The factory still returns the unfiltered list.
- The bound vertical CRS is `BoundCRS::create` of a vertical CRS, EPSG:4979, and a transformation whose method is `GravityRelatedHeight to Geographic3D` with the geoid file parameter. The compound with EPSG:32631 exports the PROJ string the C++ test asserts.

## Left open

- `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type`: the method is on `Crs`, so a conversion argument cannot be represented.
- `factory.AuthorityFactory_createDynamicVerticalDatum`: the `TEST()` is under `#ifdef no_more_dynamic_vertical_datum` and is not compiled in PROJ 9.9.0. EPSG:1096 has no frame reference epoch. See `docs/PORTING-factory-unit.md`.
- `factory.ogc_timecrs`: `Crs` has no temporal variant (`AnsiDate`, `JulianDate`, `UnixTime`).
- `factory.AuthorityFactory_build_all_concatenated`: EPSG codes 8422, 8481, 8482, 8565, 8566, 8572, 11005, 11398, and 11206 construct. PROJ throws `Inconsistent chaining of CRS in operations`. This crate has no chaining check.
- `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name`: `create_coordinate_operation("9484", Proj)` stores the PROJ grid name, and `export_to_proj_string` returns `Unimplemented ETRS89 to NN54 height (1)`, so the definition string is not available to assert `no_kv_href2008a.tif`.
- `CApi.proj_coordoperation_get_grid_used`: `DatabaseContext::grid_info` returns the database URL. It does not rewrite it with a context endpoint (`https://example.com/ca_nrc_ntv1_can.tif`).
- `CApi.proj_coordoperation_get_grid_used_fullname_caching`: the C++ test is under `TIFF_ENABLED` and opens `tests/test_vgrid_int16.tif`.
- `CApi.proj_coordoperation_is_instantiable`: `isPROJInstantiable` is not ported.
- `CApi.proj_create_operations`, `proj_get_suggested_operation_for_NAD83_to_NAD83_HARN`, `proj_create_operations_prime_meridian_non_greenwich`, `proj_get_suggested_operation_with_operations_without_area_of_use`, `proj_create_operations_discard_superseded`, `proj_create_operations_dont_discard_superseded`, `proj_create_operations_with_pivot`, `proj_create_operations_allow_ballpark_transformations`: `CoordinateOperationFactory::create_operations` returns the registered operations and a pivot only when `set_intermediate_crs` names one. It does not choose a suggested operation by coordinate, discard superseded operations inside that search, or reproduce the C++ counts and names (NAD27 to NAD83 count 10, first name `NAD27 to NAD83 (4)`).
- `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter`, `proj_create_crs_to_crs_from_pj_ballpark_filter`: `Proj::create_crs_to_crs_from_pj` rejects every non-empty option (`CRS-to-CRS options are not built`). The C++ calls succeed with no options and return null for `ACCURACY=0.05` and `ALLOW_BALLPARK=NO`.
- `CApi.proj_normalize_for_visualization_with_alternatives`, `proj_normalize_for_visualization_with_alternatives_reverse`: the expected result is the Roma coordinate through the EPSG:4326 to EPSG:3003 pipeline, including the exact Helmert definition. That pipeline is not reproduced.
- `CApi.proj_coordoperation_create_inverse`: `ProjLines::Multi` wraps at 80 columns with a two-space indent. The C++ string uses `INDENTATION_WIDTH=4` and `MAX_LINE_LENGTH=40`.
- `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations` and the two NAD83(2011) geoid-grid compound tests: they need `proj_create_crs_to_crs` through a bound vertical CRS and a ballpark horizontal step, then a numeric transform.
- `CApi.proj_create_vertical_crs_ex`, `proj_create_vertical_crs_ex_with_geog_crs`, `proj_create_vertical_crs_ex_implied_accuracy`: each asserts the first operation from a compound CRS to a geographic 3D CRS, including a geoid model and an accuracy. That search is the same gap as `proj_create_operations`.
- `CApi.proj_trans_bounds_ignore_inf`, `proj_trans_bounds_ignore_inf_geographic`, `proj_trans_bounds_to_compound_crs`, `proj_trans_bounds_3D_ignore_inf`: `Proj::new_known_crs` does not read `OGC:CRS84`, inline WKT, ESRI codes, or compound EPSG:9707. Recorded in `docs/PORTING-c-api-unit.md`.

`CApi.proj_crs_promote_to_3D_on_coordinate_metadata` is in the ledger and is not in this lot's core list.

## Commands

Linker for this machine: `CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=/tmp/zig-sdk/zigcc`, `CC` and `AR` the same, `SDKROOT` unset. `cargo` prints `xcode-select: note: No developer tools were found` from the build script. That note is not a rustc warning.

```sh
cargo test --offline --test unit_capi_factory_core   # 40 passed
cargo fmt --all
cargo clippy --offline --all-targets --all-features -- -D warnings   # clean
cargo test --offline --test transform_export          # 32 passed
cargo test --offline --test test_crs -- identify     # 2 passed
cargo test --offline --test docs_in_sync             # fails: UNIT-TRIAGE.md and README.md counts are generated and left unchanged
```
