# Porting sheet: three PROJ strings of concatenated operations (lot `unit-op-string-exports`)

Branch `unit-op-string-exports`, base `origin/main`. Round 3 (coordinator round 2 probes).

## Status

| Row | Status |
|---|---|
| `operation.nadgrids_with_pm` | Part closed: first assertion (EPSG:4326). Test `tests/unit_op_nadgrids_with_pm.rs`, commit `528e5fa`. The EPSG:4258 assertion and the WKT2 round trip are open (probe below). |
| `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | Open. First differing step found; C++ writer not yet ported. |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | Open. First differing step found; caller not yet identified. |

The rows stay in `docs/UNPORTED-UNIT.md` until the coordinator updates the ledger. Commits of this
lot: `8ec9629`, `461c755` (sheets), `528e5fa` (test). Compile of `528e5fa`:
`devhub run --kind build -- cargo check -p proj-rs --tests`, job 3868, succeeded.

## Row 1: `operation.nadgrids_with_pm` (test_operationfactory.cpp:4044-4098)

Source `+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 +ellps=intl
+nadgrids=foo.gsb +pm=lisbon +units=m +type=crs`.

| Assertion | Port (coordinator probe, round 2) | PROJ 9.9.0 (`projinfo ... -o PROJ`) | Status |
|---|---|---|---|
| EPSG:4326, line 4058-4065 | `+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 +k=1 +x_0=200000 +y_0=300000 +ellps=intl +pm=lisbon +step +proj=hgridshift +grids=foo.gsb +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1`, 1 candidate | same string, 1 candidate | equal; test committed (`528e5fa`) |
| EPSG:4258, line 4067-4079 (`ASSERT_GE(list.size(), 1)`) | not measured | same expected string | open, probe `probe_row1_rest_4258_and_wkt` |
| WKT2_2019 round trip, line 4081-4097 | not measured | same expected string | open, same probe |

Why the earlier round said "25 tokens with a longlat step": that was the output of a draft of the whole
TEST(), not of the first assertion. The current port gives the reference string for EPSG:4326.
The reference evidence on the rotation (lisbon to Greenwich, `inv longlat +pm=lisbon`) is in the
round-2 version of this sheet, commit `461c755`.

Reference command for the 4326 assertion (the `TEST()` context is the default one; `--grid-check none`
is the closest option, the grid `foo.gsb` is absent in both):
`projinfo -s "+proj=tmerc ... +pm=lisbon +units=m +type=crs" -t EPSG:4326 -o PROJ --spatial-test intersects --grid-check none --single-line`.

## Row 2: `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` (test_operationfactory.cpp:4013-4040)

Source `EPSG:4937`, target `+proj=sterea +lat_0=52.15616055555555 +lon_0=5.38763888888889 +k=0.9999079
+x_0=155000 +y_0=463000 +ellps=bessel +nadgrids=rdtrans2008.gsb +geoidgrids=naptrans2008.gtx
+geoid_crs=horizontal_crs +units=m +type=crs`.

Probe: not run yet. The coordinator's round-2 run did not include it; the probe is written in
`port-scratch/unit-op-string-exports/probe_unit_op_strings_r2.rs` (`probe_row2_etrs89_3d_geoid`), with
EPSG:4937 built by `AuthorityFactory::create(&db, "EPSG").create_coordinate_reference_system("4937")`.
The API of that call is the one used by `tests/unit_crs_strings_misc.rs`; not compiled by the author.

Reference (PROJ 9.9.0, `projinfo -s EPSG:4937 -t "<sterea>" -o PROJ --spatial-test intersects --grid-check none`):

- Candidate 1 (the expected string of the C++ test, 27 tokens):
  `+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +inv +proj=hgridshift +grids=rdtrans2008.gsb +step +inv +proj=vgridshift +grids=naptrans2008.gtx +multiplier=1 +step +proj=sterea ...`.
  Name: `ETRS89 to WGS 84 (1) + Inverse of unknown to WGS84 + Null geographic offset from unknown (geog2D) to unknown (geog3D) + Inverse of unknown to unknown ellipsoidal height + unknown`.
- Candidate 2: same string, name `Ballpark geographic offset from ETRS89 to WGS 84 + ...`.

Port (round 1 measurement, the draft of the whole TEST()): 25 tokens, the first `unitconvert` is
`+xy_in=deg +xy_out=rad`; the z pair `z_in=m z_out=m` is absent.

First differing step: the first `unitconvert` (`z_in=m` / `z_out=m` missing).

C++ read in this round:

- The null 2D-to-3D offset is `createBallparkGeographicOffset` (coordinateoperationfactory.cpp:2298-2349),
  which calls `Transformation::createGeographic3DOffsets` for a 3D CRS. Its export
  (singleoperation.cpp:3964-4018, `EPSG_CODE_METHOD_GEOGRAPHIC3D_OFFSETS`) is the inverse of
  `sourceCRSGeog->addAngularUnitConvertAndAxisSwap` plus `targetCRSGeog->addAngularUnitConvertAndAxisSwap`,
  with a `geogoffset` step only when an offset is non-zero.
- `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (crs.cpp:2446-2475) writes `z_in`/`z_out` when the CRS
  has three axes and `omitZUnitConversion()` is false.
- Rust: `transform_export.rs:2411-2432` `angular_unit_and_axis_swap` does the same, including the z terms
  for 3 axes and the omit flag (`transform_export.rs:2422`). The Geographic3D offsets export of the port
  must therefore be called with a 2D CRS on one side, or the z terms are lost elsewhere. Not verified.
- The omit-z flag is pushed and popped in balance in `export_proj_based` (`transform_export.rs:2634-2686`),
  so a leak of the flag is not the cause.

Cause status: the writer is the same function on both sides; the difference is in which CRS (2D or 3D)
reaches it, or in a simplification of the z pair. The next probe that settles it: the operation list of the
port before the simplification of the pipeline (not exposed by the public API). Until then no code.

## Row 3: `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` (test_operationfactory.cpp:3423-3459)

Source: the WKT `COMPD_CS["NAD83 + Ellipsoid (US Feet)", ...]` of the test, target `EPSG:4979`.

| | Port (coordinator probe, round 2) | PROJ 9.9.0 |
|---|---|---|
| Candidates | 1 | 1 |
| Name | not printed | `Transformation from NAD83 (Ellipsoid (US Feet)) to NAD83 + Transformation from NAD83 to WGS84` |
| PROJ string | `+proj=pipeline +step +proj=unitconvert +z_in=us-ft +z_out=m +step +proj=axisswap +order=2,1` | `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` |

First differing step: the first `unitconvert` (PROJ has `xy_in=deg` / `xy_out=deg` merged with the z pair;
the port has the z pair alone). Second difference: the port ends with `axisswap order=2,1`, PROJ has none.

Reference facts:

- The second leg alone, NAD83 (2D) to WGS 84 (3D) (`projinfo -s EPSG:4269 -t EPSG:4979 -o PROJ`), is
  `+proj=noop` in the null candidates, so the horizontal `deg` terms of the expected string come from the
  first leg, `Transformation from NAD83 (Ellipsoid (US Feet)) to NAD83`.
- Reading the PROJ string as a sequence (formatter rules io.cpp:9106-9139 and 9248-9279): the expected
  step is the merge of `[unitconvert xy deg->rad][unitconvert z us-ft->m][unitconvert xy rad->deg]`.
  The first and last steps are the angular conversions of a geographic CRS in degrees, i.e. the source and
  target `addAngularUnitConvertAndAxisSwap`; the axis swaps of a source and a target both in lat/lon cancel
  (io.cpp:9382-9389 and 9369-9380), which is why PROJ has no `axisswap`. This is a derivation from the
  formatter rules, not measured.

C++ read: the first leg's vertical unit change is written by `exportToPROJStringChangeVerticalUnit`
(singleoperation.cpp:3251-3290), which writes only `z_in`/`z_out`. So the horizontal terms are added by a
caller that exports the CRS modifiers: `Conversion::_exportToPROJString` with `applySourceCRSModifiers`
(conversion.cpp:3887-3915) and the target modifiers (conversion.cpp:4422). Which of these applies to the
first leg (the bound transformation, `createOperationsBoundToGeog` 5787-5812 or 5856-5948) is not
established.

Rust read: `bound_ops.rs:304-453` has the vertical block with the order of coordinateoperationfactory.cpp
5856-5948. The horizontal terms are missing from its first leg.

Cause status: the caller that writes the horizontal terms of the first leg is not identified. Next step:
read conversion.cpp:3887-3915 and 4422-4440 for the case of a change of vertical unit with a 3D geographic
source, then the Rust path of the same export in `transform_export.rs`.

## Probes requested (not in the tree)

1. `port-scratch/unit-op-string-exports/probe_unit_op_strings_r2.rs`, two tests:
   `probe_row1_rest_4258_and_wkt` (row 1, assertions 2 and 3) and `probe_row2_etrs89_3d_geoid` (row 2).
   Copy to `tests/probe_unit_op_strings_r2.rs`, run `cargo test -p proj-rs --test probe_unit_op_strings_r2 -- --nocapture`.
   Not compiled by the author.
2. Row 3 with the operation name printed: the same probe as round 2 plus `op.name()` if it is public; if
   not, the coordinator's output of the candidate name is enough.
3. Row 2 has no measured port output yet; the probe in (1) provides it.

## Callers at risk

None: the only code change of the lot is a new test file (`528e5fa`). No shared code is changed.

## Tests

- `tests/unit_op_nadgrids_with_pm.rs`, `operation_nadgrids_with_pm_to_epsg_4326`: expected to pass.
  Reason: the port's output for exactly this source, target and context was measured in round 2 (probe
  `probe_row1_nadgrids_with_pm`, identical string, one candidate); the expected string is copied from
  test_operationfactory.cpp:4059-4065. The author did not run it.

Tests written, not run by the author: the coordinator runs them.
