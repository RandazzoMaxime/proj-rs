# Port — geographic 3D steps: vertical unit and depth reversal (lot `geog3d-steps`)

Source: PROJ 9.9.0, tag `9.9.0` of OSGeo/PROJ. The PROJ lines cited below are from
`src/iso19111/operation/coordinateoperationfactory.cpp`, `operation/conversion.cpp`,
`operation/singleoperation.cpp`, `io.cpp` and `crs.cpp`, and from `test/unit/test_operationfactory.cpp`.

The lot is about the PROJ string of operations that end in, or start from, a geographic 3D CRS.
It is run as one cause per commit. Each section says what was wrong, what PROJ does, and which
tests the commit closes.

## Method and reference

The reference is PROJ 9.9.0 `projinfo` (`-o PROJ`, `--grid-check none --spatial-test intersects`),
and a traced build of the same tag (`PROJ_DEBUG=3`). The comparisons were made pair by pair, on the
candidate list and on the PROJ string of each candidate, before any code was written.

## Commit 1 — a same-datum vertical unit change is a conversion (closes 4 tests)

### What was wrong

`compound::vertical_to_vertical` built every vertical change as a `Transformation` (method 1069 or
1068). PROJ builds a `Conversion` when the two vertical CRS share a datum, and a
`Transformation` (ballpark) when they do not (`createOperationsVertToVert`,
coordinateoperationfactory.cpp:5995-6066, conversion at 6050-6064, ballpark at 6039-6049).

Two consequences show in the PROJ strings of the depth candidates:

1. A `Conversion` and a `Transformation` are not treated alike by the vertical pass. PROJ's
   `getInterpolationGeogCRS` (coordinateoperationfactory.cpp:6392-6433) only counts
   `Transformation` steps. With the depth change as a `Transformation`, the registry candidate
   `Conversion from NAVD88 depth to NAVD88 height + Inverse of NAD83(2011) to NAVD88 height` had two
   transformations, so no interpolation CRS was found, and the candidate went through the plain
   `combine` path. The `axisswap +order=1,2,-3` of the depth then came out before the horizontal
   axis swap, instead of between the unit conversion and the grid shift (PROJ:
   `MyPROJStringExportableHorizVerticalHorizPROJBased`, coordinateoperationfactory.cpp:2445-2561).
2. The same-datum change of unit with a height/depth reversal was written as method 1068 alone,
   so the unit factor was lost. PROJ writes method 1069 with the negated factor
   (`-0.304800609601219` for ftUS depth to metre, conversion at 6050-6056).

### What PROJ does, and what is now ported

| PROJ 9.9.0 | Rust (`src/iso19111`) |
|---|---|
| `createOperationsVertToVert`, same datum: `createChangeVerticalUnit` or `createHeightDepthReversal` (coordinateoperationfactory.cpp:6050-6064) | `compound::vertical_to_vertical` (`compound.rs`), same-datum branch |
| `createOperationsVertToVert`, other datum: ballpark `Transformation` (6039-6049) | `compound::vertical_to_vertical`, the first branch |
| `Conversion::inverse`, method 1069 with the reciprocal factor, 1068, 1104 (conversion.cpp:2706-2766) | `Conversion::inverse`, `Conversion::inverse_by_method` (`buildop.rs`) |
| `exportToPROJStringChangeVerticalUnit` (singleoperation.cpp:3564-3569) | `Conversion::vertical_unit_proj_string` (`buildop.rs`), through `transform_export::change_vertical_unit_by_factor` |
| height/depth reversal `axisswap +order=1,2,-3` (singleoperation.cpp:3571-3575) | `Conversion::vertical_unit_proj_string` (1068), and `conversion_export::export_body` |

Notes on the port:

- `Conversion::inverse` of a change of vertical unit is a new conversion with the reciprocal
  factor and the `Inverse of` name (`createPropertiesForInverse`), not an `InverseConversion`.
  Without it the inverse of the depth change came out as `+inv +proj=affine +s33=-3.28…` where
  PROJ writes `+proj=affine +s33=-0.3048…` (see `vertical_geog_fallback`, which inverts the
  conversion through the metre height, and `applyInverse` at coordinateoperationfactory.cpp:5162).
- The standalone PROJ string of a vertical conversion is written without a CRS pair, as PROJ's
  `exportToPROJString` does for these methods. Before this, a 1069 conversion had no standalone
  PROJ string here, which broke `operation_vert_crs_to_vert_crs` (`tests/unit_opfactory_core_b.rs`)
  at once. That test is the regression guard for this part.
- Other methods of `Conversion::inverse` (axis order reversal, geographic/geocentric) keep the
  `InverseConversion` form. Their PROJ strings are the same after the formatter's inverse
  normalisation, and no test of this lot needs them. Not changed here.

### Tests

File `tests/unit_op_geog3d_steps.rs` (PROJ 9.9.0 expected strings, copied from
`test/unit/test_operationfactory.cpp`):

| Rust test | PROJ `TEST()` / ledger name | Status |
|---|---|---|
| `operation_compound_to_geog_with_height_depth_reversal` | `compoundCRS_to_geogCRS_with_height_depth_reversal` | closed, commit 1 |
| `operation_compound_to_geog_with_vertical_unit_change_and_height_depth_reversal` | `compoundCRS_to_geogCRS_with_vertical_unit_change_and_height_depth_reversal` | closed, commit 1 |
| `operation_compound_to_geog_3d_with_3d_helmert_same_geog_src_target_context` | `compoundCRS_to_geogCRS_3D_with_3D_helmert_same_geog_src_target_context` | closed, commit 1 (passed on the base once the draft was run; the triage row was out of date) |
| `operation_geog_crs_3d_to_proj_crs_with_2d_geocentric_translation` | `geogCRS_3D_to_projCRS_with_2D_geocentric_translation` | closed, commit 1 (failed on the base: 30 tokens, `+xy_in=deg +xy_out=rad` only) |

The four ledger lines are removed from `docs/UNPORTED-UNIT.md`, and the counts regenerated
(`UPDATE_DOCS=1 cargo test --test docs_in_sync`, `UPDATE_DOCS=1 cargo test --test progress_svg`).

## Still open (drafts kept out of the tree)

The drafts that still fail are kept in the scratch directory and are not committed, as the rules of
the lot require. Their causes, as measured:

| Test | Ours | PROJ 9.9.0 | Cause (measured) |
|---|---|---|---|
| `operation_compound_ftus_to_geog_ft` (`compoundCRS_ftUS_to_geogCRS_ft`) | the first candidate matches; the last one is `unitconvert +z_in=us-ft +z_out=ft` (ballpark) | 41 candidates; the last is `Transformation from NAD83(2011) to NAD83(2011)` + the ballpark vertical change, `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=ft` | Candidate set, not the string. For a foot target the horizontal part is a height-unit change (`geographic_height_change`), and it is searched against the 2D target (see the next finding). |
| `operation_compound_to_geog_with_vertical_unit_change_and_complex_horizontal_change` | 48 candidates | 49 | The ftUS candidate of the foot target is missing (same cause). |
| `operation_etrs89_3d_to_proj_string_with_geoidgrids_nadgrids` (`ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids`) | `unitconvert +xy_in=deg +xy_out=rad` (25 tokens) | `unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m` (27 tokens) | The z terms of the first unitconvert of a concatenated candidate are not written. Not traced yet. |
| `operation_bound_crs_to_geog_crs_hub_crs_and_target_crs_same_but_base_crs_not` (`boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`) | `unitconvert +z_in=us-ft +z_out=m` | `unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m` | The horizontal deg conversion of the bound CRS is missing (the `skipHorizontalTransformation` path of `createOperationsBoundToGeog`). Not in this commit. |

Finding for the next commit: `compound::compound_to_geog3d` and `compound_vertical_pass` search the
horizontal part against `target.demote_to_2d()`. PROJ's `createOperationsCompoundToGeog` searches it
against the full target: `horizTransforms = createOperations(componentsSrc[0], sourceEpoch,
targetCRS, targetEpoch, context)` (coordinateoperationfactory.cpp:6699-6704). The 2D search is the
"geographic-to-geographic search for a 3D target" of PROJ, which the comment cites, but not the
branch of the compound that the foot target takes.

A separate attempt (not committed): wrapping the null-horizontal `combine` in
`createHorizVerticalPROJBased` (`proj_based_horiz_vertical`, as PROJ's
`MyPROJStringExportableHorizVertical` does, coordinateoperationfactory.cpp:2384-2417). It did not
close any draft, and the foot target's last candidate still differs, so it was not kept.

## Commands run

- `cargo test --all-features --no-fail-fast` with `PATH=<scratch>/reftest/build/bin`,
  `PROJ_DATA=<scratch>/oracle-db`, `PROJ_CS2CS_REQUIRED=1`, `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`,
  on the base, with the drafts in place: 1652 passed (the one draft that passed included), 4 failed
  (the drafts still open), 32 ignored.
- After commit 1, with the drafts still in place: 1655 passed, 4 failed (the same open drafts, nothing else),
  32 ignored. `cli_yaml` passes and `docs/UNPORTED-CLI.md` is not reopened.
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check`: clean.
