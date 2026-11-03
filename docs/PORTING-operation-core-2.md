# Port — `operation-core-2`

PROJ 9.9.0 `test/unit/test_operation.cpp`, the `core` names still open in
`docs/UNIT-TRIAGE.md`, plus the still-open `niche` names of
`test_operationfactory.cpp`, `test_io.cpp`, `test_crs.cpp`, and
`test_c_api.cpp` where one is cheap. Tests live in
`tests/unit_operation_core_2.rs`. One Rust function per C++ `TEST()`.

Expected strings are copied from the 9.9.0 sources. Nothing was recomputed.

## Ported (13)

| C++ `TEST` | Rust |
|---|---|
| `operation.transformation_createTOWGS84` | `operation_transformation_create_towgs84` |
| `operation.createAxisOrderReversal` | `operation_create_axis_order_reversal` |
| `operation.PROJ_based` | `operation_proj_based` |
| `operation.PROJ_based_empty` | `operation_proj_based_empty` |
| `operation.PROJ_based_with_global_parameters` | `operation_proj_based_with_global_parameters` |
| `operation.export_of_Geographic3D_to_GravityRelatedHeight_gtx_unknown_grid` | `operation_export_of_geographic3d_to_gravity_related_height_gtx_unknown_grid` |
| `operation.export_of_boundCRS_with_proj_string_method` | `operation_export_of_bound_crs_with_proj_string_method` |
| `operation.Geographic3DToGravityRelatedHeight` | `operation_geographic3d_to_gravity_related_height` |
| `operation.inverse_of_Geographic3DToGravityRelatedHeight` | `operation_inverse_of_geographic3d_to_gravity_related_height` |
| `operation.CoordinateFrameRotationFullMatrixGeog2D` | `operation_coordinate_frame_rotation_full_matrix_geog2d` |
| `operation.operation_Geog3D_to_Geog2D_GravityRelatedHeight_with_compoundCRS` | `operation_operation_geog3d_to_geog2d_gravity_related_height_with_compound_crs` |
| `operation.helmert_between_geog3D_and_compound` | `operation_helmert_between_geog3d_and_compound` |
| `operation.operation_Geographic2D_Offsets_by_TIN_Interpolation_JSON` | `operation_operation_geographic2d_offsets_by_tin_interpolation_json` |

## API mapping

| PROJ | Rust |
|---|---|
| `SingleOperation::createPROJBased` | `Conversion::create_proj_based` |
| `Conversion::createAxisOrderReversal(bool)` | `Conversion::create_axis_order_reversal(AxisOrderReversal)` |
| `Transformation::createTOWGS84` | `Transformation::create_towgs84` |
| `exportToPROJString` of a transformation or concatenated operation | `export_to_proj_string`, now with grid steps |
| colon form `PROJ-based operation method: …` on a transformation | ingested in `export_forward` |
| inverse method names of EPSG:1136 / 1131 | `inverse_constant_geoid_height` |
| `AuthorityFactory::createCoordinateOperation` | `create_coordinate_operation` with `AlternativeGridNames::Official` |

## Deliberate differences

- `create_axis_order_reversal` takes `AxisOrderReversal::{TwoD, Geographic3d}` instead of a `bool`. The EPSG conversion, method, and identifier are the same (15498/9843 and 15499/9844).
- `create_towgs84` rejects the two inputs the C++ test throws on. A successful call uses the existing geocentric-translation and position-vector constructors (`9603`/`1031`, `9606`/`1033`). It does not switch a 3D geographic source to `1035` or `1037`. That success path is not in the C++ test.
- A nested `+proj=pipeline` is refused by `create_proj_based`. PROJ may throw from `create` or from the following `exportToPROJString`. The test only requires the call to fail.
- WKT1 of a PROJ-based conversion returns `FactoryError::Format("PROJBasedOperation can only be exported to WKT2")`. The C++ test only checks that WKT1 throws.

## Preserved quirks

- A PROJ-based forward string is the stored text with whitespace collapsed. `+ellps=WGS84` before the first `+step` stays. Re-ingesting that string would drop it.
- An empty PROJ-based definition exports `+proj=noop` in both directions, including the inverse. The WKT2 method text keeps the trailing space: `PROJ-based operation method: `.
- The inverse of method 1136 is `+inv` around `+proj=affine +zoff=-10`. Method 1136 is not a translation-inverse method, so the values are not negated.
- `ID["INVERSE(EPSG)",1136]` is not an EPSG code. The positive `zoff` is chosen from the method name, and the vertical CRS is the source argument of the forward exporter. Inversion swaps the CRS and then swaps them back, so that source stays the vertical CRS.
- `export_to_proj_string` writes grid steps (`vgridshift`, `tinshift`). `export_to_proj_string_with`, which `cs2cs` uses, still does not. Helmert methods do not read that flag.
- EPSG:10753 and EPSG:10854 are exported with the official grid name and no database substitution, matching `PROJStringFormatter::create()` without a database. 10854 keeps `no_kv_ETRS89NO_NGO48_TIN.json`.

## Open

Five `core` names and every still-open `niche` name of this lot. None of the niche cases is a one-function gap. `w2-opfac` owns operation-factory source; this lot did not change candidate selection.

| name | reason |
|---|---|
| `operation.normalizeForVisualization` | `normalize_for_visualization` is `pub(crate)` and returns `None` for a compound reversal. The C++ test also needs EPSG:4326/32631/3040/4979/3855/8047 equivalence, a bound NAD83 position-vector pipeline, and the concatenated operation's name suffix, accuracy, remarks, domain, and WKT round-trip. |
| `operation.PointMotionOperation_with_epochs` | `POINTMOTIONOPERATION` sets `point_motion`, and `export_steps` then returns `Unimplemented`. Epochs, `+proj=deformation +dt`, `+proj=set +omit_fwd`, and an inverse name that swaps the epochs are not stored. |
| `operation.operation_Position_Vector_geocen_and_geocen_translations_NEU_velocities_gtg_with_source_epoch` | EPSG:10814 is a position-vector step plus `+proj=deformation +dt=-2.44`. Deformation export only exists for the New Zealand method 1079 (`defmodel`), and the database formatter's grid substitution (`eur_nkg_nkgrf17vel.tif`) is part of that missing concatenation. |
| `operation.operation_geocen_translations_by_grid_and_geocen_translations_NEU_velocities_gtg` | EPSG:10811 concatenates `xyzgridshift` with a velocity-grid deformation (`+dt=-5`). `xyzgridshift` is written for method 1087 only, and the deformation step is the same missing export as EPSG:10814. |
| `operation.operation_geocen_translations_NEU_velocities_gtg` | EPSG:10809 is `+proj=deformation +t_epoch=2000` (the inverse drops the pipeline wrapper). That step is not the New Zealand `defmodel` path. |
| `wkt_parse.DerivedGeographicCRS_GDAL_PROJ4_EXSTENSION_hack` | Strict-false WKT1 `PROJECTION["Rotated_pole"]` plus `EXTENSION["PROJ4","+proj=ob_tran …"]` must become a derived geographic CRS. The importer only reads `EXTENSION` `PROJ4_GRIDS`. |
| `crs.derivedGeographicCRS_to_PROJ` | The derived geographic CRS can be imported, and its PROJ string still needs `createOperation` to build the `ob_tran` pipeline. That is not a one-function export. |
| `crs.derivedGeographicCRS_with_affine_transform_to_PROJ` | `createOperation` from the base to the derived geographic CRS does not export `+proj=affine`. |
| `crs.normalizeForVisualization_derivedprojected_operation` | The operation factory does not emit `utm` + the unimplemented step + the metre-to-foot `unitconvert` + the two axisswaps, and normalization is not applied to that operation. |
| `CApi.proj_create_crs_to_crs_coordinate_metadata_in_src` | `ITRF2014@2025.0` is not read as a CRS at an epoch, and `Proj::create_crs_to_crs_from_pj` refuses an epoch. |
| `CApi.proj_create_crs_to_crs_coordinate_metadata_in_target` | Same epoch refusal, with the epoch on the target. |
| `CApi.proj_crs_promote_to_3D_on_coordinate_metadata` | `ITRF2014@2010.0` is not parsed, and `promote_to_3d` does not keep an epoch or the dynamic identifier 7912. |
| `CApi.proj_trans_bounds_world_geodetic_to_spilhaus` | `Proj::new_known_crs` does not read `OGC:CRS84` or `ESRI:54099`. |
| `CApi.proj_crs_has_point_motion_operation` | There is no point-motion query that reports an operation for a dynamic CRS. `point_motion` is only a flag, and export of that flag is `Unimplemented`. |
| `operation.boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs` | `create_operations` does not chain a bound-CRS transformation whose source is the base of the derived source CRS. |
| `operation.compoundCRS_with_derivedVerticalCRS_ellipsoidal_height_to_geog` | A compound with a derived vertical component is not treated as a deriving conversion. |
| `operation.compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical` | The bound horizontal CRS and the derived vertical CRS are not chained together. |
| `operation.compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height` | Same derived-vertical compound path. |
| `operation.compoundCRS_with_derived_vertical_CRS` | Same derived-vertical compound path. |
| `operation.geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected` | The factory does not combine a bound vertical CRS with a derived projected component. |
| `derivedGeographicCRS_with_to_wgs84_to_geographicCRS.src_from_proj` | `+proj=ob_tran` is not read as a CRS, so the derived geographic source is never built. |
| `derivedGeographicCRS_with_to_wgs84_to_geographicCRS.src_from_wkt2` | WKT has no `Rotated_pole` / PROJ4 extension that would build that same `ob_tran` CRS. |
| `operation.createOperation_ossfuzz_47873` | The source is an `ob_tran` CRS. `projstring.rs` does not build one. |
| `operation.createOperation_ossfuzz_47873_simplified_if_i_might_say` | Same `ob_tran` CRS, then a pipeline. |
| `operation.createOperation_derived_projected_crs` | The deriving method is a PROJ-based pipeline ending in `+proj=affine`, which the derived-projected factory path does not export. |
| `operation.createOperation_spherical_ocentric_to_geographic` | `+proj=longlat +geoc` is not a CRS here, and a spherical CRS has no PROJ string. The test expects `+proj=geoc`. |
| `operation.createOperation_geographic_to_spherical_ocentric` | Same missing spherical planetocentric CRS, in the other direction. |
| `operation.createOperation_spherical_ocentric_to_geocentric` | Same missing `+proj=geoc` CRS. |
| `operation.createOperation_bound_of_spherical_ocentric_to_same_type` | The bound CRS cannot be built while its source `+proj=latlong +geoc` is refused. |
| `operation.createOperation_spherical_ocentric_to_projected` | Same missing spherical planetocentric CRS, with a projected target. |
| `operation.createOperation_spherical_ocentric_to_projected_of_spherical_ocentric` | Both the source and the projected base are spherical planetocentric CRS this crate does not read. |
| `operation.createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | Needs the planetocentric CRS and an ographic axis swap. |
| `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | Planetographic west-oriented axes are not a CRS kind here. |
| `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | Same planetographic CRS, with a planetocentric projected base. |
| `operation.createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | Same pair, reversed. |
| `operation.createOperation_point_motion_operation_geog2D` | The factory can set `point_motion`, and export then returns `Unimplemented`. Epoch steps are not written. |
| `operation.createOperation_point_motion_operation_geog3D` | Same point-motion export. |
| `operation.createOperation_point_motion_operation_geocentric` | Same point-motion export. |
| `operation.createOperation_point_motion_operation_geocentric_to_geog3D` | Same point-motion export, across a geocentric to geographic 3D pair. |
| `operation.createOperation_point_motion_operation_NAD83_CSRS_v7_TO_ITRF2014` | Datum pivot does not build the epoch-change operation between those dynamic datums. |
| `operation.createOperation_point_motion_operation_ITRF2014_to_NAD83_CSRS_v7` | Same missing epoch-change pivot, reversed. |
| `operation.createOperation_point_motion_operation_nkg` | The NKG velocity-grid point-motion operation is not built. |
| `operation.createOperation_compound_to_compound_with_point_motion_operation` | Compound-to-compound does not insert the point-motion step. |
| `operation.createOperation_compound_to_compound_with_point_motion_operation_special_case_CGVD2013a` | The CGVD2013a special case is the same missing point-motion concatenation. |
| `operation.createOperation_ossfuzz_18587` | PROJ accepts a broken `type=crs proj=pipeline` string and only requires that `createOperation` not recurse. `create_crs_from_proj_string` returns `Invalid("unsupported +proj= for a CRS")`, so the CRS is never built. |

## Verification

```text
cargo fmt
cargo test --offline --test unit_operation_core_2
cargo test --offline --test unit_crs_operation_core --test transform_export --test test_operation_transformations
cargo test --offline --test unit_opfactory_core_a --test unit_opfactory_core_b --test test_operationfactory
cargo clippy --all-targets --all-features --offline -- -D warnings
cargo test --offline --test docs_in_sync
```

`unit_operation_core_2` passed (13). The regression and factory tests named above passed. Clippy is clean. `docs_in_sync` fails on the generated counts in `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md`, and `docs/progress.svg` after the ledger deletions. Those files are left for the orchestrator.
