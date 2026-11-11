# Port — bounding-box intersection and `findOpsInRegistryDirectTo` (lot `lot-registry-direct-to`)

Lot `lot-registry-direct-to`, branch `lot-registry-direct-to`. Source: PROJ 9.9.0,
`src/iso19111/metadata.cpp`, `src/iso19111/operation/coordinateoperationfactory.cpp`,
`src/iso19111/operation/oputils.cpp`, `src/iso19111/factory.cpp`, `test/unit/test_metadata.cpp`,
`test/unit/test_operationfactory.cpp`, `test/unit/test_crs.cpp`, `test/cli/test_projinfo.yaml`.

Follows `docs/PORTING-compound-lambert93-ngf.md` ("Open: the superseded `(4)` candidates").

## Status

| Step | State |
|---|---|
| 1. `GeographicBoundingBox::Private::intersection` | **Ported**, commit `6baca41`, with `tests/unit_metadata_extent.rs`. |
| 2. Cause of `issue_4550` | **Found and fixed**, commit `8fc3e7e`: the extents of the outermost pair reach every sub-call. See "Step 2". |
| 3. `findOpsInRegistryDirectTo` | **Ported**, commit `f35d484`. |
| 4. `issue_3328` test context | Committed, `c2c901d`. |

The three code commits hold together: each alone reopens a test, the three together reopen none
(measured, see "Measured by the coordinator").

## Measured by the coordinator

All through the job queue, on Windows, without the reference `cs2cs` on the PATH.

| Tree | Result |
|---|---|
| `f35d484` alone, full suite | reopens `unit_op_compound_derived::operation_compound_to_geog_2d_promote_to_3d_context` (131 candidates for 133) |
| `f35d484` + step 1 + step 2 (`8fc3e7e`), full suite | 1682 passed, 11 failed, 28 ignored: the same 11 tests fail on `main` (`c57d7d2`) on that machine (its `PROJ_DATA` points at an OSGeo4W install with real grids; cause not confirmed by a second run). No test fails on this branch only. `cli_yaml` passes, `docs/UNPORTED-CLI.md` unchanged. |
| same tree, `unit_op_compound_to_compound_2` | 11 passed, 2 ignored, in 24 s (119 s on `main`) |
| same tree, probe `EPSG:5720` -> `EPSG:4979` | 4 candidates, `(3)`, `(5)`, `(2)`, ballpark: the reference list (was 5 with `(4)`) |
| `f35d484` alone, probe `EPSG:2154+5720` -> `EPSG:4979` | 6 candidates, as the reference (was 8). Not probed again on the final tree. |
| `f35d484` alone, probe `EPSG:5720` -> `EPSG:4965` | 2 candidates, as `test_projinfo.yaml:1276` |
| same tree, `issue_4550` pair, default criterion | **1 candidate**, as the reference. On `main` the port gives 310 (see "Step 2"). |
| `cargo clippy -p proj-rs --all-targets --all-features -- -D warnings`, `cargo fmt --check` | clean |

## Reference PROJ 9.9.0 (run by the author)

Command forms: `projinfo ... --spatial-test intersects --grid-check none` on the build of
`tools/build-reference.sh`. The outputs are not in the tree.

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
| `GeographicBoundingBox::Private::intersection` (metadata.cpp:422-493) | `model.rs` `BoundingBox::intersection` | ported, commit `6baca41` |
| `Extent::intersection`, single boxes (metadata.cpp:889-910) | `model.rs` `BoundingBox::extent_intersection`, used by `factory.rs` `intersect_extents` | ported, commit `6baca41` |
| `Private::Context(sourceCRSExtent, targetCRSExtent, context)` (coordinateoperationfactory.cpp:8082-8094) | `factory.rs` `create_operations_between_epochs`, `inherited_extents` | fixed, commit `8fc3e7e` |
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
| `TEST(metadata, extent)` (test_metadata.cpp:66-280), box cases about the antimeridian | `tests/unit_metadata_extent.rs` `metadata_extent_antimeridian_boxes` | passes |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_3328)` (test_operationfactory.cpp:6643) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_3328` | context authority `""` (commit `c2c901d`), passes |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_4550)` (test_operationfactory.cpp:7063) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_4550` | unchanged, passes; the port now gives the one candidate of PROJ |
| `projinfo -s EPSG:5720 -t EPSG:4965 -o PROJ` (test/cli/test_projinfo.yaml:1276) | `tests/cli_yaml/test_projinfo.yaml:1276` (copied from PROJ) | existing; the finder must keep it |

No new test is committed for the registry port: PROJ has no `TEST()` for the candidate list of
`EPSG:5720` -> `EPSG:4979`. The pairs to compare are in the table of the reference section; the
coordinator probed them (see "Measured by the coordinator").

## Step 1: box intersection (commit `6baca41`)

`BoundingBox::intersection` is `GeographicBoundingBox::Private::intersection` (metadata.cpp:423-493),
line for line: a box that crosses the antimeridian is split and its larger part is kept, and two
boxes that only touch in longitude do not intersect (`resW < resE`). `BoundingBox::extent_intersection`
(`pub(crate)`) is the `Extent::intersection` shortcut of metadata.cpp:889-910, used by the extent-level
callers (`select.rs` `pseudo_area_of`, `factory.rs` `intersect_extents` and `SortExtents::new`).
Test: `tests/unit_metadata_extent.rs`, the antimeridian cases of `TEST(metadata, extent)`.

Before, the intersection was a min/max. For `EPSG:32610+5703` (UTM zone 10N, -126..-120, with NAVD88,
172.42..-66.91 across the antimeridian) it gave the box 172.42..-120 where PROJ gives -126..-120.

## Step 2: cause of `issue_4550` (commit `8fc3e7e`)

The previous attempt found that step 1 alone reopens `issue_4550`. The cause is not in step 1.

What PROJ does, from a build with `-DTRACE_CREATE_OPERATIONS -DENABLE_TRACING` (`PROJ_TRACE_FILE`)
and from `projinfo`:

1. The source of the test is a WKT1 compound whose horizontal part has `TOWGS84`: a `BoundCRS`.
   `CRS::getResolvedCRS` (coordinateoperationfactory.cpp:8219-8360) resolves geographic, projected and
   compound CRS only, so the extent of the source is **unknown**. Check: `--crs-extent-use intersection`
   gives 330 candidates (no area of interest, which needs one unknown extent), and `--bbox` on the
   extent of EPSG:2927 gives 30 candidates that use GEOID18.
2. The area of interest (`SMALLEST`, `computeAreaOfInterest`, 1263-1291) is then the extent of the
   target, -126..-120 by 14.51..71.4. No CONUS geoid operation contains it (their north is 49.38).
3. PROJ builds its private context once (`Private::Context(sourceCRSExtent, targetCRSExtent, context)`,
   8082-8094) and every registry lookup of every sub-call filters on those two extents
   (`FilterResults` at 1953, 2021 and 2279). The trace shows `findOpsInRegistryDirectTo({any} -->
   NAVD88 height)`: `filtering reduced from 7 to 0`, in each sub-call.
4. So `WGS 84 3D -> target` and `NAD83(HARN) 3D -> target` start with a ballpark operation,
   `hasNonTrivialTargetTransf` is false (7485-7487), the intermediate branch leaves `res` empty, and the
   vertical-to-vertical fallback (7653-7656) gives the one candidate of the test.

What the port did:

- `create_operations_between_epochs` set `inherited_extents` only when resolving a CRS had changed
  an extent. Here nothing changes (the source is unknown, the target is unchanged), so each sub-call
  took the extents of **its own** CRS (`crs_extents_of`).
- With that, on `main`, the pair gives 310 candidates. The first applies `NAD83(HARN) to NAVD88 height
  (9)` (Alaska) and then its inverse; the two cancel in the PROJ string, which is why the test passed.
- With step 1 alone, the target extent becomes right, and the list is empty.

The fix: the extents of the outermost pair are always inherited. Step 3 is needed too: without the
`FilterResults` of `findOpsInRegistryDirectTo`, the leg `WGS 84 3D -> target` keeps 111 non-ballpark
candidates in the port (measured with a temporary trace) where PROJ has only the ballpark one.

Still different from PROJ on this pair, not in this lot: the one candidate is named `Inverse of
unknown + Conversion from NAVD88 height (ftUS) to NAVD88 height + NAD83(HARN) to WGS 84 (1) + UTM zone
10N` with the extent of `NAD83(HARN) to WGS 84 (1)`, where PROJ has `Inverse of unknown +
Transformation from NAD83(HARN) to WGS84 + Conversion ... + UTM zone 10N` with the extent World: PROJ
keeps the `TOWGS84` transformation of the bound source, the port takes the registered one. The PROJ
strings are the same. `EPSG:4957` -> `EPSG:5703` also differs: 0 candidates here, 1 (the ballpark)
in PROJ, on `main` as on this branch.

Callers at risk of `8fc3e7e`: every sub-call of every pair whose CRS extents were not changed by
the resolution, which is most pairs. The full suite is the measure (table above).

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

Measured: see "Measured by the coordinator".

## Step 4: test context of `issue_3328` (committed)

`operation_compound_to_compound_issue_3328` built its context with the authority `"EPSG"`. The C++
`TEST()` builds its authority factory with `std::string()` (test_operationfactory.cpp:6644-6645).
The Rust test now uses `""` (commit `c2c901d`). Run by the coordinator: it passes.

## Existing callers at risk (summary)

- `model.rs` `BoundingBox::intersection`: commit `6baca41`.
- `select.rs` `pseudo_area_of`, `factory.rs` `intersect_extents`, `SortExtents::new`, `has_too_small_areas`,
  `operation_extent`, `crs_extent`, `derived.rs` `joint_extent`, `compound.rs` `compute_metadata`: commit
  `6baca41`.
- `factory.rs` registry block 6120-6127 and `compound.rs` `vertical_geog_fallback`,
  `geog_to_vertical_alternative`: commit `f35d484`.
- Every sub-call of `create_operations`: commit `8fc3e7e` (the inherited extents).

## Deliberate deviations (committed)

See step 3 (1 to 4).

## Commands run

- The reference: `tools/build-reference.sh` (PROJ 9.9.0), `projinfo` with the options of the reference
  section; a second build of the same sources with `-DCMAKE_CXX_FLAGS="-DTRACE_CREATE_OPERATIONS
  -DENABLE_TRACING"` and `PROJ_TRACE_FILE=<file>` for the trace of step 2.
- `cargo test -p proj-rs --test unit_op_compound_to_compound_2 --test unit_metadata_extent --test
  unit_op_compound_derived`, then `cargo test -p proj-rs --all-features --no-fail-fast`.
- `cargo clippy -p proj-rs --all-targets --all-features -- -D warnings`, `cargo fmt --check`.
- Probes (throwaway tests, not in the tree) printed the candidate count, names and extents of the
  pairs of "Measured by the coordinator".

Steps 3 and 4 and the patch of step 1 were written by a port agent (Claude Haiku 5.5) that could not run
them. The coordinator session traced the cause of `issue_4550` with the reference build, wrote step 2,
and ran the tests.