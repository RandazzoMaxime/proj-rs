# Port — `test_cs2cs_various.yaml#139` (EPSG:3912+EPSG:5779 to EPSG:3794+EPSG:8690, `-d 3`)

Status: **#139 stays open.** This lot lands two rules and closes two CLI rows:

1. `projinfo --grid-check` sets the grid policy (`test_projinfo.yaml#37`, `#68` close).
2. A derived source keeps its registry candidates and appends the chained ones (`EPSG:3912 ->
   EPSG:3794` and `EPSG:23031 -> EPSG:25831` now list PROJ's candidates by name and order).

The yaml point still gives `477131.509 95651.636` (PROJ `476763.303 95620.222`). The cause is in
the selection of `cs2cs`, and the horizontal candidates that PROJ builds for the compound pair are not
ported yet (see "Compound pair").

## Rule 1: `projinfo --grid-check`

PROJ 9.9.0 `src/apps/projinfo_lib.cpp:1550-1553` (default `USE_FOR_SORTING`), `1769-1788` (`none` ->
`IGNORE_GRID_AVAILABILITY`, `discard_missing` -> `DISCARD_OPERATION_IF_MISSING_GRID`, `sort` ->
`USE_FOR_SORTING`, `known_available` -> `KNOWN_AVAILABLE`). The port's `src/bin/projinfo.rs` read the
option and dropped it. It now sets `set_grid_availability_use`, as `candidate_operations` already set the
other context options.

Closed: `test_projinfo.yaml#37` (`--grid-check none`), `#68` (`--grid-check known_available`). No row
reopened.

## Rule 2: the derived source keeps its registry candidates

PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp`:

| PROJ 9.9.0 | This port |
|---|---|
| `createOperations`, 3652-3656: `createOperationsFromDatabase` fills `res` with the registry candidates | `create_candidates`, the direct registry operations (`direct_registry_operations`) |
| 3694-3701: `if (derivedSrc)` appends `createOperationsDerivedTo` to `res` and returns | `create_candidates`, the block before `if found.is_empty()` (`factory.rs`), `derived::derived_candidates` |
| 4351, 4577: `hasPerfectAccuracyResult` makes `createOperationsFromDatabase` return true (no derived branch) | `has_perfect_accuracy_result` (unchanged) |
| 5523-5575 `createOperationsDerivedTo`: inverse of the deriving conversion, chained with `createOperations(base, target)` | `derived::derived_to` (unchanged) |
| 1731 `filterAndSort`, `FilterResults::sort` (`967-972` `gridsAvailable`, `975-980` `gridsKnown`), `removeUninterestingOps` (`1608-1658`) | `select::sort`, `factory::filter_and_sort` (the grid policy now reaches them) |

Before the rule, the derived branch ran only when the registry gave nothing (`if found.is_empty()`).

### Evidence from the traced PROJ 9.9.0 build

The trace (`$L/projtrace-build/bin/projinfo`, `PROJ_DEBUG=3`, same sources as tag 9.9.0 plus
`TRACE_CREATE_OPERATIONS`) shows that **PROJ enters `createOperationsDerivedTo` for all three pairs**,
including `test_projinfo.yaml#58`:

| Pair | Registry result (`findOpsInRegistryDirect`) | `createOperationsDerivedTo` | `filterAndSort` (in -> out) |
|---|---|---|---|
| `EPSG:3912 -> EPSG:3794` | 24 | entered | 36 candidates |
| `EPSG:23031 -> EPSG:25831` | 1 (`EPSG:5166`) | entered | 15 -> 11 |
| `KKJ / Finland Uniform -> ETRS89 / TM35FIN(E,N)` (#58) | 1 (`EPSG:10703`) | entered | 5 -> 2 |

So there is no condition that skips the derived branch for #58. The difference is in the sort and the
filter. With `--grid-check none` (the yaml case), PROJ sorts `EPSG:10703` (0.03 m) first and keeps the
derived `Inverse of Finland Uniform ... + KKJ to ETRS89-FIN [EUREF-FIN] (2) + TM35FIN` (0.5 m): the
output is `EPSG:10703` as the first candidate, so `-o PROJ` gives the tinshift pipeline. The earlier
gate on grids was a stand-in for the grid policy, which the port did not read.

### What remains different for #58

The port lists 4 candidates against PROJ's 2 under `--grid-check none`. The two extra ones are the
`Ballpark geographic offset` concatenation and a three-times-TM35FIN concatenation (not exportable).
`removeUninterestingOps` (`coordinateoperationfactory.cpp:1608-1658`) removes a candidate with more
steps, equal or worse accuracy and the same extent as the last one kept; PROJ does remove them, the port
does not. The likely cause is the equality of the extents of the derived candidates (`sameExtent`), not
traced yet. The yaml case passes, because `-o PROJ` takes the first candidate.

## Oracle tables

Candidates: `projinfo -s S -t T --summary --spatial-test intersects` (default grid policy), PROJ 9.9.0
on the grid-less copy of `proj.db` (`scratchpad/oracle-db`) against this port on `data/` only.
A pair is SAME when the list of names, in order, is identical.

| Label | Build | Identical to PROJ (of 44) |
|---|---|---|
| `origin/main` (`d8dbb42`) | before the lot | 32 |
| this lot (rules 1 and 2) | | 35 |

Pairs changed by rule 2 (all three now SAME):

| Pair | before (PROJ) | after (PROJ) |
|---|---|---|
| `EPSG:3912 -> EPSG:3794` | 24 (36) | 36 (36) |
| `EPSG:3794 -> EPSG:3912` | 24 (36) | 36 (36) |
| `EPSG:23031 -> EPSG:25831` | 1 (11) | 11 (11) |

No other pair changed. The 32 pairs that match before the lot still match:
`EPSG:26915 -> EPSG:26715`, `EPSG:26715 -> EPSG:26915`, `EPSG:4267 -> EPSG:26915`,
`EPSG:4269 -> EPSG:26710`, `EPSG:32631 -> EPSG:32632`, `EPSG:27700 -> EPSG:4258`,
`EPSG:2154 -> EPSG:4326`, `EPSG:3035 -> EPSG:4258`, `EPSG:25832 -> EPSG:4258`,
`EPSG:3006 -> EPSG:4258`, `EPSG:31467 -> EPSG:4258`, `EPSG:2056 -> EPSG:4258`,
`EPSG:27200 -> EPSG:4326`, `EPSG:3857 -> EPSG:4326`, `EPSG:2039 -> EPSG:4258`,
`EPSG:31370 -> EPSG:4258`, `EPSG:2180 -> EPSG:4258`, `EPSG:2169 -> EPSG:4258`,
`EPSG:2927 -> EPSG:4269`, `EPSG:26910 -> EPSG:4269`, `EPSG:26917 -> EPSG:4269`,
`EPSG:4746 -> EPSG:4230`, `EPSG:4312 -> EPSG:4230`, `EPSG:4267 -> EPSG:4269`,
`EPSG:4746 -> EPSG:4978`, `EPSG:11108 -> EPSG:11128`, `EPSG:4326 -> EPSG:4258`,
`EPSG:2393 -> EPSG:3794`, `EPSG:3912 -> EPSG:32633`, `EPSG:27700 -> EPSG:2154`,
`EPSG:2169 -> EPSG:2180`, `EPSG:3416 -> EPSG:31370`.

Still different from PROJ (default list; counts PROJ / port):

| Pair | PROJ | port | Cause |
|---|---|---|---|
| `EPSG:3912+EPSG:5779 -> EPSG:3794+EPSG:8690` | 26 | 26 | the horizontal legs (compound, below); names and order differ |
| `EPSG:2393 -> EPSG:4258`, `EPSG:4258 -> EPSG:2393` | 3 | 3 | names differ; the same family as #58 (`removeUninterestingOps`, extents), not traced further |
| `EPSG:3416 -> EPSG:4258` | 2 | 1 | not traced |
| `EPSG:4284 -> EPSG:4258` | 9 | 6 | see `PORTING-pivot-candidate-set.md` |
| `EPSG:4230 -> EPSG:4978` | 36 | 36 | names or order differ (see `PORTING-pivot-candidate-set.md`) |
| `EPSG:4269 -> EPSG:4978` | 54 | 53 | not traced |
| `EPSG:4258 -> EPSG:4978` | 2 | 1 | not traced |
| `EPSG:4230 -> EPSG:4258` | 17 | 15 | see `PORTING-pivot-candidate-set.md` |

The candidate names also differ in the extents of the derived candidates: PROJ prints
`Slovenia - onshore.` where the port prints `unknown domain of validity` for the 12 derived legs of the
compound pair (the extent of a concatenation is the intersection of the steps' extents,
`ConcatenatedOperation::createComputeMetadata`, not reproduced for the vertical leg). That changes the
`pj_get_suggested_operation` box for points outside Slovenia; it does not change the yaml point.

## Compound pair `#139`

Oracle, PROJ 9.9.0, `EPSG:3912+EPSG:5779 -> EPSG:3794+EPSG:8690`:

| Mode | Count | Content |
|---|---|---|
| default | 26 | 12 grid-missing `Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (n) + SVS2000 height to SVS2010 height (1) + Slovene National Grid`, one `Ballpark geographic offset` leg with the same vertical, and 13 ballpark-vertical legs |
| DISCARD (`--grid-check discard_missing`) | 37 | the 13 ballpark-vertical legs above, and 24 `D48/GK to D96/TM (n) + Transformation from SVS2000 height to SVS2010 height (ballpark vertical transformation)` |

The port gives 26 by default and 13 under DISCARD (the lot's measurement with the library). The
missing 24 are the horizontal registry legs combined with the ballpark vertical. In PROJ that is
`createOperationsCompoundToCompound` (`coordinateoperationfactory.cpp:7214-7847`) with different
horizontal parts. `compound.rs` does not port it (its doc: "Geographic parts that differ ... are not
ported. The result is empty"). This is the candidate rule that remains for `#139`.

## The yaml point

`cs2cs -d 3 EPSG:3912+EPSG:5779 EPSG:3794+EPSG:8690` at `477134.28 95134.21 5`:
PROJ `476763.303 95620.222 5.000`. `EPSG:3912 -> EPSG:3794` in 2D gives the same
`476763.303 95620.222` (`tests/cli_yaml/test_cs2cs_various.yaml`, the case just before). The port
`477131.509 95651.636`, the zero-offset `Ballpark geographic offset from MGI 1901 to ETRS89-SVN [D96]` leg.

The choice is made by `cs2cs` at run time, not by `projinfo`:
- `src/crs_to_crs.cpp:668-790`: the first list is kept only when no instantiable non-ballpark
  candidate exists; the rerun with `DISCARD_OPERATION_IF_MISSING_GRID` keeps the first list's
  non-ballpark legs and appends the rerun list (the port: `select_alternatives`, rerun branch).
- `src/trans.cpp:47-176` `pj_get_suggested_operation`, `233-346` `proj_trans` retries (the port:
  `Alternatives::suggest`, `Alternatives::forward`).

Not yet done: attributing the expected value to one leg. The pipelines written by hand (the zero-offset
leg and the MGI helmert leg) failed in `cct` ("Point outside of projection domain"), so the leg is not
identified.

## Tests

`tests/slovenia_candidates.rs` (3 tests, names from `projinfo` 9.9.0, not from this port):
- `d48_gk_to_slovene_grid_has_the_thirty_six_candidates_of_projinfo` (`EPSG:3912 -> EPSG:3794`, 36 names).
- `slovene_grid_to_d48_gk_has_the_thirty_six_candidates_of_projinfo` (`EPSG:3794 -> EPSG:3912`, 36).
- `ed50_utm31_to_etrs89_utm31_has_the_eleven_candidates_of_projinfo` (`EPSG:23031 -> EPSG:25831`, 11 names).

## Ratchet

`cargo test --test cli_yaml`: passes. Rows closed by this lot: `test_projinfo.yaml#37`, `#68`. No row
reopened. The cs2cs open rows stay `#96`, `#126`, `#139`.

## Commands

```bash
# traced PROJ 9.9.0 (sources: $L/projtrace-src, tag 9.9.0 plus TRACE_CREATE_OPERATIONS)
PROJ_DEBUG=3 PROJ_DATA=<scratch>/lot/refdata:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
    <scratch>/lot/projtrace-build/bin/projinfo -s "KKJ / Finland Uniform Coordinate System" \
    -t "ETRS89 / TM35FIN(E,N)" --spatial-test intersects --summary
# oracle: PROJ 9.9.0 projinfo on the grid-less copy of its proj.db
PROJ_DATA=<scratch>/oracle-db <scratch>/reftest/build/bin/projinfo -s EPSG:3912 -t EPSG:3794 \
    --summary --spatial-test intersects --grid-check discard_missing
# the port
PROJ_DATA=$PWD/data PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES target/debug/projinfo \
    -s EPSG:3912 -t EPSG:3794 --summary --spatial-test intersects --grid-check sort
cargo test --test slovenia_candidates
cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
```
