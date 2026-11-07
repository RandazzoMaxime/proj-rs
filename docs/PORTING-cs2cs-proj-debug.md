# cs2cs: PROJ's debug lines of a transformation (`PROJ_DEBUG`)

Port of the log lines PROJ 9.9.0 writes while `proj_create_crs_to_crs` and `proj_trans` run, when a
candidate's grid is missing, for `cs2cs` under `PROJ_DEBUG`. Closes `test_cs2cs_various.yaml#126`
(the stderr of a compound pair with a missing grid, `PROJ_DEBUG=2`). The rest of PROJ's debug output
is listed under *Open*.

## Where PROJ writes each line

| PROJ 9.9.0 | Line | Level |
|---|---|---|
| `src/ctx.cpp:84-114` `createDefault`: `PROJ_DEBUG` `ON` = 2, `OFF` = 1, a number = its level (below -3, TRACE), other values refused with `Invalid value for PROJ_DEBUG` | the debug level | |
| `src/log.cpp:104-115` `pj_log`: written when `level <= debug_level` | the gate | |
| `src/proj.h:373-381` `PJ_LOG_ERROR` = 1, `PJ_LOG_DEBUG` = 2, `PJ_LOG_TRACE` = 3 | levels | |
| `src/trans.cpp:179-211` `pj_warn_about_missing_grid`: `Attempt to use coordinate operation <name> failed.`, one ` Grid <g> is not available. Consult ...` per missing grid, and, while `warnIf` is set and `errorIf` is not, the suffix of 199-206 | `DEBUG` under warn, `ERROR` under `ONLY_BEST=YES` | |
| `src/crs_to_crs.cpp:570-576` first list read with `KNOWN_AVAILABLE` (not `DISCARD`) unless `ONLY_BEST=NO` | | |
| `src/crs_to_crs.cpp:617-651` one candidate in the first list, not instantiable, warn: `pj_warn_about_missing_grid` at 650, `warnIf` cleared, then the search goes on (rerun 685-773) | creation line | `DEBUG` |
| `src/crs_to_crs.cpp:776-795` several candidates kept: `alternativeCoordinateOperations` | the alternatives list | |
| `src/trans.cpp:248-303` retry loop: `proj_errno_reset` and the errno text (257-261), `Did not result in valid result. Attempting a retry with another operation.` (263-265) | retry lines | `DEBUG` |
| `src/trans.cpp:269-276` `Using coordinate operation <name>` when `iCurCoordOp` changes (`iCurCoordOp` starts at -1) | first use | `DEBUG` |
| `src/trans.cpp:289-298` the first failure under warn: `pj_warn_about_missing_grid(alt.pj)`, `warnIf` cleared, `skipNonInstantiable` set | per-point warning | `DEBUG` |
| `src/trans.cpp:305-345` no candidate: `Using coordinate operation <name> as a fallback due to lack of more appropriate operations` for the first grid-free candidate (`iCurCoordOp` check at 323) | fallback line | `DEBUG` |
| `src/fwd.cpp:282-293` `pj_fwd4d` of a pipeline with no converter sets `PROJ_ERR_OTHER_NO_INVERSE_OP`, text `No inverse operation` (`src/strerrno.cpp:42`) | errno text, logged by the next retry | `DEBUG` |

## Code

- `src/iso19111/alternatives.rs`: `LogLevel`, `LogLine`, `WARN_SUFFIX`. `Alternatives::forward_logged`
  is the per-point choice with its log lines (`forward` is the same without them). The
  `current` (`iCurCoordOp`), `warn` and `last_failure` cells carry PROJ's per-instance state.
- `src/iso19111/select.rs`: `creation_warning` (the 617-651 check), `PipelineLog`,
  `best_pipeline_logged`. `Item::name` is the operation name of a candidate.
- `src/bin/cs2cs.rs`: `proj_debug_level` and `debug_level_of` (ctx.cpp:90-111), `print_line` (the
  level gate of log.cpp:104-115), `Engine::Logged` and `Engine::LoggedFailing` (the first-use line,
  written once), `logged_pipe` and `logged_failing` (the creation lines).

The library returns the lines as data. Only the binary writes them, at the level the environment
asks for.

## Behaviour

- Warn policy (no `--only-best`), compound or geodetic pair, a grid missing:
  - one candidate in the first list (KNOWN_AVAILABLE) and its grids missing: the creation line, then
    the first use of the first instantiable candidate (`crs_to_crs.cpp:650`, `trans.cpp:269`).
    This is `#126`.
  - several candidates: the first use, then each failing candidate's warning, errno and retry line,
    the warning once per instance, and the fallback line when a grid-free candidate is used
    (`trans.cpp:248-345`).
- `--only-best` (`ONLY_BEST=YES`): each point logs the first use when the list has several
  candidates, then the error line (`trans.cpp:269`, `289-295`), at `ERROR` level.
- Nothing is written at the default level for a warn-policy line (`DEBUG`). `PROJ_DEBUG=2` writes
  them. `PROJ_DEBUG=1` and unset write only the error lines.

## Deviations from PROJ

1. Candidate order of the per-point choice. `select_alternatives` reads the first list with the
   default availability (`UseForSorting`), where PROJ reads it with `KNOWN_AVAILABLE`
   (`crs_to_crs.cpp:570-576`). Trying `KNOWN_AVAILABLE` for the first list was measured and
   **rejected**: it fixed `EPSG:4230 -> EPSG:4258` (the candidate `ED50 to ETRS89 (12)` first) but
   the factory's `KNOWN_AVAILABLE` set drops `NAD27 to NAD83 (7)`, which PROJ keeps, so
   `NAD27 -> NAD83` at 40N 100W took `(4)` (Canada, NTv2) instead of `(7)`. With grids present a US
   point would then get the Helmert fallback where PROJ uses NADCON5. The factory's known-grid set
   needs its own fix first.
2. The `No inverse operation` errno line is written only for a candidate whose grids are missing
   (`!instantiable`), where `pj_fwd4d` has no converter (`fwd.cpp:290-293`). Other failures (a point
   outside a projection's domain, for instance) write no errno line: PROJ's errno text there is not
   ported. Each oracle run that writes it failed on a missing grid (rows 17, 19, 24, 27 and 33).
3. Compound pairs (`select_alternatives` returns an error for them, deviation 2 of
   `PORTING-cs2cs-alternatives.md`) take the legacy path, which picks the first instantiable candidate
   of the list for the first use. PROJ's per-point retry for them (`EPSG:4326+3855 -> EPSG:4979`,
   three candidates, the first one's warning and retry) is not ported. The first use is therefore the
   candidate of the list, not the one PROJ's suggestion picks for the point. The per-point retry lines
   are not written for compound pairs.
4. A single candidate of a geodetic pair under the warn policy, with its grids missing, keeps the
   single-candidate path (`select_alternatives` returns `Selection::Single` before
   `crs_to_crs.cpp:650`). PROJ logs the creation line and reruns. Not ported: the creation line is
   written by the legacy path only.
5. `PROJ_DEBUG=3` TRACE lines (`pj_open_lib`, `pj_ellipsoid`, the pipeline steps) are not written.
   The `pj_open_lib` lines are dropped by the yaml harness (`grep-v`). Negative levels are read as
   silent: PROJ's errno-triggered logging (`ctx.cpp:99-107`) is not ported.
6. The candidate's name in `Using coordinate operation <name>` is the name of our operation. For a
   concatenated operation built from a PROJ string, our naming of the inverse legs differs
   (`Inverse of axis order change (2D)` where PROJ writes `axis order change (2D)`). The chosen
   candidate and the coordinates agree.

## Oracle table

PROJ 9.9.0 `cs2cs` (`/tmp/.../scratchpad/reftest/build/bin/cs2cs`) and this port, same environment:
the grids missing rows use a directory holding only `proj.db`, the grids-present rows use
`PROJ_DATA=<reftest data>:data/tests` for both. `PROJ_DISPLAY_PROGRAM_NAME=NO` where the banner
would differ. `pj_open_lib` lines are dropped on both sides, as the yaml harness does. `SAME` means
identical stderr and stdout and exit code.

| # | Pair, input | Options | Grids | `PROJ_DEBUG` | Result |
|---|---|---|---|---|---|
| 1 | `EPSG:4326+5773 -> EPSG:4326+5782`, `39 -3 0` | | missing | unset | SAME, silent |
| 2 | same | | missing | 1 | SAME, silent |
| 3 | same | | missing | 2 | SAME: creation line, first use |
| 4 | same | | missing | 3 | SAME (after the `pj_open_lib` filter) |
| 5 | same | `--only-best` | missing | 2 | SAME: the error line and `cannot initialize`, no first use (no alternatives list) |
| 6 | same | `--only-best=no` | missing | 2 | SAME, silent |
| 7 | `EPSG:4326+3855 -> EPSG:4979`, `49 2 0` | `--only-best` | missing | 2 | SAME: first use, error |
| 8 | `EPSG:4326+3855 -> EPSG:4326`, `40 -3 0` | `--only-best` | missing | 2 | SAME, silent |
| 9 | `EPSG:4326+3855 -> EPSG:4979`, `49 2 0` | | missing | 2 | DIFF stderr: deviation 3 (per-point retry of a compound pair) |
| 10 | `EPSG:4326+5782 -> EPSG:4326+5773`, `49 2 0` | | missing | 2 | SAME: creation line, first use |
| 11 | same, `39 -3 0` | `--only-best` | missing | 2 | SAME |
| 12 | `EPSG:4326 -> EPSG:3035`, `49 2 0` | | missing | 2 | SAME: first use |
| 13 | `EPSG:4746 -> EPSG:4978`, `49 2 0` | | missing | 2 | SAME: fallback line |
| 14 | `EPSG:4978 -> EPSG:4746`, `49 2 0` | | missing | 2 | SAME: fallback line |
| 15 | `EPSG:4746 -> EPSG:4978`, `49 2 0` | `--only-best` | missing | 2 | SAME |
| 16 | `EPSG:4230 -> EPSG:4326`, `45 10 0` | | missing | 2 | SAME: first use |
| 17 | `EPSG:4272 -> EPSG:4167`, `-41 174 0` | | missing | 2 | SAME: retry lines |
| 18 | `EPSG:4289 -> EPSG:4326`, `52 5 0` | | missing | 2 | SAME, silent |
| 19 | `NAD27 -> NAD83`, `40 -100 0` | | missing | 2 | SAME: retry lines |
| 20 | same | `--only-best` | missing | 2 | SAME |
| 21 | same | `--only-best --no-ballpark` | missing | 2 | SAME |
| 22 | same | `--only-best=no` | missing | 2 | DIFF stdout and stderr; the base commit gives the same stdout (`40dN 100dW`), so it predates this lot |
| 23 | `EPSG:4267 -> EPSG:4326`, `40 -100 0` | | missing | 2 | SAME |
| 24 | `EPSG:4230 -> EPSG:4258`, `40 -3 0` | | missing | 2 | DIFF stderr: deviation 1 (candidate `(16)` before `(12)`), stdout SAME |
| 25 | same | `--only-best` | missing | 2 | DIFF stderr: deviation 1, stdout SAME |
| 26 | same | `--only-best=no` | missing | 2 | SAME |
| 27 | `+proj=latlong +datum=WGS84 +to +proj=utm +zone=10 +datum=NAD27`, `-111.5 45.25919444444 0` | | missing | 2 | DIFF stderr: deviation 6 (operation names), stdout SAME |
| 28 | `EPSG:26915 -> EPSG:26715`, `569704.566 4269024.671 0` | | missing | 2 | SAME |
| 29 | `NAD27 -> NAD83`, `50 -100 0` | | present | unset | SAME, silent |
| 30 | same | | present | 2 | SAME: first use only, `NAD27 to NAD83 (4)` |
| 31 | same | | present | 3 | DIFF: TRACE lines, deviation 5 |
| 32 | `EPSG:4267 -> EPSG:4269`, `50 -100 0` | | present | 2 | SAME |
| 33 | `EPSG:4267 -> EPSG:4326`, `45 -75 0` | | present | 2 | SAME: first use only |
| 34 | `EPSG:4326+5773 -> EPSG:4326+5782`, `39 -3 0` | | present | 2 | SAME |
| 35 | `EPSG:4230 -> EPSG:4258`, `40 -3 0` | | present | 2 | SAME: first use only |
| 36 | `EPSG:4272 -> EPSG:4167`, `-41 174 0` | | present | 2 | SAME: first use only |
| 37 | `EPSG:4746 -> EPSG:4978`, `49 2 0` | | present | 2 | SAME |

Summary: 31 SAME, 6 DIFF (rows 9, 22, 24, 25, 27, 31). Row 22 is a stdout difference that predates
this lot, and its output is the same with the base commit.

## Test cases

Closed by this lot (`cargo test --test cli_yaml`): `test_cs2cs_various.yaml#126`.

Unit and integration tests, expected values from PROJ 9.9.0's output above:

- `tests/proj_debug_log.rs`: the creation line and the first use of `#126` (`best_pipeline_logged`),
  nothing logged under `ONLY_BEST=YES` at creation, and the per-point retry of `NAD27 -> NAD83` with
  its once-per-instance warning (`Alternatives::forward_logged`).
- `src/bin/cs2cs.rs` `proj_debug_values_follow_the_context_reader`: the values of `PROJ_DEBUG`.

No `cli_yaml` case changed its stderr expectation. The `#126` row left `docs/UNPORTED-CLI.md`.

## Open

- Deviations 1, 3, 4, 5 and 6 above. Deviation 1 needs the factory's `KNOWN_AVAILABLE` set (the
  `NAD27 to NAD83 (7)` candidate dropped) fixed before the first list can read with it.
- Row 22: `--only-best=no` for `NAD27 -> NAD83` gives `40dN 100dW` on both base and head. The Discard
  path of the alternatives is not the cause (this lot does not change its results).
- The banner `Rel. 9.9.0, September 15th, 2026`: PROJ prints `Rel. 9.9.0 (1df2be6), ...`. Not this lot.

## Commands

```bash
# Reference PROJ 9.9.0, the oracle (as in PORTING-cs2cs-alternatives.md):
PROJ_DEBUG=2 PROJ_DATA=<reftest data>:data/tests <reftest>/bin/cs2cs EPSG:4326+5773 EPSG:4326+5782
# This port, grids missing (a directory holding only proj.db):
PROJ_DEBUG=2 PROJ_DATA=<dir with proj.db> target/debug/cs2cs EPSG:4326+5773 EPSG:4326+5782
cargo test --test cli_yaml
cargo test --test proj_debug_log
cargo test --bin cs2cs proj_debug
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```
