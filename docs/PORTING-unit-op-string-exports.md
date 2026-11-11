# Porting sheet: three PROJ strings of concatenated operations (lot `unit-op-string-exports`)

Branch `unit-op-string-exports`, base `origin/main`. Status: the three rows are still open. No
code is changed and no test is added in this round. The reference PROJ 9.9.0 was used (round 2)
to read the candidates and their PROJ strings. It located the difference for each row, but not
yet the C++ function that causes it in the port. No draft test is committed: a draft that fails
is not committed (lot rules), and the port of the missing branch was not identified.

Open rows of `docs/UNPORTED-UNIT.md`, all three still listed there:

| PROJ `TEST()` | Ours (as the status of round 1 recorded it) | PROJ 9.9.0 |
|---|---|---|
| `operation.nadgrids_with_pm` | 25 tokens, with `+step +proj=longlat +ellps=intl +pm=lisbon` after `+inv +proj=tmerc` | 21 tokens, no longlat step |
| `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | first `unitconvert` is `+xy_in=deg +xy_out=rad` | `+xy_in=deg +z_in=m +xy_out=rad +z_out=m` |
| `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | `+proj=unitconvert +z_in=us-ft +z_out=m` | `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` |

Reference commands (`ref.ps1` in the brief directory, which runs PROJ 9.9.0). They are the
options that match the C++ context of each `TEST()`; the Rust-side counterpart is the probe
`port-scratch/unit-op-string-exports/probe_unit_op_strings.rs` (not in the tree).

- Row 1, `nadgrids_with_pm`: source `+proj=tmerc +lat_0=39.66666666666666 +lon_0=1 +k=1 +x_0=200000
  +y_0=300000 +ellps=intl +nadgrids=foo.gsb +pm=lisbon +units=m +type=crs`, target `EPSG:4326`, with
  `--spatial-test intersects --grid-check none`. The C++ context is the default one (no grid option),
  so `--grid-check none` is the closest option; the grid `foo.gsb` is absent in both.
- Row 2, `ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids`: source `EPSG:4937`, target the
  `+proj=sterea ... +nadgrids=rdtrans2008.gsb +geoidgrids=naptrans2008.gtx +geoid_crs=horizontal_crs +units=m +type=crs`
  string, same options.
- Row 3, `boundCRS_to_geogCRS_...`: source the WKT `COMPD_CS["NAD83 + Ellipsoid (US Feet)", ...]` of
  the test, target `EPSG:4979`, `--spatial-test intersects --grid-check none` (the C++ context is
  `IGNORE_GRID_AVAILABILITY` + `PARTIAL_INTERSECTION`).

## Row 1: `operation.nadgrids_with_pm` (test_operationfactory.cpp:4044-4098)

### Reference (PROJ 9.9.0)

- Candidates: 1. Name: `Inverse of unknown + Transformation from unknown to unknown (with Greenwich
  prime meridian) + unknown (with Greenwich prime meridian) to WGS84`, has ballpark, grid missing.
- PROJ string (equal to the first assertion of the test, 21 tokens):
  `+proj=pipeline +step +inv +proj=tmerc +lat_0=39.6666666666667 +lon_0=1 +k=1 +x_0=200000 +y_0=300000
  +ellps=intl +pm=lisbon +step +proj=hgridshift +grids=foo.gsb +step +proj=unitconvert +xy_in=rad +xy_out=deg
  +step +proj=axisswap +order=2,1`.
- The longitude rotation on its own (lisbon geographic to Greenwich copy, `projinfo -s "+proj=longlat
  +ellps=intl +pm=lisbon +type=crs" -t "+proj=longlat +ellps=intl +type=crs"`) is written in full:
  `+step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=intl +pm=lisbon
  +step +proj=unitconvert +xy_in=rad +xy_out=deg`. So PROJ does write the rotation as an inverse
  `longlat` with `+pm=lisbon` (singleoperation.cpp:4844-4872, the branch where the well-known name
  is taken from the offset itself, `PrimeMeridian::_exportToPROJString`).
- The same tmerc without nadgrids, to EPSG:4326, gives `+proj=pipeline +step +inv +proj=tmerc ...
  +pm=lisbon +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1`:
  the rotation to the WGS 84 meridian is absent from the string, while its candidate name is
  `Inverse of unknown + Transformation from unknown to unknown altered to use prime meridian of WGS 84 + ...`.
- The reverse, longlat with lisbon to the tmerc, keeps the rotation: `+step +proj=unitconvert
  +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=intl +pm=lisbon +step +proj=tmerc ... +pm=lisbon`.

### What the port does (C++ and Rust read, not run)

- Simplification rules of the PROJ formatter (io.cpp:8895-9428, read in full for the unit and axis
  rules): none mentions `pm`. The only `longlat` rule (io.cpp:9018-9025) removes a `longlat` that has
  only `ellps`; the rotation has `ellps` and `pm`, so that rule does not remove it.
- The rotation is therefore dropped before the string is written, or by the export of the inverse
  tmerc: `Conversion::_exportToPROJString` (conversion.cpp:3768-3920) exports the base geographic CRS
  of the projected CRS (`setOmitProjLongLatIfPossible(true)`, conversion.cpp:3910-3914), and
  `GeodeticCRS` export omits `longlat` when possible (crs.cpp:2504-2511). The `applyTargetCRSModifiers`
  switch (conversion.cpp:4110-4118) is the next thing to read. Not verified.
- The port (`transform_export.rs:2437-2488`, `longitude_rotation`) writes the rotation with the same
  branches as singleoperation.cpp:4844-4874, so the difference is the caller that decides whether the
  rotation is exported at all, on the tmerc side. That caller is not yet traced in the Rust
  (`transform_export.rs` conversion export of an inverse projection, `bound_ops.rs`).

### Cause status

Not identified. Next step: read conversion.cpp:3768-3800 and 4100-4125 and 4422-4440 to find which
branch of `Conversion::_exportToPROJString` drops the target-side geographic step for an inverse
projection with a non-Greenwich prime meridian; then the Rust counterpart in `transform_export.rs`.
A test is written only after that.

## Row 2: `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` (test_operationfactory.cpp:4013-4040)

### Reference (PROJ 9.9.0)

- Candidates: 2.
- Candidate 1 name: `ETRS89 to WGS 84 (1) + Inverse of unknown to WGS84 + Null geographic offset from
  unknown (geog2D) to unknown (geog3D) + Inverse of unknown to unknown ellipsoidal height + unknown`.
  Its PROJ string equals the expected string of the C++ test (27 tokens):
  `+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad
  +z_out=m +step +inv +proj=hgridshift +grids=rdtrans2008.gsb +step +inv +proj=vgridshift
  +grids=naptrans2008.gtx +multiplier=1 +step +proj=sterea ...`.
- Candidate 2 (ballpark, `Ballpark geographic offset from ETRS89 to WGS 84 + ...`) has the same string.
- So the missing part is exactly the `z_in=m z_out=m` pair in the first `unitconvert`, which the
  formatter merges with the xy step (io.cpp:9106-9139, the `combine unitconvert (xy) and unitconvert (z)` rule).

### What the port does (read, not run)

- `transform_export.rs:652-703` `geoid_grid_shift` writes its horizontal part with
  `horizontal_unit_and_axis_swap` (`transform_export.rs:1270-1284`), which never writes z. That matches
  singleoperation.cpp:4733-4738, where `pushOmitZUnitConversion` is used.
- So the z-only step is not written by the geoid part, in PROJ or in the port. It comes from the
  `Null geographic offset from geog2D to geog3D` operation (the 2D to 3D step between the Amersfoort
  geographic CRS and its 3D version), whose PROJ export must write the z unit conversion. Its export
  is not traced: the `createBallparkGeographicOffset` branch (coordinateoperationfactory.cpp:2298-2342)
  calls `Transformation::createGeographic3DOffsets` for a 3D CRS, and the export of that method
  (`singleoperation.cpp`, the geographic offsets branch) was not read.

### Cause status

Not identified: which PROJ export writes the z-only `unitconvert` of the null 2D-to-3D offset.
Next step: read the export of `Geographic3D Offsets` / the null offset in singleoperation.cpp, and the
Rust counterpart in `geodetic_ops.rs` (`geographic_offset`) and `transform_export.rs`.

## Row 3: `operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` (test_operationfactory.cpp:3423-3459)

### Reference (PROJ 9.9.0)

- Candidates: 1. Name: `Transformation from NAD83 (Ellipsoid (US Feet)) to NAD83 + Transformation from
  NAD83 to WGS84`.
- PROJ string: `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` (5 tokens), equal to the test.
- The second leg alone, NAD83 (2D) to WGS 84 (3D), has `+proj=noop` in the reference
  (`projinfo -s EPSG:4269 -t EPSG:4979 -o PROJ`, the candidate `Ballpark geographic offset from NAD83
  to WGS 84`, and the null `NAD83 to WGS 84 (1)` candidate). So the horizontal `xy_in=deg xy_out=deg`
  of the expected string comes from the first leg, the bound transformation.
- The first leg writes the z conversion `exportToPROJStringChangeVerticalUnit` (singleoperation.cpp:3251-3290),
  which writes only `z_in`/`z_out`. The horizontal `deg` terms are therefore written by a caller
  that exports the source and target CRS of the transformation (the `applySourceCRSModifiers` branch,
  conversion.cpp:3887-3915, with the 3D geographic source `NAD83 (Ellipsoid US Feet)`), not by the
  unit-change method itself. Not traced yet.

### What the port does (read, not run)

- `bound_ops.rs:304-453` `bound_to_geog` has the vertical block (430-450) with the same order as
  coordinateoperationfactory.cpp:5856-5948. The output is the 3-token form: the first leg writes the
  z conversion alone, the second is `noop`-like. So the port lacks the horizontal `deg` terms of the
  first leg. Which function of the port writes the z conversion of a bound transformation with the
  horizontal part of its CRS is not yet traced (`transform_export.rs`, the change-of-vertical-unit
  branch and the CRS modifiers of the transformation export).

### Cause status

Not identified: the caller that adds the horizontal terms for the first leg.

## Probes for the coordinator (not in the tree)

`port-scratch/unit-op-string-exports/probe_unit_op_strings.rs` has two tests, `probe_row1_nadgrids_with_pm`
and `probe_row3_bound_to_geog_hub_same_base_not`. They print the PROJ string of every candidate the
port gives for those pairs and assert nothing. To run them, copy the file to `tests/probe_unit_op_strings.rs`
and run `cargo test --test probe_unit_op_strings -- --nocapture`. The API names used come from
`tests/unit_op_bound_family.rs` and `tests/unit_op_towgs84_pm_3d.rs`. The probe is not compiled
by the author: an error in it is to be reported back, not fixed in the tree.

Please send the output of the two probes in the next round. It gives the candidates of the port to
set against the reference lists above.

Row 2 has no probe: the constructor for `EPSG:4937` is not confirmed in the crate, so a probe for it
was not written.

## Callers at risk

None: no shared code is changed by this lot in this round.

## Tests

No test is written or committed in this round.

Tests written, not run by the author: the coordinator runs them. (Here: none.)
