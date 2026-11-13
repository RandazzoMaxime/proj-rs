# Porting sheet: coordinates (`coordinates.hpp`) and the coordinates lot

PROJ 9.9.0 sources: `src/iso19111/coordinates.cpp`, `test/unit/test_coordinates.cpp`.
The audit of every assertion is in `docs/AUDIT-coordinates.md`.

## Map

| PROJ | Rust |
|---|---|
| `CoordinateMetadata::create(crs, epoch, db)` | `iso19111::CoordinateMetadata::create(&crs, epoch, &db)` |
| `CoordinateMetadata::create(crs, epoch)` (null context) | `CoordinateMetadata::create_without_database(&crs, epoch)` |
| `CoordinateMetadata::create(crs)` | `CoordinateMetadata::create_without_epoch(&crs)` |
| `coordinateEpochAsDecimalYear()` | `coordinate_epoch() -> Option<f64>` (see the audit, gap 2) |
| `CoordinateMetadata::promoteTo3D(newName, db)` | `promote_to_3d(Some(&db)) -> CoordinateMetadata` |
| `exportToWKT` (WKT2:2019 `COORDINATEMETADATA[..., EPOCH[..]]`) | `CoordinateMetadata::export_to_wkt(&mut WktFormatter)` (`src/iso19111/wkt.rs`) |
| `WKTParser` on `COORDINATEMETADATA` | `WktParser::create_from_wkt` gives `WktObject::CoordinateMetadata` (`src/iso19111/wkt_import.rs`, `build_coordinate_metadata`) |
| `exportToJSON` / `createFromUserInput` | `UserObject::CoordinateMetadata` export (already present) and import (`json.rs`, `read_coordinate_metadata`) |
| `DynamicGeodeticReferenceFrame::create(props, ellipsoid, anchor, pm, epoch, model)` | `DynamicGeodeticReferenceFrame::create(name, ellipsoid, anchor, pm, epoch, model)` (`src/iso19111/model.rs`) |
| `GeographicCRS::create(props, datum, cs)` | `GeographicCRS::create(name, Some(frame.into()), None, cs)` (already public; no new constructor) |

## Deliberate differences

- **Two constructors instead of a nullable context.** C++ `create(crs, epoch)` and
  `create(crs, epoch, dbContext)` are `create_without_database` and `create`. The
  database argument stays `&DatabaseContext`, so no caller changes.
- **`DynamicGeodeticReferenceFrame` is a wrapper.** The crate represents a dynamic
  datum as a `GeodeticReferenceFrame` with a frame epoch. The wrapper requires the
  epoch, so a dynamic frame is always built with one. It derefs to
  `GeodeticReferenceFrame` and converts into it with `From`. The epoch is a decimal
  year, where C++ takes a `Measure`.
- **`promote_to_3d` has no name argument.** C++ `promoteTo3D(newName, db)` with
  an empty name keeps the CRS name, which is what the test passes. A non-empty
  `newName` is not supported.
- **`promote_to_3d` does not re-check the epoch.** As in C++, the promoted object is
  built directly, without `create`'s point motion check.

## Fixes made to match PROJ (in src, not deviations)

- **Lat/lon axis abbreviations.** `lat`/`lon` in the `EllipsoidalCS` constructors.
  The strict CRS comparison with a WKT round trip depends on it.
- **Datum ensemble equivalence** is the name comparison (`ObjectUsage` →
  `IdentifiedObject`). The previous comparison also compared accuracy and members,
  which PROJ does not, so a WKT round trip of an ensemble CRS (members written
  without frame epochs) was not equivalent to its source.
- **JSON nested CRS.** A `CoordinateMetadata` reads its `crs` with the strict
  reader (`json_read::read_crs_value`) and falls back to the lenient one. The lenient
  reader drops the frame epoch and the deformation model, so a dynamic CRS read
  through it is static.
- **`#[expect(clippy::large_enum_variant)]` on `WktObject`** removed: the lint no
  longer fires, so the expectation was unfulfilled under `-D warnings`.

## Not ported

See the audit's gaps: the C facade `proj_create` on WKT (gap 1), the epoch rounding
(gap 2), and the environment-variable case of the user-writable directory test (gap 3).

## Tests

- `tests/unit_coordinates_static.rs`: `static_crs`.
- `tests/unit_coordinates_dynamic.rs`: `dynamic_crs`.
- `tests/unit_coordinates_point_motion.rs`: point motion and promotion to 3D.
- `tests/unit_context_files.rs`: `dictionary_absent_before_user_writable_directory_is_set`.
- `tests/test_c_api.rs`: `object_kind` names the new `WktObject` variant.

Expected values are PROJ 9.9.0's, taken from `test_coordinates.cpp`.

## Commands

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --all-features --test unit_coordinates_static --test unit_coordinates_dynamic \
  --test unit_coordinates_point_motion --test unit_context_files
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```
