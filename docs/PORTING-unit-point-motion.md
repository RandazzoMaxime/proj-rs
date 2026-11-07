# Unit tests: point motion between CoordinateMetadata (lot `point-motion`)

Lot: the operations between two `CoordinateMetadata` (a CRS at a coordinate epoch) and the point
motion operations of PROJ 9.9.0. Base: `unit-core-passing-2`. The tests are the `TEST()`s of
`test/unit/test_operationfactory.cpp` named in each commit, with the PROJ 9.9.0 expected strings.

## Function map

| PROJ (`src/iso19111/operation/coordinateoperationfactory.cpp`) | Rust (`src/iso19111/`) |
|---|---|
| `createOperationsGeodToGeod`, `forceBallpark`, geographic/geocentric pair, same datum (`:5282-5289`) | `factory.rs` `forced_geodetic_operations`, `geodetic_ops.rs` `forced_geographic_geocentric` |
| same, different datums: geographic intermediate of the geocentric CRS, full `createOperations` from it (`:5314-5331`) | `factory.rs` `forced_geodetic_operations`, `geodetic_ops.rs` `geocentric_intermediate` |
| `Transformation::setHasBallparkTransformation(true)` | `model.rs` `Transformation::ballpark` (read by `has_ballpark_transformation`) |

## Commits

### 1. Forced ballpark of a geographic/geocentric pair in the point motion branch

`createOperationsFromDatabase`'s point motion branch ends with `createOperationsGeodToGeod(...,
forceBallpark=true)` (`:4330-4336`). For a geocentric and a geographic CRS of the same datum that
is `createGeodToGeodPROJBased` flagged as a ballpark (`:5282-5289`): world domain, name
`Transformation from <src> to <dst>`, PROJ string of the conversion. The flag is the new
`Transformation::ballpark`, because the ballpark status of a `Transformation` was read from its name
only and PROJ's `hasBallparkTransformation` is a flag. The different-datum case calls the full
factory from the geographic intermediate (not the forced one), as PROJ does. The reverse pair
(geographic to geocentric) is the inverse of the forced geocentric-to-geographic one.

Test: `operation_point_motion_operation_geocentric_to_geog3d` (`tests/unit_op_geog_point_motion_ctx.rs`).
Ledger: the line `operation.createOperation_point_motion_operation_geocentric_to_geog3D` removed.
