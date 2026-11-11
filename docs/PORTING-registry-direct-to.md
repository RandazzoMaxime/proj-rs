# Port — bounding-box intersection and `findOpsInRegistryDirectTo` (lot `lot-registry-direct-to`)

Lot `lot-registry-direct-to`, branch `lot-registry-direct-to`. Source: PROJ 9.9.0,
`src/iso19111/metadata.cpp`, `src/iso19111/operation/coordinateoperationfactory.cpp`,
`src/iso19111/operation/oputils.cpp`, `src/iso19111/factory.cpp`, `test/unit/test_metadata.cpp`,
`test/unit/test_operationfactory.cpp`, `test/unit/test_crs.cpp`, `test/cli/test_projinfo.yaml`.

Follows `docs/PORTING-compound-lambert93-ngf.md` ("Open: the superseded `(4)` candidates").

## Status

| Step | State |
|---|---|
| 1. `GeographicBoundingBox::Private::intersection` | Written (patch in scratch), **not committed**: it reopens `issue_4550` (measured by the previous attempt, see below). |
| 2. Trace of `issue_4550` | Open. The reference gives the expected candidate and its extent; the Rust cause is not found. |
| 3. `findOpsInRegistryDirectTo` | **Ported**, commit `f35d484`. Compiles (`cargo check -p proj-rs --tests`, job 3858, succeeded). Effect on the candidate lists **not measured**: the coordinator runs the probe and the suites. |
| 4. `issue_3328` test context | Committed, `c2c901d`. Not run by the author. |

Nothing above is closed by a measurement of the author. The rules of the lot apply: the
coordinator decides whether commit `f35d484` stays.

## Reference PROJ 9.9.0 (run by the author)

Command forms: `ref.ps1 projinfo ... --spatial-test intersects --grid-check none`. Outputs are in
`port-scratch\lot-registry-direct-to\` (`ref-*.txt`), not in the tree.

| Pair | PROJ 9.9.0 candidates | Names (summary) |
|---|---|---|
| `EPSG:5720` -> `EPSG:4979` (`-o PROJ`, `--spatial-test intersects --grid-check none`) | 4 | `INVERSE(DERIVED_FROM(EPSG)):8885` Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (3); `:9876` Inverse of ETRS89-FRA [RGF93 v2b] to NGF-IGN69 height (5); `:8371` Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (2); ballpark `Transformation from NGF-IGN69 height to WGS 84`. No `(4)`. |
| `EPSG:2154+5720` -> `EPSG:4979` (`--summary`, `--spatial-test intersects --grid-check none`) | 6 | `Inverse of Lambert-93 + Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (3) using ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1)`; the same with `(2)`; `... + ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1) + ETRS89-FRA [RGF93 v2] to ETRS89-FRA [RGF93 v2b] + Inverse of ETRS89-FRA [RGF93 v2b] to NGF-IGN69 height (5) + ETRS89-FRA [RGF93 v2b] to WGS 84 (1)`; `... Inverse of ETRS89 to ETRS89-FRA [RGF93 v1] + ETRS89 to ETRS89-FRA [RGF93 v2b] + ...(5) + ...`; `... + ETRS89-FRA [RGF93 v1] to WGS 84 (1) + Transformation from NGF-IGN69 height to WGS 84 (ballpark ...)`; `... + Ballpark geographic offset from ETRS89-FRA [RGF93 v1] to WGS 84 + Transformation from NGF-IGN69 height to WGS 84 (ballpark ...)`. |
| `EPSG:5720` -> `EPSG:4965` (`-o PROJ`, `--grid-check none`) | 2 | Same as `tests/cli_yaml/test_projinfo.yaml:1276`: `INVERSE(DERIVED_FROM(EPSG)):10000` (grid `fr_ign_RAF18.tif`), then the ballpark `Transformation from NGF-IGN69 height to ETRS89-FRA [RGF93 v1]`. |
| `issue_4550` WKT -> `EPSG:32610+5703`, `--spatial-test contains --grid-check none` (the default criterion of PROJ is `STRICT_CONTAINMENT`, coordinateoperationfactory.cpp:173-174) | **1** | `Inverse of unknown + Transformation from NAD83(HARN) to WGS84 + Conversion from NAVD88 height (ftUS) to NAVD88 height + UTM zone 10N`, extent **World**. PROJ string: `+proj=pipeline +step +proj=unitconvert +xy_in=us-ft +xy_out=m +step +inv +proj=lcc +lat_0=45.3333333333333 +lon_0=-120.5 +lat_1=47.3333333333333 +lat_2=45.8333333333333 +x_0=500000.0001016 +y_0=0 +ellps=GRS80 +step +proj=unitconvert +z_in=us-ft +z_out=m +step +proj=utm +zone=10 +ellps=WGS84`. |

`--spatial-test intersects` on the same `issue_4550` pair gives a long list (not counted here).
`PROJ_DEBUG` on the reference prints only file opens (`pj_open_lib`): the C++ internal steps are
not in the reference output.

The test-to-command map: `unit_op_compound_to_compound_2` helpers `ignore_grids` /
`partial_ignore_grids` use `IGNORE_GRID_AVAILABILITY`; `ignore_grids` keeps PROJ's default
(`STRICT_CONTAINMENT`), so `issue_4550` is `--spatial-test contains --grid-check none`, and
the `partial_ignore_grids` tests are `--spatial-test intersects --grid-check none`.

## What PROJ does (read)

### 1. `GeographicBoundingBox::Private::intersection`, metadata.cpp:422-493

- 433-436: no intersection when `N < oS || S > oN`.
- 440-449: a world box against a box crossing the antimeridian (`oW > oE`) returns
  `(oW, max(S,oS), oE, min(N,oN))`; symmetric at 447-450.
- 453-462: normal boxes, `resW = max(W,oW)`, `resE = min(E,oE)`, returned only when `resW < resE`.
- 464-483: bail out when `oW > 180 || oE < -180`; else the parts `(oW..180)` and `(-180..oE)` are
  intersected with `this`, the larger width is kept (`>` strict).
- 485-491: this box crosses, `oW <= oE`: `otherExtent.intersection(*this)`; both cross:
  `(max(W,oW), max(S,oS), min(E,oE), min(N,oN))`, returned without check.

### 2. `Extent::intersection` for single boxes, metadata.cpp:889-910

`contains(other)` returns `other`; `other->contains(self)` returns `self`; else the box
intersection of 1. Empty result: `nullptr`.

### 3. `GeographicBoundingBox::contains` and `intersects`, metadata.cpp:284-331 and 336-392

Identical to `model.rs` `BoundingBox::contains` (model.rs:246-265) and `intersects` (model.rs:269-304).

### 4. Extents of steps and operations

- `getExtent(ops, ...)`, oputils.cpp:529-555: intersection of the sub-extents; a null intersection
  sets `emptyIntersection` and returns `nullptr`; a null sub-extent is skipped unless
  `emptyIntersection` is set.
- `getExtentPossiblySynthetized`, oputils.cpp:502-525: a compound CRS takes the intersection of its
  components' extents.

### 5. Candidate filter, coordinateoperationfactory.cpp:1380-1415 (`FilterResults`)

`PARTIAL_INTERSECTION`: keeps an operation whose extent intersects both CRS extents
(`Extent::intersects`, metadata.cpp:861-879). 1392-1398: an operation that contains both extents
and has no grid is marked.

### 6. Compound to compound, coordinateoperationfactory.cpp:7214-7612

- 7452-7545: `opsSrcToGeog` x `opsGeogSrcToGeogDst` x `opsGeogToTarget`, two passes (ballpark
  excluded first), each candidate built by `ConcatenatedOperation::createComputeMetadata(...,
  disallowEmptyIntersection())` (7517-7527). The `issue_4550` candidate is built here.
- 7547-7612: `createOpsInTwoSteps`.

### 7. `findOpsInRegistryDirectTo`, coordinateoperationfactory.cpp:1973-2033

For each identifier of the target (`buildCRSIds`, 1776-1825), for each authority of
`getCandidateAuthorities(target, target)`: `createFromCoordinateReferenceSystemCodes("", target,
usePROJAlternativeGridNames, discardIfMissingGrid, knownGridsAvailable, discardSuperseded,
tryReverseOrder=true, reportOnlyIntersectingTransformations=true, extent1, extent2)`, the results of
the authorities accumulate; `PROJ` does not stop the loop; a non-empty result is returned through
`FilterResults`.

### 8. `AuthorityFactory::createFromCoordinateReferenceSystemCodes`, factory.cpp:7205-7542, empty source

- SQL 7342-7401: with an empty source and a target, `((cov.source = T) OR (cov.target = T))`.
- ORDER BY 7408-7410: pseudo area DESC, accuracy NULL last, accuracy.
- `setTransf` 7412-7420: the (auth, code) of every row, before the intersection test.
- Intersection pass 7422-7473: a row intersects when its box intersects both extents.
- Report 7480-7488: with `reportOnlyIntersectingTransformations` only the intersecting rows.
- Superseded rows skipped 7489-7505 (and 7431-7447).
- Instantiation and reversal 7514-7538: `op->inverse()` when the row target is not the target CRS
  (`tryReverseOrder`, 7517-7524); `rejectOpDueToMissingGrid` 7525-7527; no duplicate check.

### 9. `createOperationsGeogToVertWithAlternativeGeog`, coordinateoperationfactory.cpp:5073-5137

Calls `findOpsInRegistryDirectTo(targetCRS)`; keeps the operations whose source is geographic; a
3D geographic source whose vertical unit is not the metre gets a unit change before the operation
(`createComputeMetadata`).

## Map PROJ function -> Rust function

| PROJ 9.9.0 | Rust | Status |
|---|---|---|
| `GeographicBoundingBox::Private::intersection` (metadata.cpp:422-493) | `model.rs` `BoundingBox::intersection` (min/max) | patch, not committed (step 1) |
| `Extent::intersection`, single boxes (metadata.cpp:889-910) | `factory.rs` `intersect_extents` | unchanged |
| `GeographicBoundingBox::contains` / `intersects` | `model.rs` `contains` / `intersects` | identical |
| `getExtent(ops)` (oputils.cpp:529-555) | `derived.rs` `joint_extent` | open point 2 below |
| `FilterResults` (coordinateoperationfactory.cpp:1380-1415) | `factory.rs` `filter_and_sort` | unchanged |
| `createOperationsCompoundToCompound` (7214-7612) | `compound.rs` 699-733, `compute_metadata` 1063-1087 | unchanged |
| `buildCRSIds` (coordinateoperationfactory.cpp:1776-1825) | `factory.rs` `find_ops_in_registry_direct_to`, the identifiers of the CRS | simplified (see deviations) |
| `findOpsInRegistryDirectTo` (1973-2033) | `factory.rs` `find_ops_in_registry_direct_to` (new) | ported, commit `f35d484` |
| `createFromCoordinateReferenceSystemCodes`, empty source (factory.cpp:7205-7542) | `factory.rs` `operations_between` with `RegistrySearch::IntoTarget` / `IntoTargetKeepingSuperseded` (new modes) | ported, commit `f35d484` |
| `createOperationsGeogToVertWithAlternativeGeog` (5073-5137) | `compound.rs` `geog_to_vertical_alternative` | rewritten on the finder, commit `f35d484` |

## Map PROJ `TEST()` -> Rust test

| PROJ `TEST()` | Rust test | Status |
|---|---|---|
| `TEST(metadata, extent)` (test_metadata.cpp:66-280), box cases about the antimeridian | none committed (step 1 patch in scratch) | open |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_3328)` (test_operationfactory.cpp:6643) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_3328` | context authority `""` (commit `c2c901d`), unverified |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_4550)` (test_operationfactory.cpp:7063) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_4550` | unchanged; expected candidate in the table above |
| `projinfo -s EPSG:5720 -t EPSG:4965 -o PROJ` (test/cli/test_projinfo.yaml:1276) | `tests/cli_yaml/test_projinfo.yaml:1276` (copied from PROJ) | existing; the finder must keep it |

No new test is committed for the registry port: PROJ has no `TEST()` for the candidate list of
`EPSG:5720` -> `EPSG:4979`. The pairs to compare are in the table of the reference section; the
probe prints them: `port-scratch\lot-registry-direct-to\probe_registry_direct_to.rs`.

## Step 1: box intersection (not committed)

The port is in `port-scratch\lot-registry-direct-to\step1-box-intersection.patch`, with the test
`unit_metadata_extent.rs` (same directory). It compiled (devhub job 3824). It adds
`BoundingBox::extent_intersection` (`pub(crate)`, the `Extent::intersection` shortcut, used by the
Extent-level callers `select.rs` `pseudo_area_of` and `factory.rs` `SortExtents::new`).

Why not committed: the previous attempt (`docs/PORTING-compound-lambert93-ngf.md`, "Attempt 2")
measured that the port reopens `issue_4550`: the target `EPSG:32610+5703` gets an extent, 470
candidates instead of 808 before the filter, and PROJ's candidate is no longer built. That was
measured on the previous tree; it has not been re-measured here.

## Step 2: trace of `issue_4550` (open)

Read so far:

1. The reference gives one candidate with `STRICT_CONTAINMENT`, extent World. The candidate is
   built in the two-pass loop 7452-7545, from op1 (the inverse of the source projection), op2 (the
   `Transformation from NAD83(HARN) to WGS84` of the source datum, null), op3 (the vertical
   conversion and UTM).
2. Inference from the code, not measured: before the port, the target `EPSG:32610+5703` has no
   extent (the min/max box intersection of UTM 10N and the antimeridian-crossing NAVD88 box is
   empty), so no filter runs. With the port it has one (-126..-120), and the filter 1388-1415 runs.
   The previous attempt measured the loss of the candidate with the port.
3. The difference is in one of: the extent of op1, of op2, of op3, or in `joint_extent`
   (derived.rs:166-175), which skips a step without an extent where `getExtent(ops)`
   (oputils.cpp:537-542) returns `nullptr` when `emptyIntersection` is set.

Next step (for the coordinator, with the probe `probe_4550_default_criterion`): the candidates of the
Rust port for the pair, with their extents, against the one reference candidate above (World). If
the Rust candidate set is empty or differs, the first comparison is the extent of op1, op2, op3.

## Step 3: `findOpsInRegistryDirectTo` (ported, commit `f35d484`)

What the commit does:

- `RegistrySearch::IntoTarget` and `IntoTargetKeepingSuperseded` (factory.rs, `operations_between`):
  the SQL `((source = T) OR (target = T))` with `T` the target identifier; the superseded rows
  dropped for `IntoTarget` (listed rows of all the rows read, as `setTransf`); the rows that do not
  intersect both CRS extents dropped (`reportOnlyIntersectingTransformations`); a row is inverted
  when its target is not `T` (`tryReverseOrder`). The existing modes are unchanged: each new branch is
  gated on the variant.
- `retain_available_grids` (factory.rs): the missing-grid rule of `direct_registry_operations`,
  moved out unchanged, used by both.
- `find_ops_in_registry_direct_to` (factory.rs): the loop over the identifiers and the authorities
  of `findOpsInRegistryDirectTo`, `PROJ` not stopping the loop, `filter_and_sort` on a non-empty
  result.
- `geog_to_vertical_alternative` (compound.rs): reads the finder's operations instead of the rows
  of `coordinate_operation_view`; keeps the geographic-source test and the unit change of C++
  5103-5134; the missing-grid test moved into the finder.
- `vertical_geog_fallback` passes `sort_extents(context, source, target)`.

Deviations (deliberate, stated in the code where relevant):

1. `buildCRSIds` checks each identifier by reading its CRS back (`_isEquivalentTo`); the finder
   takes every identifier with a non-empty authority as it is.
2. `operations_between` drops a repeated (authority, code) pair; C++ does not. Kept from the
   existing function.
3. The extents of the registry filter are the bounding boxes of the CRS extents; C++
   `Extent::intersects` also compares vertical and temporal elements.
4. The order of the rows is PROJ's (pseudo area, accuracy). The old `geog_to_vertical_alternative`
   read the rows in table order, so the first candidate of a pair that goes through this fallback can
   change.

Existing callers at risk, all through `factory.rs` (registry block, 6120-6127): `vertical_geog_fallback`
(the only caller of `geog_to_vertical_alternative`), for every geographic-to-vertical pair that has no
direct registered operation, such as `EPSG:5720` -> `EPSG:4979`, `EPSG:2154+5720` -> `EPSG:4979`
and `EPSG:5720` -> `EPSG:4965` (cli yaml). The existing modes of `operations_between`
(`Forward`, `ForwardWithoutSuperseded`, `Both`, `BothKeepingSuperseded`) are not changed by the
new branches.

Not measured: whether the candidate lists match the reference above. The probe does it.

## Step 4: test context of `issue_3328` (committed)

`operation_compound_to_compound_issue_3328` built its context with the authority `"EPSG"`. The C++
`TEST()` builds its authority factory with `std::string()` (test_operationfactory.cpp:6644-6645).
The Rust test now uses `""` (commit `c2c901d`). The previous attempt's note says the test passes
with `""`; not run by the author.

## Existing callers at risk (summary)

- `model.rs` `BoundingBox::intersection`: step 1 patch only.
- `select.rs` `pseudo_area_of`, `factory.rs` `intersect_extents`, `SortExtents::new`, `has_too_small_areas`,
  `operation_extent`, `crs_extent`, `derived.rs` `joint_extent`, `compound.rs` `compute_metadata`: step 1
  patch only.
- `factory.rs` registry block 6120-6127 and `compound.rs` `vertical_geog_fallback`,
  `geog_to_vertical_alternative`: commit `f35d484`.
- `tests/unit_op_compound_to_compound_2.rs` `issue_4550`: reopened by the step 1 patch (measured by the
  previous attempt).

## Deliberate deviations (committed)

See step 3 (1 to 4).

## Commands run

- `node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests`:
  step 1 tree job 3824, step 4 tree job 3833, registry port tree job 3858: all succeeded.
- `ref.ps1` with the options of the reference section.
- `cargo fmt`.

## Probe for the coordinator (not in the tree)

`port-scratch\lot-registry-direct-to\probe_registry_direct_to.rs`: copy to `tests/`, run
`cargo test --test probe_registry_direct_to -- --nocapture --test-threads=1`, and return the `PROBE`
lines. Four tests: `5720 -> 4979`, `2154+5720 -> 4979`, `5720 -> 4965`, and `issue_4550` with the
default criterion. Each prints the candidate count, then the name and the PROJ string of each candidate.

Tests written, not run by the author: the coordinator runs them.
