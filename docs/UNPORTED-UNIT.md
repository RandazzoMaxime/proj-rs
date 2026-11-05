# Unported unit tests

These `TEST()` names from PROJ 9.9.0 `test/unit` are not executed.
They build ISO 19111 objects, WKT, database factories, or a projection
this crate does not run. The names that do run are in `tests/proj_unit.rs`
and `PORT-NOTES-tests.md`.

## test_crs.cpp

- `crs.projectedCRS_identify_db`
- `crs.scope_area_bbox_remark`
- `crs.VerticalCRS_ensemble_exception_in_create`
- `crs.compoundCRS_identify_db`
- `crs.boundCRS_crs_link`
- `crs.boundCRS_identify_db`
- `crs.crs_createBoundCRSToWGS84IfPossible`
- `crs.normalizeForVisualization_derivedprojected_operation`
- `crs.norway_ntm`

## test_factory.cpp

- `factory.AuthorityFactory_createDynamicVerticalDatum`
- `FactoryWithTmpDatabase.AuthorityFactory_test_with_fake_EPSG_database`
- `FactoryWithTmpDatabase.AuthorityFactory_test_inversion_first_and_last_steps_of_concat_op`
- `FactoryWithTmpDatabase.AuthorityFactory_test_with_fake_EPSG_and_OTHER_database`
- `FactoryWithTmpDatabase.AuthorityFactory_test_sorting_of_coordinate_operations`
- `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_source_equals_target`
- `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_target_pivot`
- `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_source_pivot_pivot_target`
- `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_pivot_target`
- `FactoryWithTmpDatabase.AuthorityFactory_createFromCRSCodesWithIntermediates_case_pivot_source_target_pivot`
- `FactoryWithTmpDatabase.AuthorityFactory_proj_based_transformation`
- `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation`
- `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation_not_wkt`
- `FactoryWithTmpDatabase.AuthorityFactory_wkt_based_transformation_not_co_wkt`
- `FactoryWithTmpDatabase.getAuthorities`
- `FactoryWithTmpDatabase.lookForGridInfo`
- `FactoryWithTmpDatabase.lookForGridInfo_from_old_name_with_new_grid_available`
- `FactoryWithTmpDatabase.custom_geodetic_crs`
- `FactoryWithTmpDatabase.custom_projected_crs`
- `FactoryWithTmpDatabase.custom_derived_projected_crs`
- `FactoryWithTmpDatabase.CoordinateMetadata`
- `factory.attachExtraDatabases_auxiliary`
- `factory.attachExtraDatabases_auxiliary_error`
- `FactoryWithTmpDatabase.createOperations_exact_transform_not_whole_area`
- `FactoryWithTmpDatabase.check_fixup_direction_concatenated_inverse_map_projection`
- `factory.objectInsertion`

## test_io.cpp

- `io.projparse_init`
- `io.createFromUserInput`

## test_operation.cpp

- `operation.normalizeForVisualization`

## test_operationfactory.cpp

- `operation.geogCRS_to_geogCRS_context_inverse_needed`
- `operation.vertCRS_to_geogCRS_context`
- `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_context`
- `operation.geog3DCRS_to_vertCRS_depth_context`
- `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_depth_context`
- `operation.geogCRS_to_geogCRS_context_lonlat_vs_latlon_crs`
- `operation.geogCRS_to_geogCRS_CH1903_to_CH1903plus_context`
- `operation.geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context`
- `operation.geogCRS_promoted_to_3D_to_geogCRS_3D_context`
- `operation.esri_projectedCRS_to_geogCRS_with_ITRF_intermediate_context`
- `operation.geogCRS_to_geogCRS_with_intermediate_no_ids`
- `operation.geogCRS_3D_source_datum_name_is_alias_to_geogCRS`
- `operation.geogCRS_different_from_baseCRS_to_projCRS_context_compatible_area`
- `operation.geogCRS_3D_to_projCRS_with_2D_geocentric_translation`
- `operation.projCRS_to_projCRS_context_grid_offsets`
- `operation.projCRS_to_projCRS_context_grid_offsets_non_metre_unit_noop`
- `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`
- `operation.boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs`
- `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids`
- `operation.nadgrids_with_pm`
- `operation.towgs84_pm_3d`
- `operation.WGS84_G1762_to_compoundCRS_with_bound_vertCRS`
- `operation.compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt`
- `operation.compoundCRS_with_vert_bound_to_bound_geog3D`
- `operation.compoundCRS_with_derivedVerticalCRS_ellipsoidal_height_to_geog`
- `operation.compoundCRS_to_compoundCRS`
- `operation.compoundCRS_to_compoundCRS_with_vertical_transform`
- `operation.compoundCRS_to_compoundCRS_issue_2232`
- `operation.compoundCRS_to_compoundCRS_context`
- `operation.compoundCRS_to_compoundCRS_WGS84_EGM96_to_WGS84_Belfast`
- `operation.compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96`
- `operation.compoundCRS_with_horizontal_boundCRS_to_compoundCRS_and_same_vertical`
- `operation.compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical`
- `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation`
- `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp`
- `operation.compoundCRS_to_compoundCRS_issue_3328`
- `operation.compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height`
- `operation.compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_ballpark_geog`
- `operation.compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound`
- `operation.compoundCRS_to_compoundCRS_issue_3191_BD72_Ostend_height_to_WGS84_EGM96`
- `operation.compoundCRS_to_compoundCRS_issue_4550`
- `operation.compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height`
- `operation.compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform`
- `operation.compoundCRS_to_geogCRS_3D_context`
- `operation.compoundCRS_to_geogCRS_3D_with_3D_helmert_same_geog_src_target_context`
- `operation.compoundCRS_to_geogCRS_3D_with_null_helmert_same_geog_src_target_context`
- `operation.compoundCRS_to_geogCRS_2D_promote_to_3D_context`
- `operation.compoundCRS_of_projCRS_to_geogCRS_3D_context`
- `operation.compoundCRS_of_projCRS_from_wkt_without_id_or_extent_to_geogCRS`
- `operation.compoundCRS_ftUS_to_geogCRS_ft`
- `operation.compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change`
- `operation.compoundCRS_to_geogCRS_with_height_depth_reversal`
- `operation.compoundCRS_to_geogCRS_with_vertical_unit_change_and_height_depth_reversal`
- `operation.compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS`
- `operation.compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS`
- `operation.compoundCRS_of_vertCRS_with_geoid_model_by_name_and_datum_ensemble`
- `operation.compoundCRS_from_WKT2_no_id_to_geogCRS_3D_context`
- `operation.compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort`
- `operation.compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform`
- `operation.boundCRS_to_compoundCRS`
- `operation.boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS`
- `operation.IGNF_LAMB1_TO_EPSG_4326`
- `operation.compoundCRS_to_proj_string_with_non_metre_height`
- `operation.compoundCRS_with_derived_vertical_CRS`
- `operation.compoundCRS_to_PROJJSON_with_non_metre_height`
- `operation.createOperation_ossfuzz_18587`
- `operation.createOperation_bound_of_spherical_ocentric_to_same_type`
- `operation.createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west`
- `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west`
- `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric`
- `operation.createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west`
- `operation.createOperation_derived_projected_crs`
- `operation.geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected`
- `operation.createOperation_point_motion_operation_geocentric_to_geog3D`
- `operation.createOperation_point_motion_operation_NAD83_CSRS_v7_TO_ITRF2014`
- `operation.createOperation_point_motion_operation_ITRF2014_to_NAD83_CSRS_v7`
- `operation.createOperation_compound_to_compound_with_point_motion_operation`
- `operation.createOperation_compound_to_compound_with_Geographic3D_Offset_by_velocity_grid`
- `operation.createOperation_compound_to_compound_with_point_motion_operation_special_case_CGVD2013a`

## test_c_api.cpp

- `CApi.proj_coordoperation_get_grid_used_fullname_caching`
- `CApi.proj_create_operations_prime_meridian_non_greenwich`
- `CApi.proj_get_suggested_operation_with_operations_without_area_of_use`
- `CApi.proj_context_set_database_path_null`
- `CApi.proj_context_set_database_path_aux`
- `CApi.proj_context_set_database_path_error_1`
- `CApi.proj_context_set_database_path_error_2`
- `CApi.proj_clone_of_obj_with_alternative_operations`
- `CApi.proj_normalize_for_visualization_with_alternatives`
- `CApi.proj_normalize_for_visualization_with_alternatives_reverse`
- `CApi.proj_coordoperation_create_inverse`
- `CApi.proj_context_copy_from_default`
- `CApi.proj_context_clone`
- `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter`
- `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter`
- `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations`
- `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762`
- `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762`
- `CApi.proj_create_vertical_crs_ex`
- `CApi.proj_create_vertical_crs_ex_with_geog_crs`
- `CApi.proj_create_vertical_crs_ex_implied_accuracy`
- `CApi.proj_context_set_sqlite3_vfs_name`
- `CApi.proj_context_set_sqlite3_vfs_name__from_global_context`
- `CApi.use_proj4_init_rules`
- `CApi.use_proj4_init_rules_from_global_context`
- `CApi.proj_get_insert_statements`
- `CApi.open_plenty_of_contexts`
- `CApi.concurrent_context`

