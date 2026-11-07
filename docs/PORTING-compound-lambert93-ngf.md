# Port — `Lambert-93 + NGF-IGN69 height` to `WGS 84` 3D (lot `compound-lambert93-ngf`)

Lot `compound-lambert93-ngf`, branch `lot-compound-lambert93-ngf`, base
`origin/unit-fix-compound-derived-2` (PR #166, c1b8f40: `createOperationsCompoundToGeog` ported
whole, `docs/PORTING-unit-compound-derived.md`). Source: PROJ 9.9.0,
`src/iso19111/operation/coordinateoperationfactory.cpp`, `src/iso19111/factory.cpp`,
`src/iso19111/metadata.cpp`, `src/iso19111/operation/singleoperation.cpp`,
`src/iso19111/operation/oputils.cpp`, traced with the `PROJ_DEBUG=3` build.

Status: **partly closed.** The `X using Y` names are fixed. The two extra candidates are
understood (supersession in `findOpsInRegistryDirectTo`) and a port of the rule exists, but it
reopens closed unit tests through two neighbouring gaps, so it is not committed (two attempts,
see "Open: the superseded `(4)` candidates").

## Target

`projinfo -s EPSG:2154+5720 -t EPSG:4979 --summary --spatial-test intersects`, both binaries on
the same `proj.db` (`PROJ_DATA=oracle-db`).

| | PROJ 9.9.0 | base | this lot |
|---|---|---|---|
| candidates | 6 | 8 | 8 |
| 1st and 3rd names | `Inverse of Lambert-93 + Inverse of ETRS89-FRA [RGF93 v2] to NGF-IGN69 height (3) using ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1)`, the same with `(2)` | the expanded chain `... RGF93 v1 to v2 (1) + Inverse of ... (3) + RGF93 v2 to WGS 84 (1)` | as PROJ |
| `RGF93 v2b to NGF-IGN69 height (4)` | absent | 2 extra candidates (through `RGF93 v2`, through `ETRS89`) | still there |
| `-o PROJ` pipelines of the PROJ candidates | | same as PROJ | same as PROJ |
| `cs2cs` `652301 6861302 100` | `48.849993430 2.349992385 100.000000000` | same | same |

`projinfo -o PROJ` in the port never writes PROJ's `Grid ... needed but not found on the system`
lines (any pair); that is not this lot.

## Commits

### 1. `SingleOperation::_isEquivalentTo` compares no accuracy

`createHorizVerticalHorizPROJBased` (coordinateoperationfactory.cpp:2795-2880) names the operation
`V using X` when `opGeogCRStoDstCRS->inverse()->_isEquivalentTo(opSrcCRSToGeogCRS, EQUIVALENT)`.
For the target, the first leg is `ETRS89-FRA [RGF93 v1] to ETRS89-FRA [RGF93 v2] (1)`
(EPSG:9882, geocentric translations geog2D, zero values, 0.05 m) and the last is `ETRS89-FRA
[RGF93 v2] to WGS 84 (1)` (EPSG:9791) promoted to 3D (geocentric translations geog3D, zero
values, 1 m). The method codes differ, both are `isTOWGS84Transf`, their TOWGS84 values agree:
`SingleOperation::_isEquivalentTo` (singleoperation.cpp:1543-1582) returns true. It compares no
accuracy, in that branch or in the parameter branch (1617-1717). The port required equal
accuracies under `Equivalent`; it now compares them only under `Strict` (unchanged there).

Test: `compound_lambert93_ngf::lambert93_ngf_ign69_to_wgs84_3d_using_names` (PROJ's first name,
PROJ's third name, first PROJ string). Also changes, toward PROJ, `EPSG:4171+5720` to `EPSG:4979`
(same two names).

### 2. Inverse name of a PROJ-based operation

`PROJBasedOperation::inverse` (projbasedoperation.cpp:157-190) names the inverse with
`createPropertiesForInverse` (oputils.cpp:231-316). A PROJ-based operation is neither a
`Transformation` nor a `ConcatenatedOperation`, so a name that joins steps with ` + ` (or starts
with `Inverse of`) is split and each part inverted, in reverse order. The port models it as a
concatenated operation and only toggled one leading `Inverse of`:
`ConcatenatedOperation::inverse` now calls `inverse_operation_name` with
`NamedOperationKind::ProjBased` when `proj_based` is set and the name is not computed. Case found
with commit 3 of the attempt below: the null-horizontal `HorizVertical` of `ETRS89 + EVRF2007
height` to `ETRS89` 3D, vertical part `Inverse of DHHN2016 height to EVRF2007 height (1) +
Inverse of ETRS89 to DHHN2016 height (1)`, whose inverse PROJ names `ETRS89 to DHHN2016 height
(1) + DHHN2016 height to EVRF2007 height (1)` and the base `DHHN2016 height to EVRF2007 height (1)
+ Inverse of ETRS89 to DHHN2016 height (1)`. On the committed tree no measured output changes.

### 3. `Vertical Offset and Slope` export

`Transformation::_exportToPROJString` for EPSG:1046 (singleoperation.cpp:4255-4311): the linear
unit of each vertical CRS around `+proj=vertoffset +lat_0 +lon_0 +dh +slope_lat +slope_lon`
(degrees, metres, arc-seconds), a compound pair with equivalent horizontal parts reduced to its
vertical parts. The port answered `Unimplemented`. Without it the `DHHN2016` / `LN02` chains to
`EVRF2007 height` cannot be exported, so `removeDuplicateOps` cannot drop their copies. Test:
`compound_lambert93_ngf::vertical_offset_and_slope_exports_vertoffset` (EPSG:7838, the PROJ
string of `projinfo urn:ogc:def:coordinateOperation:EPSG::7838 -o PROJ`).

## Open: the superseded `(4)` candidates

### What PROJ does (trace)

The vertical transforms PROJ considers (`Considering vertical transform`) are `(3)`, `(5)`, `(2)`,
`RGF93 v1 (1)` and the ballpark: no `(4)`. They come from `createOperations(NGF-IGN69 height ->
WGS 84 3D)`, whose `createOperationsGeogToVertWithAlternativeGeog` reads
`findOpsInRegistryDirectTo({any} -> NGF-IGN69 height)` (coordinateoperationfactory.cpp:1973-2033).
That calls `createFromCoordinateReferenceSystemCodes` with an empty source, `discardSuperseded`,
`tryReverseOrder` and `reportOnlyIntersectingTransformations` (factory.cpp:7205-7548). The
supersession table has EPSG:9786 (`(4)`) replaced by EPSG:9876 (`(5)`), same source and target,
known grid: `(4)` is not returned. `projinfo -s EPSG:5720 -t EPSG:4979` lists 4 candidates in PROJ,
5 in the port. The port's `geog_to_vertical_alternative` reads every row of
`coordinate_operation_view` to the vertical CRS, in table order, with no supersession and no
extent rule.

### Attempt 1: port `findOpsInRegistryDirectTo`

`registry_operations_to` (candidate authorities, both directions with the reverse rows inverted,
sorted by area and accuracy, superseded rows dropped, rows not meeting the CRS extents dropped,
missing-grid filter) over a `registered_operations` that takes an optional source. The target and
`EPSG:5720` to `EPSG:4979` then match PROJ exactly (6 and 4 candidates, names and pipelines), and
`EPSG:2154+5720` / `EPSG:4171+5720` to `EPSG:7423` reach PROJ's `DHHN2016` and `LN02` chains (with
commits 2 and 3, exactly PROJ's output). Reopened:

- `unit_op_compound_derived::operation_compound_to_geog_2d_promote_to_3d_context`: 131 instead
  of 133. Compared name by name with the reference libproj (`proj_create_operations`, authority
  `EPSG`, partial intersection, grids ignored): the base had 133 by chance, 3 extra
  (`... NAD83(2011) to NAVD88 height (1) using NAD83 to NAD83(2011) (1)`, superseded by `(3)`, and
  two ballpark sandwiches) and 3 missing (`... NAD83(NSRS2007) to NAVD88 height (1|2) using NAD83
  to WGS 84 (1|6) + Inverse of NAD83(NSRS2007) to WGS 84 (1)`). The attempt removes 2 of the
  extra ones; the 3 missing remain.
- `unit_op_compound_to_compound_2::operation_compound_to_compound_issue_3328`: the Rust test
  uses the authority `EPSG` where the C++ `TEST()` uses `std::string()`; with `""` it passes. With
  `EPSG`, `PROJ:EPSG_9123` (`NAD83(CSRS) to CGVD28 height (1)`) is not read, as
  `getCandidateAuthorities` says. The test context should follow the C++ one.

### Attempt 2: `GeographicBoundingBox::Private::intersection`

The 3 missing NSRS2007 candidates come from `NAD83` to `NAD83(NSRS2007)`: the reference lists 56,
the port 54, without `NAD83 to WGS 84 (1|6) + Inverse of NAD83(NSRS2007) to WGS 84 (1)`. Their
extent is `None` in the port: `BoundingBox::intersection` does not handle a box crossing the
antimeridian (`NAD83(NSRS2007) to WGS 84 (1)`, 167.65..-63.88), so the pair counts as having no
extent and is filtered. Porting metadata.cpp:423-493 (split of the crossing box, larger part kept,
strict `resW < resE`) gives exactly PROJ's 56 operations in PROJ's order and closes the 133 test,
but reopens `unit_op_compound_to_compound_2::operation_compound_to_compound_issue_4550`: the
target `EPSG:32610+5703` now has an extent (UTM 10N meets the antimeridian-crossing NAVD88 box),
the candidate set changes (470 instead of 808 before the filter) and PROJ's only candidate
(`Inverse of unknown + Transformation from NAD83(HARN) to WGS84 + Conversion from NAVD88 height
(ftUS) to NAVD88 height + UTM zone 10N`) is no longer built. Not traced further.

Both attempts are reverted. The code is kept as a patch in the scratch directory of the session
(`attempt-registry-to.patch`), not in the tree.

### Next step

1. The bounding box intersection of PROJ (attempt 2), with the `issue_4550` path traced: which
   branch builds the bound-CRS candidate in PROJ when the compound target has an extent.
2. Then `findOpsInRegistryDirectTo` (attempt 1), and the `issue_3328` test context set to the C++
   one (authority `""`).

## Function map

| PROJ 9.9.0 | Rust |
|---|---|
| `SingleOperation::_isEquivalentTo`, singleoperation.cpp:1489-1750 | `buildop.rs` `Transformation::is_equivalent_to_with` |
| `PROJBasedOperation::inverse`, `createPropertiesForInverse` | `buildop.rs` `ConcatenatedOperation::inverse`, `geodetic_ops.rs` `inverse_operation_name` (`NamedOperationKind::ProjBased`) |
| `Transformation::_exportToPROJString`, `EPSG_CODE_METHOD_VERTICAL_OFFSET_AND_SLOPE` | `transform_export.rs` `vertical_offset_and_slope` |
| `findOpsInRegistryDirectTo` | not ported (attempt 1) |
| `GeographicBoundingBox::Private::intersection` | not ported (attempt 2); `model.rs` `BoundingBox::intersection` is a plain min/max |

## Neighbours (reference `projinfo`, `--summary` and `-o PROJ`, both spatial tests)

| pair | result on the committed tree |
|---|---|
| `EPSG:2154+5720` -> `EPSG:4979` | names 1 and 3 as PROJ, still 8 vs 6 |
| `EPSG:4171+5720` -> `EPSG:4979` | names 1 and 3 as PROJ (were expanded), still 8 vs 6 |
| `EPSG:5720` -> `EPSG:4979` | unchanged (5 vs 4, `(4)`) |
| `EPSG:2154+5720` -> `EPSG:7423` | unchanged from the base |
| `EPSG:27700+5701` -> `EPSG:4979` | same as PROJ, unchanged |
| `EPSG:28992+5709` -> `EPSG:4937` | same as PROJ, unchanged |

## Tests

| Test | Status |
|---|---|
| `compound_lambert93_ngf::lambert93_ngf_ign69_to_wgs84_3d_using_names` | new, passes |
| `compound_lambert93_ngf::vertical_offset_and_slope_exports_vertoffset` | new, passes |
| full candidate list of the target (6 names) | not committed: fails (the `(4)` candidates) |
