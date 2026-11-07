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

### 2. The datum pivot with the epochs

`createOperationsWithDatumPivot` (coordinateoperationfactory.cpp:3167-3541) passes the epochs to its
sub-calls. The epoch change is `createTransformations` (`:3214-3235`): with two epochs that differ,
the operations of the source candidate at the two epochs when its datum has a point motion
(`getPointMotionOperationsFor`), else the same for the target candidate. Those operations are
inserted between the first leg and the second operation, and the loop runs once per epoch change
(`iEpochChange`), as `pivot_chains` does.

Rust (`src/iso19111/factory.rs`): `Epochs` (the `sourceEpoch`/`targetEpoch` of a private
`createOperations`), `candidates_at_epochs` (the point motion branch of `createOperationsFromDatabase`
then the candidates), `create_candidates_at` (the candidates with the epochs; `create_candidates`
is the call without them), `datum_pivot_operations` with `pivot_epoch_changes` and `pivot_chains`.

The point motion of the pivot is the one of the source datum: for NAD83(CSRS)v7 to ITRF2014 the
chain is the `NAD83(CSRS)v7 (geocentric)` candidate, the velocity operation from epoch 2002 to epoch
2005, the Helmert pivot, and the ITRF2014 conversion. The second result of the epoch loop is the
ballpark geocentric translation of the same candidate (`Ballpark geocentric translation`), which
PROJ also keeps.

Tests: `operation_point_motion_operation_nad83_csrs_v7_to_itrf2014` and
`operation_point_motion_operation_itrf2014_to_nad83_csrs_v7` (`tests/unit_op_geog_point_motion_ctx.rs`).
Ledger: the two lines removed from `docs/UNPORTED-UNIT.md`.
