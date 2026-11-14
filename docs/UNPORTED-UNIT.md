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

## test_common.cpp

- `common.identifiedobject_alias_invalid_type`
- `common.identifiedobject_alias_array_of_invalid_type`

## test_network.cpp

- `networking.initial_check`
- `networking.basic`
- `networking.download_whole_files`
- `networking.file_api`
- `networking.pyproj_issue_1192`

## test_projinfo_lib.cpp

- `projinfo_lib.works_without_cb`
