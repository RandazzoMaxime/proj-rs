# Port — projinfo datum pivot passes (lot E)

Source: PROJ 9.9.0, the tag `9.9.0` of OSGeo/PROJ. The PROJ line numbers below are of that tag.
The reference is PROJ 9.9.0 `projinfo` built from the same tag, run on the grid-less copy of
`proj.db` (`PROJ_DATA=<scratch>/oracle-db`) with `PROJINFO_NO_GRID_CHECK=YES`, as the yaml cases
are. The trace is PROJ built with `PROJ_DEBUG=3` (`<scratch>/lot/projtrace-build`).

Base: `lot-projinfo-pivots-3` (lot D). Branch: `lot-projinfo-datum-pivot-passes`.

## Result

| case | yaml text | state | closed by |
|---|---|---|---|
| `test_projinfo.yaml#88` | ETRF2005 to ETRF2014, 3D, through ITRF only, 2 candidates | closed | commit 2 (with commit 1) |
| `test_projinfo.yaml#89` | ETRS89-POL [PL-ETRF2000] to ETRF2014, 2 candidates | closed | commit 2 (with commit 1) |

The ledger `docs/UNPORTED-CLI.md` went from 12 projinfo rows to 10. No case that passed before
this lot fails after it: `cargo test --test cli_yaml` passes after each commit, and no row was
reopened. `cs2cs` rows are unchanged.

Both cases give the candidate list of projinfo 9.9.0 line for line (the `run.sh` check below).
Neither needs anything but the datum pivot: #88 is found by pass 1, and #89 by the same pass.

## Function map

| PROJ 9.9.0 (`src/iso19111/...`) | Rust (`src/iso19111/factory.rs`) |
|---|---|
| `createOperationsFromDatabase`, the datum pivot call (`coordinateoperationfactory.cpp:4404-4416`) | `create_candidates`, the `datum_pivot_operations` call |
| `createOperationsWithDatumPivot`, pass 0 (`:3452-3498`, `useOnlyDirectRegistryOp`) | `datum_pivot_operations`, first loop over `pairs` (direct registry) |
| `createOperationsWithDatumPivot`, passes 1 and 2, loop A (`:3455-3498`, named CRS, kind filter `:3464-3471`) | `datum_pivot_operations`, second loop, `iteration`, `kind`, named pairs |
| `createOperationsWithDatumPivot`, passes 1 and 2, loop B (`:3500-3536`, every other pair) | `datum_pivot_operations`, second loop, `linked_pair` for the other pairs |
| `createTransformations` (`:3219-3436`), the legs and `createComputeMetadata` | `linked_pair` (legs by `pivot_leg`), `pivot_concatenation` |
| `createOperationsFromDatabase`, the intermediate search after the pivot (`:4449-4450`, the search `:4458`) | `create_candidates`, `crs_codes_needed` (unchanged) |
| same, the block at `:4467-4515` (`bTriedThroughIntermediateCRS`, `findsOpsInRegistryWithIntermediate(..., true)`) | `create_candidates`, `intermediate_tried`, `datum_based_intermediate_operations` |
| `hasTooSmallAreas` (`:3897-3959`) | `has_too_small_areas` |
| `useOnlyReplacedOperations` (`:4031-4040`) | `use_only_replaced_operations` |
| `useOnlyIntermediateCRSThroughDatumEnsembleNullOp` (`:3979-4029`) | `only_ensemble_null_operations` (already ported) |
| `hasResultSetOnlyResultsWithPROJStep` (`:3136-3165`) | not ported (see Open) |

## Changes, one cause per commit

1. **The intermediate search after the datum pivot follows PROJ's conditions** (no yaml case
   changes). `createOperationsFromDatabase` runs `findsOpsInRegistryWithIntermediate(..., true)`
   after the pivot when the result is empty, when its operations are replaced, too small for the
   area of interest (`hasTooSmallAreas`), all time-dependent between static CRS, or ensemble null
   operations (`:4467-4515`). The port ran it only when the pivot was empty. When the area is too
   small, PROJ adds only the operations whose names are not already in the result (`oSetNames`).
   In the trace of EPSG:11108 to EPSG:11128, this search (`:4503`, line 498 of the trace) is what
   gives the ETRS89 candidate (`Inverse of ETRS89 to ETRS89-PRT [1995] + ETRS89 to ETRS89-ESP
   [ERGNSS]`). Commit 2 needs this block. Commit 1 alone keeps every case (the yaml ratchet and the
   pivot tests pass with the gate still there).

2. **The datum pivot's passes 2 and 3 run for identified CRS** (closes #88 and #89).
   `datum_pivot_operations` returned at once when both CRS had a registry identifier. PROJ does not
   have that gate: its passes 2 and 3 run whatever the identifiers, and return at the first result
   (`:3452-3541`). In PROJ's trace of #88, pass 0 (`iter=0`) finds nothing for any pair; pass 1
   (`iter=1`) takes the pair ETRF2005 (geocentric) to ETRF2014 (geocentric) through
   `findsOpsInRegistryWithIntermediate` (trace lines 437-499), and returns the two ITRF and ETRF2000
   candidates. No intermediate search runs at the top level after it. The port reached the same
   pass 1 result once the gate is gone: the ETRS89 candidate that came from the intermediate search
   no longer appears.
   Without commit 1, this change alone fails `pivot_candidate_set` (EPSG:11108 loses its ETRS89
   candidate), `concat_accuracy`, `unit_op_itrf2000_etrs89` and `unit_opfac_core_c_pivot2`, and
   EPSG:9989 to EPSG:4937 drops from 48 candidates to 9. Commit 1 is what makes it safe.

3. **Every other pair in passes 2 and 3** (no output change on the pairs below). PROJ's loop B
   (`:3500-3536`) runs for each iteration over all candidate pairs, the named pair of the request
   left aside, with no kind filter. The port had only loop A, the named candidates. `linked_pair`
   is the body of one pair, now shared by both loops. Its cost is one extra pass over the pairs when
   loop A gives nothing (the ITRF2020 to ETRS89 run takes 11 s with it, 10.5 s without, in a debug
   build).

## Open

- **`hasResultSetOnlyResultsWithPROJStep`** (`:3136-3165`) is not ported: PROJ goes on to the next
  pair when the results of a pass only have PROJ steps. The port returns at any non-empty result.
  No pair checked here shows it.
- **`hasIdentifiers`** (`:3110-3124`, the test of each registry operation of the second leg, `:3279`)
  is replaced by the port's ballpark filter, and the axis order reversal skip (`:3285-3287`) is not
  ported. The epoch change of `createTransformations` (`:3229-3262`) is not ported.
- **The leg `opFirst` / `opThird`** of `createTransformations` is `pivot_leg`, not
  `createOperations` between the CRS and its candidate, except for the axis-order case.
- **The entry to the pivot** tests the filtered direct result (`direct_empty`), where PROJ tests the
  unfiltered one (`res.empty()`, `:4404`).
- **The ensemble step** (`:4524`) is not skipped when `bTriedThroughIntermediateCRS` is set
  (`:4465`, `:4524-4525`); the port does not track it. Pre-existing.
- **The intermediate search of PROJ with `useCreateBetween...`** is the datum-based one
  (`datum_based_intermediate_operations`); the CRS-code one (`crs_codes_intermediate_operations`)
  is still gated by `pivot.is_empty()`, not by PROJ's `res.empty() || !foundInstantiableOp`
  (`:4449-4450`).
- **Ballpark with `--spatial-test intersects`**: ITRF2000 to ETRS89 and ETRS89 (4919) to ETRS89
  (4258) now give the same candidates as PROJ, plus one "Ballpark geographic offset" line that PROJ
  does not print. This lot's change is the candidate set (5 to 12 registry and pivot candidates,
  PROJ 12). The ballpark is added by `create_candidates` when no candidate is of perfect accuracy
  under the strict filter; the PROJ rule is not checked here.
- **EPSG:4936 to EPSG:7789** (neighbour): the port returns the registry inverses `Inverse of
  ITRF2014 to ETRF2014 (2)`, `(1)`, `ETRF2020`, `ETRF2000` (`INVERSE(EPSG):8880` etc.), where PROJ
  returns four concatenations through `ETRS89 (geog2D)` and `ETRS89 to ETRF2014`. Before and after
  this lot the output is the same, so it is not caused here. Not investigated further.
- **EPSG:4230 to EPSG:4258** (ED50 to ETRS89, `--summary` without `--spatial-test`): the port gives
  1 candidate, PROJ 52. Same before and after this lot; the yaml case `test_cs2cs_various` uses
  other arguments and passes.

## Oracle comparison

`projinfo SOURCE TARGET ... --summary` on the grid-less `oracle-db`, PROJ 9.9.0 against the port.
"match" means the whole output is the reference's, line for line. `base` is `lot-projinfo-pivots-3`
(`642892e`), `ours` is this branch (`5944ce2`).

| pair | base | ours | ref candidates |
|---|---|---|---|
| ETRF2005 to ETRF2014, 3D, `--authority EPSG` [yaml 88] | differs | match | 2 |
| ETRF2000-PL to ETRF2014 [yaml 89] | differs | match | 2 |
| EPSG:11108 to EPSG:11128, `--spatial-test intersects` (test `pivot_candidate_set`) | match | match | 3 |
| ETRF2000 to ETRF2014 | match | match | 1 |
| ITRF2014 to ETRF2000 | match | match | 1 |
| ETRS89 to ETRS89-DNK | match | match | 1 |
| ETRF2005 to ETRS89 | match | match | 1 |
| ETRF2014 to ITRF2014 | match | match | 2 |
| EPSG:9945 to WGS 84 [yaml 83] | match | match | 2 |
| AGD66 to WGS 84 | match | match | 1 |
| EPSG:4746 to EPSG:4230, `--spatial-test intersects` | match | match | 7 |
| EPSG:26915 to EPSG:26715, `--spatial-test intersects --grid-check discard_missing` | match | match | 12 |
| EPSG:4746 to EPSG:4978, `--spatial-test intersects --grid-check discard_missing` | match | match | 1 |
| WGS 84 (G1150) to WGS 84 (G2296) [yaml 90] | match | match | 1 |
| EPSG:4202 to EPSG:9057 | match | match | 1 |
| EPSG:4202 to EPSG:7844 | match | match | 1 |
| EPSG:4258 to EPSG:4326 | match | match | 1 |
| EPSG:4936 to EPSG:7789 (neighbour) | differs | differs | 4 |
| EPSG:9989 to EPSG:4937, `--spatial-test intersects` | differs (48) | differs (48) | 47 |
| EPSG:9067 to EPSG:4289 | differs | differs | 2 |
| EPSG:9067 to EPSG:4289, `--spatial-test intersects` | differs (9) | differs (9) | 2 |
| ITRF2000 to ETRS89, `--spatial-test intersects` | differs (5) | differs (13) | 12 |
| ITRF2020 to ETRS89 | differs | differs | 4 |
| ITRF2014 to ETRS89 | differs | differs | 4 |
| EPSG:4230 to EPSG:4258 | differs (1) | differs (1) | 52 |
| EPSG:4919 to EPSG:4258, `--spatial-test intersects` | differs (5) | differs (13) | 12 |
| EPSG:3906 to EPSG:4258 | differs | differs | 1 |

The 28 pairs of the list above are the run; the base matches 15 of them, this branch 17. The two
additions are #88 and #89. ITRF2000 and EPSG:4919 to ETRS89 (`--spatial-test intersects`) gain their
candidates (base 5, now 13 against PROJ's 12): the only difference left is one "Ballpark geographic
offset" line that PROJ does not print (see Open). The counts are PROJ's.

## Tests

The yaml ratchet runs after each commit: `cargo test --test cli_yaml` passes, and the ledger
drops #88 and #89 in commit 2. The unit tests of the sheet's list (`pivot_candidate_set`,
`concat_accuracy`, `unit_op_itrf2000_etrs89`, `unit_opfac_core_c_pivot2`, `datum_pivot`) pass after
commits 1, 2 and 3. The full bar is run once on the head of the branch (see the final report).
