# Porting sheet: factory-tmpdb-2

PROJ 9.9.0 `test/unit/test_factory.cpp`, the `FactoryWithTmpDatabase` cases that run on a
temporary `proj.db` (`createStructure` and `populateWithFakeEPSG`). The Rust tests are
`tests/unit_factory_tmp_db_2.rs`; the fixture is `tests/common/tmp_db.rs`.

## Closed in this lot (10 of the 13 cases listed)

| PROJ `TEST_F` | Rust test | Ledger |
|---|---|---|
| `AuthorityFactory_proj_based_transformation` | `factory_with_tmp_database_authority_factory_proj_based_transformation` | closed |
| `AuthorityFactory_wkt_based_transformation` | `..._wkt_based_transformation` | closed |
| `AuthorityFactory_wkt_based_transformation_not_wkt` | `..._wkt_based_transformation_not_wkt` | closed |
| `AuthorityFactory_wkt_based_transformation_not_co_wkt` | `..._wkt_based_transformation_not_co_wkt` | closed |
| `lookForGridInfo` | `factory_with_tmp_database_look_for_grid_info` | closed |
| `lookForGridInfo_from_old_name_with_new_grid_available` | `factory_with_tmp_database_look_for_grid_info_from_old_name_with_new_grid_available` | closed |
| `custom_geodetic_crs` | `factory_with_tmp_database_custom_geodetic_crs` | closed |
| `CoordinateMetadata` | `factory_with_tmp_database_coordinate_metadata` | closed |
| `createOperations_exact_transform_not_whole_area` | `factory_with_tmp_database_create_operations_exact_transform_not_whole_area` | closed |
| `check_fixup_direction_concatenated_inverse_map_projection` | `factory_with_tmp_database_check_fixup_direction_concatenated_inverse_map_projection` | closed |

Not closed: `custom_projected_crs`, `custom_derived_projected_crs`, `factory.objectInsertion`
(see the last section).

Each closed case checks what the C++ asserts, with the Rust API. The `not_wkt` and
`not_co_wkt` cases check that the operation is refused (`is_err()`), not the error variant.

## Foundation taken from another branch

The fixture (`tests/common/tmp_db.rs`), `DatabaseContext::open` and the example file
`tests/unit_factory_tmp_db.rs` (with `docs/PORTING-factory-tmpdb-1.md`) were not on `main`
nor in this worktree. They are on `claude/upbeat-meitner-32qhxg`, in the commits
`08bf7e1` (database open, `docs/PORTING-database-open.md`) and `8d65808` (temporary database
helper). Only those two commits were cherry-picked. The ledger hunk of `8d65808`, which
closes ten `createFromCRSCodesWithIntermediates` cases of another lot, was left out: those
lines stay in `docs/UNPORTED-UNIT.md`.

## Code changes in `src/`

| Where | PROJ | Change |
|---|---|---|
| `iso19111/factory.rs`, `other_transformation` | `createOperation`, method `WKT` (factory.cpp) | The `COORDINATEOPERATION` (or conversion, or concatenated operation) that the method name spells keeps its own name and gets the row's source and target CRS. Anything else is an error. Method `PROJString` keeps the generic path, whose method name is the PROJ string. |
| `iso19111/factory.rs`, `geodetic_crs` | `createGeodeticCRS`, `text_definition` branch | A row whose `text_definition` is a PROJ string or an object reference is expanded with `create_from_user_input`. The row's name, identifier and flags replace the expanded object's (`cloneWithProps`). A bound CRS gives its base with the canonical bound CRS. A text naming a row already being expanded is an error (`RecursionDetector`, kept as a thread-local stack of `(authority, code)`). `pj_add_type_crs_if_needed` is `add_type_crs_if_needed`. |
| `iso19111/transform_export.rs`, `export_forward` | `PROJBasedOperation::_exportToPROJString` | A transformation whose method is `PROJString` writes its method name, the PROJ string, as the pipeline. |
| `iso19111/transform_export.rs`, `epsg_method_code` | `OperationMethod::getEPSGCode` | A method with no EPSG identifier takes the code of its name (`parammap::method_by_name`). |
| `iso19111/transform_export.rs`, `measure_by_code` | `OperationParameter::getEPSGCode` | A parameter with no EPSG identifier is found by the name of the EPSG parameter of the method. |
| `iso19111/select.rs`, `collect_grids`, `collect_file_parameters` | `PROJBasedOperation::gridsNeeded`, `PROJStringFormatter::getUsedGridNames` | The grids of a PROJ-method transformation are the `grids` and `file` names of its PROJ string (`pipeline::used_grid_names`), not only its file parameters. |
| `iso19111/pipeline.rs` | `PROJStringSyntaxParser`, `PROJStringFormatter::toString` | The global parameters of an ingested pipeline (`+proj=pipeline +ellps=WGS84 +step ...`) are kept and printed after `+proj=pipeline`, which then takes the pipeline form. Before they were dropped. |
| `iso19111/coordinate_metadata.rs`, `coordinate_metadata_from_urn` | `createFromURNPart`, type `coordinateMetadata` | New. Reads `urn:ogc:def:coordinateMetadata:AUTH:VERSION:CODE` and returns the coordinate metadata. Exported from `iso19111`. |

## Why the PROJ string of a `PROJString` row is exported and graded

The candidate sort of `select.rs` puts exportable operations first, then compares grid
availability, then accuracy. A `PROJString` row used to export nothing, so the NKG
concatenated operation `NKG:ITRF2014_TO_DK` (accuracy 0.01 m) was not exportable and ranked
after `EPSG:10894` (0.006 m). Once its PROJ strings export, it ranks first, unless its grids
are graded as PROJ grades them. PROJ's `PROJBasedOperation::gridsNeeded` reads the grids from
the PROJ text, and `EPSG:10894` then wins on grid availability, which is the order of
`tests/cli_yaml/test_projinfo.yaml#65`. The export and the grid reading were ported together
for that reason. Two other forms were tried and reverted: a `Conversion` for the PROJ string
(its accuracy reads 0, `compound.rs::get_accuracy`, so the order changed), and a
transformation whose method is written `PROJ-based operation method: ...` (same ordering
as the `PROJString` form, but a different method name, so it was not kept as the simpler
choice).

## Deviations

- **Error kind.** `EXPECT_THROW(..., FactoryException)` becomes `is_err()`: the Rust API
  returns `FactoryError`, and the test checks that the operation is refused, not the variant.
- **Recursion guard.** PROJ's `RecursionDetector` lives on the `DatabaseContext`. Here a
  thread-local stack holds the `(authority, code)` rows being expanded. It refuses the same
  row twice in one thread, as PROJ does for one context.
- **`coordinate_metadata_from_urn`** returns `Result<Option<CoordinateMetadata>>`, not an
  object of a generic `createFromUserInput`, which returns `Crs` only in this crate. For a
  versioned URN it looks the version up once and does not retry the other versions of the
  authority, as `createFromURNPart` does.
- **Name fallback for methods and parameters.** PROJ's `getMethodNameCodes` and the
  parameter name table are read through `parammap::method_by_name`, which matches the WKT2
  names of the crate's own table (case and separators ignored). A method missing from that
  table keeps no code, as before.
- **Global parameters.** The pipeline formatter keeps them only for pipelines ingested as
  text (`ingest_proj_string`). `init=` is still read as an ordinary parameter of a step, as
  before, so it may print differently from PROJ.
- **`lookForGridInfo`** is `DatabaseContext::grid_info`, the `considerKnownGridsAsAvailable =
  false` form. The fixture grid `data/tests/egm96_15_uncompressed_truncated.tif` is found
  through `PROJ_DATA`, set by `common::use_fixture_search_path`.

## Not closed, and why

- **`custom_projected_crs`.** It needs projected `text_definition` rows (`+proj=mbt_s ...`),
  the same expansion as the geodetic one, and `identify` of PROJ-string CRS against the
  database (`merc` and `merc2` by ellipsoid, the WKT1_GDAL name), then `getCRSInfoList`.
  The identify part is a larger port than this lot.
- **`custom_derived_projected_crs`.** The derived projected text definitions and their
  identification are larger still. Not started.
- **`factory.objectInsertion`.** It writes objects into a database and reads the SQL back.
  `insert.rs` produces the statements as text only; there is no writable `DatabaseContext`
  path to run them. Not started.

## Commands and results

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --test unit_factory_tmp_db_2          # 10 passed
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
  PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --all-features --test cli_yaml       # passes, ledger unchanged
```

Full run, one test target at a time (`docs_in_sync` and `progress_svg` left out, as asked),
then `--doc`: every target exits 0; the sum of the test results is 1733 passed, 0 failed,
27 ignored.

The expected values are the ones of `test_factory.cpp` (PROJ 9.9.0). The reference
`projinfo` of `target/proj-reference/build/bin` (PROJ 9.9.0) agrees on the EPSG rows the
fake database reuses: `EPSG:4326` exports as `+proj=longlat +datum=WGS84 +no_defs +type=crs`
and `EPSG:32631` is named `WGS 84 / UTM zone 31N`.
