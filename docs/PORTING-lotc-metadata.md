# Port — `lotc-metadata` (CoordinateMetadata, lot C)

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the five lot-C cases that need a `CoordinateMetadata` (an epoch attached to a CRS). They are listed as "Needs CoordinateMetadata" in `docs/PORTING-opfac-core-c.md`, which this lot does not edit.

**Result: 2 of 5 cases are ported. 3 stay open: the HT2 compound cases need a compound to compound chain the crate does not build.**

Rust tests: `tests/unit_opfac_core_c_metadata.rs`. Expected PROJ strings are copied from the C++ file. Each doc comment carries the `TEST(operation, ...)` name.

## C++ read (PROJ 9.9.0)

| File | What was read |
|---|---|
| `src/iso19111/coordinates.cpp` | `CoordinateMetadata::create(crs)` (no epoch, static CRS only) and `create(crs, epoch, dbContext)` (a static CRS is accepted only with point motion operations on its geodetic CRS). |
| `src/iso19111/factory.cpp` | `AuthorityFactory::createCoordinateMetadata(code)`, `getPointMotionOperationsFor`, `createGeodeticCRSFromDatum(datum)` (the datum-name fallback). |
| `src/iso19111/operation/coordinateoperationfactory.cpp` | the four `createOperations` overloads with `CoordinateMetadata` (lines 8139-8198): they set the context epoch, then call the CRS to CRS version. Each returned operation gets `setSourceCoordinateEpoch` / `setTargetCoordinateEpoch` (around line 8110). |
| `src/iso19111/operation/singleoperation.cpp` | `setSourceCoordinateEpoch` and its inverse swap (lines 300-325, 5558); the helmert export (3690-3790); the NZ deformation model export (4930-4950); the `set` steps of the point motion export (5820-5860). |

## Rust API added

| Item | File | C++ |
|---|---|---|
| `CoordinateMetadata` (`create`, `create_without_epoch`, `crs`, `coordinate_epoch`) | `src/iso19111/coordinate_metadata.rs` | `coordinates::CoordinateMetadata` |
| `AuthorityFactory::create_coordinate_metadata` | `src/iso19111/factory.rs` | `createCoordinateMetadata` |
| `CoordinateOperationFactory::create_operations_from_metadata` (CM, CRS) | `factory.rs` | `createOperations(CoordinateMetadata, CRS, ctx)` |
| `CoordinateOperationFactory::create_operations_to_metadata` (CRS, CM) | `factory.rs` | `createOperations(CRS, CoordinateMetadata, ctx)` |
| `CoordinateOperationFactory::create_operations_between_metadata` (CM, CM) | `factory.rs` | `createOperations(CoordinateMetadata, CoordinateMetadata, ctx)` |

`CoordinateMetadata::create` takes `&Crs`, the epoch as a decimal year, and `&DatabaseContext`. The C++ null database is not offered: the static-CRS exception needs a database, so the check runs without it only as `create_without_epoch`.

## Epochs on transformations

- `Transformation` stores `source_epoch` and `target_epoch` (decimal years). The three `create_operations_*_metadata` methods set them on each returned transformation (`with_epochs`). Other kinds (conversion, concatenated) keep none, since PROJ writes no `set` for them in these cases.
- `Transformation::inverse` swaps the two epochs, as PROJ's inverse does.
- The export of an inverted transformation reads the epochs of its forward operation, as PROJ exports `forwardOperation()`.
- The helmert export writes `set v_4=<source>` before the `helmert` step and `set v_4=<target>` after it. The defmodel export writes the same pair around `defmodel`, in the order `singleoperation.cpp` uses. A side with no epoch takes the other side's year (`epoch_years`). A year of zero or less counts as none.
- The rounding is the existing `rounded_epoch_in_decimal_year` (`json.rs`), which is `getRoundedEpochInDecimalYear`.
- `extract_geodetic` follows `CRS::extractGeodeticCRS` for geographic, geodetic, projected (base) and compound (first component) CRS.

## Cases ported (2)

| C++ TEST | Rust test | Notes |
|---|---|---|
| `createOperation_time_dependent_helmert_directly_from_database` | `operation_time_dependent_helmert_directly_from_database` | ITRF2014 geocentric (7789) at 2026 to GDA2020 geocentric (7842). Forward, inverse, and the reverse direction, as in the C++ file. |
| `createOperation_defmodel_from_database` | `operation_defmodel_from_database` | ITRF96 (7907) at 2026 to NZGD2000 (4959). Grid name `nz_linz_nzgd2000-20180701.json` is the PROJ name, written through `export_to_proj_string_with_database`. The model file is absent; `IgnoreGridAvailability` keeps the operation. |

## Cases not ported (3)

| C++ TEST | Reason |
|---|---|
| `createOperation_compound_to_compound_with_epoch_HT2_1997_CGG2013a` | The compound to compound chain is not built. Without any epoch, `NAD83(CSRS)v7 / MTM zone 7 + CGVD28 height` (HT2_1997) to `NAD83(CSRS)v7 / UTM zone 19 + CGVD2013a(1997) height` gives 0 operations, with or without the geoid model. The vertical side, CGVD28 to CGVD2013a(1997) (EPSG:5713 to 20035), does build, but the grid name is the official `HT2_1997_CGG2013a.byn` and not the PROJ name. The same gap is recorded in `docs/PORTING-opfac-core-c.md` (`compoundCRS_to_compoundCRS_context_extent_use_none`). It lies in the vertical and compound code that the vert lot works on. |
| `createOperation_compound_to_compound_with_epoch_HT2_2002_CGG2013a` | Same chain (`HT2_2002v70`), same 0 operations. |
| `createOperation_compound_to_compound_with_epoch_HT2_2010_CGG2013a` | Same chain (`HT2_2010v70`), same 0 operations. |

The three cases also start from `AuthorityFactory::create_coordinate_metadata("NAD83_CSRS_…")`, which does not work yet (next section).

## Gaps found, not fixed

- **`create_coordinate_metadata` fails on all 198 rows of `coordinate_metadata`.** Each row is NRCAN with a PROJJSON `crs_text_definition`. The NAD83(CSRS)v7 datum in that JSON has no identifier. `get_point_motion_operations_for` looks the datum up by identifier only. PROJ falls back to the datum name (`createObjectsFromName` with `GEODETIC_REFERENCE_FRAME`, one match, equivalence check, then `createGeodeticCRSFromDatum` by identifier). The result is `Coordinate epoch should not be provided for a static CRS`, while PROJ accepts the row. The fallback belongs in `factory.rs` and needs `createObjectsFromName` plus a datum equivalence check. It was not written in this lot.
- **The epoch does not drive the search.** PROJ's `Private::createOperations` also uses the epochs: point motion operations, `cloneWithEpochs` when both epochs differ, and the epoch-change candidates. The crate searches without them and attaches the epochs only. No ported case has two different epochs.
- **Only transformations store the epochs.** A conversion or a concatenated operation returned for a CM pair has no epoch, and PROJ writes none for them either in the ported cases.
- **Deprecated earlier notes.** `docs/PORTING-transformation-export.md`, `docs/PORTING-projinfo-grids-deformations.md`, `docs/PORTING-opfac-core-2.md` and `docs/PORTING-cs2cs-epochs.md` say that epochs are not stored. That is true for the helmert and defmodel exports now. They are not edited here (outside the lot).

## Commands and results

```
cargo test --test unit_opfac_core_c_metadata   -> 2 passed; 0 failed
cargo fmt --check                              -> ok (after cargo fmt on the new code)
cargo clippy --all-targets --all-features -- -D warnings  -> exit 0
PROJ_RS_LIBPROJ_DIR=<projbottle>/lib PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --all-features --no-fail-fast      -> 1374 passed, 0 failed, 26 ignored (62 test binaries)
```

Oracle checks (PROJ 9.9.0 `projinfo` with `PROJ_DATA` set to the bottle's `share/proj`):

- `projinfo -s EPSG:7789 --s_epoch 2026 -t EPSG:7842 --grid-check none --spatial-test intersects -o PROJ` gives the set, helmert, set pipeline of the test.
- `projinfo -s EPSG:7907 --s_epoch 2026 -t EPSG:4959 --grid-check none --spatial-test intersects -o PROJ` gives the defmodel pipeline, with the `nz_linz_nzgd2000-20180701.json` name.
- `projinfo` with the HT2 1997 PROJJSON pair gives the `tmerc`, `vgridshift ca_nrc_HT2_1997_CGG2013a.tif +multiplier=-1`, `utm` pipeline. The Rust `projinfo` gives 0 operations for that pair. `--s_epoch` is accepted but does not change the Rust output.

The expectations in the test file are the C++ strings, not the oracle output. The oracle checks above confirm that they are the PROJ 9.9.0 output.
