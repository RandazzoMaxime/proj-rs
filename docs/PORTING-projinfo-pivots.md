# Port — projinfo geodetic pivots and realizations (lot D)

Source: PROJ 9.9.0, the tag `9.9.0` of OSGeo/PROJ, read in a scratch checkout. The PROJ
line numbers below are of that tag. The reference binaries are PROJ 9.9.0 `projinfo` and
`cs2cs` built from the same tag (`.../scratchpad/reftest/build/bin`), run on the grid-less
`proj.db` with `PROJINFO_NO_GRID_CHECK=YES`, as the yaml cases are.

## Result

Seven yaml cases were targeted. Five are closed, two stay open:

| case | yaml text | state | closed by |
|---|---|---|---|
| `test_projinfo.yaml#36` | AGD66 to WGS 84 (G1762), 19 candidates | closed | flatten the pivot (commit 4) |
| `test_projinfo.yaml#65` | ITRF2014 to ETRS89-DNK, time-dependent | closed | ensemble step (commit 2) and the time-dependent suffix (commit 3) |
| `test_projinfo.yaml#83` | North Macedonia CRS, `+towgs84` | closed | bound CRS within the CRS extent (commit 1) |
| `test_projinfo.yaml#88` | ETRF2005 to ETRF2014 through ITRF only | open | see "Open" |
| `test_projinfo.yaml#89` | ETRF2000-PL to ETRF2014 | open | see "Open" |
| `test_projinfo.yaml#90` | WGS 84 (G1150) to WGS 84 (G2296) | closed | flatten the pivot (commit 4) |
| `test_projinfo.yaml#106` | Christmas Island identity, `+proj=noop` | closed | engineering identity (commit 5) |

The ledger `docs/UNPORTED-CLI.md` went from 34 projinfo rows to 29. No case that passed
before this lot fails after it: `cargo test --test cli_yaml` passes, and no row was reopened.
`cs2cs` rows stay at 0.

## Function map

| PROJ 9.9.0 (`src/iso19111/...`) | Rust |
|---|---|
| `CRS::createBoundCRSToWGS84IfPossible` (`crs.cpp:516-765`), extent (554-585), context (690-692) | `bound.rs`, `candidate_operations`, `bound_extent` |
| `FilterResults` strict containment (`operation/coordinateoperationfactory.cpp:1380-1400`) | `factory.rs`, `filter_and_sort` (already ported) |
| `createOperations`, ensemble step (`coordinateoperationfactory.cpp:4524-4572`) | `factory.rs`, `create_candidates`, `ensemble_step_applies` |
| `createOperationsEnsembleCRSToOtherGeodCRS` (`:4047-4242`) | `factory.rs`, `ensemble_to_other_geodetic` |
| `isDatumInEnsemble` (`:3964-3973`) | `factory.rs`, `member_of_ensemble` (already ported) |
| intermediate guards (`:4449-4450`, `:4467-4468`) | `factory.rs`, `create_candidates`, `intermediate_needed` |
| `ConcatenatedOperation::createComputeMetadata` (`operation/concatenatedoperation.cpp:674-698`) | `factory.rs`, `pivot_concatenation` calls `flatten_steps` |
| `projinfo_lib.cpp:964-966`, `", time-dependent operation"` | `src/bin/projinfo.rs`, `operation_heading` |
| identity for an engineering CRS (`:3809-3833`) | `factory.rs`, end of `create_candidates` |

## Changes, one cause per commit

1. **Bound CRS within the CRS extent** (closes #83). `createBoundCRSToWGS84IfPossible` gives
   its context the domain of validity of the CRS as the area of interest (crs.cpp:690-692).
   The port passed none. Without it the Bosnia-wide "MGI 1901 to WGS 84 (1)" was bound.
   With the area, the strict containment of `FilterResults` removes it, and only "MGI 1901 to
   WGS 84 (10)" remains. Two domains are handled as crs.cpp:554-585: the larger one when one
   contains the other, none otherwise.
   Deviation: a CRS without any domain takes no area (PROJ runs `getResolvedCRS` first).

2. **Ensemble step** (no case closed on its own; needed by #65). When exactly one of the two
   geodetic CRS has a datum ensemble, PROJ also runs `createOperationsEnsembleCRSToOtherGeodCRS`
   (`:4524-4572`). It loops over the members of the ensemble whose domain meets the target
   and the area of interest, reads each member's operations to the target (through the
   geocentric CRS for a 3D geographic target), and joins the null step from the source
   ensemble to the member. The recursion runs with the three anti-recursion guards set, as
   `AntiRecursionGuard` sets them. The port adds `GUARD_GEOG_ALTERNATIVE` for
   `inCreateOperationsGeogToGeogWithAlternativeGeog`.
   The intermediate search is off inside a guarded recursion (`:4467-4468`, and `:4449-4450`
   for the registry search). Without that guard, the member pairs gained candidates through
   the ETRS89 realization that PROJ does not give: `#15`, `#37` and `#68` regressed in the
   first try of this commit, and pass again after the guard.
   Deviations: `bTriedThroughIntermediateCRS` is not tracked, so the step runs even where
   PROJ has tried the registered intermediates; the interpolation CRS of a member operation is
   not set; a member without an identifier is skipped, since `createGeodeticCRSFromDatum`
   needs one.

3. **"time-dependent operation" suffix** (closes #65). `projinfo_lib.cpp:964-966` appends
   the text after the grid check when `requiresPerCoordinateInputTime()` holds. The port never
   printed it. The flag itself is already ported (`buildop.rs`, `requires_per_coordinate_input_time`),
   and it matches PROJ for the EPSG:10894 steps (a time-dependent position vector) and NKG.

4. **Flattened pivots** (closes #36 and #90). `createComputeMetadata` takes the steps of a
   concatenated step in place of it (`concatenatedoperation.cpp:686-697`). The pivot builder
   kept a registered concatenated operation as one nested step, so "WGS 84 (G1150) to WGS 84
   (G2296)" (PROJ, EPSG:7660 to EPSG:10604) appeared as one step, where PROJ prints four. The
   GDA94 chains of #36 are of the same kind. `pivot_concatenation` now calls `flatten_steps`,
   which the extent and accuracy of the pivot already used (`compound.rs`).
   Not ported: the removal of consecutive inverse steps (`concatenatedoperation.cpp:700-`).
   No yaml case needs it yet.

5. **Identity of an engineering CRS** (closes #106). `createOperations` returns, for an
   engineering CRS equivalent to the target, a Cartesian grid offset of zero over the world
   named "Identity transformation from X to Y", with accuracy 0 (`:3809-3833`). The method
   is EPSG:9656 (`EPSG_CODE_METHOD_CARTESIAN_GRID_OFFSETS`, `proj_constants.h:1012`), and its
   export writes no step for zero offsets, which gives `+proj=noop`.

6. This commit: the PROJ line references in the comments of commits 1 to 5 corrected.

## Open

**#88 and #89** (ETRF2005 and ETRF2000-PL to ETRF2014, 3D). PROJ gives two candidates, both
through ITRF (ITRF2005, ITRF2014, ITRF2020, ETRF2000). The port adds a third one, through
ETRS89 (`Null geographic offset ... + Inverse of ETRS89 to ETRF2005 + ETRS89 to ETRF2014`).

The cause: in `datum_pivot_operations` (`factory.rs`), the passes 2 and 3 of
`createOperationsWithDatumPivot` (`coordinateoperationfactory.cpp:3437-3541`, the
`createTransformations` without `useOnlyDirectRegistryOp`) return at once when both CRS are
identified: `registry_identity(source) && registry_identity(target)`. In PROJ these passes
find the geocentric ITRF and ETRF2000 chains (trace: `iter=1, try ETRF2005 (geocentric) ...`),
and `findsOpsInRegistryWithIntermediate` is then skipped for the top-level pair, since a
result is instantiable (`:4449-4450`). The port runs the registry search on the top-level pair,
and the ETRS89 pivot of `crs_codes_intermediate_operations` appears.

Attempts, both reverted:
- Attempt 1: remove the identity restriction, so that passes 2 and 3 run with
  `create_candidates`. #88 and #89 match. Five unit tests regress: `pivot_candidate_set`
  (`etrs89_ensemble_pair_has_the_three_candidates_of_projinfo`), `concat_accuracy`
  (`two_etrs89_legs_through_the_ensemble_count_once`), `unit_op_itrf2000_etrs89`
  (`operation_itrf2000_to_etrs89`), and `unit_opfac_core_c_pivot2` (two tests, ETRF2000 to
  Amersfoort and ETRS89 xxx to yyy using ETRF2000).
- Attempt 2: as attempt 1, with `create_operations` in place of `create_candidates`, which is
  what `createTransformations` calls. #88 and #89 still match, but `EPSG:9067 to EPSG:4289`
  still gets the ETRS89 pivot that PROJ does not give. The unit tests were not run, since the
  projinfo check already failed.

Next step: port `createOperations` and `createOperationsWithDatumPivot` as written, with their
guards, so that the recursion of passes 2 and 3 is the PROJ one, not a restricted copy. Until
then #88 and #89 stay open.

Also open, and not caused by this lot: the "Note: using '--spatial-test intersects' would
bring more results (N)" count differs for some pairs (ITRF2000 to ETRS89: PROJ 12, port 5);
"MGI 1901 to ETRS89" misses the WGS 84 pivot "MGI 1901 to WGS 84 (1) + Inverse of ETRS89 to
WGS 84 (1)" (the note count with `--spatial-test intersects` agrees, 21); ETRS89 to ETRS89-NOR gives the registry operation where PROJ gives the null
geographic offset; ITRF2020 to ETRS89 names its source as geographic 2D where PROJ takes the
3D CRS.

## Oracle comparison

`projinfo` summary output, PROJ 9.9.0 (oracle), the base commit `e5957c7` and this branch.
Each row runs `projinfo -s SOURCE -t TARGET [options] --summary`; "match" means the output is
the oracle's line for line, blank lines aside.

| pair | base e5957c7 | ours | ours = base |
|---|---|---|---|
| ITRF2014->ETRS89 (4936) [yaml 65] | differs | match | no |
| ETRS89 (4936)->ITRF2014 | differs | match | no |
| ITRF2008->ITRF2014 | differs | match | no |
| ITRF2000->ETRS89 (4258) | differs | differs | no |
| ITRF2020->ETRS89 (4936) | differs | differs | no |
| ETRF2005->ETRF2014 3D [yaml 88] | differs | differs | no |
| ETRF2000-PL->ETRF2014 [yaml 89] | differs | differs | no |
| ETRF2005->ETRS89 (4258) | match | match | yes |
| ETRF2014->ITRF2014 | differs | match | no |
| ETRS89 (4258)->WGS 84 (4326) | match | match | yes |
| WGS 84 (G1150)->WGS 84 (G1762) | match | match | yes |
| WGS 84 (G1150)->WGS 84 (G2296) [yaml 90] | differs | match | no |
| WGS 84 (G1762)->WGS 84 (G2296) | differs | match | no |
| WGS 84 (G730)->WGS 84 | match | match | yes |
| WGS 84 (G1674)->WGS 84 (G2139) | differs | match | no |
| WGS 84 (Transit)->WGS 84 (G1762) | differs | match | no |
| WGS 84 (G2296)->ITRF2020 | match | match | yes |
| AGD66->WGS 84 (G1762) [yaml 36] | differs | match | no |
| AGD66->GDA94 | match | match | yes |
| AGD66->GDA2020 | match | match | yes |
| GDA94->GDA2020 | match | match | yes |
| GDA94->WGS 84 | match | match | yes |
| GDA2020->WGS 84 (G1762) | differs | match | no |
| MGI 1901->ETRS89 (4258) | differs | differs | no |
| EPSG:9945 (MGI 1901 CRS)->WGS 84 [yaml 83] | match | match | yes |
| NAD83->NAD83(2011) | match | match | yes |
| NAD83(CSRS)->NAD83(CSRS)v7 3D | match | match | yes |
| ED50->ETRS89 (4258) [yaml 15] | match | match | yes |
| DHDN->ETRS89 [yaml 37] | match | match | yes |
| ED50 UTM30N->ETRS89 UTM [yaml 68] | match | match | yes |
| ETRS89 (4258)->ETRS89-NOR [EUREF89] | differs | differs | yes |
| ETRS89 (4258)->ETRS89-DNK (10890) | differs | differs | no |
| ETRS89-POL [PL-ETRF2000]->ETRF2014 | differs | differs | yes |
| Christmas Island Grid 1985 identity [yaml 106] | differs | match | no |
| EPSG:9067 -> EPSG:4289 (RD) [unit pivot2] | differs | differs | yes |
| NZGD2000->WGS 84 | match | match | yes |
| Pulkovo 1942(58)->ETRS89 | match | match | yes |
| ED50->WGS 84 | match | match | yes |
| CH1903->ETRS89 | differs | differs | yes |

39 pairs: the base matches 18, this branch 29. Of the 17 pairs whose output changed, 11 now
match; the other 6 still differ (ITRF2000 and ITRF2020 to ETRS89, the two #88 and #89 pairs, MGI
1901 to ETRS89, ETRS89 to ETRS89-DNK), where the change is the time-dependent suffix or a candidate
list that is still not PROJ's. None of the 18 base matches changed.

`cs2cs -f %.9f SOURCE +to TARGET`, one input line per pair, compared as printed (the epoch
warning included). The inputs are: 6 55 0 2020.5 (ITRF2014 to ETRS89); 5 50 0 (ETRF2005 to
ETRS89); 12 55 0 (ETRS89 to ETRS89-DNK); 1000000 5000000 3000000 (WGS 84 G1150 to G2296, geocentric);
145 -37 (AGD66); 21 41.5 0 (EPSG:9945); 12 56 (ED50); 12 56 0 (ITRF2000); 105.7 -10.5 0 (Christmas
Island); 19 52 0 (ETRF2000-PL); 20 42 (MGI 1901).

| pair | base | ours | ours = base |
|---|---|---|---|
| ITRF2014 to ETRS89 (EPSG:7789 to 4936), yaml 65 | differs | match | no |
| ETRF2005 to ETRS89 (8399 to 4258) | match | match | yes |
| ETRS89 to ETRS89-DNK geocentric (4258 to 10890) | match | match | yes |
| WGS 84 (G1150) to WGS 84 (G2296) (7660 to 10604), yaml 90 | match | match | yes |
| AGD66 to WGS 84 (G1762) (4202 to 9057), yaml 36 | match | match | yes |
| AGD66 to GDA2020 (4202 to 7844) | match | match | yes |
| EPSG:9945 to WGS 84, yaml 83 | match | match | yes |
| ED50 to ETRS89 (4230 to 4258), yaml 15 | match | match | yes |
| ITRF2000 to ETRS89 (4919 to 4258) | differs | differs | yes |
| Christmas Island Grid 1985 identity (6715), yaml 106 | differs | match | no |
| ETRF2000-PL to ETRF2014 (9702 to 9069), yaml 89 | match | match | yes |
| MGI 1901 (3906) to ETRS89 | match | match | yes |

12 pairs: base matches 9, this branch 11, output changed in 2 (both are the fixes, 65 and 106).
The `cs2cs` rows of the yaml ledger stay at 0.

## Commands

```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<oracle>/bin:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
# PROJ 9.9.0 trace (PROJ_DEBUG=3) with the traced build, and the base binary built from e5957c7
PROJ_DEBUG=3 <projtrace-build>/bin/projinfo -s "AGD66" -t "WGS 84 (G1762)" --spatial-test intersects --summary
```
