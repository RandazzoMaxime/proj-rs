# Porting notes: grid search follows PROJ's search path

Reference: PROJ 9.9.0 (tag `9.9.0` of `OSGeo/PROJ`, shallow clone kept outside the repo).

PROJ sources:

- `src/filemanager.cpp`
  - `FileManager::getProjDataEnvVar`, lines 1113-1131: `PROJ_DATA`, else the legacy `PROJ_LIB`.
  - `dontReadUserWritableDirectory`, 1537-1542: `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY`.
  - `pj_open_lib_internal`, 1544-1680: the lookup chain. Explicit search paths (1608), then the
    user-writable directory (1622-1633), then `PROJ_DATA` (1637-1641), then the compile-time data
    directory (1645-1655), then the bare name (1665-1668).
  - `pj_get_default_searchpaths`, 1718-1759: user-writable first, then `PROJ_DATA` if set, else the
    compile-time directory.
  - `FileManager::open_resource_file`, 1786-1831: the retry with the `proj_grid_name` alternative.
  - `proj_context_set_search_paths`, 2174-2188.
- `src/ctx.cpp`, `pj_ctx::set_search_paths`, 131-140.
- `cmake/ProjTest.cmake`, `proj_test_set_properties`, 5-15: every ctest run gets
  `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES` and `PROJ_DATA=<build>/data/for_tests`. A case's own
  `env` replaces it. `test/unit/CMakeLists.txt`, 101-105, is the same for the unit tests.

## Where the fixtures came from before this lot

`src/gridpath.rs` `directories()` returned `data/grids`, `tests/gie/tests`, `data/tests` and
`tests/gie` whatever the environment. Every grid lookup went through it: `gridpath::resolve`,
the deformation-model `candidates` (which also added `data/` and the repository root), and the
`require_grid` check of `tests/gie_run.rs`. So no test had to set anything, and no test could
remove the fixtures.

- `tests/gie_run.rs`: no environment, the same four directories by hand.
- `tests/cli_yaml.rs`: no global environment. A case's `PROJ_DATA: $tmpdir` never removed the
  fixtures, which is why the grid-missing cases found `data/tests` grids.
- In-process tests: seven lib tests (`network::tests::alaska_short_name_is_resolved_before_the_cdn`
  and six in `projections::models::tests`) and four integration tests (`tests/network_grid.rs`,
  `tests/ported_names.rs`, two in `tests/proj_unit.rs`) resolved vendored grids through the same list.
- `tests/gie/tests` is a copy of `data/tests`: its 44 files are byte-identical to the ones in
  `data/tests` (checked with `cmp`), and `tests/gie/` holds no grid. `data/tests` alone is enough.

## What changed

| Where | PROJ | Rust |
|---|---|---|
| `src/network.rs` `data_directories`, `proj_data_env`, `data_path_value`, `split_directories` | `getProjDataEnvVar`: `PROJ_DATA`, else `PROJ_LIB`; an empty value counts as unset (1113-1131). The value replaces the default directory (`pj_get_default_searchpaths`, 1744-1756) | the same precedence. The default is `data/grids` (git-ignored). Read on each lookup, as `getProjDataEnvVar` is called on each `pj_open_lib_internal` |
| `src/network.rs` `split_directories` | `:` on Unix, `;` on Windows (`dirSeparator`) | `:` on Unix, and `;` and `\|` on every platform (`;` and `\|` were already accepted). Empty entries are dropped |
| `src/gridpath.rs` `search_directories` | user-writable first, then `PROJ_DATA`, then explicit paths (1622-1641) | cache first (when enabled and not skipped), then `data_directories`, then `set_search_paths` paths |
| `src/gridpath.rs` `directories` | not in PROJ: the compile-time directory is one directory | removed. No fixture directory is searched without `PROJ_DATA` |
| `src/network.rs` `read_proj_ini` | `pj_load_ini` reads `proj.ini` from the search directories (once per context) | reads `proj.ini` from the `PROJ_DATA` directories, then `<repo>/data`, as before, with the same split |
| `src/projections/models/mod.rs` `resolve` | `pj_open_lib_internal` for every resource | goes through `gridpath::resolve`. The hard-coded `data/` and repository-root candidates and the duplicated `PROJ_DATA` split are removed |
| `src/proj.rs` `set_search_paths` | `proj_context_set_search_paths` (2174) | doc updated to the new order. Behaviour unchanged (see deviations) |
| `tests/common/mod.rs` `fixture_search_path`, `use_fixture_search_path` | `proj_test_set_properties` | new: `PROJ_DATA=<repo>/data:<repo>/data/tests`, `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES` |
| `src/gridpath.rs` `use_fixture_search_path` (`cfg(test)`) | same | same values, for the lib unit tests |
| `tests/cli_yaml.rs` `run_in` | same | each case gets the fixture path, then its `env` overrides it |
| `tests/gie_run.rs` `vendored_gie_ratchet`, `grid_present` | same | sets the same path before the first lookup. `require_grid` checks `data/` and `data/tests/` |
| `tests/network_grid.rs`, `tests/ported_names.rs`, `tests/proj_unit.rs` (two tests) | same | call `common::use_fixture_search_path()` first |
| `src/network.rs` tests `proj_data_replaces_proj_lib_and_empty_hides_it`, `search_path_is_split_on_colon_and_drops_empty_entries` | | new. Pure functions, so they do not touch the environment |

The data directory is read from the environment on each lookup, so an in-process test that sets it
after another test has resolved a grid still gets the right search path.

## Deviations from PROJ

These were in the port before this lot and are kept:

1. Explicit search paths are added to the list. In PROJ they replace it (`else if
   (!ctx->search_paths.empty())` at 1608 returns before the user-writable and `PROJ_DATA` branches).
   The public doc of `set_search_paths` says so.
2. A bare name that exists in the working directory is taken before the search directories. PROJ
   tries the working directory last (1665-1668). `gridpath::locate`.
3. A name with a directory part (`tests/simple_model_degree_horizontal.json`) is also looked up as
   `<dir>/<file name>`. PROJ opens a relative name as given. The gie `defmodel` files need this form.
4. The default data directory is `data/grids`, the port's stand-in for PROJ's compile-time share
   directory. Nothing is installed there, and the directory is git-ignored.

## Cases

`cargo test --test cli_yaml`:

- Before: 50 unported rows. After: 50 rows. Closed: none. Reopened: none.
- Changed row: `test_cs2cs_various.yaml#124`. The grid is no longer found when `PROJ_DATA` is
  `$tmpdir`, so the output is now `1402224.57 5076275.42 0.00`. Before it was `1402285.98 ...`. The
  expected PROJ value is `1402288.54 5076296.64 0.00`. The grid-missing result of this port is 64 m
  off in easting, so the remaining difference is in the operation chosen without the grid, not in
  the search. `docs/PORTING-cs2cs-grids-v2.md` says the cause of 124 was not diagnosed. It now is.
- Unchanged output, still open: 118-122, 125, 126, 128, 130, 132, 134, 135, 139.
- `#120` and `#121` need `PROJ_ONLY_BEST_DEFAULT` and `only_best_default` in `proj.ini`, which the
  port does not implement (no reference in `src/`). They are not affected by this lot.

`cargo test --test gie_run`: offline ratchet unchanged at 6811 passed (`tests/gie/passing.txt`).

## Commands run

- `git log --oneline -1` (4de4e6e), `git checkout -b lot-grid-search-scope`.
- Baseline, before any change: `cargo test --test cli_yaml` (passed, 50 rows, no diff),
  `cargo test --test gie_run` (passed), `cargo test --all-features` (1534 passed, 0 failed, 26 ignored).
- PROJ 9.9.0 sources: `git clone --depth 1 --branch 9.9.0 https://github.com/OSGeo/proj` into the
  session scratch directory. Not part of the repository.
- After the change: `cargo test --test cli_yaml`, `cargo test --test gie_run`,
  `cargo test --lib`, the affected integration targets, `cargo test --all-features --no-fail-fast`,
  `UPDATE_DOCS=1 cargo test --test docs_in_sync`, `cargo test --test docs_in_sync`,
  `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`.
