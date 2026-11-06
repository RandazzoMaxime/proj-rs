# Port: vertical fallback for an identified geographic CRS (`createOperationsGeogToVertWithAlternativeGeog`)

Status: **not closed**. No change is committed to `src/`. The two attempts are recorded below and
were reverted under the rule "max 2 attempts, then revert and write the status". The CLI ledger
`docs/UNPORTED-CLI.md` is unchanged. Open targets: `test_cs2cs_various.yaml#126` (stderr) and
`#128` (exit 3). `#127` passes and must keep passing.

This sheet first recorded the blockers of the earlier lot on compound to geographic vertical
branches (the four open unit tests of `tests/unit_op_compound_to_compound_2.rs`, `#126`). The
PROJ branches it names are unchanged.

Source: PROJ 9.9.0 (tag `9.9.0`). References are `file:line` in the PROJ tree:

- `src/iso19111/operation/coordinateoperationfactory.cpp`:
  - `createOperationsFromDatabaseWithVertCRS` 5142–5230. Its steps: the intermediate vertical CRS
    (5154–5164), the NAVD88-style geog 3D to 2D retry (5169–5192), the vertical pivot (5199–5207),
    then the alternative geographic fallback (5223–5230). The guard of the last step is
    `res.empty() && !context.inCreateOperationsGeogToVertWithAlternativeGeog && geogSrc && vertDst`
    (and the mirror with `geogDst && vertSrc`). **No test on the identifiers of the geographic CRS.**
  - `createOperationsGeogToVertWithAlternativeGeog` 5073–5137: `findOpsInRegistryDirectTo(target)`,
    each operation whose source is geographic is kept, through a unit change when the geographic
    source has a 3D axis in another unit (5107–5129). No identifier test, no ballpark test.
  - `createPROJBased` 2633: returns a `SingleOperation`.
  - `findOpsInRegistryDirect` / `findOpsInRegistryDirectTo` 1900–2034: the grid availability of the
    context (`DISCARD_OPERATION_IF_MISSING_GRID` or `KNOWN_AVAILABLE`) is applied at the registry
    search, before the fallbacks run.
- `src/iso19111/operation/concatenatedoperation.cpp`:
  - `createComputeMetadata` 674–698: flattens a `ConcatenatedOperation` only (`dynamic_cast`).
- `src/crs_to_crs.cpp` (`proj_create_crs_to_crs`):
  - grid availability 570–576: `KNOWN_AVAILABLE` when ONLY_BEST=YES or a warning is set,
    otherwise `DISCARD_OPERATION_IF_MISSING_GRID`.
  - one operation, 617–647: with ONLY_BEST=YES and a missing grid, `pj_warn_about_missing_grid` is
    printed and creation returns NULL (629–641).
  - rerun with DISCARD, 648–685 and 685–773.
- `src/trans.cpp`:
  - `pj_warn_about_missing_grid` 179–211: one line, `This might become an error` only when no
    error is set. The DEBUG level is printed when `PROJ_DEBUG>=2`.
  - per-point retry 233–303 (`alternativeCoordinateOperations`).
- `src/apps/cs2cs.cpp` 981–999: `ONLY_BEST=YES|NO` then `cannot initialize transformation`, exit 3.

## The guard in Rust

`vertical_geog_fallback` (`src/iso19111/compound.rs`, 1152 on `d6fc3d2`) returns early when the
geographic CRS has an identifier, unless the vertical CRS is not in metres. PROJ has no such test.
Removing it is the faithful port, and it is the only change that makes the PROJ candidate appear
for `EPSG:4326+5773` to `EPSG:4326+5782` (see the measurements).

## Measurements

The oracle is `projbuild/bin` (PROJ 9.9.0). The case is the grid-missing setup of the yaml test:
`proj.db` alone in a directory, `PROJ_DATA` set to it, `PROJ_NETWORK=OFF`. The two `proj.db` files
differ (oracle `refdata` against `data/proj.db`), so each side uses its own.

Oracle, `projinfo -s EPSG:4326+5773 -t EPSG:4326+5782 -o PROJ --spatial-test intersects`:

- one candidate: `Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1)
  using ETRS89-ESP [REGENTE] to WGS 84 (1)`, 3.05 m, `Spain - mainland onshore.`, at least one grid
  missing; the PROJ string is the four-step pipeline with `us_nga_egm96_15.tif` and
  `es_ign_egm08-rednap.tif`.
- `--grid-check discard_missing` gives one candidate: `Transformation from EGM96 height to Alicante
  height (ballpark vertical transformation)`, World.

Ours after the guard port, same database: the same candidate name, accuracy 3.05 and PROJ string
(`projinfo` ignores `--grid-check`, so the discard list is not shown; the missing-grid suffix is not
printed by ours).

`cs2cs` (oracle), the yaml cases:

| Case | Oracle stdout | Oracle stderr | Exit |
|---|---|---|---|
| `#126` default | `39.00 -3.00 0.00` | `Attempt to use … Inverse of WGS 84 to EGM96 height (1) + … failed. Grid es_ign_egm08-rednap.tif … Grid us_nga_egm96_15.tif … This might become an error … (for the current transformation instance).` then `Using coordinate operation Transformation from EGM96 height to Alicante height (ballpark vertical transformation)` | 0 |
| `#127` `--only-best=no` | `39.00 -3.00 0.00` | none | 0 |
| `#128` `--only-best` | `*` | `Attempt to use … failed. Grid es_ign_egm08-rednap.tif … Grid us_nga_egm96_15.tif …` then `cannot initialize transformation` | 3 |

The numbers compare equal to ours (ours prints `39dE 3dS 0.000`, which the harness reads as the
same angles).

## Attempts

1. **Guard port and flatten rule** (`vertical_geog_fallback` without the identifier test, and
   `compute_metadata` not flattening a `ConcatenatedOperation` with `proj_based`, as
   `createComputeMetadata` does for a `SingleOperation` from `createPROJBased`). Result: `#126`
   projinfo matches the oracle. `#127` **regressed** (exit 3, `unknown CRS EPSG:4326+5773`): the
   registry geoid operation, whose grid is missing, stays in the candidate list, and the DISCARD
   fallback (the ballpark) is never reached. The flatten rule alone has no observable effect.
2. **Attempt 1, plus DISCARD and the single-candidate refusal:**
   - `GridAvailabilityUse::DiscardIfMissing` (the registry drops an operation whose grid is not on
     disk, `findOpsInRegistryDirect*`), and `OnlyBest::Discard` for `--only-best=no` only, so that
     the facade keeps `OnlyBest::Off`;
   - `BestOperation::Refused` when ONLY_BEST=YES leaves one candidate with a missing grid
     (`crs_to_crs.cpp` 617–641), and `cs2cs` ends the creation with the `cannot initialize`
     block (`InitError::Grid`);
   - the grid names of the message sorted and de-duplicated by the name the pipeline carries
     (`gridsNeeded` is a `std::set`, 1 name per grid). The official name `WW15MGH.GRD` sorts before
     `es_…`, so the sort must follow the PROJ name `us_nga_egm96_15.tif`.

   Result: `#128` closes (stderr and exit 3 equal to the oracle). `#127` passes. The CLI ledger has
   no reopened case. `#126` is unchanged (stdout equal, stderr empty). **Unit reopened:**
   - `tests/compound_operation.rs:74`, `compound_pair_with_the_same_geographic_part_keeps_the_ballpark`
     (`CoordinateOperationContext` with authority `EPSG`). The list for `EPSG:4326+5773` →
     `EPSG:4326+5782` is now the geoid operation alone, which is the oracle's list (one candidate).
     The test asserts a ballpark candidate, which PROJ does not list in that search.
   - `tests/unit_opfactory_core_b.rs:102`, `operation_compound_crs_with_bound_geog_crs_to_geog_crs`.
     The fallback now runs for the identified `EPSG:4979` target, and a registry operation from an
     ETRS89 (GRS80) geographic CRS comes first (`+inv +proj=cart +ellps=GRS80`). PROJ's list for the
     same pair (oracle `projinfo`, 33 candidates, grid missing) has ballpark-led operations first,
     and the test's expected operation is not in the first place of PROJ's list either.
   - `tests/proj_facade.rs:95` failed only with the shared `Off` mode. The split above fixes it.

   Both unit failures come from the faithful guard port. The rule "zero reopened cases" forbids
   keeping them, and their expectations are not PROJ's list. Reverted. The complete change set is
   kept as a patch outside the repository (scratchpad `lot/attempt-final-guard-discard-refusal.patch`)
   for the maintainer's decision on those two expectations.

## Open items and causes

- **`#128` needs the guard port and the refusal** (attempt 2). It is blocked by the two unit
  expectations above, which need a decision (PROJ's list is the reference for both).
- **`#126` stderr**: two parts are missing, and both are larger than this lot:
  - `select_alternatives` (`src/iso19111/select.rs`) rejects a compound CRS: `reference_base` returns
    `None`, so cs2cs never takes the per-point alternatives for `EPSG:4326+5773`. The legacy path
    returns one `BestOperation`. PROJ's `pj_create_prepared_operations` (`crs_to_crs.cpp` 362–466)
    builds the source and target legs of a compound CRS too.
  - The PROJ debug diagnostics: `pj_warn_about_missing_grid` at `PROJ_DEBUG>=2` (one line, with the
    `This might become an error` text) and `Using coordinate operation …` (`trans.cpp` 269–273). No
    such text exists in `src/` (checked by grep). Only `#126` of the yaml uses them.
- **`--only-best` and `--only-best=no` for the legacy path**: `OnlyBest::Discard` is new in
  attempt 2, and `cs2cs` maps `Some(false)` to it. The default (no option) stays `Off`.
- **Facade**: `Proj::new_known_crs` keeps `OnlyBest::Off`, so `EPSG:4267` to `EPSG:4269` still
  returns an error when the grid is missing (`tests/proj_facade.rs:95`). PROJ's `proj_create` with
  the default warning policy would fall back here; that is a facade contract, not this lot.
- **Docs after attempt 2**: `docs/UNPORTED-CLI.md` loses `#128`, so `README.md`, `ROADMAP.md` and
  `docs/progress.svg` would need `UPDATE_DOCS=1 cargo test --test docs_in_sync` and
  `UPDATE_DOCS=1 cargo test --test progress_svg`. Reverted, so nothing to regenerate here.

## Commands

```bash
# Oracle, grid-missing setup of test_cs2cs_various.yaml#126
PROJ_DATA=<oracle-dir with proj.db> PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF \
  <projbuild>/bin/projinfo -s EPSG:4326+5773 -t EPSG:4326+5782 -o PROJ --spatial-test intersects
echo "39 -3 0" | PROJ_DATA=<oracle-dir> PROJ_DEBUG=2 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  PROJ_NETWORK=OFF <projbuild>/bin/cs2cs EPSG:4326+5773 EPSG:4326+5782
echo "39 -3 0" | PROJ_DATA=<oracle-dir> PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF \
  <projbuild>/bin/cs2cs --only-best EPSG:4326+5773 EPSG:4326+5782

# Ours, same setup
cargo test --test cli_yaml            # the ledger; git checkout docs/UNPORTED-CLI.md before re-measuring
cargo test --test compound_operation
cargo test --test unit_opfactory_core_b operation_compound_crs_with_bound_geog_crs_to_geog_crs
```
