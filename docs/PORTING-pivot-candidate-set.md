# Pivot candidate set (PROJ 9.9.0)

The candidates of a pair of CRS that has no direct operation: the pivots through a datum or
through a registered CRS. Four rules are ported here, one commit each. Before them, for
`EPSG:26915 -> EPSG:26715` under DISCARD, the port listed about 100 candidates where PROJ lists 11
(`test_cs2cs_various.yaml#125` stayed open in combination with the grid rerun, see
`docs/PORTING-concat-accuracy.md`, last section).

## Rules

### 1. Authorities of the pair (commit `b9bab1a`)

PROJ reads the legs of an intermediate through the authorities of the pair, not every authority.

| PROJ 9.9.0 | This port |
|---|---|
| `src/iso19111/operation/coordinateoperationfactory.cpp:1862-1883` `getCandidateAuthorities`: with no authority in the factory, the `allowed_authorities` of the pair (`authority_to_authority_preference`, `PROJ,EPSG,NKG` for EPSG to EPSG); `any` is `""` | `factory.rs` `candidate_authorities` (already ported for direct search), used by `pivot_authorities` |
| `coordinateoperationfactory.cpp:2073-2298` `findsOpsInRegistryWithIntermediate`: `authorities` is passed when there are several (`authFactory->getAuthority() != "any" && authorities.size() > 1`) | `factory.rs` `pivot_authorities` |
| `src/iso19111/factory.cpp:7819-7839` and `8397-8408`: `AND v.auth_name IN (...)` on the legs | `factory.rs` `datum_based_intermediate_operations` (`allowed_authorities`) |
| `factory.cpp:7840-7844` and `8409-8411` `hasAuthorityRestriction`: a named authority is the only restriction | unchanged (`context.authority`) |

Deviation: none. ESRI legs are no longer read for an EPSG pair.

### 2. Registered CRS between two CRS first (commit `cae809e`)

With no direct operation and no datum pivot, PROJ reads the operations through a registered CRS X
with `createFromCRSCodesWithIntermediates` (`factory.cpp:7630-8241`), called from
`createOperationsFromDatabase` (`coordinateoperationfactory.cpp:4436-4450`). The datum-based search
(`createBetweenGeodeticCRSWithDatumBasedIntermediates`, `factory.cpp:8262-8783`) runs after it and
only when it gives nothing (the block after the intermediate search, `coordinateoperationfactory.cpp`
4455-4500). The port had only the datum-based search, with every NAD83 to WGS 84 (n) leg of the
datum.

| PROJ 9.9.0 | This port |
|---|---|
| `factory.cpp:7745-7751`, `8034-8044`, `8094-8099`, `8174-8184`: the four joins (`source->X->target`, `source->X<-target`, `X->source->target`... the orientation is in the `WHERE`) and the inverse of each leg | `factory.rs` `crs_codes_intermediate_operations`, the four `joins` |
| `factory.cpp:7767-7807`, `8100-8123`: the intermediate CRS of a geographic pair is geographic, with a datum not older than the pair (`minDate`) | `criterion` |
| `factory.cpp:7845-7876`: the legs intersect the extents of the CRS | the `extra` filter |
| `factory.cpp:7816`, `8217`: the legs intersect each other unless `CrsExtentUse::NONE` (`disallowEmptyIntersection`) | `skip_intersection` |
| `factory.cpp:7908-7953`: `filterOutSuperseded` on each result set | the `retain` after the four joins |
| `factory.cpp:7955-7987` `checkPivot` (ETRF to ETRF through an ITRF outside the two dates; NAD83(CSRS) through NAD83 or WGS 84) | `pivot_allowed` |
| `factory.cpp:8003-8016`: `useIrrelevantPivot` | `useless_pivot` |
| `coordinateoperationfactory.cpp:2279-2285`: `FilterResults(res, ...)` on the result | `select_candidates` in `create_candidates` |
| `factory.cpp:8232-8238`: `rejectOpDueToMissingGrid` on the concatenation when the grid policy discards | `without_missing_grids` |
| `coordinateoperationfactory.cpp:4436-4450` (the call), `4455-4500` (the datum-based search only when the result is empty) | `create_candidates`, `crs_codes_needed` and `intermediate_needed` |

Deviations (also in the function's doc comment):
- The explicit intermediate CRS list (`getIntermediateCRS`) is not ported: the call is skipped when
  one is given (`context.intermediates`).
- `IntermediateCrsUse::Always` is treated as `Allow`.
- The exportability test of a single result (`coordinateoperationfactory.cpp:4399-4407`) is not
  ported: a non-empty datum pivot skips the SQL search.
- An invalid row is skipped, as the datum-based search does; PROJ throws.
- `intersects_bbox` is evaluated in Rust with `BoundingBox::intersects`.

### 3. Grid rejection on the datum-based pivots (commit `0f0f225`)

| PROJ 9.9.0 | This port |
|---|---|
| `factory.cpp:8774-8780`: `rejectOpDueToMissingGrid` at the end of `createBetweenGeodeticCRSWithDatumBasedIntermediates` | end of `datum_based_intermediate_operations`, `without_missing_grids` |
| `factory.cpp:8232-8238`: the same rejection in `createFromCRSCodesWithIntermediates` | `without_missing_grids` (rule 2) |
| `coordinateoperationfactory.cpp:1889-1972` `findOpsInRegistryDirect`: `DISCARD_OPERATION_IF_MISSING_GRID` | `direct_registry_operations` (unchanged, now the same helper's rule) |

Deviation: none.

### 4. Empty authority in projinfo (commit `70b9a1d`)

| PROJ 9.9.0 | This port |
|---|---|
| `src/apps/projinfo_lib.cpp:1079-1083`: the context is created on `AuthorityFactory::create(dbContext, authority)` with `authority` empty by default (`projinfo_lib.cpp:1562`) | `src/bin/projinfo.rs` `candidate_operations`: `CoordinateOperationContext::create(db, "")` (was `"EPSG"`) |

The library default (`CoordinateOperationContext::create` callers in `select.rs`, `bound.rs`) is
unchanged.

## Oracle comparison

Candidates of `projinfo -s S -t T --summary --spatial-test intersects` (PROJ 9.9.0,
`--grid-check discard_missing` for DISCARD), 42 pairs (`pairs.txt`, the list of
`docs/PORTING-concat-accuracy.md` plus the pairs of this lot). Both sides run on a grid-less copy of
`proj.db` (`PROJ_DATA` set to that directory only). A pair is SAME when the list of
(id, name, accuracy) is identical in order. Counts are pairs out of 42.

| commit | default, ctx `""` | DISCARD, ctx `""` | default, ctx `EPSG` (projinfo before rule 4) | DISCARD, ctx `EPSG` |
|---|---|---|---|---|
| base `7317a4d` | 22 | 24 | 22 | 23 |
| rule 1 `b9bab1a` | 23 | 24 | 22 | 23 |
| rule 2 `cae809e` | 25 | 31 | 24 | 30 |
| rule 3 `0f0f225` | 25 | 33 | 24 | 32 |
| rule 4 `70b9a1d` (projinfo binary) | 25 | not measured (no `--grid-check`) | 24 | not measured |

No pair identical to PROJ before a rule is different after it: checked pair by pair on each
intermediate table (the default and DISCARD tables of each rule, and the projinfo binary of rule 4).

Pairs that changed:
- `EPSG:4746 -> EPSG:4230` (default): 15 candidates, then 7 (PROJ 7). Rule 1.
- `EPSG:4312 -> EPSG:4230` (default): 19, then 8 (PROJ 7). Rule 1. One candidate more than PROJ
  (`MGI to ETRS89 (5) + Inverse of ED50 to ETRS89 (10)`, the one the previous lot noted); cause not
  identified.
- `EPSG:26915 -> EPSG:26715` (DISCARD): 103, then 11 (PROJ 11), in PROJ's order. Rule 2.
- `EPSG:26715 -> EPSG:26915`, `EPSG:4267 -> EPSG:26915`, `EPSG:4269 -> EPSG:26710` (DISCARD): the
  same change as the first pair.
- `EPSG:4746 -> EPSG:4978` (DISCARD): 2, then 1 (PROJ 1). Rule 3.
- `EPSG:11108 -> EPSG:11128` (projinfo, default): 2 with ctx `EPSG`, 3 with `""` (PROJ 3). Rule 4.

Pairs still different (default or DISCARD), with the reason known or not:
- `EPSG:4284 -> EPSG:4258`: 9 in PROJ, 6 here. PROJ lists `Pulkovo 1942 to WGS 84 (16) + Inverse of
  ETRS89 to WGS 84 (1)` (5.5 m), `(20)` (4 m), `(1)`, and the direct `EPSG:1274` (9.0 m); the port
  lacks them. Cause not identified (not the authority or grid rules).
- `EPSG:3912 -> EPSG:3794` (MGI 1901 to ETRS89-SVN [D96]): 36 in PROJ, 24 here (both grid policies).
  The port lacks some of PROJ's candidates; cause not identified.
- `EPSG:4230 -> EPSG:4978`, `EPSG:4267 -> EPSG:4978`, `EPSG:4269 -> EPSG:4978`, `EPSG:4258 -> EPSG:4978`,
  `EPSG:4230 -> EPSG:4258`, `EPSG:4230 -> EPSG:4936`: counts or order differ by one or two.
- `EPSG:4267 -> EPSG:4269` and reverse: under the default policy, the same candidates with the same
  accuracies, but PROJ names them `DERIVED_FROM(EPSG):1313` and `INVERSE(DERIVED_FROM(EPSG)):1313`,
  the port `EPSG:1313`. That is the identifier of `addModifiedIdentifier`
  (`coordinateoperationfactory.cpp:3260-3280`, in `createOperationsWithDatumPivot`), not ported; it is
  among the causes of the text of `test_projinfo.yaml#13` and `#14`. Under DISCARD, 25 in PROJ, 24 here.
- `EPSG:3912+EPSG:5779 -> EPSG:3794+EPSG:8690` (the compound of `test_cs2cs_various.yaml#139`):
  26 in PROJ, 26 here by default (the same count, the candidates differ), 37 against 13 under DISCARD.
  `#139` stays open.

## Test expectations that disagree with the oracle

None found in this lot. The CLI ratchet (`tests/cli_yaml`) was run after each rule; no row of
`docs/UNPORTED-CLI.md` was reopened.

## CLI ratchet (`tests/cli_yaml`)

- Base: 43 open rows, 7 of them cs2cs (`#71`, `#72`, `#96`, `#125`, `#126`, `#129`, `#139`).
- Rule 1 closes `test_cs2cs_various.yaml#125` (`EPSG:26915 -> EPSG:26715`, grid missing): 42 rows,
  6 cs2cs.
- Rules 2 to 4: no row changes.
- Open cs2cs rows: `#71`, `#72` (SCH), `#96` (`--authority` no-op), `#126`, `#129`, `#139`
  (`EPSG:3912+EPSG:5779 -> EPSG:3794+EPSG:8690`, the compound: not closed by this lot).

## Tests

`tests/pivot_candidate_set.rs` (new, 4 tests, expected values from projinfo 9.9.0, not from this port):
- `ed50_pivot_reads_only_the_authorities_of_the_pair` (rule 1): `EPSG:4746 -> EPSG:4230`, 7 names.
- `utm15n_pair_without_grids_pivots_through_wgs84` (rule 2): `EPSG:26915 -> EPSG:26715` under
  DISCARD, 11 names.
- `pivot_leg_with_a_missing_grid_is_discarded` (rule 3): `EPSG:4746 -> EPSG:4978` under DISCARD, 1
  name.
- `etrs89_ensemble_pair_has_the_three_candidates_of_projinfo` (rule 4): `EPSG:11108 -> EPSG:11128`,
  3 names.

The tests set `PROJ_DATA` to `data/` only: `data/tests` holds the NADCON grid `conus`, which would
make the NAD27 legs available.

## Open items

- `test_cs2cs_various.yaml#139` (the compound pivot, see above).
- The candidate gaps of `EPSG:4284 -> EPSG:4258` and `EPSG:3912 -> EPSG:3794`.
- `DERIVED_FROM(EPSG)` identifiers of the datum pivot (`addModifiedIdentifier`), the cause of
  `test_projinfo.yaml#13` and `#14`.
- `IntermediateCrsUse::Always`, the explicit intermediate CRS list, and the exportability test
  (deviations of rule 2).
- Superseded-operation filter of the datum-based search (`factory.cpp:8577-8660`, PROJ's
  `discardSuperseded`): not ported; not measured to change any pair of the table.

## Commands

```bash
# oracle: PROJ 9.9.0 projinfo on a grid-less copy of its proj.db
PROJ_DATA=<oracle-db> <oracle>/bin/projinfo -s EPSG:26915 -t EPSG:26715 --summary \
    --spatial-test intersects --grid-check discard_missing
# the port: throwaway probe (not committed) calling CoordinateOperationFactory::create_operations
# with GridAvailabilityUse::DiscardIfMissing, grid-less PROJ_DATA
cargo test --test pivot_candidate_set
cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
PATH=<oracle>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_DATA=<oracle-db> cargo test --test agree_cs2cs
```
