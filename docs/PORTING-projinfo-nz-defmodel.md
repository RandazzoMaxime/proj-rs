# Port: NZGD2000 to ITRFxx, New Zealand deformation model (projinfo)

Lot: `test_projinfo.yaml#52` to `#57`, `projinfo -s NZGD2000 -t ITRFxx -o PROJ -q --spatial-test intersects --hide-ballpark`.
Reference: PROJ 9.9.0. `coordinateoperationfactory.cpp`, `singleoperation.cpp`, `concatenatedoperation.cpp` and
`factory.cpp` of the tag, traced with `PROJ_DEBUG=3`.

Result: all six cases close (`#52` to `#57` are removed from `docs/UNPORTED-CLI.md`, none added). Neighbours checked
against PROJ 9.9.0: `NZGD2000 -> ITRF96`, `ITRF96 -> NZGD2000`, `NZGD2000 -> ITRF97`, `NZGD2000 -> ITRF2000`,
`NZGD2000 -> ITRF2005`, `NZGD2000 -> ITRF2008`, `NZGD2000 -> ITRF2014`, `EPSG:4959 -> EPSG:4918` identical.

## Function map

| PROJ (file:lines) | Rust (`src/iso19111/`) |
|---|---|
| `SingleOperation::_isEquivalentTo`, singleoperation.cpp:1483-1750 | `buildop.rs` `Transformation::is_equivalent_to_with`; `equiv.rs` `transformation_equivalent` |
| `ConcatenatedOperation::_isEquivalentTo`, concatenatedoperation.cpp:954-975 | `buildop.rs` `ConcatenatedOperation::is_equivalent_to` |
| `createOperationsWithDatumPivot` and `createTransformations`, coordinateoperationfactory.cpp:3167-3520 | `factory.rs` `datum_pivot_operations` |
| `setCRSs` of the first and last subOp, 3365-3372 | `factory.rs` `pivot_endpoints` |
| `Private::setCRSs` = `setCRSsUpdateInverse`, coordinateoperationfactory.cpp:3127-3132, singleoperation.cpp:240-272 | `bound_ops.rs` `set_endpoints_update_inverse` |
| `addModifiedIdentifier` with `derivedFrom`, 3297-3305 | `factory.rs` `derived_from_identifier` |
| `createComputeMetadata` of one operation, 3399-3405 | `factory.rs` `datum_pivot_operations` (a single step is returned as is) |
| `createFromCRSCodesWithIntermediates`, factory.cpp:7630-8100 | not ported as a whole; not reached for these cases (see open points) |
| `ConcatenatedOperation::createComputeMetadata`, concatenatedoperation.cpp:686-790 | `factory.rs` `pivot_concatenation`, `buildop.rs` `concatenate` |

## Commits of this lot

1. `docs: status sheet of the NZGD2000 to ITRF projinfo lot` and its correction (the first attempt's sheet, kept).
2. `iso19111: an operation's equivalence does not compare its CRS` (`equiv.rs`). PROJ's `SingleOperation::_isEquivalentTo`
   compares the name, the method and the parameter values; it never compares the source or target CRS.
   `transformation_equivalent` compared them strictly. This is the cause of the regression in
   `tests/unit_op_no_id_crs.rs` (`operation_geog_crs_without_id_to_geog_crs_3d_context`) that stopped attempt 1 and 2.
3. `iso19111: a transformation's equivalence does not compare its CRS` (`buildop.rs`). The same comparison in
   `Transformation::is_equivalent_to_with`, the one the factory's operations use. The interpolation CRS, the accuracy
   and the inversion are still compared (not PROJ's behaviour; left for a later lot, no test needs it changed).
4. `projinfo: the datum pivot sets the request endpoints, as createTransformations` (`factory.rs`). The first and the
   last step of each pivot get the request endpoints (`pivot_endpoints`); a pivot of one step is that step (no
   `Concatenated`); a null leg marks the registry operation `DERIVED_FROM`. Closes `#52`.
5. `projinfo: the leading step of a pivot takes the request source` (`factory.rs`, `set_leading_source`). The first
   leaf of a concatenated first step keeps the 3D source of the registry operation, so its PROJ string wrote
   `+z_in=m +z_out=m`. The request source goes down to that leaf. Closes `#53` to `#57`.
6. `iso19111: the pivot endpoints follow setCRSsUpdateInverse` (`bound_ops.rs`, `factory.rs`). Replaces commit 5
   with the PROJ mechanism. `createTransformations` calls `Private::setCRSs`, which is
   `CoordinateOperation::setCRSsUpdateInverse`: on a concatenated operation it also sets, recursively, the source of
   its first step and the target of its last step. That is why the first leaf of `Inverse of ITRF97 to NZGD2000 (1)`
   gets the 2D request source in PROJ. Same cases closed, the special `set_leading_source` is gone.

## Open points (not blocking this lot)

- Commit 3: the interpolation CRS of a transformation is still compared. PROJ's `SingleOperation::_isEquivalentTo`
  does not compare it.
- The candidate of `#53` to `#57` is the datum pivot's first result in both: the port's first datum pivot pair
  with a registry operation gives the one registry concatenation `Inverse of ITRF97 to NZGD2000 (1)`, as PROJ's
  iteration 0 does for the pair `4959 -> 4918` (trace of `NZGD2000 -> ITRF97`). The intermediate search
  (`createFromCRSCodesWithIntermediates`, factory.cpp:7630) is not ported as a whole; its candidates come after
  this one and are not compared by these cases.

## Result per case (`-o PROJ -q --spatial-test intersects --hide-ballpark`)

| yaml | target | before the lot | after the lot |
|---|---|---|---|
| #52 | ITRF96 | `z` steps, `unknown id` concatenation | identical to PROJ 9.9.0, closed |
| #53 | ITRF97 | `z` steps only | identical to PROJ 9.9.0, closed |
| #54 | ITRF2000 | `z` steps only | identical to PROJ 9.9.0, closed |
| #55 | ITRF2005 | `z` steps only | identical to PROJ 9.9.0, closed |
| #56 | ITRF2008 | `z` steps only | identical to PROJ 9.9.0, closed |
| #57 | ITRF2014 | `z` steps only | identical to PROJ 9.9.0, closed |

## Follow-up: two regressions of commit 4 (branch `lot-projinfo-nz-defmodel-6`)

Commit 4 returns a single-step pivot as is instead of a concatenation of one step. Two unit cases relied on that
wrapper and failed from `443dc2f`; their expected values are PROJ 9.9.0's (`test_c_api.cpp`,
`test_operationfactory.cpp`), confirmed with the reference `projinfo` and a traced build (`PROJ_DEBUG=3`).

7. `iso19111: the vertical transformations of a compound stop at the hub` (`compound.rs`, `bound_ops.rs`,
   `factory.rs`). Case: `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_
   against_WGS84_to_WGS84_G1762`, EPSG:6340 + a vertical CRS bound to EPSG:4979 (`egm96_15.gtx`) to EPSG:7665.
   - PROJ: `createOperationsCompoundToGeog` computes `verticalTransforms` under `SetSkipHorizontalTransform`
     (coordinateoperationfactory.cpp:6745-6784). With `skipHorizontalTransformation`,
     `createOperationsBoundToGeog` returns for a bound vertical CRS only the operations to its hub, plus a change
     of vertical unit when the height axes differ (5856-5914). The vertical transform is `unknown to WGS 84
     ellipsoidal height`, its interpolation CRS WGS 84, and `createHorizVerticalHorizPROJBased` chains
     `NAD83(2011) to WGS 84 (1)`, the geoid grid and `WGS 84 to WGS 84 (G1762)` (6936-7140).
   - Port before: `bound_to_geog` always went on from the hub to the target. The vertical operation was
     `unknown to WGS 84 ellipsoidal height + WGS 84 to WGS 84 (G1762)`. Before commit 4 the second step was a
     one-step concatenation, which `interpolation_geographic` (`getInterpolationGeogCRS`) does not count; after it,
     a second transformation, so no interpolation CRS was found and `bound_compound_to_geog3d` fell back to the
     horizontal operation to the target: the time-dependent `NAD83(2011) -> ITRF2008 -> WGS 84 (G1762)` pivot,
     without the geoid grid.
   - Port now: the flag `GUARD_SKIP_HORIZONTAL` of the context is set where PROJ sets it (`compound_vertical_pass`,
     `bound_compound_to_geog3d`) and read in `bound_to_geog` (`skip_horizontal_transformation`). Both candidates
     have PROJ's pipelines and extents.
   - Remaining difference, not compared by the yaml or the unit case: the name of candidate 1 is `Inverse of UTM
     zone 11N + NAD83(2011) to WGS 84 (1) + unknown to WGS 84 ellipsoidal height + WGS 84 to WGS 84 (G1762)`;
     PROJ writes `Inverse of UTM zone 11N + unknown to WGS 84 ellipsoidal height using NAD83(2011) to WGS 84 (1)`.
     PROJ first splits a derived horizontal component into its base (`NAD83(2011) + VerticalDummyCrs`,
     6474-6510) and names the inner operation with `using` because the inverse of `WGS 84 to WGS 84 (G1762)` is
     equivalent to `NAD83(2011) to WGS 84 (1)` (2821-2839). The port does not split a derived component for a
     bound vertical part.
8. `projinfo: a 3D pivot rebuilds a registered concatenated operation` (`factory.rs`,
   `promote_pivot_operation_to_3d`). Case: `operation.geogCRS_promoted_to_3D_to_geogCRS_3D_context`, EPSG:4269
   promoted to 3D to EPSG:6319 and back.
   - PROJ: `createTransformations` with `sourceAndTargetAre3D` rebuilds a concatenated second step with
     `createComputeMetadata` from its steps promoted to 3D (3318-3349): computed name, no identifier, summed
     accuracy. The `DERIVED_FROM` marking of a null leg (3290-3317) is done before and dropped by the rebuild.
   - Port before: each step's endpoints were promoted and the registered operation kept. With the null legs
     dropped by commit 4, `PROJ:NAD83_TO_NAD83_2011_CONUS` came out as `NAD83 to NAD83(2011) (NADCON5, CONUS)`,
     0.1 m, first. PROJ lists `NAD83 to NAD83(HARN) (47) + ... + NAD83(NSRS2007) to NAD83(2011) (1)`, 0.2 m.
   - Port now: the same five candidates as PROJ, in its order, with its names, identifiers and accuracies.

`test_projinfo.yaml` `#52` to `#57` stay closed; `cli_yaml` leaves `docs/UNPORTED-CLI.md` unchanged.
