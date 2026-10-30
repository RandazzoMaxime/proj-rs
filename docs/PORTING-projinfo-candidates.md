# projinfo: candidate operations between two CRS

Port of the candidate-listing path of `projinfo` from PROJ 9.9.0 that
`-s X -t Y` takes: the area of interest (`--bbox`, `--area`), the spatial
test (`--spatial-test`, and the automatic note), `--crs-extent-use`,
`--accuracy`, and `--summary`. The C++ it follows is `outputOperations` in
`src/apps/projinfo_lib.cpp`, `makeBboxFilter`, and in
`src/iso19111/operation/coordinateoperationfactory.cpp`: `createOperations`,
`filterAndSort` and `FilterResults` (`filterOut`, `sort`, `andSort`). The
registry query is `AuthorityFactory::createFromCoordinateReferenceSystemCodes`
in `factory.cpp`.

Not covered: the grids and deformation models (another lot), the ballpark
operations, the datum pivot, and the compound and vertical paths. See below.

## API mapping

| PROJ (C++) | Rust |
|---|---|
| `outputOperations` (candidate count, note, summary) | `candidates`, `candidate_operations` (`src/bin/projinfo.rs`) |
| `makeBboxFilter` | `candidate_area` (`--bbox` and `--area`, several matches an error) |
| `CoordinateOperationContext::setAreaOfInterest`, `setSpatialCriterion`, `setSourceAndTargetCRSExtentUse`, `setDesiredAccuracy` | `CoordinateOperationContext::set_area_of_interest`, `set_spatial_criterion`, `set_crs_extent_use`, `set_desired_accuracy` |
| `SpatialCriterion` | `SpatialCriterion` (`StrictContainment`, `PartialIntersection`) |
| `SourceTargetCRSExtentUse` | `CrsExtentUse` (`Ignore`, `Both`, `Intersection`, `Smallest`) |
| `getExtent(op)`, `getExtentPossiblySynthetized(crs)` | `operation_extent`, `crs_extent` |
| `Extent::contains`, `intersects`, `intersection`, `getPseudoArea` | `extent_contains`, `extent_intersects`, `intersect_extents`, `BoundingBox::{contains, intersects, intersection, pseudo_area}` (`src/iso19111/model.rs`) |
| `FilterResults::filterOut`, `andSort` (extent and accuracy part) | `filter_and_sort`, `accuracy_allowed` |
| `getAccuracy` | `accuracy_allowed` (`Conversion` is 0 m, unknown is -1) |
| `createFromCoordinateReferenceSystemCodes`, the `tryReverseOrder` and `discardSuperseded` rows, the intersecting pass | `operations_between` with `RegistrySearch::Both` |
| the same, public overload (`tryReverseOrder` false) | `AuthorityFactory::create_from_coordinate_reference_system_codes` with `RegistrySearch::Forward` |
| `outputOperations`, accuracy text `accuracies[0]->value()` | `CoordinateOperation::accuracy_text`, `Transformation::accuracy_text` |

New public items: `BoundingBox`, `SpatialCriterion`, `CrsExtentUse`,
`ObjectDomain::from_bounding_box`, `ObjectDomain::bounding_box`,
`CoordinateOperation::accuracy_text`, and the context setters above.
`BoundingBox` replaces the CLI's `GeoBox`, so the list-crs filter and the
candidate filter share one implementation.

## Test mapping

Yaml cases, `tests/cli_yaml/test_projinfo.yaml`, numbered as the runner numbers them.

| Cases | Options | Status |
|---|---|---|
| 15, 17, 18 | `--bbox`, `--area EPSG:3237`, `--area 'Denmark - onshore'` | pass |
| 19, 20, 21 | `--area` with several matches, no match, `WRONG:CODE` | pass (exit 1 and stderr) |
| 51 | `--accuracy 0.05` | pass |

Rust tests: `contains_follows_the_antimeridian_rules` and
`intersects_follows_the_antimeridian_rules` now run on `BoundingBox`
(`src/bin/projinfo.rs`), with the same cases as before.

## Open candidate cases and their causes

Each cause was checked against the reference output for the case unless marked.

| Cases | Reference | Cause |
|---|---|---|
| 11, 12 | Ballpark geographic offset NAD27 to NAD83 (World) | `createOperationsGeodToGeod` does not synthesize the ballpark operation. |
| 34 | Ballpark geocentric translation, then EPSG:9225 | Same, for geocentric CRS (`createBallparkGeocentricTranslation`). |
| 13, 14 | `DERIVED_FROM(EPSG):` prefix on some ids, `, at least one grid missing` suffix | The suffix needs grid availability, which is the grids lot. The prefix is not investigated. |
| 16, 31 | `Inverse of UTM zone 31N + ...` | `Conversion` has no inverse, so a registry conversion read in reverse is dropped. |
| 25, 27, 38, 39 | Vertical or height operations | The vertical and geoid paths (`createOperationsVertToGeog`, `createOperationsGeogToVertFromGeoid`) are not ported. Not investigated per case. |
| 26 | 3 candidates, ours 2 | Probably the vertical path as above. Not investigated. |
| 29, 30, 32 | `WGS 84 + EGM96 height`, `EPSG:4326+3855` | Compound CRS: `createOperationsCompoundToGeog` is not ported. |
| 36, 37, 68, 108 | Concatenated operations through a datum | `createOperationsWithDatumPivot` is not ported. |
| 88, 89, 90 | `Conversion from ... (geocentric) + ...` chains | Same datum-pivot path, with geocentric conversions. Not investigated per case. |
| 84 | `Canada velocity grid` step | Grid lot (deformation grid). |
| 106 | `+proj=noop` for a same CRS | Not investigated. |
| 107 | Ballpark geographic offset NAD83(CSRS) to NAD83(CSRS)v2 | Same as 11, 12. |

## Deliberate differences

- **Ballpark operations** are not synthesized. Cases with a ballpark result
  stay open.
- **Only registry operations, the deriving conversion, and the pivots of
  `CoordinateOperationContext::set_intermediate_crs`** are candidates. No
  datum-pivot, compound, or vertical path.
- **Inverses** are built for registry transformations only (`Transformation::inverse`).
  A reverse-direction registry conversion or concatenation is dropped, where
  PROJ returns its inverse.
- **Supersession** is always applied, as PROJ does by default. `--show-superseded`
  is not implemented.
- **Sort**: candidates keep the registry order (pseudo area, then accuracy).
  PROJ's `SortFunction` (grid availability, step count, PROJ exportability)
  is not ported. No closed case depends on it yet.
- **Not ported**: `removeSyntheticNullTransforms`, `removeUninterestingOps`,
  `removeDuplicateOps`, and the `hasOpThatContainsAreaOfInterestAndNoGrid`
  flag they read.
- **`BoundingBox::intersection`** handles boxes that do not cross the
  antimeridian. PROJ's intersection for a box that crosses it is not ported.
- **Accuracy text**: a database `REAL` accuracy keeps SQLite's text (`1.0`,
  `0.113`). An accuracy that PROJ synthesizes uses Rust's `f64` text (`5`).
- **Area of interest** from `--area` uses the extent's description as well as
  its box, as `FilterResults` does with `foundExtentWithExpectedDescription`.

## Verification

- `cargo test --test cli_yaml` with the PROJ 9.9.0 binaries on `PATH`, on a
  fresh `origin/main` and on this branch: 59 open `projinfo` rows on `main`,
  52 here. Cases 15, 17, 18, 19, 20, 21 and 51 closed. No row open here
  that was passing on `main`, in any yaml suite.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --all-features --no-fail-fast`: every test binary reports 0 failed.
