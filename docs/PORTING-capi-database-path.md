# Porting sheet: database path of the C context

PROJ 9.9.0 `test/unit/test_factory.cpp` `TEST(factory, attachExtraDatabases_auxiliary)` and
`attachExtraDatabases_auxiliary_error`, and `test/unit/test_c_api.cpp`
`CApi.proj_context_set_database_path_null`, `_aux`, `_error_1` and `_error_2`.

This lot closes 6 cases of `docs/UNPORTED-UNIT.md`. It is built on the foundation of
`docs/PORTING-database-open.md` (`DatabaseContext::open`, `create_with_auxiliary`). Rust
tests: `tests/unit_database_open.rs` (factory cases) and `tests/unit_capi_database_path.rs`
(C API cases).

## Map

| PROJ | Rust | Test |
|---|---|---|
| `attachExtraDatabases_auxiliary`, block 1: `create(std::string(), {aux})`, EPSG and OTHER found, structure unchanged | `create_with_auxiliary(None, &[aux])` | `auxiliary_objects_are_found_next_to_the_main_ones` |
| `attachExtraDatabases_auxiliary`, block 2: `{aux, ":memory:"}`, EPSG and OTHER found | `create_with_auxiliary(None, &[aux, ":memory:"])` | `auxiliary_file_and_memory_auxiliary_are_both_attached` |
| `attachExtraDatabases_auxiliary`, block 3: `{":memory:"}`, EPSG found, OTHER throws `FactoryException` | `create_with_auxiliary(None, &[":memory:"])`, `NotFound` | `memory_auxiliary_database_is_empty` |
| `attachExtraDatabases_auxiliary_error`: `{"i_dont_exist_db"}` throws | `create_with_auxiliary(None, &["i_dont_exist_db"])` is `Err`, the file is not created | `missing_auxiliary_database_is_an_error_and_is_not_created` |
| `proj_context_set_database_path(ctx, NULL, NULL, NULL)` returns true, EPSG:4326 found | `create_with_auxiliary(None, &[])` is `Ok`, factory lookup | `proj_context_set_database_path_null` |
| `proj_context_set_database_path(ctx, NULL, {aux}, NULL)` returns true, EPSG:4326 and OTHER_4326 found | `create_with_auxiliary(None, &[aux])` | `proj_context_set_database_path_aux` |
| `proj_context_set_database_path(ctx, "i_do_not_exist.db", NULL, NULL)` returns false, default still answers | `create_with_auxiliary(Some(missing), &[])` is `Err`, then `DatabaseContext::create()` | `proj_context_set_database_path_error_1` |
| `proj_context_set_database_path(ctx, NULL, {"i_do_not_exist.db"}, NULL)` returns false, default still answers | `create_with_auxiliary(None, &[missing])` is `Err`, then `DatabaseContext::create()` | `proj_context_set_database_path_error_2` |

The C `PJ_CONTEXT` is the `DatabaseContext` in this port. `proj_create_from_database` on it is
`AuthorityFactory::create(&db, authority).create_geographic_crs(code)`. The C++ block 3
`FactoryException` is `FactoryError::NotFound`.

## Expected values

- `WGS 84` for EPSG:4326: the name of the reference `projinfo` 9.9.0 output and of the
  shipped `proj.db`.
- `OTHER_4326` row: the literal inserted by `attachExtraDatabases_auxiliary` and by
  `proj_context_set_database_path_aux` (`'OTHER','OTHER_4326','WGS 84',NULL,'geographic 2D',
  'EPSG','6422','EPSG','6326',NULL,0`).
- The auxiliary database of `proj_context_set_database_path_aux` is built with every
  statement of the embedded structure, triggers included, as the C++ test does. The factory
  test keeps its trigger-free `create_empty_structure` helper, as the C++ factory test does.

## Deviations

- **No mutable context.** The C tests check that a `PJ_CONTEXT` keeps the path after a
  successful call, and that a failed call leaves the context on the previous database. A Rust
  `DatabaseContext` is a value: the constructor is the "set", and the caller passes the value
  to the factory. The "default still answers" step is `DatabaseContext::create()`. There is no
  shared state to check, so nothing is lost on the observable side.
- **Files, not shared-cache memory.** The C aux is `file:...?mode=memory&cache=shared`. Here it
  is a scratch file, removed on drop. `:memory:` is still an empty auxiliary, as in PROJ.
- **`DatabaseContext::create()` takes no path and no `PJ_CONTEXT`.** It stays the embedded
  database. The C `NULL` path is `path: None`.
- **No façade wiring.** `Proj` and `ProjBuilder` build transforms through the embedded
  `proj.db` (`iso_pipeline`). Passing a database to them would mean threading a
  `DatabaseContext` through the pipeline builder, which is not a small change, and no
  C-API test of this lot needs it. The lot therefore stays on the iso19111 layer.
- **Error variant.** The C++ `FactoryException` is checked as the Rust variant `NotFound` for
  the missing auxiliary object, and as `Sql("Open of ... failed")` for a missing file.

## Still open (not closed here)

| PROJ `TEST()` | Reason |
|---|---|
| `factory.objectInsertion` | About 840 lines (`test_factory.cpp:4411-5252`). It needs `startInsertStatementsSession` and `stopInsertStatementsSession` with their nesting exception, `getInsertStatementsFor` that refuses to run without a session, and `suggestsCodeFor`, which counts the codes already inserted in the session. The Rust `insert_statements` is stateless; the session is not ported. |
| `CApi.proj_context_copy_from_default` | Reads the global default database path (`proj_context_get_database_path(NULL)`), sets it globally, then checks that a new context inherits it. There is no global context here. |
| `CApi.proj_context_clone` | Checks that a clone keeps `use_proj4_init_rules`. That state and `proj_context_use_proj4_init_rules` do not exist in Rust. |
| `CApi.proj_context_set_sqlite3_vfs_name` | A dummy SQLite VFS name must make `proj_create` fail, and the restored default must work. The Rust `DatabaseContext` has no VFS option. |
| `CApi.proj_context_set_sqlite3_vfs_name__from_global_context` | Same reason, with the global context. |

`docs/PORTING-database-open.md` (the foundation) says the two `factory.attachExtraDatabases_*`
cases stay open. This lot closes them, with the block 2 test and the `NotFound` check added.

## Ledger

Removed from `docs/UNPORTED-UNIT.md`:

- `factory.attachExtraDatabases_auxiliary`
- `factory.attachExtraDatabases_auxiliary_error`
- `CApi.proj_context_set_database_path_null`
- `CApi.proj_context_set_database_path_aux`
- `CApi.proj_context_set_database_path_error_1`
- `CApi.proj_context_set_database_path_error_2`

`UPDATE_DOCS=1 cargo test --test docs_in_sync` was not run, as the lot instructions say. The
counts in `README.md`, `ROADMAP.md` and `docs/UNIT-TRIAGE.md` therefore no longer match the
ledger until the maintainer regenerates them.

## Commands

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo test --test unit_database_open --test unit_capi_database_path
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
  PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --all-features --no-fail-fast
```

Reference value: `PROJ_DATA=target/proj-reference/build/data target/proj-reference/build/bin/projinfo EPSG:4326 -o WKT2_2019 -q`.
