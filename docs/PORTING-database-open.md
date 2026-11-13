# Porting sheet: database-open

PROJ 9.9.0 `DatabaseContext::create(path, auxiliary_dbs)`, `DatabaseContext::Private::open`,
`attachExtraDatabases` and `SQLiteHandle::checkDatabaseLayout` in
`src/iso19111/factory.cpp`. Rust tests are `tests/unit_database_open.rs`.

This is a foundation lot: it opens a `proj.db` from disk and attaches auxiliary databases.
It does not close any case of `docs/UNPORTED-UNIT.md`. The ledger is unchanged, because
closing a case there needs `UPDATE_DOCS=1 cargo test --test docs_in_sync`, which this lot
does not run.

## Public API

| PROJ | Rust (`src/iso19111/factory.rs`) |
|---|---|
| `DatabaseContext::create()` (embedded) | `DatabaseContext::create() -> Result<Self, FactoryError>` (unchanged) |
| `DatabaseContext::create(path, {})` | `DatabaseContext::open(path: &Path) -> Result<Self, FactoryError>`, read-only |
| `DatabaseContext::create(path, aux, ctx)` | `DatabaseContext::create_with_auxiliary(path: Option<&Path>, auxiliary: &[PathBuf]) -> Result<Self, FactoryError>` |

`path: None` is the embedded `proj.db`, which PROJ spells as an empty path. `ctx` has no
counterpart: there is no `PJ_CONTEXT` in the Rust API.

`DatabaseContext::database_structure()` already existed. With auxiliaries attached it reads
`db_0.sqlite_master`, as PROJ's `getDatabaseStructure` does.

## Map

| PROJ | Rust |
|---|---|
| `DatabaseContext::create(path, aux)` with empty aux | `open` or `create` |
| `SQLiteHandle::open` (`SQLITE_OPEN_READONLY`) | `open`, `Connection::open_with_flags(READ_ONLY)` |
| `SQLiteHandle::checkDatabaseLayout` (main) | `check_main_layout`, `layout_from_rows` |
| `SQLiteHandle::checkDatabaseLayout` (aux) | `check_auxiliary_layout` |
| `attachExtraDatabases`: `ATTACH ... AS db_0` | `attach(conn, "db_0", path)` or `ATTACH ':memory:'` + `deserialize_bytes` for the embedded bytes |
| `attachExtraDatabases`: `ATTACH ... AS db_i` | `attach(conn, "db_i", path)` |
| `attachExtraDatabases`: `CREATE TEMP VIEW` union | `create_temp_views`, including the `conversion_method` `UNION` quirk |
| `attachExtraDatabases`: views re-created as temp | `create_temp_views` (`CREATE VIEW` to `CREATE TEMP VIEW`) |
| `run()` through `handle()` | `query`, `text_rows`, `lock()` unchanged: the temp views make the union transparent |

## Behaviour kept

- A missing auxiliary file is an error. PROJ's connection has no `SQLITE_OPEN_CREATE`,
  so `ATTACH` cannot create it. Here the connection is opened without `SQLITE_OPEN_CREATE`
  too, and the test checks that no file appears.
- A sparse auxiliary database (no `metadata` table, or no layout rows) is accepted. An
  auxiliary with a layout different from the main one is refused.
- The error texts are PROJ's, for the layout checks and for the auxiliary mismatch. A file
  that is not SQLite fails on its first query, as in PROJ.

## Deviations

- **Main file opened read-write when attached.** `open` is read-only, as PROJ's. The
  auxiliary path attaches the main file through a connection without `SQLITE_OPEN_CREATE`,
  but read-write, because the URI form of `ATTACH` would need escaping. The code never writes.
- **`PROJ_AUX_DB` is not read.** PROJ's `DatabaseContext::create` takes the auxiliary list
  from that environment variable when the list is empty. Here `create_with_auxiliary` uses
  only its argument, so an empty list never reads the environment.
- **Column names are quoted** in the temp views. PROJ joins them raw. The result is the same
  for every table of `proj.db`, and quoting keeps reserved words working.
- **A missing first table is handled.** PROJ concatenates a `UNION` separator before the
  first `SELECT` if the main database lacks a table. Here the separator is added only
  between selects that exist.
- **Layout check message on the main file.** `create()` now uses PROJ's messages
  (`lacks DATABASE.LAYOUT...`, `whereas a number >= 7 is expected`) instead of the earlier
  `proj.db layout is ...` text. No test depended on the old text.
- **No triggers** are copied to the temp layer, as in PROJ's `attachExtraDatabases`.
- **Thread safety.** The Rust connection is behind the same `Mutex` as `create()`. PROJ
  uses `SQLiteHandleCache`; that cache is not ported.

## Tests (`tests/unit_database_open.rs`, 15 tests)

Expected values:

- `WGS 84` for EPSG:4326: the reference `projinfo` 9.9.0 WKT2 output, `GEOGCRS["WGS 84"`
  (`PROJ_DATA=target/proj-reference/build/data`). The layout rows 1 and 7 are the ones in
  the shipped `proj.db`.
- The `OTHER_4326` row: the literal of `TEST(factory, attachExtraDatabases_auxiliary)`.
- The error texts: `factory.cpp`, `checkDatabaseLayout`, `DatabaseContext::Private::open`
  (`Open of ... failed`), `attachExtraDatabases`.
- The structure equality after attaching: `EXPECT_EQ(dbStructure, tableStructureBefore)`
  in the same C++ test.

The auxiliary database is built with `create_empty_structure`, the Rust form of
`createStructure` in `test_factory.cpp`. It takes `database_structure()` of the embedded
database and skips triggers, as the C++ auxiliary test does.

`rusqlite` turns foreign keys on by default; the test helper turns them off for the
auxiliary insert, as PROJ's test connection leaves them off.

The two PROJ cases `factory.attachExtraDatabases_auxiliary` and
`factory.attachExtraDatabases_auxiliary_error` are covered in substance by
`auxiliary_objects_are_found_next_to_the_main_ones` and
`missing_auxiliary_database_is_an_error_and_is_not_created`. They stay open in the ledger
for the reason given above.

## Not done here

- `populateWithFakeEPSG` and the `FactoryWithTmpDatabase` cases in `docs/UNPORTED-UNIT.md`
  still need a fake EPSG population; the foundation now exists for them.
- `CApi.proj_context_set_database_path_*` and `proj_context_copy_from_default` need the C API
  to call `create_with_auxiliary`; not wired.
- `factory.objectInsertion` needs a writable database; `insert_statements` remains text only.

## Commands

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo test --test unit_database_open
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
  PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --all-features --no-fail-fast
```

The reference value was read with
`PROJ_DATA=target/proj-reference/build/data target/proj-reference/build/bin/projinfo EPSG:4326 -o WKT2_2019 -q`.
