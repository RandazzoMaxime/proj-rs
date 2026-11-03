# Port — niche CRS I/O and operation factory (lot niche-io-opfactory)

Source: PROJ 9.9.0 `test/unit/test_io.cpp` (58 `TEST()` of class `niche`) and
`test/unit/test_operationfactory.cpp` (37). Rust side:
`tests/unit_niche_io_opfactory.rs`. Expected text is copied from those files.

This lot ports **63** tests and leaves **32** open. Each open name stays in
`docs/UNPORTED-UNIT.md`.

## 1. Ported (63)

| PROJ 9.9.0 | Rust |
|---|---|
| `wkt_parse.COMPOUNDCRS_spatio_parametric_2015` / `_2019`, `COMPOUNDCRS_spatio_temporal_2015` / `_2019` | `wkt_parse_compoundcrs_spatio_*` |
| `wkt_parse.TemporalDatum`, `TemporalDatum_no_calendar`, the six temporal CRS `TEST()`s | `wkt_parse_temporal*` / `wkt_parse_datetime*` |
| `wkt_parse.ENGINEERINGDATUM`, `ENGINEERINGCRS`, four `LOCAL_CS_*` | `wkt_parse_engineering*` / `wkt_parse_local_cs_*` |
| `wkt_parse.PARAMETRICDATUM`, `PARAMETRICCRS`, `PARAMETRICCRS_PARAMETRICDATUM` | `wkt_parse_parametric*` |
| derived geographic, geodetic, vertical, engineering, parametric, temporal, projected (including ordinal, non-metre base, and the EPSG horizontal-code interpolation parameter), and the eight `invalid_*` | `wkt_parse_derived*` / `wkt_parse_invalid_*` |
| `io.projparse_topocentric` | `io_projparse_topocentric` |
| `io.createFromUserInput_OGC_AUTO` | `io_createfromuserinput_ogc_auto` |
| twelve `json_import.*` of this lot (temporal extent, parametric, engineering, temporal, and the derived CRS kinds, including the EPSG horizontal code) | `json_import_*` |
| five `operation.geogCRS_to_geogCRS_*celestial_body*` | `operation_geogcrs_to_geogcrs_*` |
| `operation.geogCRS_to_derived_geogCRS_3D` | `operation_geogcrs_to_derived_geogcrs_3d` |

`json_import.engineering_crs_affine_CS` is not in this lot. Its ledger line stays.

## 2. API mapping

| PROJ | Rust |
|---|---|
| WKT import/export of temporal, parametric, engineering, and derived CRS | `src/iso19111/wkt_import/niche.rs`, `src/iso19111/wkt/niche.rs`, `src/iso19111/niche.rs` |
| `createFromUserInput` for a PROJJSON CRS | `create_from_user_input` → `Crs` |
| `createFromUserInput` for a PROJJSON datum | `create_object_from_user_input` → `UserObject` |
| `WMS AUTO` / `urn:ogc:def:crs:OGC::AUTO…` | `create_from_user_input` (`crs_from_ogc_auto`) |
| `+proj=topocentric` as a CRS | `create_crs_from_proj_string` |
| `createOperations` when a CRS is derived | `derived::derived_candidates` |
| `Geographic3D offsets` (EPSG:9660) as a conversion step | `geographic_offsets` |
| datum of an ensemble CRS (EPSG:6326 on WGS 84) | `datum_identity` reads the ensemble code |
| 2D base of a 3D derived geographic CRS | `promote_base_to_3d` at WKT import; datum-pivot Helmert endpoints promoted when both CRS are 3D |

## 3. Deliberate differences

- `create_from_user_input` returns a CRS. A PROJJSON datum is `FactoryError::Invalid("PROJJSON is not a CRS")`. The datum tests call `create_object_from_user_input`.
- A spherical CRS has no PROJJSON export (`FactoryError::Format`). The writer does not invent a spherical object.

## 4. Preserved quirks

- OGC AUTO method codes, zone edges (`-180` accepted, `180` refused), unit codes 9001/9002/9003, and the exact `+proj=` strings from the C++ test, including `+k` on transverse Mercator.
- An all-digit parameter or method code is a JSON number. Greenwich is omitted. The interpolation CRS parameter is stripped on import and written back on export.
- `geogCRS_to_derived_geogCRS_3D` keeps the name `Inverse of CH1903+ to WGS 84 (1) + Offset on ellipsoidal height` and the 3-parameter Helmert `+x=-674.374 +y=-15.056 +z=-405.346`.

## 5. Open (32)

### WKT `PROJ4` extension (1)

- `wkt_parse.DerivedGeographicCRS_GDAL_PROJ4_EXSTENSION_hack`: strict-false WKT1 `PROJECTION["Rotated_pole"]` plus `EXTENSION["PROJ4","+proj=ob_tran …"]` must become a derived geographic CRS and a `unitconvert` + `ob_tran` pipeline. The importer only reads `EXTENSION` `PROJ4_GRIDS`.

### Bound and compound CRS with a derived component (6)

`create_operations` does not chain a bound-CRS transformation, and the compound paths do not treat a derived vertical or derived projected component as a deriving conversion.

- `operation.boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs`
- `operation.compoundCRS_with_derivedVerticalCRS_ellipsoidal_height_to_geog`
- `operation.compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical`
- `operation.compoundCRS_to_compoundCRS_with_derivedVerticalCRS_ellipsoidal_height`
- `operation.compoundCRS_with_derived_vertical_CRS`
- `operation.geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected`

### `ob_tran` / pole rotation as a CRS (5)

`+proj=ob_tran` is not a CRS in `projstring.rs`, and WKT has no `Rotated_pole` / `PROJ4` extension. These tests need that CRS, then a pipeline (`+proj=ob_tran` or `+proj=affine`).

- `derivedGeographicCRS_with_to_wgs84_to_geographicCRS.src_from_proj`
- `derivedGeographicCRS_with_to_wgs84_to_geographicCRS.src_from_wkt2`
- `operation.createOperation_ossfuzz_47873`
- `operation.createOperation_ossfuzz_47873_simplified_if_i_might_say`
- `operation.createOperation_derived_projected_crs` (deriving method is `PROJ-based operation method: +proj=pipeline … +proj=affine`)

### Spherical planetocentric / planetographic (10)

`+proj=longlat +geoc` and `+proj=latlong +geoc` are not read (`geoc` does not appear in `src/iso19111`). A spherical CRS also has no PROJ string (`a spherical CRS has no PROJ string in this crate`). The tests expect `+proj=geoc` or an ographic axis swap between those CRS and a geographic, geocentric, or projected CRS.

- `operation.createOperation_spherical_ocentric_to_geographic`
- `operation.createOperation_geographic_to_spherical_ocentric`
- `operation.createOperation_spherical_ocentric_to_geocentric`
- `operation.createOperation_bound_of_spherical_ocentric_to_same_type`
- `operation.createOperation_spherical_ocentric_to_projected`
- `operation.createOperation_spherical_ocentric_to_projected_of_spherical_ocentric`
- `operation.createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west`
- `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west`
- `operation.createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric`
- `operation.createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west`

### Point motion and epochs (9)

The factory stores `point_motion` on a transformation loaded from `proj.db`, and `export_steps` then returns `Unimplemented`. Datum pivot does not build epoch-change operations, and `create_operations` takes a CRS, not PROJ's coordinate metadata with an epoch.

- `operation.createOperation_point_motion_operation_geog2D`
- `operation.createOperation_point_motion_operation_geog3D`
- `operation.createOperation_point_motion_operation_geocentric`
- `operation.createOperation_point_motion_operation_geocentric_to_geog3D`
- `operation.createOperation_point_motion_operation_NAD83_CSRS_v7_TO_ITRF2014`
- `operation.createOperation_point_motion_operation_ITRF2014_to_NAD83_CSRS_v7`
- `operation.createOperation_point_motion_operation_nkg`
- `operation.createOperation_compound_to_compound_with_point_motion_operation`
- `operation.createOperation_compound_to_compound_with_point_motion_operation_special_case_CGVD2013a`

### Malformed PROJ string (1)

- `operation.createOperation_ossfuzz_18587`: PROJ accepts `type=crs proj=pipeline step proj=merc … proj="\nproj=pipeline step\n"` and only requires that `createOperation` not recurse. `create_crs_from_proj_string` returns `Invalid("unsupported +proj= for a CRS")`, so that CRS is never built.

## 6. Verification

```text
cargo fmt -- src/iso19111/{conversion_export,derived,factory,json,mod,model,projstring,transform_export,wkt_import}.rs src/iso19111/wkt_import/niche.rs tests/unit_niche_io_opfactory.rs
cargo clippy --all-targets --all-features --offline -- -D warnings
cargo test --test unit_niche_io_opfactory --offline
cargo test --test docs_in_sync --offline
```

`unit_niche_io_opfactory`: 63 passed, 0 failed. Clippy with `-D warnings` is clean. `docs_in_sync` fails on generated counts in `README.md` and `docs/UNIT-TRIAGE.md` (and the README check also covers `ROADMAP.md` once that file is reached). Those files are not edited. `UPDATE_DOCS` was not set.
