# Porting notes: grid old names (`open_resource_file` retry)

Reference: PROJ 9.9.0 (tag `9.9.0` of `OSGeo/PROJ`, shallow clone kept outside the repo).
Branch `lot-grid-old-names-2`, from `ee9ad11` (grid search follows PROJ's search path).

## PROJ sources

| PROJ 9.9.0 | Behaviour | Rust |
|---|---|---|
| `src/filemanager.cpp:1786` `FileManager::open_resource_file` | opens the name as given first | `gridpath::locate` (unchanged) |
| `filemanager.cpp:1802-1831` | name without `.tif`, not a path, not a URL, not `proj.db`: on failure, `getProjGridName(name)` (old name to PROJ name) and open that | `gridpath::resolve`, branch without `.tif`, through `network::proj_grid_filename` (unchanged) |
| `filemanager.cpp:1833-1857` | name **with** `.tif` (`strstr`), not a path, not a URL: on failure, `getOldProjGridName(name)` (PROJ name to old name) and open that | `gridpath::resolve`, branch with `.tif`, through the new `network::old_grid_filename` |
| `src/iso19111/factory.cpp:3563` `getProjGridName` | `SELECT proj_grid_name ... WHERE old_proj_grid_name = ?` | `network::proj_grid_filename` (unchanged) |
| `factory.cpp:3575` `getOldProjGridName` | `SELECT old_proj_grid_name ... WHERE proj_grid_name = ?`, first row; empty when there is no row | `network::old_grid_filename` |

The lookup that was missing is the second `.tif` row. Before this lot, `gridpath::resolve` only
mapped an old name to its PROJ name, so a PROJ name such as `de_adv_BETA2007.tif` could never reach
`BETA2007.gsb`, and `bare_short_name` excluded every `.tif` name from the old-name query.

`network::plain_name` is the port of PROJ's `!is_rel_or_absolute_filename(name)`, `http://`,
`https://` tests. Both fallbacks use it. `bare_short_name` (the forward query) is `plain_name` and
no `.tif`, which is the old behaviour.

Database rows used (`data/proj.db`, `grid_alternatives`):

| `original_grid_name` | `proj_grid_name` | `old_proj_grid_name` | Fixture on the PROJ_DATA path |
|---|---|---|---|
| `BETA2007.gsb` | `de_adv_BETA2007.tif` | `BETA2007.gsb` | `data/tests/BETA2007.gsb` |
| `WW15MGH.GRD` | `us_nga_egm96_15.tif` | `egm96_15.gtx` | `data/tests/egm96_15.gtx` |
| `conus.las` | `us_noaa_conus.tif` | `conus` | `data/tests/conus` (CTable2) |
| (no-`.tif` direction) | `fr_ign_RAGTBT2016.tif` | `RAGTBT2016.gtx` | `data/tests/fr_ign_RAGTBT2016.tif` |

## What changed

- `src/network.rs`: `old_grid_filename` (new, PROJ `getOldProjGridName`), `plain_name` (split out of
  `bare_short_name`).
- `src/gridpath.rs`: `resolve` picks the fallback by `.tif`, as `open_resource_file` does.
  Unit tests `gridpath::tests` (four): `de_adv_BETA2007.tif` gives `BETA2007.gsb`,
  `RAGTBT2016.gtx` gives `fr_ign_RAGTBT2016.tif`, a path with `.tif` is not renamed, an unknown
  `.tif` name stays missing.

## Deviations from PROJ

1. `getOldProjGridName` returns the first row, as PROJ does. A row whose `old_proj_grid_name` is
   NULL or empty gives no fallback. The Rust query has no `ORDER BY`, as the PROJ query has none.
2. The network retry (`filemanager.cpp:1859-1921`) is not ported in this lot. It only runs when the
   network is enabled, and it fetches the name as given. Unchanged behaviour (`network::decide_download`).
3. The `proj.db` exclusion of the first branch is not ported. No `grid_alternatives` row has the name
   `proj.db`.

## Cases

`cargo test --test cli_yaml`, against the baseline at `ee9ad11` (64 open rows, `docs/UNPORTED-CLI.md`):

- Closed: none. Reopened: none (the ratchet is green, the set of open ids is unchanged).
- Output changed, still open:
  - `test_cs2cs_various.yaml#130` (forward BETA2007): `3995986.86 4846735.07 1100568.03`
    becomes `4002857.22 705811.73 4897853.73`. The grid is found now. The result is the
    geocentric value without the datum shift, because the cs2cs choice still rejects the grid
    operation as not exportable (`select.rs`, "has no PROJ pipeline"). That is lot 1b.
  - `#131` (inverse BETA2007): `10dE -47.747` becomes `50d29'53.573"N 9d59'55.692"E 700.308`.
  - `#134`, `#135` (outside the grid): the same mechanism, different numbers.
- Unchanged: `#132` and `#133` (the grid is missing under `PROJ_DATA=$tmpdir`), `#122`, `#124`,
  `#125`. #132 still gives the legacy output, because the cs2cs choice does not take the grid
  operation yet.

Nothing closes in this lot. The status sheet of the previous attempt
(`PORTING-grid-old-names.status.md`, scratchpad) reported #131 and #133 as passing at baseline. The
strict harness of `ee9ad11` already reports them open, so that attempt's green was from the lax
comparison, not from the port.

`cargo test --lib`: `crs::epsg_pipeline_tests::nad_grid_is_not_a_coordinate` passes. With the old
`data/tests` search of `gridpath::directories()` it would find `us_noaa_conus.tif` through `conus`.
That search is gone on `ee9ad11`, so the test sees no `conus` unless a test before it sets
`PROJ_DATA`. Those tests (`network::`, `projections::models`) sort after `crs::` and run after it.
The test is not weakened. It depends on that order, which is an existing property of the lib tests
that call `use_fixture_search_path`.

## Commands run

- `git checkout -B lot-grid-old-names-2 ee9ad11`
- `PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`
  (baseline, then after the change)
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --lib` (three runs)
- `python3` queries of `data/proj.db` `grid_alternatives`
- Reading PROJ 9.9.0 `src/filemanager.cpp` 1780-1860 and `src/iso19111/factory.cpp` 3555-3600,
  and `data/CMakeLists.txt` (the `for_tests` fixtures, which include `conus`)
