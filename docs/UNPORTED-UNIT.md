# Unported unit tests

These `TEST()` names from PROJ 9.9.0 `test/unit` are not executed.
They build ISO 19111 objects, WKT, database factories, or a projection
this crate does not run. The names that do run are in `tests/proj_unit.rs`
and `PORT-NOTES-tests.md`.

## test_crs.cpp

- `crs.scope_area_bbox_remark`

## test_factory.cpp

- `factory.AuthorityFactory_createDynamicVerticalDatum`
- `FactoryWithTmpDatabase.custom_projected_crs`
- `FactoryWithTmpDatabase.custom_derived_projected_crs`
- `factory.objectInsertion`

## test_io.cpp

- `io.createFromUserInput`

## test_operation.cpp


## test_operationfactory.cpp

- `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_context`
- `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`
- `operation.nadgrids_with_pm`
- `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation`
- `operation.createOperation_bound_of_spherical_ocentric_to_same_type`

## test_c_api.cpp

- `CApi.proj_clone_of_obj_with_alternative_operations`
- `CApi.proj_normalize_for_visualization_with_alternatives`
- `CApi.proj_coordoperation_create_inverse`
- `CApi.proj_context_copy_from_default`
- `CApi.proj_context_clone`
- `CApi.proj_context_set_sqlite3_vfs_name`
- `CApi.proj_context_set_sqlite3_vfs_name__from_global_context`
- `CApi.use_proj4_init_rules_from_global_context`
- `CApi.proj_get_insert_statements`
- `CApi.open_plenty_of_contexts`

