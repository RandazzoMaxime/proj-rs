# Audit: coordinates lot

PROJ 9.9.0 sources: `test/unit/test_coordinates.cpp` (3 `TEST()`),
`test/unit/proj_context_test.cpp` (the user-writable directory negative case).
Fiche du lot : `docs/PORTING-coordinates.md`.

## test_coordinates.cpp (3 TEST())

| C++ TEST() | Rust test | Status |
|---|---|---|
| `coordinateMetadata.static_crs` | `tests/unit_coordinates_static.rs::static_crs` | Ported, with the gaps below. |
| `coordinateMetadata.dynamic_crs` | `tests/unit_coordinates_dynamic.rs::dynamic_crs` | Ported, with the gap below. |
| `coordinateMetadata.crs_with_point_motion_operation_and_promote_to_3D` | `tests/unit_coordinates_point_motion.rs::crs_with_point_motion_operation_and_promote_to_3d` | Ported. |

### Assertion by assertion

`static_crs`

| C++ assertion | Rust |
|---|---|
| `create(EPSG_4326)` is equivalent to `EPSG_4326`, no epoch | `create_without_epoch`, `is_equivalent_to_with(Strict)`, `coordinate_epoch() == None` |
| `create(EPSG_4326, 2025.0)` does not throw | `create_without_database(crs, 2025.0)` is `Ok` (WGS 84 is dynamic under the TreatAsDynamic reading) |
| `create(verticalCRS, 2025.0)` throws | `create_without_database(vertical, 2025.0)` is `Err` |
| WKT2:2019 export, parse, CRS equivalent, no epoch | `export_to_wkt` with `Wkt2_2019`, `WktParser::create_from_wkt` gives `WktObject::CoordinateMetadata` |
| PROJJSON export, `createFromUserInput`, CRS equivalent, no epoch | `UserObject::CoordinateMetadata(..).export_to_json`, `create_object_from_user_input` |
| `proj_create(ctxt, wkt)` gives `PJ_TYPE_COORDINATE_METADATA` | Not ported: see gap 1 |
| `proj_coordinate_metadata_get_epoch` is NaN | Not ported: see gap 1 |
| `proj_get_source_crs(ctxt, obj)` is not null | Not ported: see gap 1 |

`dynamic_crs`

| C++ assertion | Rust |
|---|---|
| `DynamicGeodeticReferenceFrame::create(...)`, `GeographicCRS::create`, `create(crs, 2023.5)` equivalent, epoch 2023.5 | `DynamicGeodeticReferenceFrame::create` (`Some` anchor and model), `GeographicCRS::create(name, Some(frame), None, cs)`, `create_without_database(crs, 2023.5)` |
| `coordinateEpochAsDecimalYear()` is 2023.5 within 1e-10 | `coordinate_epoch()` is 2023.5 within 1e-10 |
| `create(crs)` throws | `create_without_epoch(crs)` is `Err` |
| WKT round trip, CRS equivalent (default criterion, Strict) and epoch | `WktParser`, `is_equivalent_to_with(Strict)` |
| `proj_create` on the WKT: type and epoch | Not ported: see gap 1 |
| PROJJSON round trip with `Criterion::EQUIVALENT` and epoch | `is_equivalent_to_with(Equivalent)`, epoch |

`crs_with_point_motion_operation_and_promote_to_3D`

| C++ assertion | Rust |
|---|---|
| EPSG:8255 `create(crs, 2023.5)` throws | `create_without_database` is `Err` |
| EPSG:8255 `create(crs, 2023.5, db)` does not throw | `create(crs, 2023.5, &db)` is `Ok` |
| promoted object CRS equivalent to `crs->promoteTo3D` | `promote_to_3d(Some(&db))`, Strict |
| epoch kept, 2023.5 within 1e-10 | `coordinate_epoch()` |
| EPSG:4267 `create(crs, 2023.5, db)` throws | `Err` |
| EPSG:4267 `create(crs)->promoteTo3D` equivalent, no epoch | `create_without_epoch(..).promote_to_3d(..)`, Strict, `None` |

## proj_context_test.cpp: read_grid_from_user_writable_directory

| C++ assertion | Rust test | Status |
|---|---|---|
| With `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`, `+init=temp_proj_dic3:...` in the user-writable directory is not found (`proj_create` gives null) | `tests/unit_context_files.rs::dictionary_absent_before_user_writable_directory_is_set` | Ported in its negative form: a dictionary written outside the context's search path is not found until the directory is set. |
| The same, with the environment variable cleared, the file is found | Not ported as a separate test. | See gap 3. |

## Gaps (not closed)

1. **`proj_create` on WKT (C facade).** The C test reads the COORDINATEMETADATA
   text through `proj_create`, then checks `PJ_TYPE_COORDINATE_METADATA`, the NaN
   epoch and `proj_get_source_crs`. The Rust facade `Proj::new` (`src/proj.rs`)
   has no object-type accessor. Its text reader `crate::crs_text::to_proj`
   converts geographic and projected CRS text to a PROJ string only, so
   `Proj::new("COORDINATEMETADATA[...]")` is not accepted. The object-level
   behaviour (WKT and PROJJSON, epoch, source CRS) is covered by the iso19111
   tests above. Cause: the facade has no object-kind concept, and adding one is
   a facade API decision, not a port of a test.

2. **Epoch rounding.** PROJ's `CoordinateMetadata::coordinateEpochAsDecimalYear`
   calls `getRoundedEpochInDecimalYear` (`src/iso19111/internal.cpp:402`), which
   snaps a value within 1e-3 of a thousandth to that thousandth. `coordinate_epoch()`
   returns the stored value unchanged. The test values (2023.5, 2025.0, 2018.5) are
   exact, so the difference is not exercised. Cause: no rounding helper is ported.

3. **Environment variable case.** The C++ test clears
   `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY` with `putenv` to show the file is then
   found. The Rust variable is read in `src/network.rs` and `src/gridpath.rs`, and
   the test process is shared by all tests of its binary, so changing it inside a
   test would race with the others. Not ported. The writable-directory behaviour
   without the variable is covered by the test above and by `unit_context_files.rs`.

## Source changes made by this lot

Found by the port, fixed to match PROJ (not deviations):

- `EllipsoidalCS::create_latitude_longitude` and the other lat/lon ellipsoidal
  constructors used the abbreviations `Lat`/`Lon`. PROJ's `createLAT_NORTH` and
  `createLONG_EAST` use `lat`/`lon` (`src/iso19111/coordinatesystem.cpp:890-900`).
- `ensemble_equivalent` compared accuracy and members as well as the name. PROJ's
  `DatumEnsemble` has no `_isEquivalentTo`; it inherits `ObjectUsage`, which is
  the name comparison of `IdentifiedObject` (`src/iso19111/common.cpp:1297`).
