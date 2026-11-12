# Port: derived vertical CRS and PROJ-based projection (lot derived-vertical-projected)

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, five `TEST()`s of `operation` that were in
`docs/UNPORTED-UNIT.md`. Rust side: `tests/unit_op_derived_vertical_projected.rs`.

Code read in PROJ 9.9.0: `coordinateoperationfactory.cpp` (`createOperationsDerivedTo` 5523-5574,
`createOperationsCompoundToGeog` 6454-7172, `SetGeogCRSOfVertCRS` 6760-6777, the same-datum test of
`createOperationsVertToGeogSynthetized` 6150-6170), `operation/conversion.cpp` (`_exportToPROJString`
3768-4450), `operation/singleoperation.cpp` (`exportToPROJStringGeneric` 3294, the `PROJ-based operation
method: ` prefix 5357, the vertical offset and slope export 4255).

## 1. Map of tests

| C++ `TEST()` (line) | Rust function | Status |
| --- | --- | --- |
| `compoundCRS_with_derived_vertical_CRS` (10210) | `operation_compound_with_derived_vertical_crs` | passes |
| `compoundCRS_with_derivedVerticalCRS_ellipsoidal_height_to_geog` (5300) | `operation_compound_with_derived_vertical_crs_ellipsoidal_height_to_geog` | passes |
| `createOperation_derived_projected_crs` (11247) | `operation_create_operation_derived_projected_crs` | passes |
| `geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected` (11310) | `operation_geog_to_compound_with_bound_vertical_and_derived_projected` | passes |
| `compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height` (7124) | `operation_compound_to_compound_with_derived_vertical_crs_ellipsoidal_height` | passes |

The first three are the lot's own tests. The last two replace the two C++ tests of the lot that could not
be ported, see section 2: `geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected` replaces the
disabled `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation`, and
`compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height` replaces the blocked
`createOperation_bound_of_spherical_ocentric_to_same_type`. The expected names and PROJ strings are copied from the C++ file. The
`geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected` fixture reproduces
`createBoundVerticalCRS()` (ODN height, EPSG:5701, bound to EPSG:4979 through `us_nga_egm08_25.tif`) as
WKT, see section 4.

The expected PROJ strings of the five tests were also checked against the reference `projinfo` 9.9.0
(`PROJ_DATA=target/proj-reference/build/data`): the output is the same string as the test in each case.

## 2. Tests not ported

- `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation` (6474) is inside
  `#if no_longer_work_since_epsg_13_001` in PROJ 9.9.0. The code is not compiled, so PROJ does not run it.
  It stays in `docs/UNPORTED-UNIT.md`, and its replacement is the bound-vertical test above.
- `createOperation_bound_of_spherical_ocentric_to_same_type` (10896) is blocked, as recorded in
  `docs/PORTING-unit-niche-ocentric.md` section 1.5: the first leg's source is a bound CRS of a spherical
  base (`geographic_geocentric` reads only geodetic endpoints, `conversion.cpp:3838-3845`), and the two
  `helmert` steps of `createOperationsBoundToBound` need a merge that the port does not do. Those are two
  ports of their own, not an API gap, so the test stays in `docs/UNPORTED-UNIT.md`.

## 3. Code changes

1. `conversion_export.rs`, `export_proj_based`: a conversion whose method is `PROJ-based operation method: `
   writes the inverse of the projected source's unit and axis order, the method's PROJ string, then the
   unit and axis order of the target (`Conversion::_exportToPROJString`, conversion.cpp:3908-3915 and
   4422-4450). It applies to a projected source and a projected or derived projected target; other pairs
   keep the projection path.
2. `conversion_export.rs`, `export_body`: methods 9616 (vertical offset) and 1046 (vertical offset and
   slope) are written for a conversion as `vertoffset`/`geogoffset`. They share
   `transform_export::vertical_offset` and `vertical_offset_and_slope`, which now take the parameter list
   (`measure_in`) instead of a `Transformation`.
3. `transform_export.rs`: `vertical_unit` accepts a derived vertical CRS (`DerivedVerticalCRS` is a
   `VerticalCRS` in PROJ). `cartesian_unit_and_axis_swap(axes)` is the body of
   `projected_unit_and_axis_swap`, now also used for a derived projected target. `projstring.rs`
   `axes_order(axes)` is the body of `projected_axis_order`.
4. `compound.rs`, `compound_to_geog3d`: a derived vertical part is accepted, as PROJ accepts it.
   `same_datum_derived_vertical`: when the derived vertical has an `Ellipsoid` base and the target
   geographic CRS has the datum of the compound's horizontal part (`geogCRSOfVertCRSStack`), PROJ's second
   leg is the same-datum change of vertical unit, which `createOperationsDerivedTo` removes as a no-op
   (5547-5565). Only the inverse of the deriving conversion remains. `vertical_to_geog_candidates` takes
   the horizontal `GeographicCRS` and compares datums with `same_geodetic_datum`: a WKT CRS with an
   ensemble and the built-in EPSG:4979 do not share a datum name, and the name comparison missed the case.
5. `derived.rs`: `derived_parts` is `pub(super)`, for the compound code.

Before this lot, `compound_to_geog3d` returned no operation for a derived vertical part, and the
conversions of methods 9616 and 1046 had no PROJ string when they were the step of a concatenation.

## 4. Deviations and fixtures

- `coordinateOperationAccuracies()` holds `"0"` in C++. The test asserts `op.accuracy() == Some(0.0)`.
  The port's `accuracy_text()` is `None` for a computed accuracy, a separate gap that this lot does not
  change.
- `createGravityRelatedHeightToGeographic3D` is not in the public Rust API. The bound vertical CRS is a
  WKT `BOUNDCRS` with `METHOD["GravityRelatedHeight to Geographic3D"]` and `PARAMETERFILE`, which the WKT
  import reads as the same transformation.
- `same_datum_derived_vertical` covers only the `Ellipsoid` base with the same datum. A derived vertical
  over another datum takes the factory's candidates (`derived_to`), which PROJ also reaches through
  `createOperationsDerivedTo`, but no test in this lot exercises that path.
- `PROJ-based` conversions with a geographic source keep the former path: PROJ also inverts the geographic
  source's unit and axis order (conversion.cpp:3908-3915), which is not written here. No test covers it.

## 5. Commands run

- `cargo fmt` (then `cargo fmt --check`: clean).
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --all-features --no-fail-fast`, with `CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0`
  to fit the disk: 1725 passed, 0 failed, 27 ignored, 192 test binaries. That run was before the edit of
  `docs/UNPORTED-UNIT.md`, so `docs_in_sync` passed; it needs `UPDATE_DOCS=1` after the five removed lines.
- `PROJ_DATA=target/proj-reference/build/data target/proj-reference/build/bin/projinfo -s <src> -t <wkt>
  -o PROJ --single-line`: the expected strings of the derived vertical, derived projected and
  bound-vertical cases.
