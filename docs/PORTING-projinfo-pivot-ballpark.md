# projinfo: ballpark operations in the candidate list

Ballpark lot of the `projinfo` candidate port (`docs/PORTING-projinfo-candidates.md`).
It ports the ballpark part of `createOperationsGeodToGeod` in
`src/iso19111/operation/coordinateoperationfactory.cpp` (PROJ 9.9.0): the
geographic offset between geographic CRS and the geocentric translation between
geocentric CRS, both for CRS of different datums, the `has ballpark
transformation` flag they carry, and the filters that act on them. The datum
pivot (`createOperationsWithDatumPivot`) and the compound path
(`createOperationsCompoundToGeog`) are not in this lot.

## API mapping

| PROJ (C++) | Rust |
|---|---|
| `createOperationsGeodToGeod`, geographic branch, different datums (`createBallparkGeographicOffset`, `forceBallpark` false) | `ballpark_operations` (`src/iso19111/factory.rs`): `Geographic2D offsets` 9619, or `Geographic3D offsets` 9660 when a CRS has three axes |
| `createBallparkGeocentricTranslation` | `ballpark_operations`: `Geocentric translations (geocentric domain)` 1031 |
| `createOperations`: geodetic ops appended when `createOperationsFromDatabase` gives no perfect result | `CoordinateOperationFactory::create_operations`, guarded by `has_perfect_accuracy_result` |
| `hasPerfectAccuracyResult` | `has_perfect_accuracy_result` (strict containment, accuracy 0) |
| `isSameGeodeticDatum` | `same_geodetic_datum` (`src/iso19111/equiv.rs`) |
| `CoordinateOperation::hasBallparkTransformation` | `CoordinateOperation::has_ballpark_transformation` (`src/iso19111/model.rs`): the name contains `ballpark` (case-insensitive), or a concatenated step has one |
| `addModifiedIdentifier` (`INVERSE(auth)` code space) | `inverse_code_space`, used by `Transformation::inverse` |
| `SortFunction`, `isApprox_` = ballpark flag | `select::sort_candidates`, called at the end of `create_operations`; `approx` now reads the flag |
| `FilterResults::filterOut` ballpark check, `setAllowBallparkTransformations` | `BallparkUse`, `CoordinateOperationContext::set_ballpark_use`, `filter_and_sort` |
| `removeSyntheticNullTransforms`, and `hasOpThatContainsAreaOfInterestAndNoGrid` set in `filterOut` | `Filtered::contains_area_without_grid`, applied after the sort in `create_operations` |
| `--hide-ballpark` (`projinfo_lib.cpp`) | `--hide-ballpark` (`src/bin/projinfo.rs`) |
| `outputOperationSummary`: `, has ballpark transformation`, the area text as is | `operation_heading` (`src/bin/projinfo.rs`) |
| `exportToPROJString` of a null geographic offset, `+proj=noop` | `null_geographic_offset` (`src/iso19111/transform_export.rs`), zero offsets only |
| `exportToWKT` of a transformation, `pushDisableUsage` around the CRS (`exportSourceCRSAndTargetCRSToWKT`) | `CoordinateOperation::export_to_wkt` (`src/iso19111/wkt.rs`); `WktFormatter::usage_disabled` hides `USAGE` and `REMARK` of the source and target CRS |
| `createFromUserInput` on a name with several matches: a 2D then a 3D geographic CRS, then a unique equivalent name | `crs_named_one` (`src/iso19111/json.rs`): geographic CRS only count for the dimension, then the `equivalent_name` passes |

New public items: `CoordinateOperation::has_ballpark_transformation`,
`CoordinateOperation::export_to_wkt`, `BallparkUse`, and
`CoordinateOperationContext::set_ballpark_use`.

## Test mapping

Yaml cases in `tests/cli_yaml/test_projinfo.yaml`, numbered as the runner numbers them.

| Cases | What they check | Status |
|---|---|---|
| 11, 12 | geographic offset, NAD27 to NAD83, default spatial test: the ballpark is the only candidate | pass |
| 34 | geocentric translation, ETRS89 to WGS 84 geocentric, sorted before `INVERSE(EPSG):9225`, which has no PROJ pipeline | pass |
| 107 | geographic offset, NAD83(CSRS) to NAD83(CSRS)v2: the target name resolves to the 2D geographic CRS, and the usage is the CRS extent | pass |
| 15 | ED50 to ETRS89 with `--bbox`: a non-ballpark operation contains the area and needs no grid, so the last ballpark is dropped | pass (regression found while porting, fixed) |

cs2cs cases 95, 96, 114, 115 of `test_cs2cs_various.yaml` also pass now. They
go through the same name resolution and `create_operations` (the cs2cs path
calls it), and were not investigated one by one.

## Deliberate differences

- **Same datum** (`Null geographic offset`, `Null geocentric translation`) is not
  synthesized. A same-datum pair with no registry operation still gives no
  candidate.
- **Prime meridian shift**: PROJ chains a longitude rotation and the ballpark
  offset. Not ported, so no ballpark is added for those pairs.
- **Geographic to geocentric pair** (different datums): PROJ goes through an
  intermediate geographic CRS. Not ported.
- **Vertical unit change** between two 3D geographic CRS: PROJ returns a
  change-of-unit conversion instead of a ballpark. Not ported; a ballpark is
  added there.
- **Non-zero geographic offsets**: `null_geographic_offset` exports zero offsets
  only. A registry `Geographic2D offsets` with other values still has no PROJ
  string, as before.
- **Usage extent of the geographic offset**: PROJ uses the source CRS extent when
  both CRS have the same extent, and `World` otherwise. Here the test is equality
  of the first usage of each CRS (`ObjectDomain` equality), not `_isEquivalentTo`.
- **Concatenated WKT** is still refused by `export_to_wkt`, as before.
- **Sort area**: `select::sort_candidates` takes the extent intersected with the
  CRS extents. PROJ's `SortFunction` uses the area of interest when one is given.
- **Fallback of `filterOut`**: when nothing is kept, the list is the accuracy-filtered
  operations, as before. PROJ's `hasOnlyBallpark` / `hasNonBallparkWithoutExtent`
  condition is not ported.
- **Name resolution**: a name with several geographic CRS of one dimension keeps
  the first one, as PROJ's `createObjectsFromNameEx` does. A name with only
  geocentric or other non-geographic matches, and no unique equivalent name, is an
  error, as PROJ's is.
- **`--hide-ballpark`** is applied in the candidate list only. `cs2cs --no-ballpark`
  is not changed (`src/bin/cs2cs.rs` is out of this lot).

## Verification

- `cargo test --test cli_yaml` with the PROJ 9.9.0 binaries on `PATH`, on a fresh
  `origin/main` and on this branch: 50 open `projinfo` rows before, 46 after.
  Across all yaml suites, 90 open rows before, 82 after; none newly open. Closed:
  `test_projinfo.yaml` 11, 12, 34, 107 and `test_cs2cs_various.yaml` 95, 96, 114, 115.
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt`: clean.
- `cargo test --all-features --no-fail-fast`: every test binary reports 0 failed.
