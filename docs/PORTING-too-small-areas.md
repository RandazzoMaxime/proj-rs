# Porting notes: intermediate CRS when the direct candidates are too small

Reference: PROJ 9.9.0. Sources:
`src/crs_to_crs.cpp` (`proj_create_crs_to_crs_from_pj`, the grid rerun at lines 685-750),
`src/iso19111/operation/coordinateoperationfactory.cpp` (`hasTooSmallAreas` at 3897-3958,
its call at 4469-4490, `createOperationsWithDatumPivot` at 3167).

Scope: `tests/cli_yaml/test_cs2cs_various.yaml` ids 125, 132 and 133
(EPSG:4746 to EPSG:4978 and back, with the BETA2007 grid missing; the GH-3613 NAD27 case).

## Finding: the intermediate search that closes 132 and 133 is the grid rerun

The lot brief said that PROJ adds intermediate-CRS candidates when the direct candidates'
areas are too small (`hasTooSmallAreas`). That term is real, but it does not close these
cases:

- EPSG:4746 (PD/83) has the area of use "Germany - Thuringen", the same box as the BETA2007
  operation. The operation covers its own source extent, so it is not "too small"
  (area > 0.5 x target area). The term stays false.
- The candidate that cs2cs falls back to, `PD/83 to ETRS89 (1) + ETRS89 to WGS 84 (1)`,
  comes from the datum-based search, which runs when the registry has no candidate left
  after the grid check. That needs the grid rerun below.

What PROJ does, in `proj_create_crs_to_crs_from_pj` (`crs_to_crs.cpp:685-750`): when no
instantiable candidate that is not a ballpark exists, it calls `proj_create_operations`
again with `DISCARD_OPERATION_IF_MISSING_GRID` (lines 691-720), prepares that list
(line 724 onward), and keeps the first list's non-ballpark operations followed by the
second list (lines 733-750). With BETA2007 missing, the registry drops the BETA pivot, the
pivot is empty, and the datum-based intermediate through ETRS89 is found.

## What changed

| Where | PROJ | Rust |
|---|---|---|
| `src/iso19111/select.rs`, `select_alternatives`, the grid rerun | `crs_to_crs.cpp:685-750`: a second `proj_create_operations` with `DISCARD_OPERATION_IF_MISSING_GRID` | The rerun now calls `CoordinateOperationFactory::create_operations` with `GridAvailabilityUse::DiscardIfMissing`. Before, it filtered the first list with `discard_missing`, which never removed the pivot that the search would have replaced. |

The change is one search, not a filter: the `kept` list (non-approximate first-list operations)
and the `discard_missing` ranking of the second list are unchanged.

## Deviations

- A failing second search keeps the first list (`unwrap_or_default`), as PROJ keeps
  `preparedOpList` when `op_list` is null (`crs_to_crs.cpp:724`). The error text is not
  printed.
- The second search reuses the first search's context (spatial criterion, CRS extent use,
  ballpark setting). `crs_to_crs.cpp:700-716` also sets the area of interest; the cs2cs path
  here sets none, so the two are the same.

## `hasTooSmallAreas`: investigated, not in this change

The term is PROJ's `hasTooSmallAreas` (`coordinateoperationfactory.cpp:3897-3958`), gated in
`createOperationsGeodToGeod` at 4469-4490 by `!resFindDirectNonEmptyBeforeFiltering`, not
NEVER, and the anti-recursion guards. A port was written and measured:

- CLI suite: no row changes on its own.
- 29 targeted pairs and 200 random geographic pairs (both with and without the grid): no
  output changes.

Without a case where the term changes an output, the port has no oracle-backed test, so it
is not part of this change. It is kept outside the tree and can be ported once a witness
pair exists.

## Oracle comparison

Oracle: PROJ 9.9.0 `cs2cs -d 8` and `projinfo` (`--pivot-crs if_no_direct_transformation`,
`--grid-check known_available` for the first list, `--grid-check discard_missing` for the
second). "Full" uses `data/tests` (grids present); "nogrid" uses a directory holding only
`proj.db`.

Changed rows (every other targeted pair has the same output before and after; the one that
still differs is EPSG:4284 to EPSG:4258, below):

| Pair | Point (lat lon) | nogrid before | nogrid after | Oracle (nogrid) |
|---|---|---|---|---|
| EPSG:4746 to EPSG:4978 | 50.5 10 | 4003345.54 705897.83 4898352.56 | 4003461.37 705832.04 4898267.94 | 4003461.37 705832.04 4898267.94 |
| EPSG:4746 to EPSG:4326 | 50.5 10 | 50.5 10.0 | 50.49880412 9.99880337 | 50.49880412 9.99880337 |
| EPSG:4267 to EPSG:4269 | 40 -100 | 40.0 -100.0 | 39.99999688 -100.00041559 | 39.99999688 -100.00041559 |

Agreement with the oracle on cs2cs outputs:

| Sample | Before | After |
|---|---|---|
| 29 targeted pairs, full | 28 / 29 | 28 / 29 |
| 29 targeted pairs, nogrid | 25 / 29 | 28 / 29 |
| 200 random geographic pairs, full | 198 / 200 | 198 / 200 |
| 200 random geographic pairs, nogrid | 198 / 200 | 198 / 200 |

The one unchanged miss in each set is EPSG:4284 to EPSG:4258 (point 55 37): the oracle moves
the point by about 0.0002 degrees, our result is the identity. It is the same before and after
this change, and comes from the candidate list (`projinfo` lists 6 candidates where PROJ lists 9
under the default). Not in this lot.

## Test results

- `tests/cli_yaml`: cases 125, 132 and 133 close. No row reopens (`docs/UNPORTED-CLI.md`
  loses exactly those three rows).
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` regenerates `README.md`,
  `ROADMAP.md` and `docs/progress.svg` (counts only).

## Commands

```bash
# oracle comparison (outputs recorded in the scratch folder, not committed)
cs2cs -d 8 EPSG:4746 +to EPSG:4978   # PROJ_DATA=<dir holding only proj.db>
projinfo -s EPSG:4746 -t EPSG:4978 --pivot-crs if_no_direct_transformation --grid-check discard_missing --spatial-test intersects --summary
cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Not done

- No benchmark case was added to `src/bin/bench.rs`: the change adds a second search only for
  pairs whose first list has no instantiable non-ballpark candidate.
- The `ONLY_BEST` and `proj.ini` paths of `select.rs` were not changed (another lot works on
  them). The rerun applies to the `Warn` and `Error` policies of `select_alternatives`, not to
  `Discard` (`ONLY_BEST=NO`), which keeps the filter of the first list as before.
