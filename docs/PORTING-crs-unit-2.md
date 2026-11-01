# Port — PROJ `test/unit/test_crs.cpp`, lot 2

Source, PROJ 9.9.0: `test/unit/test_crs.cpp`. Also read for the cases that stay open: `src/iso19111/io.cpp` (`buildProjectedCRS`, the `krovak` and `omerc` method choice, around lines 11559 and 12030).

Lot 1 is `docs/PORTING-crs-unit.md`. This lot takes the five open `TEST()`s that build a CRS from WKT1 or EPSG and check its WKT output with the crate's existing API. The other candidates in the ledger need an API this crate does not expose, or they reach a bug listed below.

## Tests ported and passing (5)

| C++ TEST | Rust test | Note |
|---|---|---|
| `crs.projectedCRS_from_WKT1_ETRS89_with_db` | `crs_projected_crs_from_wkt1_etrs89_with_db` | The WKT1 projected CRS is read with the database, and the base datum name is checked. |
| `crs.projectedCRS_from_EPSG_as_WKT1_ESRI_s_jtsk03_krovak_east_north` | `crs_projected_crs_from_epsg_8353_as_wkt1_esri` | EPSG:8353 exported as WKT1 ESRI. |
| `crs.projectedCRS_from_EPSG_with_deprecated_ESRI_name_as_WKT1_ESRI` | `crs_projected_crs_from_epsg_5186_with_deprecated_esri_name_as_wkt1_esri` | EPSG:5186 exported as WKT1 ESRI, with the non-deprecated ESRI names. |
| `crs.usage` | `crs_usage` | A geocentric CRS with `USAGE` is read and written as WKT2_2019, with the EPSG IDs the database adds. |
| `crs.compoundCRS_isEquivalentTo` | `crs_compound_crs_is_equivalent_to` | See "Deliberate differences". |

## Attempted and left open (this lot)

Each of these was run with the crate's current code. The reason is the cause found, not a guess.

Bugs in the Rust code, not fixed here (the fix is not small, or it changes datum or method naming for every import):

- `crs.projectedCRS_from_WKT1_ESRI_ETRS89_with_db`: the ESRI datum `European_Terrestrial_Reference_System_1989` resolves to `Rete Dinamica Nazionale 2008`. In `proj.db`, the exact name is the ensemble row 6258, `European Terrestrial Reference System 1989 ensemble`, which `geodetic_datum_equivalent` (`src/iso19111/factory.rs`) does not match by name. The alias lookup then takes an arbitrary row.
- `crs.projectedCRS_from_WKT1_ESRI_as_WKT1_ESRI_s_jtsk03_krovak_east_north`: the ESRI `Krovak` with `X_Scale`, `Y_Scale` and `XY_Plane_Rotation` imports as EPSG:9819 (Krovak), not EPSG:1041 (Krovak North Orientated). The WKT2 output differs in method and parameter set.
- `crs.projectedCRS_Krovak_with_approximate_alpha_as_PROJ_string`: `+proj=krovak` with neither `+axis` nor `+czech` is refused (`src/iso19111/projstring.rs`, `method_for_projection`). PROJ falls through to a generic method choice here.
- `crs.identical_after_import_from_proj4_and_wkt1_export`: the WKT1 `Lambert_Conformal_Conic_2SP` reads back with `Latitude of natural origin` (8801) instead of `Latitude of false origin` (8821). The parameter table in `src/iso19111/parammap.rs` has 8821 for `latitude_of_origin`, so the fault is in the WKT1 reimport path. Probe: the PROJ string `+proj=lcc +lat_0=25 +lon_0=-95 +lat_1=25 +lat_2=25` exports the WKT1 correctly; reimporting that text gives the wrong parameters.

Features the crate does not have:

- `crs.WKT1_DATUM_EXTENSION_to_WKT1_and_PROJ_string`, `crs.WKT1_VERT_DATUM_EXTENSION_to_WKT1` (and the `_to_WKT2` and `_to_PROJ_string` ones): `EXTENSION["PROJ4_GRIDS", ...]` is dropped, and the reader returns a plain CRS, not a `BoundCRS`.
- `crs.compoundCRS_as_PROJ_string`: no PROJ-string export for a compound CRS (`this CRS kind has no PROJ string export`).
- `crs.compoundCRS_as_WKT1_GDAL`: the expected output has `AUTHORITY["codespace","code"]`, and the public API cannot set a compound identifier. The same limit is in lot 1.
- `crs.projected_is_equivalent_to_with_proj4_extension`: `+proj=omerc ... +no_rot` is refused (`"omerc" if present("no_rot") => None`). The projected CRS type also has only the strict `is_equivalent_to`, with no criterion.
- `crs.ETRF2000_PL_CS92`, `crs.norway_ntm`: both compare projected CRS with the EQUIVALENT criterion, which the crate does not have for projected CRS. Both also need `identify` against the EPSG factory.
- `crs.projectedCRS_3D_is_WKT1_equivalent_to_WKT2`: the strict `is_equivalent_to` returns false for the WKT1 and WKT2 round trip. The C++ criterion is `EQUIVALENT_EXCEPT_AXIS_ORDER_GEOGCRS`, which the crate lacks. The cause of the strict failure was not isolated.
- `crs.engineeringCRS_unknown_equivalence`: the strict `is_equivalent_to` returns false for the two engineering CRS (the datum names and axis names differ). The EQUIVALENT criterion is not available for engineering CRS.
- `crs.projectedCRS_identify_wrong_auth_name_case`: `identify_with` with an anonymous authority finds no candidate for a CRS carrying `AUTHORITY["Esri",54034]`.
- `crs.scope_area_bbox_remark`: `ObjectDomain` has scope, area and bounding box, but no vertical or temporal extent.
- `crs.VerticalCRS_ensemble_exception_in_create`, `crs.verticalCRS_datum_ensemble`, `crs.verticalCRS_identify_db`, `datum.vdatum_with_anchor`: no public `VerticalCRS::create` or `VerticalReferenceFrame` constructor with an ensemble or anchor.
- `crs.derivedProjectedCRS_WKT2_2015` (and the other derived tests): no public `Conversion` constructor, and `DERIVEDPROJCRS` is not parsed.
- `crs.projectedCRS_identify_no_db`: no public UTM `Conversion` constructor.
- `crs.getNonDeprecated`, `crs.default_identify_method`, `datum.cs_with_MERIDIAN`: no API for the deprecation filter, parametric CRS, or the `MERIDIAN` axis.

## Deliberate differences

- `crs_compound_crs_is_equivalent_to` omits `isEquivalentTo(createUnrelatedObject())`, as lot 1 did. The second compound CRS has an empty name, which the crate names `WGS 84 / UTM zone 31N + ODN height`, as `CompoundCRS::create` does for an unnamed compound. The expected `false` holds for that name difference, as in C++.
- `crs_usage` writes the expected WKT as a raw string, because the expected text is multi-line.

## Commands

```
cargo test --test test_crs
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

`cargo test --test test_crs` runs 110 tests, all passing. The `test_crs` ledger went from 94 to 89 open.
