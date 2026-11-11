# Port — bounding-box intersection and `findOpsInRegistryDirectTo` (lot `lot-registry-direct-to`)

Lot `lot-registry-direct-to`, branch `lot-registry-direct-to`. Source: PROJ 9.9.0,
`src/iso19111/metadata.cpp`, `src/iso19111/operation/coordinateoperationfactory.cpp`,
`src/iso19111/operation/oputils.cpp`, `src/iso19111/factory.cpp`, `test/unit/test_metadata.cpp`,
`test/unit/test_operationfactory.cpp`.

Follows `docs/PORTING-compound-lambert93-ngf.md` ("Open: the superseded `(4)` candidates").

Status: **status sheet, partly closed.** Step 4 (test context of `issue_3328`) is committed.
Steps 1 to 3 are not committed: the box intersection port is written, compiles, and is kept as a
patch, because the previous attempt measured that it reopens `issue_4550`. The trace of
`issue_4550` stops at the candidate construction and the extent algebra; the cause is not found.
`findOpsInRegistryDirectTo` is not ported.

## Target

`projinfo -s EPSG:5720 -t EPSG:4979` lists 4 candidates in PROJ 9.9.0 and 5 here (the superseded
`(4)` kept). `EPSG:2154+5720` to `EPSG:4979`: 6 in PROJ, 8 here. Not changed by this lot.

## What PROJ does (read)

### 1. `GeographicBoundingBox::Private::intersection`, metadata.cpp:422-493

- lines 433-436: no intersection when `N < oS || S > oN`.
- lines 440-449: a world box (`W == -180 && E == 180`) against a box crossing the antimeridian
  (`oW > oE`) returns `(oW, max(S,oS), oE, min(N,oN))`; the symmetric case at 447-450.
- lines 453-462: normal boxes, `resW = max(W,oW)`, `resE = min(E,oE)`, returned only when
  `resW < resE` (strict; a zero-width overlap is `nullptr`).
- lines 464-483: bail out when `oW > 180 || oE < -180`; otherwise the two parts
  `(oW..180)` and `(-180..oE)` are intersected with `this`, and the one with the larger width is
  kept (`>` strict, so the second part wins a tie).
- lines 485-491: this box crosses, `oW <= oE`: `otherExtent.intersection(*this)`; both cross:
  `(max(W,oW), max(S,oS), min(E,oE), min(N,oN))`, returned without check.

### 2. `Extent::intersection` for single boxes, metadata.cpp:889-910

`contains(other)` returns `other`; `other->contains(self)` returns `self`; otherwise the box
intersection of 1. An empty result is `nullptr`.

### 3. `GeographicBoundingBox::contains` and `intersects`, metadata.cpp:284-331 and 336-392

Compared with `model.rs` `BoundingBox::contains` (model.rs:246-265) and `intersects`
(model.rs:269-304): **identical** branch by branch. No change needed.

### 4. Extents of steps and operations

- `getExtent(ops, ...)`, oputils.cpp:529-555: intersection of the sub-extents with
  `Extent::intersection`; a null intersection sets `emptyIntersection` and returns `nullptr`; a
  sub-extent that is null is skipped unless `emptyIntersection` is already set.
- `getExtentPossiblySynthetized`, oputils.cpp:502-525: a compound CRS takes the intersection of
  its components' extents.

### 5. Candidate filter, coordinateoperationfactory.cpp:1380-1415 (`FilterResults`)

`PARTIAL_INTERSECTION`: keeps an operation whose extent intersects the source and target CRS
extents (`Extent::intersects`, metadata.cpp:861-879); lines 1392-1398 mark an operation that
contains both extents without grids.

### 6. Compound to compound, coordinateoperationfactory.cpp:7214-7612

- 7228-7243: intermediate geographic CRSs promoted to 3D.
- 7290-7324: the hub path for two bound CRS components.
- 7326-7396: vertical transforms; `bTryThroughIntermediateGeogCRS` is set when the vertical
  transform is ballpark or when a non-ballpark 3D geographic path exists (7386-7393).
- 7398-7446: `createWithIntermediateCRS`.
- 7452-7545: `opsSrcToGeog` x `opsGeogSrcToGeogDst` x `opsGeogToTarget`, two passes (ballpark
  excluded first), each candidate built by `ConcatenatedOperation::createComputeMetadata(...,
  disallowEmptyIntersection())` (7517-7527). **The candidate of `issue_4550`
  (`Inverse of unknown + Transformation from NAD83(HARN) to WGS84 + Conversion from NAVD88
  height (ftUS) to NAVD88 height + UTM zone 10N`) is built here**: op1 is the inverse of the
  Lambert projection of the source, op2 the NAD83(HARN) to WGS 84 transformation, op3 the vertical
  conversion of `NAVD88 height (ftUS)` merged with the UTM projection.
- 7547-7612: `createOpsInTwoSteps`.

## Map PROJ function -> Rust function

| PROJ 9.9.0 | Rust | Status |
|---|---|---|
| `GeographicBoundingBox::Private::intersection` (metadata.cpp:422-493) | `model.rs` `BoundingBox::intersection` (min/max, model.rs:318-330) | Port written (patch), not committed |
| `Extent::intersection`, single boxes (metadata.cpp:889-910) | `factory.rs` `intersect_extents` (contains shortcut, then box intersection) | unchanged |
| `GeographicBoundingBox::contains` / `intersects` | `model.rs` `contains` / `intersects` | identical |
| `getExtent(ops)` (oputils.cpp:529-555) | `derived.rs` `joint_extent` (derived.rs:166-175) | see open point 2 |
| `getExtentPossiblySynthetized`, compound (oputils.cpp:502-525) | `factory.rs` `crs_extent` (factory.rs:6463-6479) | unchanged |
| `FilterResults`, PARTIAL_INTERSECTION (coordinateoperationfactory.cpp:1380-1415) | `factory.rs` filter, `intersects` on the box (factory.rs:7387-7410, 7627-7775) | unchanged |
| `createOperationsCompoundToCompound` (7214-7612) | `compound.rs` through-intermediate decision (699-733), `compute_metadata` (1063-1087) | unchanged |
| `findOpsInRegistryDirectTo` (1973-2033, factory.cpp 7205-7548) | not ported | open |

## Map PROJ `TEST()` -> Rust test

| PROJ `TEST()` | Rust test | Status |
|---|---|---|
| `TEST(metadata, extent)` (test_metadata.cpp:66-280), box cases about the antimeridian | none committed (the port is in the patch) | open, see the patch |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_3328)` (test_operationfactory.cpp:6643) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_3328` | context authority `""` (step 4) |
| `TEST(operation, compoundCRS_to_compoundCRS_issue_4550)` (test_operationfactory.cpp:7063) | `tests/unit_op_compound_to_compound_2.rs` `operation_compound_to_compound_issue_4550` | unchanged, reopened by the port (see below) |

## Step 1: box intersection (not committed)

The port is in `port-scratch\lot-registry-direct-to\step1-box-intersection.patch`, with the test
`unit_metadata_extent.rs` (same directory). The patch includes:

- `BoundingBox::intersection` ported whole (metadata.cpp:422-493), no containment shortcut;
- `BoundingBox::extent_intersection` (`pub(crate)`): the `Extent::intersection` shortcut
  (contains, then box intersection), used by the Extent-level callers so that their behaviour does
  not change;
- `select.rs` `pseudo_area_of` and `factory.rs` `SortExtents::new` switched to `extent_intersection`.

It compiled with `cargo check -p proj-rs --tests` (devhub job 3824, succeeded). Its test was not
run and is not committed. The expected values of the test are the boxes that PROJ's `equals`
checks in `TEST(metadata, extent)`, derived by hand from the C++ code above.

Why not committed: `issue_4550` is reopened by this port. The previous attempt
(`docs/PORTING-compound-lambert93-ngf.md`, "Attempt 2") measured it: the target `EPSG:32610+5703`
gets an extent (UTM 10N meets the antimeridian-crossing NAVD88 box), 470 candidates instead of
808 before the filter, and PROJ's only candidate is no longer built. The rule "a lot never
reopens a case" applies, so the port is not committed alone. This measurement was not repeated
here: no test was run.

## Step 2: trace of `issue_4550` (open)

Read so far, without running anything:

1. The candidate is built in the two-pass loop of 7452-7545 (`createComputeMetadata` with
   `disallowEmptyIntersection`). Its extent is the intersection of op1, op2, op3 (metadata.cpp
   `Extent::intersection` through oputils.cpp:529-555).
2. Inference from the code, not measured: before the port, the extent of the target
   `EPSG:32610+5703` is the intersection of UTM 10N (lon -126..-120) and the NAVD88 box, which
   crosses the antimeridian; the min/max box intersection gives an empty result, so the compound
   has no extent and the filter is skipped. Measured by the previous attempt: with the port, the
   compound has an extent (-126..-120), the filter runs (1388-1415), and the candidate is lost.
3. PROJ keeps the same candidate with the same extents. So one of the following differs between
   the two sides, and this is the open question: the extent of op1 (Washington South), of op2
   (NAD83(HARN) to WGS 84), of op3 (vertical conversion merged with UTM; in PROJ the intersection
   of the NAVD88 box with UTM 10N through the crossing rule of metadata.cpp:483-491), or the way
   `joint_extent` (derived.rs:166-175) handles a step without an extent, which differs from
   `getExtent(ops)` (oputils.cpp:537-542) when `emptyIntersection` is set.

Next step: a per-candidate dump of op1, op2, op3 extents and of the joint extent, on both sides
(reference `projinfo` with `PROJ_DEBUG`, and `proj-rs` on the same candidate list), before any
change. Not done here: the coordinator would run it.

## Step 3: `findOpsInRegistryDirectTo` (open, not ported)

Not started in this lot beyond the reading recorded in
`docs/PORTING-compound-lambert93-ngf.md`. The reading of coordinateoperationfactory.cpp:1973-2033
and factory.cpp:7205-7548 is the next step, after step 2.

## Step 4: test context of `issue_3328` (committed)

`operation_compound_to_compound_issue_3328` built its context with the authority `"EPSG"`. The C++
`TEST()` builds its authority factory with `std::string()` (test_operationfactory.cpp:6644-6645).
The Rust test now uses `""`, as the C++ test does. The previous attempt's note says that with `""`
the test passes and with `EPSG` the `PROJ:EPSG_9123` candidate is not read. Not run here.

## Existing callers at risk

These callers would change with the step 1 port (box-level results change for boxes that cross the
antimeridian and for zero-width overlaps). None changed in the committed tree.

- `model.rs` `BoundingBox::intersection`: definition.
- `select.rs` `pseudo_area_of` (select.rs:661-686): three box intersections.
- `factory.rs` `intersect_extents` (factory.rs:6508-6521): used by `operation_extent`
  (6481-6503), `crs_extent` (6463-6479), `joint_extent` (derived.rs:166-175),
  `has_too_small_areas` (7857-7909), and the candidate extent checks.
- `factory.rs` `SortExtents::new` (factory.rs:6567-6600): the area of interest of
  `computeAreaOfInterest`.
- `compound.rs` `compute_metadata` (1063-1087) and the two `disallows_empty_intersection` checks
  (1279, 1399): through `joint_extent`.
- `tests/unit_op_compound_to_compound_2.rs` `issue_4550`: reopened by the port (measured by the
  previous attempt).

## Deliberate deviations

None committed. The step 4 change follows the C++ test; it is not a deviation.

## Commands run

- `node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests`
  on the step 1 tree: succeeded (job 3824). On the committed tree: see the final message.
- `cargo fmt`.

Tests written, not run by the author: the coordinator runs them.
