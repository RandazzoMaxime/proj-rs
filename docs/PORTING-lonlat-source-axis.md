# cs2cs: longitude-first CRS (OGC:CRS84) and PROJ-string pairs

Lot `lonlat-source-axis`. Two faults, both in the way `cs2cs` builds a pair, and the candidates that
depend on them:

1. A longitude-first source or target (`OGC:CRS84`) got a candidate with no axis change for its own
   side. `cs2cs OGC:CRS84 EPSG:4269` printed `122dS 45dE` (PROJ: `45dN 122dW`), and
   `cs2cs OGC:CRS84 EPSG:26710` printed `*`.
2. A pair of two PROJ strings (`+proj=...`) skipped the ISO selection, which has the alternatives of
   PROJ's `proj_create_crs_to_crs`. Case `#124` needed them.

Both are closed. The ledger `docs/UNPORTED-CLI.md` goes from 17 open cs2cs rows to 12. No previously
passing case reopened.

## Root cause of fault 1 (verified, not the export)

The brief's hypothesis was that the ISO export of a candidate for a longitude-first source starts
with an axis change. That is true of one candidate, but it is the symptom, not the cause.

The candidates of `OGC:CRS84` to a CRS on another geodetic datum come from the datum pivot through
WGS 84 (`createOperationsWithDatumPivot`). Its first leg is `createOperations(CRS84, WGS 84)`, which
is the axis order change of `OGC:CRS84` (PROJ string `+proj=axisswap +order=2,1`). The port's
`pivot_leg` treated any two geographic CRS of one datum as the identity, so that leg was dropped:

- `Inverse of NAD83 to WGS 84 (1)` (a null transformation, `+proj=noop` in the port) was used alone.
  The output is the input: `-122 45` stays `-122 45` (latitude first, as NAD83 is). PROJ's
  candidate is `OGC:CRS84 to WGS 84 + Inverse of NAD83 to WGS 84 (1)`, `+proj=axisswap +order=2,1`.
- For `EPSG:26710`, the candidate `Inverse of NAD27 to WGS 84 (33) + UTM zone 10N` starts with the
  axis swap of its own source leg, which is WGS 84 and latitude first. With the leg of `OGC:CRS84`
  missing, that swap is applied to the longitude-first input, so the grid and the projection receive
  latitude first. PROJ has the two swaps, which cancel (`projinfo` shows no axis step for that
  candidate).

So the swap at the start of the port's candidate is right for a latitude-first source. It is the
change of `OGC:CRS84` that was lost.

The area of a datum-pivot candidate was also wrong (fault 1, second part): the port gave the
concatenation no area, so the 1 m candidate `Inverse of NAD27 to WGS 84 (88)`, whose area is Cuba,
was chosen for a point in Oregon (`cs2cs OGC:CRS84 EPSG:4267` printed `45d0'0.421"N 121d59'56.668"W`
for the first attempt, PROJ `45.000155 -121.998809`). PROJ's `ConcatenatedOperation::createComputeMetadata`
gives the concatenation the intersection of its legs' areas (`getExtent`), so the Cuba candidate is
not suggested for Oregon.

## Code

- `src/iso19111/geodetic_ops.rs`: `axis_reversal_between` (the test of `axisReversal2D` and
  `axisReversal3D`, moved out of `geographic_to_geographic`) and `axis_order_change` (EPSG:15498 or
  EPSG:15499, the conversion of `createAxisOrderReversal` with its CRS pair).
- `src/iso19111/factory.rs`: `pivot_leg` takes the axis order change between two geographic legs of
  one datum; `pivot_domain` gives the datum-pivot concatenation the intersection of its legs' areas,
  set in `datum_pivot_operations`.
- `src/bin/cs2cs.rs`: `iso_serves_pair` decides whether a pair goes through the ISO selection (path
  (1) of `docs/PORTING-cs2cs-paths.md`). A PROJ string goes there unless it has `+approx`, `+over`
  or `+lon_wrap`.

## PROJ file:line map

| PROJ 9.9.0 | This port |
|---|---|
| `src/apps/cs2cs.cpp:833`, `:837`: `proj_create` of each side from its PROJ string | `build_engine` (cs2cs.rs): every pair goes to `select_alternatives` unless a flag is in the way |
| `src/apps/cs2cs.cpp:989`: `proj_create_crs_to_crs_from_pj` for every pair, PROJ strings included | `select_alternatives` for pairs of PROJ strings (`iso_serves_pair`) |
| `src/iso19111/operation/coordinateoperationfactory.cpp:2967-2993` `axisReversal2D` / `axisReversal3D` | `geodetic_ops::axis_reversal_between` |
| `coordinateoperationfactory.cpp:3019-3020` `createAxisOrderReversal` when the CRS differ by their axis order | `geodetic_ops::axis_order_change`, used by `geographic_to_geographic` and `pivot_leg` |
| `coordinateoperationfactory.cpp:3167` `createOperationsWithDatumPivot` | `datum_pivot_operations` (factory.rs) |
| `coordinateoperationfactory.cpp:3265` `opsThird`, `:3480-3486` `opsFirst` and `isNullFirst` of the source leg | `pivot_leg(source, left)` and `pivot_leg(right, target)` |
| `coordinateoperationfactory.cpp:3351-3352` `if (!isNullFirst) subOps.emplace_back(opFirst)` | the legs of `linked` (factory.rs) |
| `src/iso19111/operation/conversion.cpp:2573` `Conversion::createAxisOrderReversal` | `geodetic_ops::axis_order_change` |
| `src/iso19111/operation/oputils.cpp:529` `getExtent(ops)`: the intersection of the areas, a step without one skipped | `factory::pivot_domain` (uses `operation_extent`, `intersect_extents`) |
| `src/iso19111/operation/concatenatedoperation.cpp:674-752` `createComputeMetadata`: the domain of the concatenation | `pivot_domain` |

## Deviations from PROJ

1. Name of the source leg. PROJ takes the first operation of `createOperations(CRS84, WGS 84)`. The
   candidate names printed by PROJ's `projinfo` (`OGC:CRS84 to WGS 84 + Inverse of NAD83 to WGS 84 (1)`)
   show that this leg is the registry transformation `PROJ:CRS84_TO_EPSG_4326` (`OGC:CRS84 to WGS 84`,
   which `projinfo` lists for `OGC:CRS84 EPSG:4326`), not an axis change made in the factory. The port takes the
   axis order change `EPSG:15498` named `axis order change (2D)`, so the candidates are named
   `axis order change (2D) + Inverse of NAD83 to WGS 84 (1)`. The PROJ string is the same
   (`+proj=axisswap +order=2,1`), and the candidate order is the same (53 candidates for
   `OGC:CRS84 EPSG:4269`, the same names apart from that prefix). The port's leg is not looked up in
   the registry.
2. Area of a datum-pivot candidate. `pivot_domain` is applied to every candidate `datum_pivot_operations`
   builds, not to the legs with an axis change only. PROJ gives every concatenation the intersection
   of its legs' areas. Other concatenated operations (for example `PD/83 to WGS 84 (1) + Conversion
   ...`, see `docs/PORTING-cs2cs-alternatives.md`, deviation 5) keep an unknown area. This overlaps the
   lot on the area of use of concatenated operations: whichever lands second must reconcile the two
   places that set `identified.domains` on a concatenation.
3. PROJ-string pairs with `+approx`, `+over` or `+lon_wrap` keep the side-pair path
   (`SidePair::open`, which reads the flags), as in the previous lot. PROJ creates those CRS with the
   flag inside `proj_create`, and the ISO import does not carry it. Routing them through the ISO
   selection reopened cases `#6`, `#20`, `#21` and `#92` (attempt 1 below). Flagged pairs are the only
   PROJ-string pairs still not on the ISO path.
4. The inverse of the axis change is not exported (fault below, open item 1).

## Oracle comparison (PROJ 9.9.0 `cs2cs` against this port)

Commands (the matrix is a shell script in the scratchpad, the same calls as below):

```bash
# PROJ 9.9.0 (oracle)
PROJ_DATA=<scratchpad>/lot/refdata:<worktree>/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  <scratchpad>/lot/projbuild/bin/cs2cs OGC:CRS84 EPSG:4269 -f %.6f <<< "-122 45"
# this port
PROJ_DATA=<worktree>/data:<worktree>/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs OGC:CRS84 EPSG:4269 -f %.6f <<< "-122 45"
```

| pair | input | PROJ 9.9.0 | this port | same |
|---|---|---|---|---|
| OGC:CRS84 -> EPSG:4269 | -122 45 | 45.000000 -122.000000 | 45.000000 -122.000000 | yes |
| OGC:CRS84 -> EPSG:4267 | -122 45 | 45.000155 -121.998809 | 45.000155 -121.998809 | yes |
| OGC:CRS84 -> EPSG:26710 | -122 45 | 578911.275196 4983237.990370 | same | yes |
| OGC:CRS84 -> EPSG:2154 | -122 45 | -5521727.411403 12731733.988284 | same | yes |
| OGC:CRS84 -> EPSG:32631 | -122 45 | -3723038.118971 13321749.056464 | same | yes |
| OGC:CRS84 -> EPSG:4269 | 2.35 48.85 | 48.850000 2.350000 | same | yes |
| OGC:CRS84 -> EPSG:4267 | 2.35 48.85 | 48.850000 2.350000 | same | yes |
| OGC:CRS84 -> EPSG:26710 | 2.35 48.85 | 4329553.837057 12986960.136004 | same | yes |
| OGC:CRS84 -> EPSG:2154 | 2.35 48.85 | 652301.564831 6861302.725900 | same | yes |
| OGC:CRS84 -> EPSG:32631 | 2.35 48.85 | 452314.891234 5410984.887604 | same | yes |
| OGC:CRS84 -> EPSG:4326 | -122 45 | 45.000000 -122.000000 | same | yes |
| EPSG:4326 -> EPSG:4269 | 45 -122 | 45.000000 -122.000000 | same | yes |
| EPSG:4326 -> EPSG:4267 | 45 -122 | 45.000155 -121.998809 | same | yes |
| EPSG:4326 -> EPSG:26710 | 45 -122 | 578911.275196 4983237.990370 | same | yes |
| EPSG:4326 -> EPSG:2154 | 48.85 2.35 | 652301.564831 6861302.725900 | same | yes |
| EPSG:4326 -> EPSG:32631 | 45 -122 | -3723038.118971 13321749.056464 | same | yes |
| EPSG:4326 -> EPSG:4269 | 48.85 2.35 | 48.850000 2.350000 | same | yes |
| EPSG:4326 -> EPSG:4267 | 48.85 2.35 | 48.850000 2.350000 | same | yes |
| EPSG:4269 -> OGC:CRS84 | 45 -122 | -122.000000 45.000000 | same | yes |
| EPSG:4267 -> OGC:CRS84 | 45 -122 | -122.001191 44.999845 | same | yes |
| EPSG:26710 -> OGC:CRS84 | 578911.275196 4983237.990370 | -122.000000 45.000000 | same | yes |
| EPSG:2154 -> OGC:CRS84 | 652301.564831 6861302.725900 | 2.350000 48.850000 | same | yes |
| EPSG:32631 -> OGC:CRS84 | 452314.891234 5410984.887604 | 2.350000 48.850000 | same | yes |
| EPSG:4326 -> OGC:CRS84 | 45 -122 | -122.000000 45.000000 | 45.000000 -122.000000 | **no** (open item 1) |
| EPSG:4326 -> OGC:CRS84 | 48.85 2.35 | 2.350000 48.850000 | 48.850000 2.350000 | **no** (open item 1) |
| OGC:CRS84 -> EPSG:4326 | -122 45 | 45.000000 -122.000000 | same | yes |
| EPSG:4267 -> OGC:CRS84 | 48.85 2.35 | 2.350000 48.850000 | same | yes |

Before this lot, the first column of failures was: `OGC:CRS84 -> EPSG:4269` wrong (`122dS 45dE`),
`OGC:CRS84 -> EPSG:4267`, `OGC:CRS84 -> EPSG:26710`, `OGC:CRS84 -> EPSG:2154` (`-122 45`) failed
(`*`), `OGC:CRS84 -> EPSG:2154` (`2.35 48.85`) wrong (`6960179.41 3114195.71`), and every
`... -> OGC:CRS84` with a projected or NAD83 source gave latitude first.

## Test cases

`tests/lonlat_source_axis.rs` (new, 11 tests, written before the change: 8 failed on the base
commit, the 3 guards passed). Expected values are PROJ 9.9.0 `cs2cs` output, as in the table above.

- `crs84_to_nad83_gives_latitude_first_output`, `crs84_to_nad27_gives_latitude_first_output`,
  `crs84_to_nad27_utm10n_gives_the_projected_point`, `crs84_to_lambert93_gives_the_projected_point`,
  `crs84_to_lambert93_far_from_france`, `crs84_to_utm31n_gives_the_projected_point`: `OGC:CRS84` as
  the source.
- `nad83_to_crs84_gives_longitude_first_output`, `nad27_utm10n_to_crs84_gives_longitude_first_output`,
  `lambert93_to_crs84_gives_longitude_first_output`: `OGC:CRS84` as the target.
- `latitude_first_pair_is_unchanged`, `latitude_first_source_to_utm10n_is_unchanged`: guards.

`src/bin/cs2cs.rs`, unit test `iso_selection_takes_plain_proj_strings_but_not_flagged_ones`: the
routing rule.

CLI ratchet (`cargo test --test cli_yaml`):

- Closed: `#9`, `#10`, `#80`, `#81` (PROJ-string pairs, now on the ISO path: geocentric x/y/z and
  `vto_meter`), and `#124` (the case of this lot). Open cs2cs rows: 17 to 12.
- Reopened: none.

## Attempts (budget: two per part)

Part 1, the axis change and the pivot domain:

1. Axis change of the pivot leg (`pivot_leg`) only: 10 of the 11 new tests pass, and the full suite
   has no reopened case. `OGC:CRS84 -> EPSG:4267` gives `45.000117`, not `45.000155`: the 1 m
   candidate `Inverse of NAD27 to WGS 84 (88)` (Cuba) is chosen, because the concatenation has no
   area. Kept (the axis change is needed either way).
2. Plus `pivot_domain` for the datum-pivot concatenations: 11 of 11 pass, the full suite passes
   (1560 tests, no failure). Kept.

Part 2, PROJ-string pairs on the ISO path (`#124`):

1. Remove the `projish` gate of `build_engine` altogether: `#124` closed, and also `#9`, `#10`,
   `#80`, `#81`, but `#6`, `#20`, `#21` and `#92` reopened (`lon_wrap`, `+approx` tmerc, `+over`).
   Reverted to the next attempt.
2. Route to the ISO selection unless a side has `+approx`, `+over` or `+lon_wrap` (`iso_serves_pair`):
   the CLI ratchet passes with no reopened case. The full suite then fails only on the generated
   docs, which are regenerated, not edited by hand: `README.md`, `ROADMAP.md` by
   `UPDATE_DOCS=1 cargo test --test docs_in_sync`, `docs/progress.svg` by
   `UPDATE_DOCS=1 cargo test --test progress_svg`. `docs/UNPORTED-CLI.md` is rewritten by the yaml
   test itself. Kept.

Why PROJ-string pairs with those flags are not routed: the flag is an argument of `proj_create` that
the ISO import of the PROJ string does not keep (`+approx` of `tmerc`, `+over` of the longitude
wrap, `+lon_wrap` of `cs2cs`). The change to route them is the ISO import of those parameters, which
is outside this lot.

## Open items

1. `EPSG:4326 -> OGC:CRS84` gives latitude first (table, two rows). The best candidate is the
   inverse of the `OGC:CRS84` axis change, `Inverse of OGC:CRS84 to WGS 84`. Its export fails,
   `select.rs` `export` refuses a conversion that is `inverted` ("has no PROJ string in this crate"),
   so path (1) fails and the facade gives the answer. PROJ exports the inverse as `+proj=axisswap
   +order=2,1`. Cause: the inverted axis change of `conversion_export.rs` / `select.rs:419`. Was wrong
   on the base commit too; not in this lot's source-side matrix.
2. Concatenated operations other than the datum pivot keep an unknown area (see deviation 2).
3. Flagged PROJ-string pairs (`+approx`, `+over`, `+lon_wrap`) stay on the side-pair path (deviation 3).
4. Not run: the bench cases of `CONTRIBUTING.md` step 6 (`src/bin/bench.rs`, `bench/record.sh`). The
   lot changes no ported function's behaviour that a benchmark case measures, so no case was added.

## Checks

```bash
cargo test --test lonlat_source_axis
cargo test --test cli_yaml            # 5 closed, 0 reopened (17 -> 12 open cs2cs rows)
UPDATE_DOCS=1 cargo test --test docs_in_sync
UPDATE_DOCS=1 cargo test --test progress_svg
cargo test --test docs_in_sync
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

Disk: the full test build of this worktree needs about 9 GB, and the shared disk filled up three
times. `cargo clean -p proj-rs` in this worktree was used for that. The full runs used
`CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0` (debuginfo and incremental state off; the tests
themselves are unchanged).
