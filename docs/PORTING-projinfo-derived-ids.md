# Port — identifiers of modified operations (`DERIVED_FROM(...)`, `INVERSE(...)`)

Lot: the identifiers PROJ 9.9.0 gives an operation it copied and changed. Cases closed in
`tests/cli_yaml/test_projinfo.yaml`: #13, #14, #25, #27, #38, #39. No case reopened.

Reference: PROJ 9.9.0 sources (tag 9.9.0), `src/iso19111/`. Oracle: the real `projinfo` 9.9.0
(`reftest/build/bin/projinfo`), run with `PROJ_DATA=<reftest>/build/data:$PWD/data/tests`.

## What PROJ does

| Step | PROJ | Port |
|---|---|---|
| `addModifiedIdentifier`: `DERIVED_FROM(auth)` and `INVERSE(...)` on each identifier | `operation/oputils.cpp:177` | `buildop.rs` `inverse_code_space` (existing); `proj_grid_substitution.rs` `similar_operation` |
| `createPropertiesForInverse(op, derivedFrom)` | `operation/oputils.cpp:231` | inverse of a substituted grid: `gtiff_horizontal` with `inverted` |
| `createSimilarPropertiesOperation`: name, remarks, domains, `DERIVED_FROM` ids, no version | `operation/singleoperation.cpp:2305` | `similar_operation` |
| `createSimilarPropertiesMethod`: name and identifier of the method | `operation/singleoperation.cpp:2349` | `geoid_model`, `geocentric_grid` |
| `substitutePROJAlternativeGridNames` | `operation/singleoperation.cpp:2402` | `substitute_proj_alternative_grid_names` |
| horizontal branch (NADCON pair, NTv1/NTv2 names, GTiff method) | `singleoperation.cpp:2411-2530` | `horizontal_grid`, `gtiff_horizontal` |
| geoid file of `Geographic3D to GravityRelatedHeight` | `singleoperation.cpp:2540-2577` | `geoid_model` |
| geocentric translation grid, method 1087 | `singleoperation.cpp:2599-2636` | `geocentric_grid` |
| candidate path with `usePROJAlternativeGridNames` | `factory.cpp:6349-6356`; `coordinateoperationfactory.cpp:1928-1941`, `:1998-2011` | `operations_between` (grids from the context), `with_proj_grid_names` |
| `projinfo` default `usePROJGridAlternatives = true`, `--no-proj-grid-alternatives` | `apps/projinfo_lib.cpp:1111`, `:1558`, `:1732` | `src/bin/projinfo.rs` |
| `getPROJ4NadgridsCompatibleFilename`, GTiff method | `singleoperation.cpp:1998-2031` | `bound.rs` `nadgrids_filename` |
| `_getHorizontalShiftGTIFFFilename` and its export | `singleoperation.cpp:2061`, `:4322-4380` | `transform_export.rs` `horizontal_grid_shift`, `is_gtiff_method` |
| `createOperationsFromDatabase`: geoid first with early return; promoted 3D registry search | `coordinateoperationfactory.cpp:4259-4276`, `:4359-4387` | `create_candidates` (geoid), `promoted_registry_vertical` |
| `createOperationsVertToGeogSynthetized` (ballpark change of unit) | `coordinateoperationfactory.cpp:6113-6203` | `compound::vertical_geog_synthesized` |
| `createOperations` dispatch for a vertical source; reversed search for a geographic source | `coordinateoperationfactory.cpp:3748-3758`, `:6070-6109` | `create_operations` |

## What changed

1. `src/iso19111/proj_grid_substitution.rs` (new): the substitution, in the three ported branches
   (horizontal GTiff, geoid, geocentric), and the `proj.db` lookup
   `DatabaseContext::proj_grid_alternative_format` (with the format column that
   `lookForGridAlternative` reads).
2. `factory.rs`: `CoordinateOperationContext::set_alternative_grid_names` (default `Official`).
   `operations_between` takes the grids of the context. `create_coordinate_operation_typed` applies
   the substitution to `grid_transformation` and `other_transformation` when the grids are `Proj`:
   built with official names, substituted, else built with the earlier file rename.
3. `factory.rs` `create_candidates`: the geoid route is read first (early return), then the registry,
   the promoted 3D search (`promoted_registry_vertical`), the fallbacks.
4. `factory.rs` `create_operations`: the ballpark change of unit of `vertical_geog_synthesized` is
   appended for a vertical source, and for a geographic source only when the reversed search found
   nothing (PROJ returns the reversed search early, `:4274-4276`).
5. `transform_export.rs`: GTiff methods are exported by name (`hgridshift`, `gridshift` for
   `GENERAL_SHIFT_GTIFF`).
6. `bound.rs`: `nadgrids_filename` for the GTiff method.
7. `compound.rs`: `vertical_geog_synthesized`, `is_synthesized_ballpark`; the vertical list of
   `compound_to_geog3d` is `createOperations(vertical, target)` alone, and
   `projected_compound_to_geog` keeps that list when it holds only ballparks (see below).
8. `src/bin/projinfo.rs`: `--no-proj-grid-alternatives` sets `Official`; the default is `Proj`.

## Finding: the ballpark of a compound CRS (replaces an outermost-only guard)

An earlier version gated the ballpark dispatch with a guard bit, so that only the outermost
`createOperations` appended it. That was not a PROJ rule. Removing the guard and tracing
`unit_compound_axis_swap.rs` (its WKT, `-t EPSG:4979`, with the traced PROJ build) showed:

- The nested call that PROJ makes for the vertical part, `createOperations(vertical, targetCRS)` in
  `createOperationsCompoundToGeog` (`coordinateoperationfactory.cpp:6782-6785`, under
  `SetSkipHorizontalTransform`), does produce the ballpark: the trace has
  `createOperationsVertToGeog` then `createOperationsVertToGeogSynthetized` inside it.
- PROJ keeps every vertical transformation of that list. The later step only looks for interpolation
  CRS candidates (`:6847-6919`, which found none here), then combines the lists
  (`:6921-6924`). A list of ballparks alone does not give an empty result.
- Our `projected_compound_to_geog` took the first registered (non-ballpark) vertical operation, and
  when there was none and the list was not empty it returned no result. The guard kept the list
  empty, so the fallback `vertical_ballpark` built the same operation and the test passed. Without the
  guard the list held the ballpark and the early return dropped it. The port now takes the first
  operation of the list, as PROJ keeps the whole list.
- `compound_to_geog3d` also appended a second copy of the ballpark to that list. Removed: the list is
  PROJ's `verticalTransforms` alone.

The trace line `TRACE_BRANCH_ONLY_VERT_BALLPARK` is in the instrumented copy of the sources, not in
PROJ 9.9.0. The conditions above are read from the pristine 9.9.0 sources.

The first ballpark of the list is taken, where PROJ loops over all of them. No test or table row
has two ballparks for one pair.

## Deviations (stated, not hidden)

- **Default context stays `Official`.** PROJ's `CoordinateOperationContext` defaults to
  `usePROJNames_ = true` (`operation/coordinateoperationfactory.cpp:175`). The library default is kept
  as `Official` so that cs2cs and the API do not change; `projinfo` sets `Proj`, as its CLI does.
  To be decided with the cs2cs lot.
- **Formats.** `proj.db` has `GTiff`, `GPKG` and `JSON` rows for grid alternatives, and no `NTv1`,
  `NTv2` or `CTable2` row. The NTv1/NTv2/CTable2 branches are not ported. A GPKG or JSON row falls
  through, as PROJ does.
- **Inverse grid direction.** One row reads a grid in the inverse direction (`rgf93_ntf.gsb`, used
  by `EPSG:15940`). The branch builds the operation as PROJ does and sets `inverted`. No yaml case or
  table row exercises it; `EPSG:4275 -> EPSG:4171` uses `EPSG:9327` (geocentric branch) instead.
- **Velocity and regular vertical grid branches.** Not ported (point motion is a separate lot). The
  file rename of the earlier code stays for those grids.
- **Same-datum vertical pairs.** `createOperationsVertToGeogSynthetized` builds a change-of-unit
  conversion for the same datum (`:6191-6195`). `vertical_ballpark` leaves that case out, as before.
- **Name-based vertical search.** `createOperationsVertToGeog` looks up a vertical CRS without
  identifiers by name first (`:6081-6105`). Not ported: the synthesised change of unit is used.
- **`hasPerfectAccuracyResult` early returns** of `createOperationsFromDatabase` (`:4351`, `:4577`)
  are not modelled in the vertical path.

## Oracle table

"Before" is the lot's baseline bca19c5. The table was measured again after the rebase onto
9a8bdb0 and after the guard was removed; it did not change.

`table.py` runs the real `projinfo` and ours on each invocation. "Normalised" removes the two text
gaps of the grid-availability lot (`, at least one grid missing`, `needed but not found`). A `DIFF`
row whose identifier count is 0 differs in blank lines or text only (checked line by line). "ids"
counts the lines whose identifier part differs.

| Invocation | Before (normalised, ids) | After (normalised, ids) | Oracle / ours candidates |
|---|---|---|---|
| `-s NAD27 -t NAD83 --summary` (#13) | DIFF, 7 | SAME, 0 | 9 / 9 |
| `-s NAD27 -t NAD83 -o PROJ` (#14) | DIFF, 7 | DIFF, 0 (blank lines) | 10 / 10 |
| `-s EPSG:4267 -t EPSG:4269 -o PROJ` | DIFF, 7 | DIFF, 0 (blank lines) | 10 / 10 |
| `-s EPSG:4267 -t EPSG:4269 --summary` | SAME | SAME | 0 / 0 |
| `-s EPSG:4267 -t EPSG:4326 -o PROJ` | SAME | SAME | 1 / 1 |
| `-s EPSG:4269 -t EPSG:4326 --summary` | SAME | SAME | 1 / 1 |
| `-s EPSG:4283 -t EPSG:4326 --summary` | SAME | SAME | 2 / 2 |
| `-s EPSG:4283 -t EPSG:4326 -o PROJ` | SAME | SAME | 2 / 2 |
| `-s EPSG:4258 -t EPSG:4326 --summary` | SAME | SAME | 1 / 1 |
| `-s EPSG:4275 -t EPSG:4326 -o PROJ` | DIFF, 1 | SAME, 0 | 2 / 2 |
| `-s EPSG:4807 -t EPSG:4326 --summary` | DIFF, 0 (`2.0 m` vs `2 m`) | DIFF, 0 (same text gap) | 1 / 1 |
| `-s EPSG:4807 -t EPSG:4326 -o PROJ` | DIFF, 0 (same text gap) | DIFF, 0 (same text gap) | 1 / 1 |
| `-s EPSG:4283 -t EPSG:5711 -o PROJ` | DIFF, 49 (1 vs 49 candidates) | SAME, 0 | 1 / 1 |
| `-s EPSG:4939 -t EPSG:5711 --summary` | DIFF, 1 (0 candidates) | DIFF, 1 (0 candidates; see open items) | 1 / 0 |
| `-s EPSG:5613 -t EPSG:4977 -o PROJ` (#25) | DIFF, 2 | DIFF, 0 (blank lines) | 2 / 2 |
| `-s EPSG:4977 -t EPSG:5613 --summary` | DIFF, 1 | SAME, 0 | 1 / 1 |
| `-s EPSG:5720 -t EPSG:4965 --summary` (#27) | DIFF, 2 (1 vs 2 candidates) | SAME, 0 | 2 / 2 |
| `-s EPSG:4965 -t EPSG:5720 --summary` | DIFF, 1 | SAME, 0 | 1 / 1 |
| `-s GDA2020 -t "AHD height" -o PROJ` (#39) | DIFF, 49 | SAME, 0 | 1 / 1 |
| `-s EPSG:4326 -t EPSG:4979 --summary` (same datum) | DIFF, 0 (`World.` period) | DIFF, 0 (same text gap) | 1 / 1 |
| `-s EPSG:4326 -t EPSG:3857 --summary` | SAME | SAME | 1 / 1 |
| `-s EPSG:2154 -t EPSG:4326 -o PROJ` | SAME | SAME | 1 / 1 |
| `-s EPSG:31370 -t EPSG:4326 -o PROJ` | SAME | SAME | 3 / 3 |
| `-s EPSG:3035 -t EPSG:4258 --summary` | SAME | SAME | 1 / 1 |
| `-s EPSG:4807 -t EPSG:4171 --summary` | DIFF, 1 (3 vs 2 candidates) | DIFF, 1 (same) | 3 / 2 |
| `-s EPSG:4807 -t EPSG:4171 -o PROJ` | DIFF, 1 (3 vs 2) | DIFF, 1 (same) | 3 / 2 |
| `-s EPSG:4171 -t EPSG:4807 -o PROJ` | DIFF, 1 (3 vs 2) | DIFF, 1 (same) | 3 / 2 |
| `-s EPSG:4275 -t EPSG:4171 -o PROJ` | DIFF, 2 | DIFF, 0 (blank lines) | 3 / 3 |

Summary of the 28 rows: 10 are SAME before and after; 7 went from DIFF to SAME; 11 are still DIFF.
No row went from SAME to DIFF. Before, 15 rows had identifier differences; after, 4 do, and each of
those 4 is a missing candidate (the prime-meridian ballpark of `EPSG:4807 -> EPSG:4171`, three rows,
and the containment retry of `EPSG:4939 -> EPSG:5711`), not a wrong identifier text. The other
still-DIFF rows differ in blank lines or text only: the blank-line gap, the `2.0 m`/`2 m` accuracy
text, and the `World.` period.

## Open items (not in this lot)

- `EPSG:4807 -> EPSG:4171`: the oracle has a third candidate, the ballpark prime-meridian operation
  `Transformation from NTF (Paris) to NTF (Paris) altered to use prime meridian ...`. Ballpark lot.
- `EPSG:4939 -> EPSG:5711` (default `--spatial-test contains`): the oracle retries with the
  intersection test and prints `DERIVED_FROM(EPSG):8451`; ours keeps the strict result (0 candidates)
  and prints a different note. `projinfo_lib.cpp` retry (`:1123-1140`), not the identifiers.
- Accuracy text: the oracle prints `2.0 m`, `1.0 m` where the database stores `2.0`, `1.0`; ours
  prints `2 m`, `1 m`. Summary and PROJ-output lot.
- The blank line after a PROJ string in `-o PROJ` output and the `World.` period of summaries are
  the known text gaps of the output lot.
- `cs2cs` and the library default of `CoordinateOperationContext` (`Official` here, `Proj` in PROJ).

## Commands

```bash
cargo test --test cli_yaml                           # before and after each criterion
# exact text of one case, ours against the yaml (the test's own comparison):
PROJ_DATA=data:data/tests target/debug/projinfo -s EPSG:4275 -t EPSG:4171 -o PROJ
# oracle, same arguments, with the reference build on PROJ_DATA:
PROJ_DATA=<reftest>/build/data:data/tests <reftest>/build/bin/projinfo -s EPSG:4275 -t EPSG:4171 -o PROJ
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<reftest>/build/bin:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```
