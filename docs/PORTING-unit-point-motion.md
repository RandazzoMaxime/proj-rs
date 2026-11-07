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
| `createOperations(source, sourceEpoch, target, targetEpoch)`, private: the point motion branch of `createOperationsFromDatabase` (`:4278-4345`), then the candidates | `factory.rs` `candidates_at_epochs`, `create_candidates_at`, `Epochs` |
| `createOperationsWithDatumPivot` with epochs; `createTransformations` epoch change (`:3167-3320`, `:3214-3235`) | `factory.rs` `datum_pivot_operations`, `pivot_epoch_changes`, `pivot_chains` |
| `createOperationsCompoundToCompound`, the point motion block (`:7228-7290`) | `compound.rs` `compound_point_motion`, called from `create_candidates_at` |
| `createOperationsCompoundToCompound`, `opsGeogSrcToGeogDst` with the epochs (`:7255-7257`) | `compound.rs` `compound_to_compound` (`candidates_at_epochs`) |

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

### 3. The point motion block of `createOperationsCompoundToCompound`

`createOperationsCompoundToCompound` (coordinateoperationfactory.cpp:7228-7290) starts with a point
motion block for two compound CRS whose horizontal parts are the same geographic CRS, at two
different epochs, when that datum has a point motion. The candidates are the operations from the
source to the geographic 3D CRS (`createOperations(sourceCRS, sourceEpoch, geog3D, sourceEpoch)`),
the operations between the two epochs of the geographic 3D CRS (the point motion, with the forced
ballpark), and the operations from it to the target (`createOperations(geog3D, targetEpoch,
targetCRS, targetEpoch)`), each triple chained with no ballpark operation at the three places. When
it gives a result PROJ returns from `createOperationsCompoundToCompound`.

Rust: `compound::compound_point_motion`, called by `create_candidates_at` for two compound CRS after
the registry block, and `return`ed when it gives something. The registry operations already in
`found` stay, as PROJ keeps them in `res`. `compound_to_compound` takes the epochs too: its
geographic pair (`opsGeogSrcToGeogDst`) is read with `candidates_at_epochs`.

Tests: `operation_compound_to_compound_with_point_motion_operation` and
`operation_compound_to_compound_with_point_motion_operation_special_case_cgvd2013a`. The
CGVD2013a test takes the same block: its vertical parts are not reached, the point motion of the
geographic 3D CRS is.
Ledger: the two lines removed from `docs/UNPORTED-UNIT.md`.

## Result table

| `test/unit/test_operationfactory.cpp` | Test | Commit |
|---|---|---|
| `createOperation_point_motion_operation_geocentric_to_geog3D` | `operation_point_motion_operation_geocentric_to_geog3d` | 1 |
| `createOperation_point_motion_operation_NAD83_CSRS_v7_TO_ITRF2014` | `operation_point_motion_operation_nad83_csrs_v7_to_itrf2014` | 2 |
| `createOperation_point_motion_operation_ITRF2014_to_NAD83_CSRS_v7` | `operation_point_motion_operation_itrf2014_to_nad83_csrs_v7` | 2 |
| `createOperation_compound_to_compound_with_point_motion_operation` | `operation_compound_to_compound_with_point_motion_operation` | 3 |
| `createOperation_compound_to_compound_with_point_motion_operation_special_case_CGVD2013a` | `operation_compound_to_compound_with_point_motion_operation_special_case_cgvd2013a` | 3 |

All five are in `tests/unit_op_geog_point_motion_ctx.rs`. The CGVD2013a test is not a separate
branch in PROJ 9.9.0's `createOperationsCompoundToCompound`: the same point motion block gives its
expected pipeline, with the vertical geoid steps (`CGG2013an83`) on the two ends.

## Deviations (stated)

- **Epochs threaded only where the lot needs them.** `Epochs` reaches `candidates_at_epochs`,
  `create_candidates_at`, the datum pivot, `compound_point_motion` and the geographic pair of
  `compound_to_compound`. PROJ also passes the epochs through `createOperationsDerivedTo`,
  `createOperationsToGeod`, `createOperationsCompoundToGeog`, the bound and the geoid paths, and the
  ensemble step. Those sub-calls run here with no epochs. They differ only when such a sub-call is a
  geodetic pair at two different epochs, which none of the five tests reaches.
- **The datum pivot iterations 1 and 2** keep this port's own structure (`linked_pair`, the
  `create_candidates` between the candidates). The epoch change is inserted there too, but their
  order is not PROJ's `createOperations` order.
- **Ballpark flag.** `Transformation::ballpark` is set only for the forced PROJ-based
  geographic/geocentric conversion. The other `setHasBallparkTransformation` calls of PROJ
  (`createOperationsGeogToGeog`, `!sameDatum`) still use the name, as before.
- **Compound point motion operands.** `ops_first` (source epoch on both ends) and `ops_last` (target
  epoch on both ends) are read with equal epochs, as PROJ reads them, so no point motion is taken
  there; the point motion is the one between the two epochs of the geographic 3D CRS only.

## Open items (not in this lot)

- The epoch change of `createOperationsGeogToGeog` (coordinateoperationfactory.cpp:3226-3257) is not
  ported, as in the epoch sheet.
- Forced ballpark between two geographic CRS with different prime meridians (the longitude rotation
  of `createOperationsGeogToGeog(forceBallpark)`) is not ported: `forced_ballpark_operations` gives
  nothing there.
- The datum pivot through a third CRS (`createOperationsWithDatumPivot`, the `iter` loops of PROJ)
  keeps the structure described above.

## Verification

- `cargo test --test unit_op_geog_point_motion_ctx`: 5 passed.
- The unit regression set (`tests/unit_op_*`, `tests/unit_capi_*`, `tests/unit_crs_*`): 262 passed,
  0 failed, 6 ignored, before and after each commit (the baseline had 257 passed and the 5 failing
  tests of this lot).
- `(ulimit -s 1024; cargo test --all-features --test cli_yaml)`: passed after each commit; no case
  reopened (`docs/UNPORTED-CLI.md` unchanged).
- `cargo test --all-features --no-fail-fast` with the reference `projinfo`/`cs2cs` on `PATH` and
  `PROJ_DATA` on the grid-less `oracle-db`, `PROJ_CS2CS_REQUIRED=1`: 1656 passed, 0 failed, 32 ignored
  (from every `test result:` line). The run after commit 2 alone gave 1654 passed, 0 failed.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`, then the same without
  `UPDATE_DOCS`: pass.

## Commands

```bash
cargo test --test unit_op_geog_point_motion_ctx
cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
```
