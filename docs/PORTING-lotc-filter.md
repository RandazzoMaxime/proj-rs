# Port — `lotc-filter` (filter fidelity with `FilterResults`)

Branch `lotc/filter-fidelity`, based on `main` cf65310. Goal: `select_candidates` in `src/iso19111/factory.rs` follows PROJ 9.9.0 `FilterResults` (`coordinateoperationfactory.cpp`), so that the four pivot cases of `test_operationfactory.cpp` can pass without changing an existing expectation.

**Result: 4 filter steps committed, all existing tests kept. The 4 target cases stay open: they need the datum-pivot and too-small-area search, which is not ported. The filter step that fixes one of them through the CLI (`removeDuplicateOps`) is not committed, see [Open finding](#open-finding-removeduplicateops).**

## Sources read

PROJ 9.9.0, fetched from `raw.githubusercontent.com/OSGeo/PROJ/9.9.0/...` and grepped:

- `src/iso19111/operation/coordinateoperationfactory.cpp`: `FilterResults` (constructor, `computeAreaOfInterest`, `filterOut`, `sort`, `removeSyntheticNullTransforms`, `removeUninterestingOps`, `removeDuplicateOps`, `andSort`), `filterAndSort`, `SortFunction::compare`, `PrecomputedOpCharacteristics`, public `createOperations` (the `INTERSECTION` early exit), `hasPerfectAccuracyResult`, `hasTooSmallAreas`, `Private::createOperations` (lines 3579–3748, 4345–4582).
- `src/iso19111/operation/oputils.cpp`: `getAccuracy`, `getExtent`.
- `test/unit/test_operationfactory.cpp`: the target `TEST` bodies.

Oracle: `projbottle/proj/9.9.0/bin/projinfo` with `PROJ_DATA=…/share/proj`.

## Changes, one commit per step

| Step | Commit | What changed |
|---|---|---|
| A1 | `dbcb1a0` | `filter_and_sort`: `hasOnlyBallpark`, `hasNonBallparkOpWithExtent`, `hasNonBallparkWithoutExtent` as `filterOut`. Fallback condition `(res.empty() && !hasNonBallparkOpWithExtent) \|\| (hasOnlyBallpark && hasNonBallparkWithoutExtent)`, and the fallback appends to the result (it replaced it before). |
| A2 | `0077cb4` | `create_operations`: `INTERSECTION` returns nothing when the two CRS extents do not intersect (public `createOperations`, line 8096). |
| A3 | `e8566b7` | `SortExtents`: the area of interest is computed once (`computeAreaOfInterest`) and shared by the filter, `hasPerfectAccuracyResult` and the sort. `SortFunction`'s area is the pseudo area of the operation extent intersected with the area of interest, not the union formula of the two CRS extents. `select::characteristics` takes the `SortExtents`. |
| A4 | `d8f1138` (amended, see below) | `andSort` order: `removeSyntheticNullTransforms`, `removeUninterestingOps`, `removeSyntheticNullTransforms`. `removeUninterestingOps` is ported with `getTransformationStepCount` (identifiers empty rule) and `getAccuracy`. |

A4 was first committed without a lint fix: `clippy::fn_params_excessive_bools` (`max-fn-params-bools = 0`) rejected the bool parameter of `remove_synthetic_null_transforms`. The fix takes `&Filtered` instead, and the commit was amended before any push.

## Not committed

**A5, `removeDuplicateOps`** (key = PROJ string plus extent box, first kept, operation kept when its PROJ string cannot be written). Patch saved outside the repo at `$TMPDIR/lotc/a5-remove-duplicate-ops.patch`. Stopped, see below.

## Target cases

Expectations copied from `test/unit/test_operationfactory.cpp`, in `tests/unit_opfac_core_c_filter.rs` (uncommitted, see below). Current status on `d8f1138`:

| C++ TEST | Status | Reason |
|---|---|---|
| `createOperation_test_createOperationsWithDatumPivot_iter_1` (8237 → 8240, default spatial test, grids ignored) | open | Oracle: 1 candidate, the 3-step `NAD83(CSRS)v2 geocentric → v3 geocentric` pivot (`(3)`, extent Canada) with `+proj=noop`. Rust: the two registered `(1)`/`(2)` operations have small extents and are not `hasTooSmallAreas`-checked; the pivot is never built. Needs `hasTooSmallAreas` and `findsOpsInRegistryWithIntermediate(useCreateBetweenGeodeticCRSWithDatumBasedIntermediates=true)`, not ported. |
| `createOperation_ETRF2000_to_Amersfoort` (both directions) | open | Rust returns only `Ballpark geographic offset from ETRF2000 to Amersfoort`. Oracle: `Amersfoort to ETRS89-NLD [AGRS2010] (9)` through `ETRS89-NLD [AGRS2010] to ETRF2000 (1)`. The registry gives no direct operation, and the intermediate search through ETRS89-NLD is not ported. |
| `createOperation_ETRS89_XXX_to_ETRS89_YYY_using_ETRF2000` (11108 → 11128) | open | Rust: one ballpark only, `list.len() < 2`. Oracle: the ETRF2000 hub, `ETRS89-PRT [1995] to ETRF2000 (2)` plus `Inverse of ETRS89-ESP [ERGNSS] to ETRF2000 (1)`, then the `+proj=noop` inverse. Needs the intermediate search and `createOperationsEnsembleCRSToOtherGeodCRS`, not ported. |
| `NAD83_to_projected_CRS_based_on_NAD83_2011` (4269 → 6414) | open | Rust: 1 candidate. Oracle: 2 candidates. The registered `NAD83 to NAD83(2011) (1) + California Albers` has a Gulf of Mexico extent, so the partial-intersection test drops it; the CONUS chain `NAD83 to NAD83(HARN) (47) + …` is the intermediate path (`hasTooSmallAreas`), not ported. The ballpark is kept as expected. |

No target case is committed as failing. The file `tests/unit_opfac_core_c_filter.rs` is untracked: it holds the 4 open cases and will be committed only with the passing ones.

## Open finding: `removeDuplicateOps`

Patch A5 (uncommitted) makes `tests/cli_yaml/test_projinfo.yaml` case `Testing NAD83(2011) + NAVD88 height -> NAD83(2011)` (`-s EPSG:6349 -t EPSG:6319 --spatial-test intersects`) match PROJ: 3 candidates, the oracle's first operation `Inverse of NAD83(2011) to NAVD88 height (3)`. This is a real improvement.

It also stops two existing tests from passing, without a change of behaviour on their side:

- `tests/cli_yaml.rs` rewrites `docs/UNPORTED-CLI.md` on every run (the row `test_projinfo.yaml#26` leaves the ledger).
- `tests/docs_in_sync.rs::counts_in_docs_match_the_ledgers` (README counts) and `tests/progress_svg.rs::progress_chart_is_current` (`docs/progress.svg`) then fail. Both are regenerated only with `UPDATE_DOCS=1`, which this task forbids, and README, ROADMAP, `UNPORTED-CLI.md` and `progress.svg` are out of this task's scope.

Full suite with A5: 1370 passed, 6 failed (the 4 target cases plus the two docs tests above). Stopped, as the task requires for any regression of an existing test. The ledger regeneration needs the owner's `UPDATE_DOCS=1` run.

## Other deviations noted, not changed

- `SortFunction` ETRS89 branch 1 (`uses_ETRF2000` ahead of the generic one): in C++ its condition contains `!iterA->second.uses_ETRF2000_` together with `iterA->second.uses_ETRF2000_`, so it can never be true. The Rust `precedes` has a live version of it. Left as is: changing it moves the sort, and no test in this task needs it yet.
- `removeDuplicateOps` keys use the database-aware PROJ export (grid renames), where C++ uses `exportToPROJString` without a database. Only equality matters.
- `SortExtents`: an area of interest without a bounding box falls back to the CRS extents for the sort area, where C++ intersects the extents directly.

## Commands and results

- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast` on the baseline `cf65310` with the new test file: 1372 passed, 4 failed (the 4 target cases only). Existing tests: 0 failed.
- Same command after A1, A2, A3, A4: 1372 passed, 4 failed (the same 4 target cases).
- Same command with A5: 1370 passed, 6 failed (see Open finding).
- `cargo fmt --check`: clean at `d8f1138`.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean at `d8f1138`.
- Disk: `/System/Volumes/Data` had 16 GiB free before the last run (threshold 8 GiB).
