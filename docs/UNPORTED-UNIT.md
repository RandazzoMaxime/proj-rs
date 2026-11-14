# Unported unit tests

These `TEST()` names from PROJ 9.9.0 `test/unit` are not executed.
They build ISO 19111 objects, WKT, database factories, or a projection
this crate does not run. The names that do run are in `tests/proj_unit.rs`
and `PORT-NOTES-tests.md`.

## test_crs.cpp


## test_factory.cpp


## test_operation.cpp


## test_operationfactory.cpp

## gie_self_tests.cpp

- `gie.proj_factors`
- `gie.proj_create_crs_to_crs_WGS84_EGM08_to_WGS84`
- `gieTest.proj_create_crs_to_crs`

## test_common.cpp

- `common.identifiedobject_alias_invalid_type`
- `common.identifiedobject_alias_array_of_invalid_type`

## test_network.cpp

- `networking.initial_check`
- `networking.basic`
- `networking.custom`
- `networking.getfilesize`
- `networking.simul_read_range_error`
- `networking.simul_file_change_while_opened`
- `networking.download_whole_files`
- `networking.file_api`
- `networking.proj_coordoperation_get_grid_used`
- `networking.pyproj_issue_1192`
- `networking.do_not_attempt_network_access_known_available_network_on`

## test_projinfo_lib.cpp

- `projinfo_lib.warning`
- `projinfo_lib.use_ctx`
- `projinfo_lib.works_without_cb`
