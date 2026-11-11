# Porting sheet: three PROJ strings of concatenated operations (lot `unit-op-string-exports`)

Branch `unit-op-string-exports`, base `origin/main`. Round 4 (coordinator round 3 probes).

## Status

| Row | Status |
|---|---|
| `operation.nadgrids_with_pm` | Partly closed: the EPSG:4326 and EPSG:4258 assertions, tests `tests/unit_op_nadgrids_with_pm.rs` (`528e5fa`, `3d83ea8`). The WKT2 round trip is open: first differing step found (the direction of the prime-meridian longlat step). |
| `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | Open. First differing step found (the z pair of the first `unitconvert`). Writer of PROJ identified, cause not yet. |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | Open. First differing step found (horizontal terms of the first leg). Caller not yet identified. |

Commits of this lot: `8ec9629`, `461c755`, `af9badc` (sheets), `528e5fa`, `3d83ea8` (tests). Compile of
`3d83ea8`: `cargo check -p proj-rs --tests`, succeeded.
Coordinator run of round 3: `unit_op_nadgrids_with_pm` 1 passed, probes ran; output in
the coordinator's run.

## Row 1: `operation.nadgrids_with_pm` (test_operationfactory.cpp:4044-4098)

Source `+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 +ellps=intl
+nadgrids=foo.gsb +pm=lisbon +units=m +type=crs`.

| Assertion | Port (measured, round 3) | PROJ 9.9.0 | Status |
|---|---|---|---|
| EPSG:4326, lines 4058-4065 | `+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 +ellps=intl +pm=lisbon +step +proj=hgridshift +grids=foo.gsb +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1`, 1 candidate | same string, 1 candidate | closed, test `operation_nadgrids_with_pm_to_epsg_4326` (`528e5fa`) |
| EPSG:4258, lines 4067-4079 (`ASSERT_GE(list.size(), 1)`) | candidate [0]: same string as above; candidate [1]: same string (duplicate) | same expected string | closed, test `operation_nadgrids_with_pm_to_epsg_4258` (`3d83ea8`) |
| WKT2_2019 round trip, lines 4081-4097 (`ASSERT_GE(list.size(), 1)`) | candidate [0]: `+proj=pipeline +step +inv +proj=tmerc ... +pm=lisbon +step +proj=longlat +ellps=intl +pm=lisbon +step +proj=hgridshift +grids=foo.gsb +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1`; candidate [1]: same | `... +pm=lisbon +step +proj=hgridshift ...` with no longlat step (line 4090-4097, comment "Check that there is no extra +step +proj=longlat +pm=lisbon") | open |

### WKT round trip: first differing step

- Port: `+step +proj=longlat +ellps=intl +pm=lisbon`, not inverted, right after `+inv +proj=tmerc`.
- PROJ: no longlat step in the string.
- Reference, the same rotation on its own (`projinfo`, `--single-line`):
  - lisbon to Greenwich: `+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=intl +pm=lisbon +step +proj=unitconvert +xy_in=rad +xy_out=deg`
  - Greenwich to lisbon: `+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=longlat +ellps=intl +pm=lisbon +step +proj=unitconvert +xy_in=rad +xy_out=deg`
- So the port's step has the direction of Greenwich to lisbon (non-inverted), while
  `createOperationsBoundToGeog` builds the intermediate from the base (lisbon) to the transformation
  source (Greenwich copy): coordinateoperationfactory.cpp:5636-5641, `opIntermediate = createOperations(geogCRSOfBaseOfBoundSrc, transformation->sourceCRS())`.
  PROJ's rotation in that direction is the inverted longlat, which the tmerc chain then drops.
- C++ that writes the step: `GeographicCRS` longitude rotation export, singleoperation.cpp:4811-4877
  (the well-known name is taken from `-offsetDeg`, else an inverted `longlat` with the offset).
  Rust counterpart: `transform_export.rs:2437-2488` (same branches). A rotation written non-inverted
  means the port's offset has the other sign, or the port's rotation has the other direction.
- Rust: `bound_ops.rs:545-561` `intermediate_operation` calls `create_candidates(base_geog, transformation_source)`
  (same direction as C++); `geodetic_ops.rs:512-531` `prime_meridian_offset` gives `left - right`
  (same as coordinateoperationfactory.cpp:2932-2940).
- Cause status: not identified. The probe below prints the port's two directions; the sign of the
  offset and the direction of the intermediate are then comparable with the two reference lines above.

Note on the first assertion: the port's EPSG:4326 candidate has no rotation at all, while PROJ's
candidate has one that the export drops. The strings agree, the candidate sets do not. This is a
difference in the candidate, not yet visible in the output; it is recorded for the next step.

## Row 2: `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` (test_operationfactory.cpp:4013-4040)

Source `EPSG:4937`, target `+proj=sterea +lat_0=52.15616055555555 +lon_0=5.38763888888889 +k=0.9999079
+x_0=155000 +y_0=463000 +ellps=bessel +nadgrids=rdtrans2008.gsb +geoidgrids=naptrans2008.gtx
+geoid_crs=horizontal_crs +units=m +type=crs`.

| | Port (measured, round 3) | PROJ 9.9.0 |
|---|---|---|
| Candidates | 2 | 2 |
| [0] | `+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=hgridshift +grids=rdtrans2008.gsb +step +inv +proj=vgridshift +grids=naptrans2008.gtx +multiplier=1 +step +proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 +k=0.9999079 +x_0=155000 +y_0=463000 +ellps=bessel` (24 tokens) | `+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +inv +proj=hgridshift +grids=rdtrans2008.gsb +step +inv +proj=vgridshift +grids=naptrans2008.gtx +multiplier=1 +step +proj=sterea ...` (27 tokens; this is the expected string of the C++ test) |
| [1] | same string as port [0] | same as PROJ [0] (candidate 2 is the ballpark variant, same string) |

First differing step: the first `unitconvert`. PROJ has `z_in=m z_out=m` merged in; the port has none, in both
candidates. Everything else is identical, including the axisswap, the inverse grids and the sterea step.
The `ETRS89 to WGS 84 (1)` step and the null 2D-to-3D offset of PROJ's candidate are not visible in
either output: in the port they give no step of their own.

C++ read:

- The 2D-to-3D null offset is `createBallparkGeographicOffset` (coordinateoperationfactory.cpp:2298-2349),
  calling `Transformation::createGeographic3DOffsets` for a 3D CRS. Its export
  (singleoperation.cpp:3964-4017, `EPSG_CODE_METHOD_GEOGRAPHIC3D_OFFSETS`) writes the inverse of
  `sourceCRSGeog->addAngularUnitConvertAndAxisSwap`, then `targetCRSGeog->addAngularUnitConvertAndAxisSwap`.
- `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (crs.cpp:2446-2475) writes `z_in`/`z_out` for three axes
  unless `omitZUnitConversion()`. Rust `transform_export.rs:2411-2432` does the same.
- So the z pair comes from an angular conversion of a 3D geographic CRS, which the port's candidate does not
  reach with three axes, or which the port's simplifier drops. The omit flag is balanced
  (`transform_export.rs:2634-2686`), so a leak is not the cause.

Cause status: the writer is the same function on both sides; the difference is the CRS (2D or 3D) that
reaches it, or a drop in simplification. Not decided. Next probe in the list below.

## Row 3: `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` (test_operationfactory.cpp:3423-3459)

Source: the WKT `COMPD_CS["NAD83 + Ellipsoid (US Feet)", ...]` of the test, target `EPSG:4979`.

| | Port (measured, round 2) | PROJ 9.9.0 |
|---|---|---|
| Candidates | 1 | 1 |
| Name | not printed | `Transformation from NAD83 (Ellipsoid (US Feet)) to NAD83 + Transformation from NAD83 to WGS84` |
| PROJ string | `+proj=pipeline +step +proj=unitconvert +z_in=us-ft +z_out=m +step +proj=axisswap +order=2,1` | `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` |

First differing step: the first `unitconvert` (PROJ has the xy `deg` terms merged with the z pair; the
port has the z pair alone). Second: the port ends with `axisswap order=2,1`, PROJ has none.

Reference facts (round 2): the second leg alone, NAD83 (2D) to WGS 84 (3D), is `+proj=noop` in the null
candidates. So the horizontal `deg` terms come from the first leg. Reading the expected string as a
sequence: `[unitconvert xy deg->rad][unitconvert z us-ft->m][unitconvert xy rad->deg]`, merged by the
formatter (io.cpp:9106-9139, 9248-9279), the first and last being the angular conversions of a geographic CRS
in degrees. The axis swaps cancel (io.cpp:9369-9389). This is a derivation from the rules, not measured.

C++ read: `exportToPROJStringChangeVerticalUnit` (singleoperation.cpp:3251-3290) writes only z. The horizontal
terms come from a caller exporting CRS modifiers: `Conversion::_exportToPROJString` with
`applySourceCRSModifiers` (conversion.cpp:3887-3915) and the target modifiers (conversion.cpp:4422). Which
applies to the first leg is not established. Rust `bound_ops.rs:304-453` has the vertical block in the order of
coordinateoperationfactory.cpp:5856-5948.

Cause status: not identified.

## Probes requested (not in the tree)

1. `probe_unit_op_strings_r3.rs`, two tests:
   `probe_rotation_lisbon_to_greenwich` and `probe_rotation_greenwich_to_lisbon`. They print the port's candidates
   for the two directions of the rotation, to set against the two reference lines of Row 1. Copy to
   `tests/probe_unit_op_strings_r3.rs`, run `cargo test -p proj-rs --test probe_unit_op_strings_r3 -- --nocapture`.
   Not compiled by the author.
2. Row 2 with the 3D case isolated: `EPSG:4937` to `EPSG:4979` (3D to 3D), and `EPSG:4937` to `EPSG:4258`
   (3D to 2D), in the port and with `projinfo ... -o PROJ --spatial-test intersects --grid-check none`.
   If the port's 4937 to 4979 has no z pair either, the source CRS reaches the writer in 2D.
3. Row 1, WKT round trip: the port's candidate names, if `op.name()` is public, for candidates [0] and [1].
4. Row 3: the candidate name, as in round 2.

## Callers at risk

Only the tests of this lot (`unit_op_nadgrids_with_pm.rs`). No shared code is changed.

## Tests

- `tests/unit_op_nadgrids_with_pm.rs`:
  - `operation_nadgrids_with_pm_to_epsg_4326`: measured by the coordinator (round 3), passes.
  - `operation_nadgrids_with_pm_to_epsg_4258`: the first candidate's string was measured by the coordinator
    (round 3) equal to the expected string; the test asserts the list is non-empty and checks
    `list[0]`. Expected to pass; not run by the author.

Tests written, not run by the author: the coordinator runs them.
