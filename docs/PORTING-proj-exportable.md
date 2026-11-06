# Porting notes: PROJ exportability and grid steps in the `cs2cs` choice

Reference: PROJ 9.9.0 (tag `9.9.0` of `OSGeo/PROJ`, shallow clone kept outside the repo).
Branch `lot-proj-exportable-2`, from `lot-grid-old-names-2` (`2b57e7e`, the old-name fallback).

## PROJ sources

| PROJ 9.9.0 | Behaviour | Rust |
|---|---|---|
| `src/iso19111/operation/coordinateoperationfactory.cpp:1503-1520` | `isPROJExportable` is true when `exportToPROJString` does not throw. The grid and deformation steps are written by that call | `select.rs` `characteristics`: `export` writes the grid steps for transformations and concatenations (`export_to_proj_string_with_database`) |
| `coordinateoperationfactory.cpp:1477-1491` | under `USE_FOR_SORTING` only, a grid not on disk clears `gridsAvailable`. `KNOWN_AVAILABLE` does not | `select.rs` `best_operation_filtered`: `OnlyBest::On` sorts with `GridAvailabilityUse::KnownAvailable`, `OnlyBest::Off` with `UseForSorting` |
| `coordinateoperationfactory.cpp:932-1060` `SortFunction::compare` | exportable, approx, ballpark vertical, null, grids available, grids known, accuracy, area, xyz gridshift, steps | `select.rs` `precedes`, unchanged |
| `src/crs_to_crs.cpp:570-576` | `KNOWN_AVAILABLE` when `ONLY_BEST` or `WARN` is set, or the network is on; otherwise `DISCARD_OPERATION_IF_MISSING_GRID` | `OnlyBest::On` / `OnlyBest::Off` |
| `crs_to_crs.cpp:597-625` | the first operation `P` is built. It is not instantiable when a grid is missing, which warns (`pj_warn_about_missing_grid`) and fails under `ONLY_BEST=YES` | `select.rs` `best_operation_filtered`: the grids of the chosen pipeline, by PROJ name, must open. Otherwise `BestOperation::MissingGrid` with the message |
| `src/grids.cpp:1631` `VerticalShiftGridSet::open` | a GTX grid is read when the file **actually opened** (`fp->name()`) ends in `gtx`, so a `grid_alternatives` fallback gives a GTX reader | `projections/gridshift/mod.rs` `open_vertical` tests `opened_name(name)`, the file `gridpath::resolve` gives |

The grid names of the candidates are the PROJ names: `createOperations` substitutes them
(`usePROJAlternativeGridNames` defaults to true, `coordinateoperationfactory.cpp:175`), so availability is read on
`fr_ign_ntf_r93.tif`, which `open_resource_file` finds through `ntf_r93.gsb` (lot 1a).

## What changed

- `select.rs`: the `GridSteps` switch is gone. Every candidate is written with its grid steps. The
  final pipeline of the `cs2cs` choice is written with `export_operation`, which writes them too.
  The missing-grid decision reads the grids of the chosen pipeline, not the sort flag.
- `select.rs`: `OnlyBest::On` ranks without grid availability (`KNOWN_AVAILABLE`).
- `transform_export.rs`: `export_to_proj_string_with` (the grid-off writer, two methods) has no caller
  and is removed. The `PipelineFormatter` grid-steps flag stays: `buildop.rs` and `conversion_export.rs`
  still write without grid steps.
- `projections/gridshift/mod.rs`: `open_vertical` dispatches GTX on the opened file.

## Deviations from PROJ

1. Candidates are not discarded at creation. PROJ's `createFromCoordinateReferenceSystemCodes` with
   `discardIfMissingGrid` (`factory.cpp:7525`, `rejectOpDueToMissingGrid` at 4325-4367) drops an
   operation whose grid is neither on disk nor known. Here the candidate stays in the list, and the
   chosen one reports its missing grid. Under `DISCARD` the first available candidate is still taken
   (`find(grids_available)`), so the default `cs2cs` result is the same in the cases checked.
2. No alternative operations. PROJ's `proj_trans` falls back to the next candidate when a point is
   outside the chosen operation's grid (the `alternatives` of the pipeline object). `cs2cs` here has
   one pipeline. See cases 134 and 135.
3. `ONLY_BEST` keeps the `Chosen::instantiable` count of `exportable && grids_available`, which now
   counts every exportable candidate under `KnownAvailable`. The count is read only by the facade
   (`OnlyBest::Off`), so the facade is not affected.

## Cases

`cargo test --test cli_yaml`, against `2b57e7e` (lot 1a, 64 open rows before lot 1a, see
`docs/PORTING-grid-old-names.md`):

- Closed (6): `test_cs2cs_various.yaml#111` (geoid height, `--only-best`, `EPSG:4326+5773`),
  `#118` and `#119` (`--only-best`, grid missing: `*  * inf`), `#130` and `#131` (BETA2007), `#143`
  (`EPSG:5488` to `EPSG:4559+5757`, Guadeloupe height).
- Reopened: none. `#110` (NTF to RGF93, `--only-best`, grid missing) still passes: EPSG:9327 is first
  under `KNOWN_AVAILABLE`, and it fails with the grid message. It was the case the earlier attempt
  reopened, when the sort followed availability and picked EPSG:15958, whose grid is on disk.
- Changed, still open:
  - `#122` (NAD27 to NAD83, `--only-best`, grid missing). The stdout matches now. The stderr message
    does not come out, and the choice is different: PROJ names EPSG:8555 (NAD27 to NAD83 (7)) and this
    port names EPSG:1313 (NAD27 to NAD83 (4)), which has a grid on disk through `ntv2_0.gsb`. Both have
    accuracy known, and the sort then compares the area of the operation with the area of interest
    (`SourceTargetCRSExtentUse::SMALLEST`, the NAD83 extent): 33.96 for (4), 20.56 for (7). By that
    reading (4) would sort first, as here, so the PROJ choice has another cause. Not diagnosed.
  - `#134` and `#135` (BETA2007 outside the grid). The old output was a wrong number, now the point
    fails (`*  * inf`). Deviation 2: PROJ falls back to another candidate for the point.
- Unchanged, still open, and not caused by exportability: `#102` and `#103` (2D source, compound
  target, `-d 3`). The candidate list of `EPSG:4326` to `EPSG:4326+5773` has only the null offset. The
  geoid candidate exists for `"WGS 84"` to `"WGS 84 + EGM96 height"` (projinfo), so the difference is
  in the 2D-to-3D promotion of the `cs2cs` path. Not diagnosed. `#132`, `#133` (BETA2007 with
  `PROJ_DATA=$tmpdir`): PROJ uses an alternative, deviation 2.

## Test-harness note

`tests/cli_yaml.rs` compares the new run with the `docs/UNPORTED-CLI.md` on disk when the test starts,
and rewrites that file at the end. To measure against the committed state, restore that file first
(`git checkout -- docs/UNPORTED-CLI.md`). A stale file from an earlier experiment makes the ratchet
report cases that are open at the commit as regressions.

## Commands run

- `PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`,
  after each criterion (A: Written ranking; B: `export_operation`; D: `KnownAvailable` for `ONLY_BEST`;
  E: grid check on the chosen pipeline; the vertical GTX dispatch). Each run with the committed
  `docs/UNPORTED-CLI.md` restored first.
- Rejected trials, not kept: (C) availability read on the original grid names. PROJ substitutes the
  PROJ names first, so C was wrong and changed nothing in the cases; reverted.
- `projinfo -s NAD27 -t NAD83 --spatial-test intersects --summary`, `projinfo -s "WGS 84" -t "WGS 84 + EGM96 height" -o PROJ`,
  `cs2cs --only-best EPSG:4326+5773 EPSG:4979`, `proj +proj=vgridshift +grids=us_nga_egm96_15.tif`.
- Reading PROJ 9.9.0 `coordinateoperationfactory.cpp` (930-1060, 1440-1560, 1900-2010), `crs_to_crs.cpp`
  (495-630), `factory.cpp` (4325-4400, 7205-7530), `grids.cpp` (1610-1650, 3097-3150), `singleoperation.cpp`
  (1756-1780, 4340-4390).
- `UPDATE_DOCS=1 cargo test --test docs_in_sync`, `UPDATE_DOCS=1 cargo test --test progress_svg`
  (the progress chart and the README counts follow the closed rows).
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: 1541 passed,
  0 failed, 26 ignored. The new unit test is `projections::gridshift::tests::vertical_grid_by_proj_name_is_read_from_its_gtx_fallback`.
