# Port — PROJ `test/unit/test_crs.cpp`, lot crs-core-2

Source, PROJ 9.9.0: `test/unit/test_crs.cpp`, the 50 `core` names under `## core` / `### test_crs.cpp (50)` in `docs/UNIT-TRIAGE.md`. Implementation notes also come from `src/iso19111/crs.cpp` (`CRS::createBoundCRSToWGS84IfPossible`).

Each passing test is one function in `tests/unit_crs_core_2.rs`, named `crs_<C++ name>` in snake case. The comment above it is the C++ `TEST()` name. Expected text is copied from `test_crs.cpp`.

## Tests ported and passing (36)

| C++ TEST | Rust test |
|---|---|
| `crs.projectedCRS_from_WKT1_ESRI_ETRS89_with_db` | `crs_projected_crs_from_wkt1_esri_etrs89_with_db` |
| `crs.projectedCRS_3D_is_WKT1_equivalent_to_WKT2` | `crs_projected_crs_3d_is_wkt1_equivalent_to_wkt2` |
| `crs.projectedCRS_identify_no_db` | `crs_projected_crs_identify_no_db` |
| `crs.verticalCRS_ESRI_115834_as_WKT1_ESRI_with_database` | `crs_vertical_crs_esri_115834_as_wkt1_esri_with_database` |
| `crs.verticalCRS_datum_ensemble` | `crs_vertical_crs_datum_ensemble` |
| `crs.compoundCRS_valid` | `crs_compound_crs_valid` |
| `crs.compoundCRS_as_WKT1_GDAL` | `crs_compound_crs_as_wkt1_gdal` |
| `crs.compoundCRS_as_PROJ_string` | `crs_compound_crs_as_proj_string` |
| `crs.boundCRS_to_WKT2` | `crs_bound_crs_to_wkt2` |
| `crs.boundCRS_to_WKT1` | `crs_bound_crs_to_wkt1` |
| `crs.boundCRS_geographicCRS_to_PROJ_string` | `crs_bound_crs_geographic_crs_to_proj_string` |
| `crs.boundCRS_projectedCRS_to_PROJ_string` | `crs_bound_crs_projected_crs_to_proj_string` |
| `crs.boundCRS_nadcon_to_PROJ_string_usePROJAlternativeGridNames_true` | `crs_bound_crs_nadcon_to_proj_string_use_proj_alternative_grid_names_true` |
| `crs.incompatible_boundCRS_hubCRS_to_WKT1` | `crs_incompatible_bound_crs_hub_crs_to_wkt1` |
| `crs.incompatible_boundCRS_transformation_to_WKT1` | `crs_incompatible_bound_crs_transformation_to_wkt1` |
| `crs.WKT1_DATUM_EXTENSION_to_WKT1_and_PROJ_string` | `crs_wkt1_datum_extension_to_wkt1_and_proj_string` |
| `crs.WKT1_VERT_DATUM_EXTENSION_to_WKT1` | `crs_wkt1_vert_datum_extension_to_wkt1` |
| `crs.WKT1_VERT_DATUM_EXTENSION_to_WKT2` | `crs_wkt1_vert_datum_extension_to_wkt2` |
| `crs.WKT1_VERT_DATUM_EXTENSION_to_PROJ_string` | `crs_wkt1_vert_datum_extension_to_proj_string` |
| `crs.extractGeographicCRS` | `crs_extract_geographic_crs` |
| `crs.extractVerticalCRS` | `crs_extract_vertical_crs` |
| `crs.default_identify_method` | `crs_default_identify_method` |
| `crs.crs_stripVerticalComponent` | `crs_crs_strip_vertical_component` |
| `crs.crs_alterGeodeticCRS` | `crs_crs_alter_geodetic_crs` |
| `crs.crs_alterCSLinearUnit` | `crs_crs_alter_cs_linear_unit` |
| `crs.alterParametersLinearUnit` | `crs_alter_parameters_linear_unit` |
| `crs.getNonDeprecated` | `crs_get_non_deprecated` |
| `crs.promoteTo3D_and_demoteTo2D` | `crs_promote_to_3d_and_demote_to_2d` |
| `crs.projected_normalizeForVisualization_do_not_mess_deriving_conversion` | `crs_projected_normalize_for_visualization_do_not_mess_deriving_conversion` |
| `crs.projected_promoteTo3D_do_not_mess_deriving_conversion` | `crs_projected_promote_to_3d_do_not_mess_deriving_conversion` |
| `crs.projected_demoteTo2D_do_not_mess_deriving_conversion` | `crs_projected_demote_to_2d_do_not_mess_deriving_conversion` |
| `crs.projected_alterGeodeticCRS_do_not_mess_deriving_conversion` | `crs_projected_alter_geodetic_crs_do_not_mess_deriving_conversion` |
| `crs.projected_alterCSLinearUnit_do_not_mess_deriving_conversion` | `crs_projected_alter_cs_linear_unit_do_not_mess_deriving_conversion` |
| `crs.projected_alterParametersLinearUnit_do_not_mess_deriving_conversion` | `crs_projected_alter_parameters_linear_unit_do_not_mess_deriving_conversion` |
| `crs.is_dynamic` | `crs_is_dynamic` |
| `crs.identical_after_import_from_proj4_and_wkt1_export` | `crs_identical_after_import_from_proj4_and_wkt1_export` |

## Left open (14)

Each of these was run against the crate. The reason is what the run produced.

| name | reason |
|---|---|
| `crs.projectedCRS_from_WKT1_ESRI_as_WKT1_ESRI_s_jtsk03_krovak_east_north` | ESRI `Krovak` with `X_Scale`/`Y_Scale`/`XY_Plane_Rotation` imports as method Krovak EPSG:9819; the expected method is Krovak (North Orientated) EPSG:1041. |
| `crs.projectedCRS_Krovak_EPSG_5221_as_PROJ_string` | Alpha `30.2881397527778` matches, but the pipeline drops `+inv +proj=longlat +ellps=bessel +pm=ferro` and `+pm=ferro` on the krovak step. |
| `crs.projectedCRS_Krovak_with_approximate_alpha_as_PROJ_string` | Alpha `30.2881397222222` matches and neither side has a leading axisswap; the same ferro inverse longlat step is missing. |
| `crs.projectedCRS_identify_db` | `ProjectedCRS::identify_with` returns no hit for EPSG:2172; a non-UTM projected CRS does not search the database. |
| `crs.projectedCRS_identify_wrong_auth_name_case` | `Esri:54034` through an anonymous factory returns no hit; an authority other than empty or EPSG returns empty, and the CRS is not UTM. |
| `crs.verticalCRS_identify_db` | `identify_with` returns no hit for EPSG:7651; vertical identify does not search the database. |
| `crs.compoundCRS_identify_db` | `identify_with` returns no hit for EPSG:8769; compound identify does not search the database. |
| `crs.boundCRS_identify_db` | The WKT parses as a bound CRS and `identify_with` returns no hit. |
| `crs.boundCRS_crs_link` | C++ checks `shared_ptr` use counts and `canonicalBoundCRS` back-pointers; Rust CRS values are owned clones with no canonical-bound link. |
| `crs.boundCRS_nadcon_to_PROJ_string_usePROJAlternativeGridNames_false` | The official grid exports as `conus.las`; PROJ's PROJ-string formatter with a database rewrites it to `us_noaa_conus.tif`, and `Crs::export_to_proj_string` has no database argument. |
| `crs.crs_createBoundCRSToWGS84IfPossible` | EPSG:2936 binds through the single returned operation "Pulkovo 1942 to WGS 84 (16)"; PROJ finds two extent-matching Helmerts and leaves it unbound. |
| `crs.normalizeForVisualization_bound` | `Crs::normalize_for_visualization` clones a bound CRS, so the trailing axisswap stays and the source axes are not swapped; `create_operation_proj_string` also fails on that CRS. |
| `crs.norway_ntm` | ESRI WKT is `GEOGCS["GCS_ETRS89-NOR_EUREF89",DATUM["D_EUREF89",...]]`; PROJ 9.9.0 writes `GCS_ETRS_1989` / `D_ETRS_1989`. |
| `crs.ETRF2000_PL_CS92` | The imported name stays `ETRF2000-PL / CS92`; the expected name is the deprecated substitution `ETRS89 / PL-1992`. |

The other eight `test_crs.cpp` ledger lines (`crs.scope_area_bbox_remark`, `crs.multiple_ID`, `crs.VerticalCRS_ensemble_exception_in_create`, `datum.vdatum_with_anchor`, `crs.derivedGeographicCRS_to_PROJ`, `crs.derivedGeographicCRS_with_affine_transform_to_PROJ`, `crs.normalizeForVisualization_derivedprojected_operation`, `crs.projected_is_equivalent_to_with_proj4_extension`) are outside this lot and stay in the ledger.

## API mapping

| C++ | Rust |
|---|---|
| `bool convertToNewUnit` on `alterParametersLinearUnit` | `ParameterLinearUnit::{Relabel, Convert}` |
| `bool considerWgs84Dynamic` on `isDynamic` | `Wgs84Dynamic::{Static, TreatAsDynamic}` via `Crs::is_dynamic_with` |
| `CRS::extractGeographicCRS` / `extractVerticalCRS` | `Crs::extract_geographic_crs` / `extract_vertical_crs` (owned clones) |
| `CRS::stripVerticalComponent` | `Crs::strip_vertical_component` |
| `CRS::alterGeodeticCRS` / `alterCSLinearUnit` | `Crs::alter_geodetic_crs` / `alter_cs_linear_unit` |
| `CRS::promoteTo3D` / `demoteTo2D` | `Crs::promote_to_3d` / `demote_to_2d` |
| `BoundCRS::createFromTOWGS84` | `BoundCRS::from_towgs84` |
| `CRS::createBoundCRSToWGS84IfPossible` | `Crs::create_bound_crs_to_wgs84_if_possible` (no intermediate-CRS policy argument) |
| `ProjectedCRS::identify` | `ProjectedCRS::identify_with(None)` for the hardcoded UTM zones |
| `datum()` on an ensemble (`datumNonNull`) | `datum_ensemble().as_datum()`; the ensemble name is stripped of ` ensemble` |

## Deliberate differences

- Pointer identity in C++ (`EXPECT_EQ` on the same object, `canonicalBoundCRS` use counts) is `is_equivalent_to` or an EPSG code plus confidence. `do_not_mess_deriving_conversion` still uses `std::ptr::eq` on the deriving conversion's target CRS.
- A database demote of a derived geographic or derived projected CRS may return the WGS 84 ensemble where the input held datum EPSG:6326. The promote/demote test compares those with `Equivalence::Equivalent` on the base, and strict equality on the conversion and on axis direction plus unit. The expected axis counts and WKT are unchanged.
- `WKT2` in PROJ 9.9.0 is `WKT2_2015` (`GEODCRS`), not `WKT2_2019`.
- WKT1 bound export of an NTv2 transformation (method 9615) whose hub is WGS 84 writes `EXTENSION["PROJ4_GRIDS", ...]` on the source datum. NADCON (a latitude grid only) is not written that way.
- `createBoundCRSToWGS84IfPossible` returns the input unchanged when the CRS has several areas of use and neither bounding box contains the other. A nested pair (ETRS89 UTM) still binds. EPSG:2936 still binds here, which is why that whole test stays open.
- `geodetic_datum_equivalent` prefers a geodetic datum whose folded name is the query plus `ensemble`, and returns the name without the ` ensemble` suffix and with an empty authority, so the import does not reload EPSG:6258 and overwrite the name. That is what makes `European_Terrestrial_Reference_System_1989` the ETRS89 ensemble instead of EPSG:1132.
- A projected CRS with a 3D base exports that base CS (PROJ issue 2122). A 2D base still omits it. `BASEPROJCRS` writes its cartesian axes when the unit is not the metre or the CS has three axes, the identifiers are empty, and the name is not a known projected name.
- Bound CRS equivalence compares name, source, target, and the transformation (method code, endpoints, parameters) or the conversion.

## Commands

```
cargo test --offline --test unit_crs_core_2
cargo test --offline --lib projected_crs_on_etrs89
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo test --offline --test unit_crs_core_2`: 36 passed, 0 failed. `projected_crs_on_etrs89_carries_the_helmert_towgs84` (EPSG:25832) still binds after the multi-domain gate.
