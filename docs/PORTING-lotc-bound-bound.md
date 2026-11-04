# Port — lot C, bound CRS to bound CRS (`wktext` case)

C++ case: `TEST(operation, createOperation_on_crs_with_bound_crs_and_wktext)` in `test/unit/test_operationfactory.cpp` (PROJ 9.9.0). Rust test: `tests/unit_opfac_core_c_bound.rs`.

Source: `+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +nadgrids=@GDA94_GDA2020_conformal.gsb +ignored1 +ignored2=val +wktext +type=crs` (bound CRS).
Target: `+proj=utm +zone=55 +south +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs` (bound CRS).

**Result: ported.** Before this change the crate returned 0 operations. The expected pipeline now matches exactly.

## Oracle

Expected values come from PROJ 9.9.0 (`projinfo` from the Homebrew bottle, extracted in a temporary directory, not installed). Command:

```sh
PROJ_DATA=<bottle>/share/proj projinfo -s "<source>" -t "<target>" -o PROJ
```

Output:

- Operation: `Inverse of UTM zone 55S + unknown to WGS84 + Inverse of Transformation from unknown to WGS84 + UTM zone 55S`, `at least one grid missing`.
- PROJ string: `+proj=pipeline +step +inv +proj=utm +zone=55 +south +ellps=GRS80 +step +proj=hgridshift +grids=@GDA94_GDA2020_conformal.gsb +step +proj=utm +zone=55 +south +ellps=GRS80`.

The grid is absent on the system, so the operation is listed with the grid missing. The test expects the same string as the C++ test.

## C++ logic ported

`createOperationsBoundToBound` (`src/iso19111/operation/coordinateoperationfactory.cpp`, 9.9.0), first block only:

- Both hubs are `GeographicCRS` and equivalent (`_isEquivalentTo`, EQUIVALENT). Here both hubs are WGS 84.
- `opsFirst` = operations from the source CRS to the source hub; `opsLast` = operations from the source hub to the target CRS.
- Every pair is concatenated, in order, as `[opFirst, opLast]`.
- If the result is non-empty, return it.

Otherwise the fallback is `createOperations(boundSrc->baseCRS(), boundDst->baseCRS())`. That matches the Rust.

Rust: `bound_to_bound` in `src/iso19111/bound_ops.rs`, dispatched from `bound_candidates` for a `(Bound, Bound)` pair. The C++ dispatch is the `boundSrc && boundDst` case of `createOperations`, after the geographic and vertical bound cases.

## Not ported

- The two vertical blocks of `createOperationsBoundToBound`: same vertical datum (ignore the bound transformation), and same geographic hub for vertical bases. Both are used only for bound vertical CRS. The Rust goes straight from the geographic-hub block to the base-to-base fallback for these. Not covered by any test here. Documented in the module doc of `bound_ops.rs`.
- The extent check of `ConcatenatedOperation::createComputeMetadata` (`disallowEmptyIntersection`). The Rust `concatenate` does not drop empty-intersection pairs.
- The projected-target branch (`derivedDst`). The test does not need it, so it was not added. A naive projected-target branch was not re-checked; it stays out of this change.
- The database lookup that C++ runs before the bound cases (`createOperationsFromDatabase`). In the Rust, `bound_candidates` runs before the registry lookup in `create_candidates`, as for `bound_to_geog` already. Same order as the existing bound code.

## Tests

- `operation_create_operation_on_crs_with_bound_crs_and_wktext` (`tests/unit_opfac_core_c_bound.rs`), passing. Without the bound-to-bound branch it fails with 0 operations.
- Full suite (`PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`): 1389 passed, 0 failed, 26 ignored.
