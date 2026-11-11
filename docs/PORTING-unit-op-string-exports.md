# Porting sheet: three PROJ strings of concatenated operations (status, lot `unit-op-string-exports`)

Branch `unit-op-string-exports`, base `origin/main`. Status sheet: the three rows stay open.
No code is changed and no test is added by this lot. The drafts were not written because no
cause was found in the C++ that a port could follow, and a draft that fails is not committed
(lot rules). Nothing was run: the reference `cs2cs`/`projinfo` was not built and no test was run.

Open rows of `docs/UNPORTED-UNIT.md`, all three still listed there:

| PROJ `TEST()` | Ours | PROJ 9.9.0 |
|---|---|---|
| `operation.nadgrids_with_pm` | 25 tokens, with `+step +proj=longlat +ellps=intl +pm=lisbon` after `+inv +proj=tmerc` | 21 tokens, no longlat step |
| `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | first `unitconvert` is `+xy_in=deg +xy_out=rad` | `+xy_in=deg +z_in=m +xy_out=rad +z_out=m` |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | `+proj=unitconvert +z_in=us-ft +z_out=m` | `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` |

## Method

- Read the C++ tests (`test/unit/test_operationfactory.cpp`) and the C++ functions named below.
- Read the Rust counterpart of each C++ function listed in the same row.
- Not done: running any of the tests, a probe, the reference build. These are the next steps.

## Row 1: `operation.nadgrids_with_pm` (test_operationfactory.cpp:4044-4098)

### PROJ functions read

- `BoundCRS::createFromNadgrids`: crs.cpp:5988-6021. Sets the source of the NTv2 to a copy of the
  base geographic CRS with the Greenwich prime meridian (`(with Greenwich prime meridian)`) when
  the base prime meridian is not 0.
- `io.cpp` `PROJStringParser::Private::buildBoundOrCompoundCRSIfNeeded`: io.cpp:11761-11800.
- `createOperations` -> `createOperationsBoundToGeog`, first branch (hub equivalent to the target):
  coordinateoperationfactory.cpp:5625-5685 (`opIntermediate` = first operation from the base
  geographic CRS to the transformation source; candidate = `opFirst`, `opIntermediate`, binding
  transformation). The early return for a known datum: 5612-5623.
- `createOperationsGeodToGeod` (5236-5283) / `createOperationsGeogToGeog` (2922-3106).
  Prime-meridian branch: 3030-3040 (ellipsoids equivalent, prime meridians differ: one longitude
  rotation).
- `isSameGeodeticDatum`: coordinateoperationfactory.cpp:2056-2066 (a datum named `unknown` is never
  the same as a named one).
- `ConcatenatedOperation::createComputeMetadata`: concatenatedoperation.cpp:674-765.
- `LongitudeRotation` export: singleoperation.cpp:4811-4877.
- `PrimeMeridian::getPROJStringWellKnownName`: datum.cpp:514-528 (tolerance 1e-10 rad).
- `io.cpp` PROJ string simplification: io.cpp:8895-9139 (longlat with ellipsoid only is a no-op,
  inverse rules, unit combination).

### Rust functions read

- `bound_ops.rs:304-453` `bound_to_geog` (branch `hub_geog ~ geog_dst`: lines 338-349),
  `bound_ops.rs:545-561` `intermediate_operation`.
- `geodetic_ops.rs:512-531` `prime_meridian_offset`, `geodetic_ops.rs:645-689` (prime-meridian
  branch, same order as C++ 3030-3099), `geodetic_ops.rs:717-731` `longitude_rotation`.
- `transform_export.rs:2437-2488` `longitude_rotation`: same branches as
  singleoperation.cpp:4844-4874 (`start_inversion`, well-known name or inverted `longlat` step).

### Finding

- As read, the C++ candidate is `[inv tmerc (projected base -> geographic base, pm lisbon),
  longitude rotation (base lisbon -> Greenwich copy), NTv2]`, with the rotation written as a
  `longlat` step. The Rust port builds the same three operations and exports the rotation with the
  same branches (`offset`, `well_known_pm_name`, inversion). Both produce the rotation step.
- The expected PROJ string has no longlat step, and its `+pm=lisbon` is on the tmerc step. The
  C++ test comments the absence (`// Check that there is no extra +step +proj=longlat +pm=lisbon`).
- Cause not identified: which PROJ mechanism leaves out the rotation. Three things were not
  checked: (a) whether, in PROJ, `createOperations(lisbon geographic, Greenwich copy)` returns an
  operation other than the rotation, or the same one with an identity offset; (b) the sign and the
  inversion state of the rotation in the export (the Rust output shows the non-inverted form, which
  is the `well_known_pm_name` branch; the sign of the offset compared there is worth checking
  against PROJ's `getPROJStringWellKnownName(-offset)`, not verified); (c) whether the simplification in
  io.cpp:8895-9139 can merge the rotation into the tmerc `+pm`.
- Next step: print PROJ's operation for EPSG:4326 from the test's source with `projinfo`
  (the reference build of `tools/build-reference.sh`), and compare its steps with the Rust list of
  operations before export. Then decide which of (a), (b), (c) applies. No code change is proposed
  before that.

### Callers at risk

None: no shared code is changed by this lot.

## Row 2: `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` (test_operationfactory.cpp:4013-4040)

### PROJ functions read

- `Transformation::_exportToPROJString` for `Geographic3D to GravityRelatedHeight`:
  singleoperation.cpp:4705-4770. The horizontal part of the source is written with
  `pushOmitZUnitConversion()` around `addAngularUnitConvertAndAxisSwap` (4733-4736 and 4763-4764),
  so the geoid step writes no `z_in`/`z_out`.
- `GeographicCRS::addAngularUnitConvertAndAxisSwap`: crs.cpp:2446-2475 (as the sheet of the
  `strings-misc` lot records).
- `ConcatenatedOperation::_exportToPROJString`: concatenatedoperation.cpp:919-949 (no simplification).
- `PROJStringFormatter::toString` simplification: io.cpp:8895-9139. The rule at 9106-9139 merges a
  `unitconvert z_in z_out` step followed by a `unitconvert xy_in xy_out` step into one step
  `xy_in z_in xy_out z_out`. That is the form of the expected first step.

### Rust functions read

- `transform_export.rs:652-703` `geoid_grid_shift`: same structure as singleoperation.cpp:4733-4764.
- `transform_export.rs:1270-1284` `horizontal_unit_and_axis_swap`: always writes `xy_in=rad`,
  `xy_out`, and the axis swap, no `z` terms.
- `pipeline.rs:787-905` and `pipeline.rs:960-975`: the unit-pair rules of the formatter, including
  the merge of a z step with an xy step (the second range was seen only through a grep).

### Finding

- The expected first step `xy_in=deg z_in=m xy_out=rad z_out=m` is a merge of a z-only step
  `z_in=m z_out=m` with the xy step. Our output has only the xy step. So the candidate either does
  not contain the z-only step, or it does and it is dropped.
- The geoid path (singleoperation.cpp:4733-4738) writes its horizontal part with z omitted, and the
  Rust `geoid_grid_shift` does the same (`horizontal_unit_and_axis_swap` writes no z). So the z-only
  step does not come from the geoid part.
- Cause not identified: which operation of the candidate writes the z-only step. The second
  operation of `list` in the test (the `+nadgrids` horizontal part and the geoid) was not traced.
- Next step: print the steps of PROJ's candidate for EPSG:4937 to the test target (the `projinfo`
  reference), and find the operation whose PROJ string has `z_in=m z_out=m`. Then check whether the
  Rust candidate has the same operation and whether its export drops it.

### Callers at risk

None: no shared code is changed by this lot.

## Row 3: `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` (test_operationfactory.cpp:3423-3459)

The test's source is a bound CRS of a vertical CRS (US survey foot) with hub NAD83 (2D geographic,
EPSG:4269), target EPSG:4979 (3D WGS 84). The context has `IGNORE_GRID_AVAILABILITY` and
`PARTIAL_INTERSECTION`; `skipHorizontalTransformation` is not set (it is set only by
`createOperationsCompoundToGeog`, coordinateoperationfactory.cpp:6751-6756).

### PROJ functions read

- `createOperationsBoundToGeog`: coordinateoperationfactory.cpp:5578-5953. Branches for this
  input, in order:
  - 5612-5623 (base datum equivalent to the target datum): not taken, the base has no geographic CRS.
  - 5627-5685 (hub equivalent to the target): not taken, the hub is 2D NAD83, the target is 3D WGS 84.
  - 5737-5785 (Clarke 1866): not taken.
  - 5787-5812 (hub equivalent to the target, base vertical): not taken, same reason.
  - 5814-5848 (`!triedBoundCrsToGeogCRSSameAsHubCRS && hubSrcGeog && geogCRSOfBaseOfBoundSrc`): not
    taken, the base has no geographic CRS.
  - 5856-5948 (vertical base, geographic hub, not derived): taken. With `skipHorizontalTransformation`
    false (the test case), 5917-5946: `opsFirst = createOperations(sourceCRS, hub)`,
    `opsSecond = createOperations(hub, target)`, and every pair whose second operation has no
    ballpark transformation is kept.
  - 5950-5952 (fallback): `createOperations(base, target)`.
- The branch with `skipHorizontalTransformation` (5861-5916, the `Change of vertical unit` with the
  degree terms left to the hub) is the one that the compound case uses. It is not the one of this test.

### Rust functions read

- `bound_ops.rs:304-453` `bound_to_geog`, the vertical block at 430-450 mirrors 5856-5948; the
  skip branch at 435-437 is taken only with `GUARD_SKIP_HORIZONTAL`, which is set in `compound.rs:2369`
  as in PROJ.
- `bound_ops.rs:455-511` `skip_horizontal_transformation`: the branch that the compound case uses.

### Finding

- The non-skip branch exists in the Rust port with the same order and the same filter on ballpark
  operations (`bound_ops.rs:438-446`).
- The Rust output is the 3-token `+z_in=us-ft +z_out=m` form. The expected 5-token form, with the
  degree terms on both sides, is the form of a pair of operations whose horizontal part is the
  identity on degrees. Which pair produces it in PROJ (`opsFirst` for the bound source to the hub,
  or `opsSecond` for NAD83 2D to WGS 84 3D) was not traced.
- Cause not identified. The first check to make: list `opsFirst` and `opsSecond` of PROJ for this
  pair (reference `projinfo`), and the Rust lists, before the concatenation.

### Callers at risk

None: no shared code is changed by this lot.

## Tests

No test was written in this lot. The three drafts are open and kept out of the tree.

Tests written, not run by the author: the coordinator runs them. (Here: none.)

## Commands run

- Read only, with the Read, Grep, and Glob tools and `rg` on the PROJ 9.9.0 sources
  (`C:\Users\maxim\AppData\Local\Temp\opencode\proj-9.9.0`).
- Build: `devhub.mjs check -- -p proj-rs --tests` on the lot's head (see the final message).
