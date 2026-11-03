# Port — lot io-capi-core-2

Source: PROJ 9.9.0 `test/unit/test_io.cpp`, `test_c_api.cpp`, and `test_factory.cpp`.
The 61 names are the `core` entries of those three files still listed in
`docs/UNIT-TRIAGE.md` at the start of the lot. Rust tests live in
`tests/unit_io_capi_core_2.rs`. A C++ `TEST()` name becomes the Rust function
name in lowercase with `.` replaced by `_`. Expected text is copied from the
C++ source.

## 1. Mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `WKTParser::getESRIMapping` and `buildProjectionFromESRI` | `import_esri_projection` in `src/iso19111/esri_parammap.rs` |
| `ESRIParamMapping::is_fixed_value` (Behrmann only) | `EsriParam::Defaulted` |
| WKT1 `PROJECTION` when the dialect is ESRI or a guess, and the GDAL fallback for an ESRI method name | `conversion_from_esri` in `src/iso19111/wkt_import.rs` |
| No `AXIS` and no `CS`: method 9819 southing/westing, 9808 westing/southing, 9810/9820 polar axes | `build_projected` and `polar_pole_cs` |
| Unmapped WKT1 method keeps the parameter names from the file | the `wkt1_parameter` rename runs only when the method code is non-empty |
| `CONCATENATEDOPERATION` `SOURCECRS` / `TARGETCRS` | `build_concatenated` |
| `+proj=longlat +axis=` and the operation from EPSG:4326 | already in `projstring.rs` and `CoordinateOperationFactory::create_operation` |

`EsriParam::Defaulted` is injected into the shared parameter map when absent and
is not a miss. A `Fixed` value that is present and unequal rejects that row.
Guessed style (`maybe_esri`) drops a row whose misses are at least its hits.
A stereographic latitude of origin of exactly ±90 selects polar stereographic
variant A (EPSG:9810). Plate Carrée on method 1028 injects standard parallel 0.
`Conversion::identify` still only renames UTM.

## 2. Ported (17)

Each name has a passing test in `tests/unit_io_capi_core_2.rs` and its line is
removed from `docs/UNPORTED-UNIT.md`.

- `wkt_parse.esri_projcs`
- `wkt_parse.wkt1_esri_krovak_south_west`
- `wkt_parse.wkt1_esri_krovak_east_north_non_standard_likely_from_GDAL_wkt1`
- `wkt_parse.wkt1_esri_ups_north`
- `wkt_parse.wkt1_esri_ups_south`
- `wkt_parse.wkt1_esri_wgs_1984_ups_north_E_N`
- `wkt_parse.wkt1_esri_wgs_1984_ups_south_E_N`
- `wkt_parse.wkt1_esri_gauss_kruger`
- `wkt_parse.wkt1_esri_goode_homolosine_without_option_0`
- `wkt_parse.wkt1_lcc_1sp_without_1sp_suffix`
- `wkt_parse.wkt1_pseudo_wkt1_gdal_pseudo_wkt1_esri`
- `wkt_parse.invalid_EngineeingCRS`
- `wkt_parse.CONCATENATEDOPERATION`
- `io.projparse_longlat_complex`
- `io.projparse_longlat_axis_enu`
- `io.projparse_longlat_axis_neu`
- `io.projparse_longlat_axis_swu`

## 3. Open (44)

One row per name still in the ledger.

| name | reason |
|---|---|
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_and_conversion` | The first step is `ID["INVERSE(EPSG)",16031]`. `build_concatenated` always stores `reverse: false` and does not read an `INVERSE()` identifier, so the PROJ string is not `+proj=pipeline +step +inv +proj=utm +zone=31 +ellps=WGS84 +step +proj=utm +zone=32 +ellps=WGS84`, and the WKT2:2019 text is not reproduced byte for byte. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_conversion` | Three steps (inverse UTM zone 11N on NAD27, `ID["DERIVED_FROM(EPSG)",15851]` CTABLE2 `conus`, forward UTM zone 11N). The expected PROJ string is `+inv +proj=utm +zone=11 +ellps=clrk66`, then `+proj=hgridshift +grids=conus`, then `+proj=utm +zone=11 +ellps=WGS84`. Inverse identifiers and the version `"my version"` are not applied on import. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_to_inverse_conversion` | The last step is the inverse of the first conversion, not a second forward UTM. The same missing `INVERSE()` step flag means the exported pipeline does not invert that conversion. |
| `wkt_parse.CONCATENATEDOPERATION_with_inverse_conversion_of_compound` | Source is the compound Amersfoort / RD New + NAP height. The expected pipeline starts with `+inv +proj=sterea` (Bessel, false easting 155000) and then `hgridshift` `nl_nsgi_rdtrans2018.tif` and `vgridshift` `nl_nsgi_nlgeo2018.tif`. The inverse of the oblique stereographic inside the compound is not detected. |
| `wkt_parse.wkt1_oracle` | After the Lambert Conformal Conic reading, `ProjectedCRS::identify_with` only returns a UTM zone. The test requires the first hit to be EPSG:2154 at confidence 90. |
| `wkt_parse.wkt1_oracle_albers_conical_equal_area` | Same identify path. The test requires EPSG:3005 at confidence 100 for the NAD83 / BC Albers WKT. |
| `io.projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` | `src/iso19111/pipeline.rs` does not rewrite a forward `hgridshift`, a `vgridshift`, and the inverse `hgridshift` into `push +v_1 +v_2`, `omit_inv` / `omit_fwd`, and `pop`. |
| `io.projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc` | The same formatter does not pull `push +v_1 +v_2` in front of axisswap, unitconvert, and `vgridshift`, nor drop the repeated axisswap and unitconvert. |
| `io.projstringformatter_krovak_to_krovak_east_north` | An inverse `krovak +axis=swu` followed by a forward `krovak` must collapse to `+proj=axisswap +order=-2,-1`, and a missing or different parameter must not. The Krovak `axis=swu` merge is not in the pipeline formatter. |
| `io.projstringformatter_krovak_east_north_to_krovak` | The opposite order (inverse Krovak, then `+axis=swu`) must collapse to the same axisswap, with the same negative cases. That merge is absent. |
| `io.projparse_init` | Compatibility mode is absent. `init=epsg:4326` must be longitude/latitude, `init=epsg:3040 +type=crs` easting/northing, `init=ITRF2000:ITRF2005` the listed Helmert, `+title=mytitle +init=epsg:4326 +over` must keep the title and `+over`, and a pipeline must keep `init=epsg:4326` as a step. The parser treats `init=` as an ordinary parameter, and non-compatibility mode must reject `init=epsg:4326 +type=crs`. |
| `io.createFromUserInput` | `create_from_user_input` returns only a `Crs`. The test also requires a datum, an ellipsoid, a prime meridian, an ensemble, coordinate-operation URNs, and `CoordinateMetadata` for `ITRF2014@2025.1`. `create_object_from_user_input` adds JSON datums only. Widening the CRS function would break its callers. |
| `json_import.bound_crs_with_source_crs_in_transformation` | `json_read::Reader::create` has no `DerivedGeographicCRS` arm, so this BoundCRS (source "CH1903+ with height offset", transformation EPSG:1676) fails before the `$schema` `"foo"` round-trip. The export side can write that type. |
| `CApi.proj_crs_create_bound_crs_to_WGS84_on_invalid_type` | The input is the WKT of a conversion, and `proj_crs_create_bound_crs_to_WGS84` must return null. `Crs::create_bound_crs_to_wgs84_if_possible` takes a CRS, so a conversion cannot be passed and the null is not an API result. |
| `CApi.proj_create_from_database_grid_alternative_null_old_proj_grid_name` | With the network enabled, the PROJ string of EPSG:9484 must contain `no_kv_href2008a.tif` because `old_proj_grid_name` is NULL. The operation export does not apply that substitution. |
| `CApi.proj_coordoperation_get_grid_used` | EPSG:1312 must report one grid, short name `ca_nrc_ntv1_can.tif`, empty package, direct download and open licence, and URL `https://example.com/ca_nrc_ntv1_can.tif` after the context endpoint changes. `DatabaseContext::grid_info` looks up a name and returns the database URL. It is not the used-grid list of an operation. |
| `CApi.proj_coordoperation_get_grid_used_fullname_caching` | Under `TIFF_ENABLED`, `proj=vgridshift grids=tests/test_vgrid_int16.tif` must report that short name and a full name containing it, on two calls. There is no TIFF grid-used cache. |
| `CApi.proj_coordoperation_is_instantiable` | EPSG:1671 must be instantiable (`1`). The crate can build the operation and has no predicate that answers whether its grids can be opened. |
| `CApi.proj_create_operations` | NAD27 (EPSG:4267) to NAD83 (EPSG:4269), partial intersection, grids ignored: count 10, first name `NAD27 to NAD83 (4)`, and coordinate (40, -100) suggests `NAD27 to NAD83 (7)`. `create_operations` does not build that ordered list or a suggestion index. |
| `CApi.proj_get_suggested_operation_for_NAD83_to_NAD83_HARN` | NAD83 (EPSG:4269) to NAD83(HARN) (EPSG:4152). Coordinate (40, -100) must select `NAD83 to NAD83(HARN) (47)`. There is no suggestion-by-coordinate. |
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | NTF Paris Lambert (EPSG:27562) to ETRS89 (EPSG:4258). The operation suggested at (136555.58288992, 463344.51894296) must use exactly one grid. There is no suggestion index and no used-grid count on the chosen operation. |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | NAD83(2011) geocentric (EPSG:6317) to NAD83(2011) 2D (EPSG:6318). The suggestion at (-463930, -4414006, 4562247) must be a non-negative index even when an operation has no area of use. There is no suggestion index. |
| `CApi.proj_create_operations_discard_superseded` | AGD84 (EPSG:4203) to WGS 84 (EPSG:4326) with superseded operations discarded must return 4 operations. `CoordinateOperationContext` has no discard-superseded flag. |
| `CApi.proj_create_operations_dont_discard_superseded` | The same AGD84 to WGS 84 pair with superseded operations kept must return 5. The context has no such flag, so the two counts cannot both be asserted. |
| `CApi.proj_create_operations_with_pivot` | ED50 (EPSG:4230) to ETRS89-FRA (EPSG:4171) has no direct transformation. The default list has one operation named `ED50 to ETRS89 (10) + ETRS89 to ETRS89-FRA [RGF93 v1]`. A pivot is used only when `set_intermediate_crs` names one. |
| `CApi.proj_create_operations_allow_ballpark_transformations` | NAD27 (EPSG:4267) to ETRS89 (EPSG:4258) must return 1 operation when ballpark is allowed and 0 when it is not. The factory's ballpark rule does not produce those two counts for this pair. |
| `CApi.proj_normalize_for_visualization_with_alternatives` | EPSG:4326 to EPSG:3003, after visualization normalization, must map (12.5, 42) to about (1789912.46264783037, 4655716.25402576849) through the listed position-vector Helmert on `intl`. The alternative pipeline is not selected or normalized to that string. |
| `CApi.proj_normalize_for_visualization_with_alternatives_reverse` | EPSG:3003 to EPSG:4326 must map that same projected point back to longitude 12.5 and latitude 42 within 1e-8, with degree output. The reverse alternative is not built. |
| `CApi.proj_coordoperation_create_inverse` | The inverse of the Everest-to-WGS84 pipeline must be the listed text with `MULTILINE=YES`, `INDENTATION_WIDTH=4`, and `MAX_LINE_LENGTH=40` (cart on WGS84 before the inverted Helmert). `PipelineFormatter` wraps at 80 columns with a two-space indent and does not take those options. |
| `CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` | EPSG:4326 to EPSG:4258 must succeed with no options and return null for `ACCURACY=0.05`. `Proj::create_crs_to_crs_from_pj` rejects every non-empty option. |
| `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter` | NAD27 (EPSG:4267) to ETRS89 (EPSG:4258) must succeed with no options and return null for `ALLOW_BALLPARK=NO`. Non-empty options are rejected before a ballpark filter runs. |
| `CApi.proj_create_crs_to_crs_with_only_ballpark_transformations` | `EPSG:25831+5773` to `EPSG:7665`, normalized for visualization, must map (500000, 4500000, 0) to about (3.0, 40.65085651660555, 47.72600023608570). The known-CRS constructor does not read that compound, and the ballpark-only pipeline is not built. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_WGS84_to_WGS84_G1762` | The compound is EPSG:6340 plus a bound vertical CRS on dummy datum with grid `egm96_15.gtx` toward EPSG:4979, transformed toward WGS 84 (G1762). There is no `proj_crs_create_bound_vertical_crs` entry and no operation from that compound. |
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762` | Same compound shape, with the geoid referenced against NAD83(2011) rather than WGS 84, then to WGS 84 (G1762). The bound vertical CRS and the operation are both absent. |
| `CApi.proj_create_vertical_crs_ex` | Builds `myVertCRS (ftUS)` on `myVertDatum` in US survey foot with grid `PROJ @foo.gtx` and `ACCURACY=123`, compounds it with EPSG:6340, and requires at least one operation to NAD83(2011) 3D (EPSG:6319). There is no `proj_create_vertical_crs_ex`. |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | The vertical CRS takes a geographic CRS argument (EPSG:4979) together with `PROJ @foo.gtx`. That overload does not exist, so the compound and the operation to EPSG:6319 are not created. |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | Vertical CRS on the AHD datum (EPSG:5711) with grid `PROJ au_ga_AUSGeoid09_V1.01.tif` and geoid CRS EPSG:4939, compounded with GDA94 (EPSG:4283). Accuracy is implied by the geoid model. The ex constructor does not exist. |
| `CApi.proj_trans_bounds_ignore_inf` | `OGC:CRS84` to `ESRI:102036`, 21 divisions, box (-180, -90, 180, 1.3). Expected bounds are about left/right ±49621755.4, bottom -116576598.5, top 50132027.2 after infinite gnomonic samples are dropped. The known-CRS constructor does not open that ESRI code, and the bound walk is not that horizon. |
| `CApi.proj_trans_bounds_ignore_inf_geographic` | Inverse: an inline Interrupted Goode Homolosine PROJCS to `OGC:CRS84`. The box (-15028000, 7515000, -14975000, 7556000) must land near (-179.2133, 70.9345, -177.9054, 71.4364). Inline WKT plus that geographic bound is not produced. |
| `CApi.proj_trans_bounds_to_compound_crs` | EPSG:4326 to EPSG:9707 (WGS 84 + EGM96 height), densify 0, box (40, -120, 64, -80) must come back within 1e-8. The known-CRS constructor does not build that compound, so the bound is not run. |
| `CApi.proj_trans_bounds_3D_ignore_inf` | Same `OGC:CRS84` to `ESRI:102036` pair as the 2D inf test, with z from 0 to 0 over 21 divisions. Expected xy matches the 2D box and both z ends stay 0. The ESRI CRS and the 3D inf filter are not in place. |
| `factory.AuthorityFactory_createDynamicVerticalDatum` | The body is under `#ifdef no_more_dynamic_vertical_datum` and is not compiled in PROJ 9.9.0. It would cast EPSG:1096 to a dynamic vertical frame and expect epoch 2000. This port does not assert a behavior 9.9.0 does not compile. |
| `factory.AuthorityFactory_build_all_concatenated` | Every EPSG concatenated-operation code must build with `usePROJAlternativeGridNames=false`, except 8422, 8481, 8482, 8565, 8566, 8572, 11005, 11398, and 11206, which must throw, and the non-deprecated set must be smaller. Those nine codes build here because there is no chaining-consistency check. |
| `factory.ogc_timecrs` | `AuthorityFactory` for OGC must create `AnsiDate`, `JulianDate`, and `UnixTime`. A temporal CRS type exists. The OGC factory does not build those three time CRS. |

## 4. Deviations

- A south-oriented transverse Mercator written as scale -1 is rewritten before
  the ESRI table is consulted, and `Transverse_Mercator_South_Orientated` does
  not enter that table. PROJ's ESRI path does not do this. Existing tests of
  that WKT depend on it.
- `Mercator_1SP` that is not an ESRI name still uses the standard WKT1 reading
  when the ESRI lookup returns nothing.
- Oracle and Oracle Albers are left open rather than dropping the identify
  assertion. `ProjectedCRS::identify_with` stays UTM-only.

## 5. Commands

```
cargo test --offline --test unit_io_capi_core_2
cargo test --offline --test unit_io_core_a --test unit_io_core_b --test test_io_wkt1 --test test_operation_conv --test test_io --test unit_crs_operation_core
cargo fmt --all
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --test docs_in_sync
```

`unit_io_capi_core_2` is 17 passed. The regression set above is green
(pre-existing ignores unchanged). `docs_in_sync` fails on the generated
counts in `README.md` and `docs/UNIT-TRIAGE.md`. Those files, `ROADMAP.md`,
and `docs/progress.svg` are not edited here.
