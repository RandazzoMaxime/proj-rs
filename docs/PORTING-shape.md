# Porting sheet: the `shape` class (test_c_api, test_crs, test_factory, test_io)

Source, PROJ 9.9.0: `test/unit/test_c_api.cpp`, `test_crs.cpp`, `test_factory.cpp` and `test_io.cpp`, fetched from the 9.9.0 tag. The `shape` class of `docs/UNIT-TRIAGE.md` holds 76 `TEST()` names that check the C++ object model or the C ABI. This lot ports the ones that still pin a value a Rust user can observe: **20 ported, 56 documented** (76 in total). Expected text and numbers are copied from the C++ tests. New tests are in `tests/unit_shape.rs`.

## Test mapping (20 ported)

| PROJ `TEST()` | Rust test in `tests/unit_shape.rs` |
|---|---|
| `wkt_parse.cs_with_MERIDIAN` | `wkt_parse_cs_with_meridian` |
| `wkt_parse.cs_with_multiple_ID` | `wkt_parse_cs_with_multiple_id` |
| `wkt_parse.cs_with_AXISMINVAL_AXISMAXVAL_RANGEMEANING` | `wkt_parse_cs_with_axisminval_axismaxval_rangemeaning` |
| `wkt_parse.cs_with_invalid_AXISMINVAL_string` | `wkt_parse_cs_with_invalid_axisminval_string` |
| `wkt_parse.cs_with_invalid_AXISMINVAL_too_many_children` | `wkt_parse_cs_with_invalid_axisminval_too_many_children` |
| `wkt_parse.cs_with_invalid_AXISMAXVAL_string` | `wkt_parse_cs_with_invalid_axismaxval_string` |
| `wkt_parse.cs_with_invalid_AXISMAXVAL_too_many_children` | `wkt_parse_cs_with_invalid_axismaxval_too_many_children` |
| `wkt_parse.cs_with_invalid_RANGEMEANING` | `wkt_parse_cs_with_invalid_rangemeaning` |
| `wkt_parse.cs_with_invalid_RANGEMEANING_too_many_children` | `wkt_parse_cs_with_invalid_rangemeaning_too_many_children` |
| `wkt_parse.COMPD_CS` | `wkt_parse_compd_cs` |
| `wkt_parse.VERTCS_with_ellipsoidal_height_ESRI` | `wkt_parse_vertcs_with_ellipsoidal_height_esri` |
| `datum.cs_with_MERIDIAN` | `datum_cs_with_meridian` |
| `crs.boundCRS_with_usage` | `crs_bound_crs_with_usage` |
| `CApi.proj_get_remarks` | `c_api_proj_get_remarks` |
| `CApi.proj_get_scope` | `c_api_proj_get_scope` |
| `CApi.proj_concatoperation_get_step` | `c_api_proj_concatoperation_get_step` |
| `CApi.proj_create_ellipsoidal_3D_cs` | `c_api_proj_create_ellipsoidal_3d_cs` |
| `CApi.proj_is_equivalent_to_with_ctx` | `c_api_proj_is_equivalent_to_with_ctx` |
| `factory.AuthorityFactory_getAuthorityCodes` | `factory_authority_factory_get_authority_codes` |
| `factory.attachExtraDatabases_none` | `factory_attach_extra_databases_none` |

## Source changes

| File | Change | Needed by |
|---|---|---|
| `src/iso19111/wkt.rs` | A WKT2 `PARAMETER` read without a unit is written without `UNIT`. An ESRI vertical CRS with an ellipsoidal-height datum (WKT1 type 2002) writes the geodetic datum of the same name. `CartesianCS::export_to_wkt`. | `cs_with_AXISMINVAL_AXISMAXVAL_RANGEMEANING`, `VERTCS_with_ellipsoidal_height_ESRI`, `datum.cs_with_MERIDIAN` |
| `src/iso19111/wkt_import.rs` | A vertical CRS on a type 2002 datum with an upward axis gets the `ellipsoidal height` axis. `BOUNDCRS` keeps its `USAGE` and remarks. An `ABRIDGEDTRANSFORMATION` rotation without a unit is in arc-seconds. | `VERTCS_with_ellipsoidal_height_ESRI`, `crs.boundCRS_with_usage` |
| `src/iso19111/model.rs` | `Meridian::create`, `Axis::with_meridian`, `EllipsoidalCS::create_longitude_latitude_height`. | `datum.cs_with_MERIDIAN`, `proj_create_ellipsoidal_3D_cs` |
| `src/iso19111/equiv.rs` | The `Equivalent` criterion compares datum and ellipsoid names with `equivalent_name`. `GeographicCRS::is_equivalent_to_with_database` also accepts a datum name that is an alias of the other official name in `proj.db`. | `proj_is_equivalent_to_with_ctx` |
| `src/iso19111/insert.rs` | An existing ellipsoid is matched by the `EQUIVALENT` name rule, and an existing prime meridian is found by name in the table. | `proj_get_insert_statements` (stateless part) |
| `src/lib.rs`, `src/iso19111/mod.rs` | `Measure` is re-exported. | `datum.cs_with_MERIDIAN` |

## Deliberate differences in the ported tests

- C++ `exportToWKT(WKTFormatter::create(...))` is `Crs::export_to_wkt(&mut WktFormatter::create(...))`. The database context is `WktFormatter::with_database`.
- C++ `setOutputId(false)` is `WktFormatter::hide_identifiers`.
- `proj_get_remarks` and `proj_get_scope` read `CoordinateOperation::remarks` and `CoordinateOperation::domains()[0].scope()`. The `+proj=noop` case of `proj_get_scope` (no scope) is not repeated: a PROJ string has no domain here.
- `proj_concatoperation_get_step` uses `ConcatenatedOperation::steps`; the out-of-range index is `steps.get(3)` returning `None`. `PROJ_GRID_AVAILABILITY_IGNORED` has no counterpart, because the factory does not check grid files.
- `proj_is_equivalent_to_with_ctx` is `GeographicCRS::is_equivalent_to_with_database`, since `Crs` has only the strict check.

## Intentionally unported (56)

Every name here stays in `docs/UNPORTED-UNIT.md`.

| PROJ `TEST()` | Reason |
|---|---|
| `CApi.proj_context_set_database_path_null` | Sets the database path or an auxiliary database on a `PJ_CONTEXT`; `DatabaseContext::create()` takes no path. |
| `CApi.proj_context_set_database_path_aux` | Sets the database path or an auxiliary database on a `PJ_CONTEXT`; `DatabaseContext::create()` takes no path. |
| `CApi.proj_context_set_database_path_error_1` | Sets the database path or an auxiliary database on a `PJ_CONTEXT`; `DatabaseContext::create()` takes no path. |
| `CApi.proj_context_set_database_path_error_2` | Sets the database path or an auxiliary database on a `PJ_CONTEXT`; `DatabaseContext::create()` takes no path. |
| `CApi.proj_clone_of_obj_with_alternative_operations` | C clone of a `PJ` that keeps its alternative-operation list; Rust values are `Clone`, with no operation-list object. |
| `CApi.proj_crs_alter_geodetic_crs` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_crs_alter_cs_angular_unit` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_crs_alter_cs_linear_unit` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_crs_alter_parameters_linear_unit` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_alter_name` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_alter_id` | No `alter*` API: CRS values are rebuilt through constructors, and `with_identifier` only appends an identifier. |
| `CApi.proj_create_cartesian_2D_cs` | C enum-selector constructor (`PJ_CART2D_*`) whose test only checks for a non-null handle; Rust has `CartesianCS::create_easting_northing` and `CartesianCS::create` for other axis lists. |
| `CApi.proj_context_copy_from_default` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.proj_context_clone` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.proj_context_set_sqlite3_vfs_name` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.proj_context_set_sqlite3_vfs_name__from_global_context` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.use_proj4_init_rules` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.use_proj4_init_rules_from_global_context` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.open_plenty_of_contexts` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.concurrent_context` | C context state (`PJ_CONTEXT`); the Rust API has no mutable context object. |
| `CApi.proj_get_insert_statements` | Stateless part ported as `c_api_proj_get_insert_statements`. The rest needs an `proj_insert_object_session` that remembers inserted objects; `insert_statements` starts from the database on every call. |
| `crs.scope_area_bbox_remark` | `ObjectDomain` has scope, area and bounding box, but no vertical or temporal extent. |
| `crs.multiple_ID` | Needs a public `GeodeticCRS` constructor with several identifiers; reading several `ID` nodes is ported as `wkt_parse_cs_with_multiple_id`. |
| `crs.VerticalCRS_ensemble_exception_in_create` | No public `VerticalCRS::create`, so there is no creation error to observe. |
| `datum.vdatum_with_anchor` | No public `VerticalReferenceFrame` constructor with an anchor or realization method. |
| `crs.projected_is_equivalent_to_with_proj4_extension` | `+proj=omerc ... +no_rot` is refused, and projected CRS has no `EQUIVALENT` criterion. |
| `FactoryWithTmpDatabase.AuthorityFactory_test_with_fake_EPSG_database` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_test_inversion_first_and_last_steps_of_concat_op` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_test_with_fake_EPSG_and_OTHER_database` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_test_sorting_of_coordinate_operations` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_source_equals_target` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_target_pivot` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_pivot_target` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_pivot_target` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_target_pivot` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_proj_based_transformation` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation_not_wkt` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation_not_co_wkt` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.getAuthorities` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.lookForGridInfo` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.lookForGridInfo_from_old_name_with_new_grid_available` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.custom_geodetic_crs` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.custom_projected_crs` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.custom_derived_projected_crs` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.CoordinateMetadata` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.createOperations_exact_transform_not_whole_area` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `FactoryWithTmpDatabase.check_fixup_direction_concatenated_inverse_map_projection` | Builds a temporary SQLite `proj.db` (`createStructure`, `populateWithFakeEPSG`); `DatabaseContext::create()` opens only the embedded database, with no path, schema or auxiliary-database API. |
| `factory.attachExtraDatabases_auxiliary` | Attaches an in-memory auxiliary SQLite database; no auxiliary-database API. |
| `factory.attachExtraDatabases_auxiliary_error` | Error paths of attaching an auxiliary database; no such API. |
| `factory.objectInsertion` | Inserts SQL into a temporary database and reads it back; `insert_statements` only returns text, and no writable database exists. |
| `json_import.several_usages` | `create_from_user_input` returns a `Crs`; a `GeodeticReferenceFrame` PROJJSON object is not imported. |
| `json_import.bound_crs_with_name_and_usage` | PROJJSON `BoundCRS` is not imported (`PROJJSON type BoundCRS is not imported in this build`). |
| `json_import.engineering_crs_affine_CS` | PROJJSON `EngineeringCRS` is not imported (`PROJJSON type EngineeringCRS is not imported in this build`). |
| `json_import.multiple_ids` | PROJJSON `Ellipsoid` object is not imported; `create_from_user_input` returns a `Crs`. |
| `json_export.coordinate_system_id` | PROJJSON export exists for `Crs` only, not for a standalone coordinate system. |

## Commands

```
cargo test --test unit_shape
cargo test --all-features
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo test --test unit_shape` runs 21 tests (the 20 ported names, plus the stateless part of `proj_get_insert_statements`), all passing. `docs/UNPORTED-UNIT.md` lost 20 lines. The generated counts in `README.md`, `ROADMAP.md` and `docs/UNIT-TRIAGE.md` are not regenerated here.
