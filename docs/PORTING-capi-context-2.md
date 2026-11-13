# Porting sheet: the context and the insert session (test_c_api)

Source, PROJ 9.9.0: `test/unit/test_c_api.cpp` (`CApi.proj_context_copy_from_default`, `CApi.proj_context_clone`, `CApi.proj_context_set_sqlite3_vfs_name`, `CApi.proj_context_set_sqlite3_vfs_name__from_global_context`, `CApi.use_proj4_init_rules_from_global_context`, `CApi.open_plenty_of_contexts`, `CApi.proj_get_insert_statements`), with `src/ctx.cpp` (`proj_context_clone`, `proj_context_create`), `src/iso19111/c_api.cpp` (`proj_insert_object_session_create`, `proj_get_insert_statements`, `proj_suggests_code_for`) and `src/iso19111/factory.cpp` (`startInsertStatementsSession`, `getInsertStatementsFor`, `stopInsertStatementsSession`, `suggestsCodeFor`, `findFreeCode`). Expected values are copied from the C++ tests. New tests are in `tests/unit_capi_context_2.rs`.

The previous sheet (`PORTING-shape.md`) gave "the Rust API has no mutable context object" as the reason for these seven tests. This lot adds the object: `Context` in `src/context.rs`.

## Test mapping (7 ported, all passing)

| PROJ `TEST_F` | Rust test in `tests/unit_capi_context_2.rs` |
|---|---|
| `CApi.proj_context_copy_from_default` | `c_api_proj_context_copy_from_default` |
| `CApi.proj_context_clone` | `c_api_proj_context_clone` |
| `CApi.proj_context_set_sqlite3_vfs_name` | `c_api_proj_context_set_sqlite3_vfs_name` |
| `CApi.proj_context_set_sqlite3_vfs_name__from_global_context` | `c_api_proj_context_set_sqlite3_vfs_name_from_global_context` |
| `CApi.use_proj4_init_rules_from_global_context` | `c_api_use_proj4_init_rules_from_global_context` |
| `CApi.open_plenty_of_contexts` | `c_api_open_plenty_of_contexts` |
| `CApi.proj_get_insert_statements` | `c_api_proj_get_insert_statements` |

`c_api_proj_get_insert_statements` also pins the stateless part of the C test (`tests/unit_shape.rs`, `c_api_proj_get_insert_statements`), which already passed.

## Source changes

| File | Change | Needed by |
|---|---|---|
| `src/context.rs` (new) | `Context`, `ContextBuilder`, `DatabaseSource`. A context holds the database source, the auxiliary paths, the SQLite VFS name and the `InitRules`. `Context::new` copies the process default (`proj_context_create`), `Clone` copies a context (`proj_context_clone`), and `Context::set_default_*` change the default (`proj_context_*(NULL, ...)`). `Context::database` calls `create_with_auxiliary`, or `open_with_vfs` when a VFS is named; `Context::create_from_user_input` reads user input with the context's `init=` rules. | all but `proj_get_insert_statements` |
| `src/projdb.rs` | `open_file` opens a `proj.db` file read-only, through a named VFS or SQLite's default. `open_embedded_with_vfs` opens the embedded database through a named VFS. `open_connection` (used by `create()`) is a writable in-memory copy of the embedded image (`rusqlite` `backup` feature). | `CApi.proj_context_set_sqlite3_vfs_name`, `CApi.proj_get_insert_statements` |
| `src/iso19111/factory.rs` | `DatabaseContext::open_with_vfs(path, vfs)` (crate-internal). The foundation's `open(path)`, `create()` and `create_with_auxiliary` are unchanged. The context carries an insert-session flag, shared by its clones. `execute_batch` runs SQL on the connection. | all |
| `src/iso19111/insert.rs` | `InsertSession::start` opens a savepoint, and fails while another session is open on the same database (`proj_insert_object_session_create` returns NULL then). `InsertSession::statements_for` runs each statement inside the session as it is produced, so later calls see it. `Drop` rolls the savepoint back (`proj_insert_object_session_destroy`). `insert_statements` opens a temporary session. `suggests_code_for` is `suggestsCodeFor`. `find_free_code` is shared with the insert path. | `CApi.proj_get_insert_statements` |
| `src/lib.rs`, `src/iso19111/mod.rs` | Re-exports: `Context`, `ContextBuilder`, `DatabaseSource`, `InitRules`, `InsertSession`, `CodeForm`, `suggests_code_for`, `create_from_user_input_with_init_rules`. | all |
| `Cargo.toml` | `rusqlite` feature `backup`. No new crate. | `src/projdb.rs` |

## Deliberate differences

- **No `PJ_CONTEXT` handle.** `Context` is a plain value. `proj_context_create` is `Context::new()`, and `proj_context_clone` is `Clone`. The process default is one `RwLock<Settings>`, and it is read when a context is created. A context created before a change keeps its settings, as PROJ's copy does.
- **`copy_from_default`.** The C test writes a copy of `proj.db` and points the default at it, through `proj_context_set_database_path`. The Rust test does the same with `Context::set_default_database(DatabaseSource::File(..))`. The C test's `#if !EMBED_RESOURCE_FILES` guard does not apply: the crate always embeds `proj.db`, and a file source is still read.
- **Unknown VFS.** SQLite refuses an unknown VFS name before it reads any file, so the embedded database also fails with the name. The error is `FactoryError::Sql("Open of proj.db failed")`. PROJ says `Open of <path> failed` with the file path; the embedded database has no path. The embedded bytes are still used when the VFS is known.
- **Auxiliary databases** are attached by the foundation's `DatabaseContext::create_with_auxiliary`, which `Context::database` calls. A named SQLite VFS applies to the main database only, and a VFS together with auxiliary databases is an error (not supported yet).
- **Writable embedded database.** `deserialize_bytes` is read-only, and an insert session writes. `DatabaseContext::create()` (through `projdb::open_connection`) therefore copies the embedded image into a writable in-memory database (about 10 MB per call). `DatabaseContext::open(path)` stays read-only, and so does a file opened with a VFS: an insert session on those fails with SQLite's read-only error. The shared slot used by `with_db` stays read-only.
- **Insert session.** PROJ writes the session's rows into a temporary in-memory database attached to the context. Here they go into a savepoint of the same connection. Like PROJ's attachment, the rows are visible to every query of that `DatabaseContext` while the session is open, and they are gone when it closes. A failed statement is an error: `Writer::push` now returns a `Result`.
- **`open_plenty_of_contexts`.** The C test lowers `RLIMIT_NOFILE` and checks that 100 contexts still work. That needs `setrlimit`, which the crate does not depend on. The Rust test checks the same property another way: each context with a file database adds exactly one entry to `/proc/self/fd`, and each lookup adds none. The fd check runs on Linux only; the lookups run everywhere.
- **`proj_insert_object_session_destroy(NULL, NULL)`** has no counterpart: dropping a session is the destroy.
- **`suggests_code_for`** takes a `CodeForm` enum where PROJ takes `numericCode`, a boolean. Public signatures do not take bool parameters (CLAUDE.md).
- **`proj_suggests_code_for` with a NULL object or authority** has no counterpart: the arguments are typed.
- **Only one session at a time** is checked per database, as in PROJ, and it is shared by the clones of a `DatabaseContext`.

## Not covered

- `CApi.use_proj4_init_rules` (the non-global variant in `test_c_api.cpp`) is not in this lot. `Context::set_init_rules` and `Context::create_from_user_input` cover its first part, but its `init=` assertions were not checked against `projinfo`.
- `docs/PORTING-shape.md` still lists the seven tests as "C context state" in its intentionally unported table. That table is not edited here.

## Commands run

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --test unit_capi_context_2 --test unit_shape
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=.../target/proj-reference/build/bin:$PATH PROJ_DATA=.../target/proj-reference/build/data \
  cargo test --all-features --no-fail-fast
```

The full run on the merged base has 1778 passed, 32 ignored and 3 failed. The failures are `docs_in_sync::counts_in_docs_match_the_ledgers`, `docs_in_sync::unit_triage_is_current` and `progress_svg::progress_chart_is_current`. They compare the README, ROADMAP, `docs/UNIT-TRIAGE.md` and the progress chart with the ledgers, and the ledger lost seven lines. Per CLAUDE.md they are regenerated with `UPDATE_DOCS=1 cargo test --test docs_in_sync` (and `progress_svg`), which this lot does not run.
