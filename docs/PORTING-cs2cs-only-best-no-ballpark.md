# Porting sheet: `cs2cs --only-best --no-ballpark` with a missing grid

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, "Test cs2cs --only-best --no-ballpark
(grid missing)" (`EPSG:4326+3855` to `EPSG:4979`), exit 3, `stderr` as below.
Oracle used: the PROJ 9.9.0 package from conda-forge, run with `PROJ_DEBUG=3`.

## Behavior

- `--only-best` is `ONLY_BEST=YES`. The chosen operation is the first candidate, whether or not
  its grid is on disk.
- With the ballpark allowed, PROJ keeps a fallback: the failure is per point (`*	* inf`).
- With `--no-ballpark` there is no fallback. PROJ fails the creation, prints the grid message,
  then `cannot initialize transformation` with `cause: File not found or invalid`.

## What changed

- `src/iso19111/select.rs`, `best_operation_filtered`: a missing grid is checked before the
  export. Before, a grid-missing operation failed the export (`has no PROJ pipeline`), so the
  result was an error and never the `MissingGrid` the CLI needs. PROJ builds the pipeline with
  the grid name and fails on the grid, so the export is not what decides.
- `src/bin/cs2cs.rs`: `build_engine` returns `InitError` (`Cause` for the usual text, `Grid` for
  the whole stderr block). `Grid` is produced only for a filtered selection with
  `ALLOW_BALLPARK=NO` and `ONLY_BEST=YES`. Without `PROJ_DISPLAY_PROGRAM_NAME=NO` the block
  goes through `report`; with it, PROJ writes no blank line before the message.

## Ceilings

- The `Grid` error is produced only for the filtered path. The unfiltered `--only-best` keeps the
  per-point result, which matches `#122`. Upgrade path: one selection path for both.
- The message is built from the library's grid text, which follows the PROJ wording for this
  case. A different grid kind (for example a `vgridshift` with a different message) is not
  pinned by a test yet.

## Ledger

`test_cs2cs_various.yaml#123` now passes and leaves `docs/UNPORTED-CLI.md`. The cases of the
same family (`#118` to `#122`, `#124` to `#128`) are unchanged.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml` : passes.
- `UPDATE_DOCS=1` for `docs_in_sync` and `progress_svg` : regenerated the generated figures.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` : clean.
- Full `cargo test --all-features --no-fail-fast` : only `agree_cs2cs::named_crs_pairs_match_cs2cs`
  fails, with the same numbers as on `main` (EPSG:4326 to EPSG:5070, 0.16 m).
