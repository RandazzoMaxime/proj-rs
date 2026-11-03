# Port — `opfac-core-c`, API cases (lot `lotc-api`)

The five cases of `docs/PORTING-opfac-core-c.md` "Not written (11)" that need an API this crate did not expose: `IntermediateCRSUse::ALWAYS`, `GridAvailabilityUse::KNOWN_AVAILABLE`, `gridsNeeded`, `isPROJInstantiable`, and `baseCRSWithCanonicalBoundCRS`. C++ source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp` and the files below.

Rust tests are in `tests/unit_opfac_core_c_api.rs`. Each function is `operation_` plus the C++ name in snake case, and its doc comment carries the exact `TEST(operation, ...)` name.

**Result: 2 of 5 cases are ported. 3 stay open, each with a reason below.**

## C++ read

- `test/unit/test_operationfactory.cpp`: the five `TEST()` bodies (`IGNF_LAMB1_TO_EPSG_4326`, `compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform`, `createOperation_ITRF2000_to_ETRS89`, `isPROJInstantiable`, `createOperation_on_crs_with_canonical_bound_crs`).
- `src/iso19111/operation/coordinateoperationfactory.cpp`: `CoordinateOperationContext` defaults; `createOperations` (canonical bound substitution); `Private::findOpsInRegistryDirect` (the `ALWAYS` condition); `findsOpsInRegistryWithIntermediate`; `FilterResults`; the `gridsAvailable` / `gridsKnown` rules in `PrecomputedOpCharacteristics`; `SortFunction`.
- `src/iso19111/factory.cpp`: `AuthorityFactory::Private::rejectOpDueToMissingGrid`; `createFromCoordinateReferenceSystemCodes` (`discardIfMissingGrid`, `considerKnownGridsAsAvailable`); `createFromCRSCodesWithIntermediates` (not ported, see below); `DatabaseContext::lookForGridInfo`.
- `src/iso19111/operation/singleoperation.cpp`: `CoordinateOperation::isPROJInstantiable`, `SingleOperation::gridsNeeded`.
- `src/iso19111/operation/concatenatedoperation.cpp`: `ConcatenatedOperation::gridsNeeded`.
- `include/proj/coordinateoperation.hpp`: `GridDescription::operator<` (ordering by `shortName`).
- `src/iso19111/operation/transformation.cpp`: `Transformation::createGeocentricTranslations`, `createNTv2`, `createTOWGS84`, `getTransformationType` (the test's inputs).
- `src/iso19111/crs.cpp`: `CRS::canonicalBoundCRS`, `CRS::setCanonicalBoundCRS`, `BoundCRS::baseCRSWithCanonicalBoundCRS`, `BoundCRS::createFromTOWGS84`.
- `src/proj_constants.h`: `EPSG_CODE_METHOD_GEOCENTRIC_TRANSLATION_GEOGRAPHIC_2D` 9603, `EPSG_CODE_PARAMETER_X/Y/Z_AXIS_TRANSLATION` 8605/8606/8607, `EPSG_CODE_METHOD_NTV2` 9615, `EPSG_CODE_PARAMETER_LATITUDE_LONGITUDE_DIFFERENCE_FILE` 8656.

## Rust API added

| Rust | C++ | Where |
|---|---|---|
| `GridAvailabilityUse::KnownAvailable` | `GridAvailabilityUse::KNOWN_AVAILABLE` | `factory.rs` |
| `GridDescription { short_name, info: GridInfo }` | `GridDescription` | `factory.rs` |
| `CoordinateOperation::grids_needed(db, known)` | `gridsNeeded(dbContext, considerKnownGridsAsAvailable)` | `select.rs` |
| `CoordinateOperation::is_proj_instantiable(db, known)` | `isPROJInstantiable(dbContext, considerKnownGridsAsAvailable)` | `select.rs` |
| `Crs::canonical_bound_crs()` | `CRS::canonicalBoundCRS()` | `model.rs` |
| `BoundCRS::base_crs_with_canonical_bound_crs()` | `BoundCRS::baseCRSWithCanonicalBoundCRS()` | `crs_ops.rs` |
| `OperationParameter::with_identifier(code_space, code)` | `OperationParameter` identifier | `model.rs` |

Supporting changes:

- `Identified` has a `canonical_bound: Option<Box<BoundCRS>>` field, as `original_compound` does for the compound CRS of a promoted CRS. Every `Crs` variant reaches its `Identified`, through `GeodeticCRS` for the geodetic variants.
- `CoordinateOperationFactory::create_operations` starts from the canonical bound CRS of each input, as `createOperations` does. Only the public call does it.
- `Transformation::create` keeps the identifier of each parameter definition as the stored parameter code. C++ reads the EPSG code of a parameter from its identifier (`getEPSGCode`). Before this change the public builder dropped it, so an exportable EPSG-coded transformation could not be built through the public API. `Conversion::create` is unchanged.
- `DatabaseContext::look_for_grid_info(name, known)` is `lookForGridInfo`. `grid_info(name)` keeps its earlier behaviour (no row gives `None`) and calls it with `UseForSorting`.

### Deviations from C++

- `considerKnownGridsAsAvailable` is a `GridAvailabilityUse`, not a `bool`, because a public signature may not take a `bool` (`CLAUDE.md`). `KnownAvailable` is `true`; `UseForSorting` and `IgnoreGridAvailability` are `false`. This is the value the C++ factory passes (`gridAvailabilityUse == KNOWN_AVAILABLE`).
- `grids_needed` keeps `null` (available, as `lookForGridInfo` gives it) and every file parameter, including the NADCON longitude file, as `gridsNeeded` does. Empty names from a comma list are dropped.
- A grid with no database row is looked up on the search path, as `lookForGridInfo` does after its query. A name that starts with `http://` or `https://` becomes the URL.
- `is_proj_instantiable` exports with `export_to_proj_string_with_database`, which renames grids. C++ exports without a database. The renamed grid changes the pipeline text, not whether the export succeeds, so the answer is the same.
- `KnownAvailable` discards a registry operation with a grid that is neither on disk nor known (`rejectOpDueToMissingGrid`). Only the direct registry search does this. C++ also does it in its intermediate-CRS searches (`createFromCRSCodesWithIntermediates`, `createBetweenGeodeticCRSWithDatumBasedIntermediates`), which this crate does not have.
- Sorting: `grids_available` is cleared only under `UseForSorting`, as the C++ `SortFunction` input (`gridsAvailable` is set false only when `USE_FOR_SORTING`).

## Cases ported (2)

| C++ TEST | Rust test |
|---|---|
| `isPROJInstantiable` | `operation_is_proj_instantiable` |
| `createOperation_on_crs_with_canonical_bound_crs` | `operation_create_operation_on_crs_with_canonical_bound_crs` |

`isPROJInstantiable` checks three transformations built with the EPSG method and parameter codes that the C++ constructors set (`createGeocentricTranslations`, `createNTv2`, and an empty method). Each one gives the answer for the reason the C++ comment gives:

- Geocentric translations export as a Helmert and need no grid: `true`.
- NTv2 with `foo.gsb` exports, and the grid is not on disk or known: `false`.
- The empty method has no PROJ string (`Unimplemented`): `false`.

The canonical-bound test checks that `create_operation` on the base CRS returns the bound CRS's transformation. Without the substitution the same call gives `Ballpark geographic offset from NAD27 to WGS 84`, so the test is not vacuous.

## Cases not ported (3)

### `IGNF_LAMB1_TO_EPSG_4326` (needs `IntermediateCRSUse::ALWAYS`)

Not added. C++ `ALWAYS` runs `findsOpsInRegistryWithIntermediate` for any pivot CRS in the database, through `createFromCRSCodesWithIntermediates`. That is an SQL search over `coordinate_operation_view` joined twice, with supersession, usage extents, and a publication-date criterion on the pivot's datum, followed by the ETRF and NAD83(CSRS) rules. The crate only pivots through pivots named with `set_intermediate_crs`. That path already runs whether or not a direct operation exists, so `IntermediateCrsUse::Allow` behaves like `ALWAYS` for named pivots. A variant with no generic search behind it would have no semantics, so none was added.

The test sets no pivot. With the crate's default context, `LAMB1 -> EPSG:4326` gives one composite operation (`Inverse of LAMBERT I NORD + NTF geographiques Paris (gr) vers NTF GEOGRAPHIQUES GREENWICH (DMS) + ... vers WGS 84`). PROJ gives two, both written without the Greenwich step.

### `compoundCRS_to_geogCRS_3D_using_intermediate_of_horizontal_transform` (needs the compound intermediate path)

`KnownAvailable` now exists, but the C++ result needs `createOperationsCompoundToGeog` to go through a horizontal intermediate (`PNG94 to WGS 84 (1)`), with the vertical steps through EGM96 (issue 4618). The crate returns one ballpark operation under both `UseForSorting` and `IgnoreGridAvailability`: `Inverse of Papua New Guinea Map Grid 1994 zone 54 + Ballpark geographic offset from PNG94 to WGS 84 (G2139) + Transformation from Kumul 34 height to WGS 84 (G2139) (ballpark vertical transformation, ...)`. PROJ gives `Inverse of Papua New Guinea Map Grid 1994 zone 54 + PNG94 to WGS 84 (1) + Inverse of EGM96 height to Kumul 34 height (1) + Inverse of WGS 84 to EGM96 height (1) + WGS 84 to WGS 84 (G2139)`. Missing: the compound path with a horizontal intermediate. `KnownAvailable` has no test here.

### `createOperation_ITRF2000_to_ETRS89` (needs the datum pivot and a matching database)

`gridsNeeded` is now public (`grids_needed`) and the grid split can be checked. The test is still open for two reasons:

1. The crate finds one operation (`Ballpark geographic offset from ITRF2020 to ETRS89`). PROJ finds registered operations, grid-based and grid-less, through the datum pivot (`createBetweenGeodeticCRSWithDatumBasedIntermediates`), which is not ported.
2. The embedded `data/proj.db` (EPSG v13.102) has EPSG:9989 as `ITRF2020`. PROJ 9.9.0's test uses 9989 as `ITRF2000` (`ITRF2000 to ITRF2005 (1)` appears elsewhere in the same file). The two databases disagree on this code, so the operation list cannot be compared with PROJ's here.

## Commands and results

Run in `/Users/maximerandazzo/Desktop/proj-rs/wt-lotc-api`, branch `lotc/api`.

- `cargo test --test unit_opfac_core_c_api`: 2 passed, 0 failed.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: exit 0, 1374 passed, 0 failed, 26 ignored (all pre-existing in other test files).
