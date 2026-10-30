# Porting notes: cs2cs grids (lot 2)

Reference: PROJ 9.9.0. Sources: `src/apps/cs2cs.cpp` (`--only-best`, `ONLY_BEST`),
`src/crs_to_crs.cpp` (`proj_create_crs_to_crs_from_pj`: `PARTIAL_INTERSECTION`,
`KNOWN_AVAILABLE`), `src/trans.cpp` (`pj_warn_about_missing_grid`, `proj_trans`),
`src/iso19111/operation/coordinateoperationfactory.cpp` (`SortFunction`, `sort()`),
`src/iso19111/operation/singleoperation.cpp` (`exportToPROJStringGeneric`, grid branches).

Scope: `tests/cli_yaml/test_cs2cs_various.yaml` ids 110, 122, 123, 124, 125, 130, 132, 134, 135,
and the grid-missing cases of 116-135 that are not compound CRS (`EPSG:A+B`).

## Measure

Run of `cargo test --test cli_yaml` with
`PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`.
The base is `origin/main` at `8060862` (the branch was rebased onto it), run on the same machine.

| | cs2cs open | total open |
|---|---|---|
| `origin/main` (8060862) | 29 | 73 |
| this branch | 25 | 69 |

Closed: 110 (lot), and 136, 137, 138 (outside the lot, see "Side effect" below).
No case went from passing to open.

## What changed

| Where | PROJ | Rust |
|---|---|---|
| `src/iso19111/select.rs` `best_pipeline_with`, `OnlyBest`, `BestOperation` | `ONLY_BEST=YES` keeps the first operation even when its grid is missing; without it, the first operation with its grids available is used (`proj_trans` retry, `skipNonInstantiable`) | new. `best_pipeline` keeps its signature and calls it with `OnlyBest::Off` |
| `src/iso19111/select.rs` `missing_grid_message` | `pj_warn_about_missing_grid`: `Attempt to use coordinate operation <name> failed.` then one `Grid <name> is not available. Consult …` per missing grid | new, same text |
| `src/iso19111/select.rs` `proj_grids` | the grid names PROJ reads (`substitutePROJAlternativeGridNames`, via `grid_alternatives`) | new; availability and the message use these names |
| `src/iso19111/select.rs` `precedes` | `xyzgridshift` operations come first (`hasXYZGridshift_ && !other → true`) | the criterion was inverted; now `return a.xyz_gridshift` |
| `src/iso19111/select.rs` `best_pipeline_with` | `proj_create_crs_to_crs` sets `PARTIAL_INTERSECTION` | `context.set_spatial_criterion(PartialIntersection)` (the default is `StrictContainment`) |
| `src/iso19111/transform_export.rs` `geocentric_grid` | `xyzgridshift` for method 1087 (`Geocentric translations (geog2D domain) by grid (IGN)`), with `push`/`cart`/`pop` and `grid_ref` | new. Other grid methods stay `Unimplemented` |
| `src/iso19111/pipeline.rs` `PipelineFormatter::with_grid_names`, `grid_name` | `formatter->databaseContext()` for the name substitution | new; `export_to_proj_string_with` passes the table |
| `src/bin/cs2cs.rs` `select_pipeline`, `Engine::Failing` | `cs2cs --only-best` reaches `proj_create_crs_to_crs` with `ONLY_BEST=YES` | `--only-best` is passed to `best_pipeline_with`; a missing grid prints the message and each point gives `*` |
| `src/iso19111/mod.rs` | | re-exports `best_pipeline_with`, `BestOperation`, `OnlyBest` |

## Proof of each change

Checked by replaying the case command of the yaml with the change in place and with it removed
(`cs2cs` built each time, same environment as the harness).

| Change | Case | Result |
|---|---|---|
| `xyzgridshift` ordering (`precedes`) | 110 | with the old sense, `Inverse of ETRS89-FRA [RGF93 v1] to NTF (2)` (NTv2) is chosen and the output differs |
| `xyzgridshift` export (method 1087) | 110 | without it, operation (1) is not exportable and the output is the ballpark `49dN 2dE 0.000` |
| `--only-best` and `MissingGrid` message | 110 | observed: stdout `49 2 0	*	* inf` and the stderr line match the reference (not ablated) |
| `PARTIAL_INTERSECTION` | 136, 137, 138 | open in the run before the change, passing in the run after it; it was the only change to the selection context between the two runs |

## Deliberate differences and open points

- The sort still applies the `gridsAvailable` criterion in every context. PROJ applies it only for
  `USE_FOR_SORTING`, and `proj_create_crs_to_crs` uses `KNOWN_AVAILABLE`. Applying it always did not change
  110, and the harness was not run on that variant, so this is not verified against PROJ on other pairs.
- The name substitution in the export text (`formatter.grid_name`) is not exercised by a case that passes:
  110 never runs its pipeline. It follows PROJ's `substitutePROJAlternativeGridNames`.
- `Engine::Failing` prints from `apply`. PROJ prints from `proj_trans` for each point, which is the same
  observable output.

## Cases still open (lot)

| Case | State | Cause |
|---|---|---|
| 130 | open | Needs the PROJ name fallback: `de_adv_BETA2007.tif` is read as `BETA2007.gsb` (`getOldProjGridName`). That fallback also makes `us_noaa_conus.tif` (`conus`) and `fr_ign_gr3df97a.tif` (`gr3df97a.txt`) available, which the reference does not have. It breaks the existing unit test `crs::epsg_pipeline_tests::nad_grid_is_not_a_coordinate` (asserts `Unsupported("grid")` for NAD27 → NAD83) and changes 110. Resolving it needs a decision on which fixtures count as data, which is the `PROJ_DATA` search change this lot does not take. The geocentric-target grid step in `crs.rs` `assemble` was also needed (its removal made 130 fail when the fallback was on) and is not in this branch. |
| 122 | open | `--only-best NAD27 NAD83`: the reference picks `NAD27 to NAD83 (7)` (CONUS, grid absent) because the choice is made per point by `pj_get_suggested_operation` (area of use contains the point, then accuracy). This port picks one operation per pair; it picks `NAD27 to NAD83 (4)` (Canada NTv2, which does not contain 40N 100W) and prints its grid message. Needs per-point selection among the alternatives. |
| 123 | open | `EPSG:4326+3855 → EPSG:4979` is a compound CRS. `createOperationsCompoundToGeog` is not ported (same cause as 116-121). |
| 124 | open | Not diagnosed. Run with `PROJ_DATA` pointing at a directory that holds only `proj.db`: the vendored fixtures are still searched (`src/gridpath.rs`), so a grid the reference does not have can be found. Not verified. |
| 125 | open | `EPSG:26915 → EPSG:26715` needs the datum pivot through WGS 84 (`createOperationsWithDatumPivot` for this pair); the grid-missing scenario depends on the same fixture search as 124. |
| 132 | open | Grid-missing scenario with `PROJ_DATA` set to a `proj.db`-only directory, same fixture-search issue as 124. The expected value is the ballpark offset, whose 116 m difference from the identity is not explained (see `docs/PORTING-cs2cs-grids-epochs.md`, which this lot does not replace). |
| 134, 135 | open | The point is outside the BETA2007 extent. The reference falls back to the ballpark at run time. The port finds no operation for `EPSG:4746 → EPSG:4978` (`best_pipeline` returns `no operation`), so the `crs.rs` pair path answers, and it has no run-time fallback. |

## Commands run

- `git fetch origin && git worktree add … -b lot-cs2cs-grids-v2 origin/main`, first from `f4c2ab0`, then rebased onto `8060862`.
- Baseline: `cargo test --test cli_yaml` on `origin/main` `8060862`: 29 cs2cs open, 73 total.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: exit 0, 552 passed, 0 failed, 1 ignored. The ignored one is `gie_urns_build_when_the_grid_is_present` (NKG), a runtime skip that is already on `origin/main` (it needs `PROJ_NETWORK=ON`). No `#[ignore]` was added.
- `docs/UNPORTED-CLI.md` is regenerated by the harness and is not part of this change.
