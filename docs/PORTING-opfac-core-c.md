# Port — `opfac-core-c`

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the 52 `operation` names of lot `opfac-c` (compound, bound and PROJ-string `createOperation(s)` cases). Expected names and PROJ strings are copied from that C++ file.

Rust tests are in `tests/unit_opfac_core_c.rs`. Each function is `operation_` plus the C++ name in snake case, and its doc comment carries the exact `TEST(operation, ...)` name.

**Result: 12 of 52 cases are ported. 40 stay open, each with a reason below.**

## Ported (12)

| C++ TEST | Rust test |
|---|---|
| `compoundCRS_of_horizCRS_with_TOWGS84_vertCRS_with_geoid_model_to_geogCRS` | `operation_compound_crs_of_horiz_crs_with_towgs84_vert_crs_with_geoid_model_to_geog_crs` |
| `createOperation_ETRS89_to_Amersfoort` | `operation_create_operation_etrs89_to_amersfoort` |
| `proj3DCRS_with_non_meter_horiz_and_vertical_to_geog` | `operation_proj3_d_crs_with_non_meter_horiz_and_vertical_to_geog` |
| `createOperation_fallback_to_proj4_strings` | `operation_create_operation_fallback_to_proj4_strings` |
| `createOperation_fallback_to_proj4_strings_bound_of_geog` | `operation_create_operation_fallback_to_proj4_strings_bound_of_geog` |
| `createOperation_fallback_to_proj4_strings_regular_with_datum_to_projliteral` | `operation_create_operation_fallback_to_proj4_strings_regular_with_datum_to_projliteral` |
| `createOperation_fallback_to_proj4_strings_geog_NAD83_to_projliteral` | `operation_create_operation_fallback_to_proj4_strings_geog_nad83_to_projliteral` |
| `createOperation_fallback_to_proj4_strings_regular_with_nadgrids_to_projliteral` | `operation_create_operation_fallback_to_proj4_strings_regular_with_nadgrids_to_projliteral` |
| `createOperation_fallback_to_proj4_strings_projliteral_to_projliteral` | `operation_create_operation_fallback_to_proj4_strings_projliteral_to_projliteral` |
| `createOperation_fallback_to_proj4_strings_regular_to_projliteral_with_towgs84` | `operation_create_operation_fallback_to_proj4_strings_regular_to_projliteral_with_towgs84` |
| `createOperation_fallback_to_proj4_strings_proj_NAD83_to_projliteral` | `operation_create_operation_fallback_to_proj4_strings_proj_nad83_to_projliteral` |
| `createOperation_fallback_to_proj4_strings_with_axis_inverted_projCRS` | `operation_create_operation_fallback_to_proj4_strings_with_axis_inverted_proj_crs` |

`createOperation_ETRS89_to_Amersfoort` was listed as open in `PORTING-opfac-core-2.md`; the existing registry search already returns the expected operations, so it is ported here without a source change.

The nine `createOperation_fallback_to_proj4_strings*` cases are the PROJ-string extension path, see [Source change](#source-change) below.

## Not ported (40)

Written and failing against this crate (29): the test was written, run, and removed from the file because it did not pass. A failing test is not kept in the tree.

| C++ TEST | Reason |
|---|---|
| `boundCRS_to_compoundCRS` | `createOperation` returns no operation for a PROJ-string pair with `+nadgrids` and `+geoidgrids`. |
| `compoundCRS_ftUS_to_geogCRS_ft` | Compound NAVD88 (ftUS) to 3D geographic: the `us_noaa_g2018u0.tif` vgridshift is not built, a unit-only conversion comes back. |
| `compoundCRS_ft_to_geogCRS_ft` | Same as above, from NAVD88 in feet. |
| `compoundCRS_m_to_geogCRS_ft` | Same as above, from NAVD88 in metres to a foot target. |
| `boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS` | No operation from the CH1903 bound CRS to `WGS 84 + EGM2008 height`: hub ensemble and `us_nga_egm08_25.tif` vgridshift not built. |
| `compoundCRS_from_WKT2_no_id_to_geogCRS_3D_context` | With official grid names the first PROJ string reads `rdtrans2018.gsb` (see Deviations); the id-less WKT2 finds fewer than 4 operations. |
| `compoundCRS_to_compoundCRS_context_extent_use_none` | `7406` to `5500` finds no operation under PartialIntersection; the compound-to-compound chain is not built. |
| `compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort` | No operation with ballparks forbidden: the NAP to ellipsoidal grid chain (`nl_nsgi_nlgeo2018.tif`) is not built. |
| `compoundCRS_to_geogCRS_with_height_depth_reversal` | One operation where PROJ gives three: NAVD88 depth compounds and their height-depth reversal are not built. |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change` | The vertical change is a ballpark, not the NAVD88 grid conversion, so the name differs. |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_height_depth_reversal` | One operation where PROJ gives three; same missing height-depth reversal of a compound. |
| `compoundCRS_to_proj_string_with_non_metre_height` | One operation where PROJ gives more: the vgridshift path into a `+vunits=us-ft` PROJ CRS is not built. |
| `compoundCRS_to_PROJJSON_with_non_metre_height` | Same as the PROJ-string case, from a PROJJSON compound. |
| `compoundCRS_with_non_meter_horiz_and_vertical_to_geog` | The final lat/lon axis swap is lost in `projected_compound_to_geog`, which cancels an east-north swap the target still needs. |
| `compoundCRS_of_bound_horizCRS_and_bound_vertCRS_to_geogCRS` | The `Geographic3D to GravityRelatedHeight (EGM)` method (9661) with a `PARAMETERFILE` does not export (`Unimplemented`). |
| `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric` | The IAU_2015 ellipsoidal CRS is written as `+proj=eqc` alone; the `+proj=geoc` steps are missing. |
| `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` | NAVD88 to NAD83 uses a ballpark vertical change, not the `us_noaa_g2012bu0.tif` GEOIDMODEL grid. |
| `compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS` | CGVD28 uses a ballpark vertical change, not the `HT2_2002v70` GEOIDMODEL grid. |
| `compoundCRS_of_vertCRS_with_geoid_model_by_name_and_several_records_to_geogCRS` | VIVD09 uses a ballpark vertical change; `us_noaa_g2012bp0.tif` and the Virgin Islands domain are not used. |
| `compoundCRS_of_vertCRS_with_geoid_model_by_name_and_datum_ensemble` | No operation from OSGB36 + ODN to ETRS89 geocentric: the OSTN15 and OSGM15 chain is not built. |
| `createOperation_ETRS89_XXX_to_ETRS89_YYY_using_ETRF2000` | Fewer than two operations; PROJ gives the ETRF2000 chain and the `+proj=noop` inverse. |
| `createOperation_ETRF2000_to_Amersfoort` | The result is a ballpark, not the ETRS89-NLD [AGRS2010] chain through `Amersfoort to ETRS89-NLD [AGRS2010] (9)`. |
| `createOperation_Geographic3D_Offset_by_velocity_grid` | The first operation is a ballpark: `ca_nrc_NAD83v70VG.tif` deformation is not used. |
| `createOperation_ITRF2014_to_ETRS89_DNK` | The first operation is a ballpark: the `eur_nkg_nkgrf17vel.tif` deformation step is not built. |
| `createOperation_test_createOperationsWithDatumPivot_iter_1` | The first operation is a ballpark; PROJ gives `+proj=noop` through the `NAD83(CSRS)v2 to NAD83(CSRS)v3 (3)` pivot. |
| `createOperation_compound_to_compound_with_Geographic3D_Offset_by_velocity_grid` | No operation for the NAD83(CSRS)v3 MTM 7 + HT2_1997 to UTM 19 + CGVD2013a(2010) compound: velocity and vertical grid chain not built. |
| `NAD83_to_projected_CRS_based_on_NAD83_2011` | One operation where PROJ gives two: the ballpark `NAD83 to NAD83(2011) + California Albers` is missing. |
| `createOperation_Vrtical_Offset_by_velocity_grid` | `Inverse of CGVD2013a(2010) height to CGVD2013a(2002) height (1)` is `Unimplemented` (vertical velocity grid, `deformation +dt=-8`). |
| `compoundCRS_from_WKT2_to_geogCRS_3D_context` | An operation of the WKT2 round trip of `EPSG:7415` is not `is_equivalent_to` the one from the EPSG CRS. |

Not written (11). The reason is an API that this crate does not expose, or a cause already documented in an earlier lot:

| C++ TEST | Reason |
|---|---|
| `IGNF_LAMB1_TO_EPSG_4326` | `IntermediateCRSUse::ALWAYS` is absent; the enum has `Allow` and `Never` only. |
| `isPROJInstantiable` | No `isPROJInstantiable` method and no grid-file check on `Transformation`. |
| `createOperation_on_crs_with_canonical_bound_crs` | No `baseCRSWithCanonicalBoundCRS` accessor. |
| `createOperation_compound_to_compound_with_epoch_HT2_1997_CGG2013a` | Needs `CoordinateMetadata` (an epoch attached to a CRS); no such type. |
| `createOperation_compound_to_compound_with_epoch_HT2_2002_CGG2013a` | Same as above. |
| `createOperation_compound_to_compound_with_epoch_HT2_2010_CGG2013a` | Same as above. |
| `createOperation_time_dependent_helmert_directly_from_database` | Needs `CoordinateMetadata`; no such type. |
| `createOperation_defmodel_from_database` | Needs `CoordinateMetadata`; no such type. |
| `createOperation_ITRF2000_to_ETRS89` | `gridsNeeded` is not exposed, so the grid-based and grid-less split cannot be checked. |
| `compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform` | `GridAvailabilityUse::KNOWN_AVAILABLE` is absent (`UseForSorting` and `IgnoreGridAvailability` only). |
| `createOperation_on_crs_with_bound_crs_and_wktext` | `+ignored1`, `+ignored2` and `+wktext` do not parse as a projected CRS; the bound pipeline needs bound-to-bound, as in `PORTING-opfactory-core-b.md`. |

## Source change

The nine `createOperation_fallback_to_proj4_strings*` cases needed PROJ's PROJ-string extension path, `createOperationsFromProj4Ext` in `coordinateoperationfactory.cpp`. It is now in `src/iso19111/projstring.rs` (`operation_from_proj_extensions`), called first in `CoordinateOperationFactory::create_candidates` (`src/iso19111/factory.rs`), as PROJ does before any other lookup:

- When a CRS, or the base of a bound CRS, carries a PROJ string extension, the operation is one PROJ-based conversion (`Conversion::create_proj_based`), built from the source written inverted, then the target.
- Each end writes its extension literal when it has one. A plain geographic end writes no step of its own. A projected end writes its export without `+type=crs`, `+units=m` and `+no_defs`, which PROJ writes only at the top level.
- A geographic end adds its angular unit conversion and axis order (`angular_unit_and_axis_swap` is now `pub(super)`).
- `attach_over_extension` now also records `+over` on a geographic CRS. It only handled projected CRS before, so `+proj=longlat +over` lost its literal.
- A projected end whose axes are north then east (EPSG:2193) gets `+proj=axisswap +order=2,1` inside the inversion, as PROJ writes it.
- In this export, NAD83 is written as `+ellps=GRS80` and not `+datum=NAD83` (`ProjExtras::legacy_crs_to_crs`), as `GeodeticCRS::addDatumInfoToPROJString` does with `getLegacyCRSToCRSContext()` in PROJ 9.9.0 (`src/iso19111/crs.cpp`).
- `+lon_wrap` is an extension parameter: a CRS string that has it keeps its whole text as `extension_proj4`, as `+over` does. See the scope note below.

Before this change the same pairs went through the EPSG pipeline builder, which picks a 7-parameter shift or a grid of its own.

## Deliberate differences

- `IntermediateCRSUse` and `GridAvailabilityUse` are the crate's enums. The C++ values `ALWAYS` and `KNOWN_AVAILABLE` have no counterpart, so those two cases are not written.
- `CrsExtentUse::Ignore` stands for C++ `NONE`. No ported case depends on it.

## Observed, not changed

- The context search returns official grid names. C++ `CoordinateOperationContext` defaults `usePROJNames_` to `true` (`src/iso19111/operation/coordinateoperationfactory.cpp`, 9.9.0), so the PROJ name is written: `nl_nsgi_rdtrans2018.tif` where the crate writes `rdtrans2018.gsb`. Switching `operations_between` to the PROJ names was tried. The full suite stayed green (1344 passed), and the one case it touches, `compoundCRS_from_WKT2_no_id_to_geogCRS_3D_context`, still fails on its operation count. The change was reverted because no ported test needs it. It belongs to a lot that also adds `setUsePROJAlternativeGridNames`.
- PROJ's `PROJStringParser` makes a CRS keep its extension whenever one of its parameters is not used by the parser (`hasUnusedParameters` in `io.cpp`, 9.9.0). That rule needs the set of parameters each projection reads. It is not ported. The crate only applies it to `OPERATION_ONLY_PARAMS` (`lon_wrap`, read by PROJ only when it runs an operation), so other unused parameters, such as `+x_0` on a method that does not use it, do not yet make an extension. A broader port would change the export of many CRS strings and needs its own lot.

## Ledger and bench

- `docs/UNPORTED-UNIT.md`: the names ported here are removed (seven in the first commit, two more in the gaps commit). `docs/UNIT-TRIAGE.md`, `README.md`, `ROADMAP.md` and `docs/progress.svg` are regenerated with `UPDATE_DOCS=1 cargo test --test docs_in_sync` and `UPDATE_DOCS=1 cargo test --test progress_svg`.
- `src/bin/bench.rs` is unchanged: these tests choose an operation and do not time a projection, as in `PORTING-opfactory-core-b.md`.

## Commands and results

- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: exit 0, 1367 passed, 0 failed, 26 ignored (all pre-existing in other test files).
- `cargo test --test unit_opfac_core_c`: 12 passed.
- Before the first change to `src/`, all 9 `createOperation_fallback_to_proj4_strings*` cases failed. After it, 7 passed. After the gaps change, all 9 pass.
