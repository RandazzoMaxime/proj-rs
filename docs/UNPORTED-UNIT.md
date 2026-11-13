# Unported unit tests

These `TEST()` names from PROJ 9.9.0 `test/unit` are not executed.
They build ISO 19111 objects, WKT, database factories, or a projection
this crate does not run. The names that do run are in `tests/proj_unit.rs`
and `PORT-NOTES-tests.md`.

## test_crs.cpp


## test_factory.cpp

- `factory.objectInsertion`

## test_io.cpp

- `io.createFromUserInput`

## test_operation.cpp


## test_operationfactory.cpp

- `CApi.proj_context_copy_from_default`
- `CApi.proj_context_clone`
- `CApi.proj_context_set_sqlite3_vfs_name`
- `CApi.proj_context_set_sqlite3_vfs_name__from_global_context`
- `CApi.use_proj4_init_rules_from_global_context`
- `CApi.proj_get_insert_statements`
- `CApi.open_plenty_of_contexts`

