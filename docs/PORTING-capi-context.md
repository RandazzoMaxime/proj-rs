# Porting sheet: the `PJ_CONTEXT` tests of test_c_api.cpp

PROJ 9.9.0 sources: `test/unit/test_c_api.cpp`, `src/ctx.cpp` (the `PROJ_USE_PROJ4_INIT_RULES`
variable), `src/iso19111/crs.cpp` (`CRS::_exportToPROJString`, `hasOver`) and
`src/iso19111/io.cpp` (`createFromUserInput`, the `init=` branch).

The C API keeps state on a `PJ_CONTEXT`: the database path, the SQLite VFS, the PROJ 4 `init=`
rules, and the global context that new contexts copy. The Rust API has no context object. The
database is the embedded `proj.db`, read by `DatabaseContext::create()`, and the rules are an
argument (`InitRules`). This lot checks the 13 `CApi` names that were candidates, and closes the
2 whose observable result exists in Rust.

## Closed: 2 `TEST()`s

| PROJ `TEST()` | Rust test in `tests/test_c_api.rs` |
|---|---|
| `CApi.use_proj4_init_rules` | `c_api_use_proj4_init_rules` |
| `CApi.concurrent_context` | `c_api_concurrent_context` |

### `use_proj4_init_rules`

- `proj_context_use_proj4_init_rules(ctx, true)` and `proj_create(ctx, "+init=...")` become
  `create_from_user_input_with_init_rules(text, &db, InitRules::Proj4)`.
- The default rules (`create_from_user_input`) refuse `init=epsg:` and `init=IGNF:`, as in PROJ.
  This is the default state that the C++ test reads back with `get_use_proj4_init_rules`.
- Checked: `+init=epsg:28992 +over` gives the datum `Amersfoort` and the PROJ 5 string of the
  C++ test. `+init=epsg:3857 +over` gives the PROJ 5 string of the C++ test, with `+over`.
- The getter and setter assertions are not ported. They read back a flag that the Rust API
  takes as an argument, so there is nothing to read back.

### `concurrent_context`

- Four threads, 60 iterations each. Every iteration opens a `DatabaseContext` (the stand-in for
  `proj_context_create`) and builds `EPSG:4326` and `EPSG:326xx`. A failure panics in the
  thread and fails the join.
- `proj_cleanup()` has no counterpart. Nothing global is kept, so dropping the handles frees
  everything.

## Fixed in the implementation

`ProjectedCRS::export_with` (`src/iso19111/projstring.rs`) returned the pseudo-Mercator string
without the `over` flag. PROJ 9.9.0 writes `+over` after `+nadgrids=@null` and before `+wktext`,
for `+init=epsg:3857 +over` (`projinfo` with `PROJ_USE_PROJ4_INIT_RULES=YES` gives the same
string). The flag is now added in that export only. `wkt1_extension_proj4` is unchanged: the WKT1
`EXTENSION` node never carried `over`.

## Left open, with the cause

| `TEST()` | Cause |
|---|---|
| `proj_context_set_database_path_null`, `_aux`, `_error_1`, `_error_2` | A database path or an auxiliary database on a context. `DatabaseContext::create()` takes no path and reads only the embedded `proj.db`. |
| `proj_context_copy_from_default` | Copies the database file and sets its path on a new context. Needs the path setter above. |
| `proj_context_clone` | Copies the context state. The Rust port has no state to copy; the flag is an argument. The test would only compare a flag with itself. |
| `proj_context_set_sqlite3_vfs_name`, `_from_global_context` | A dummy SQLite VFS must make the database open fail. The embedded database is opened from bytes, so there is no VFS to set. |
| `use_proj4_init_rules_from_global_context` | Inheritance of a global default by new contexts. The Rust port has no global context. |
| `open_plenty_of_contexts` | Checks that one context consumes one file descriptor while descriptors are exhausted. The embedded database opens no file. Exhausting descriptors in the shared test binary would disturb the parallel tests, and the check would be vacuous. |
| `proj_get_insert_statements` | The stateless part is `c_api_proj_get_insert_statements` (`tests/unit_shape.rs`). The rest needs an insert session that remembers the objects it inserted (second call returns an empty list). `insert_statements` has no session. |

The three `CApi` names in the same section that are not candidates (`proj_coordoperation_*`,
`proj_clone_of_obj_with_alternative_operations`, `proj_normalize_for_visualization_with_alternatives`)
were not touched.

## Commands

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH
export PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data

cargo test --test test_c_api -- c_api_use_proj4_init_rules c_api_concurrent_context

# Reference values for the +over export (projinfo has no rules option; the env var sets them)
PROJ_USE_PROJ4_INIT_RULES=YES projinfo -q -o PROJ "+init=epsg:3857 +over"
PROJ_USE_PROJ4_INIT_RULES=YES projinfo -q -o PROJ "+init=epsg:28992 +over"
```
