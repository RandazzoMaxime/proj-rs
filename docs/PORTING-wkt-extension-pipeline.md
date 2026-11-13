# Port — seven stale `#[ignore]` tests: WKT EXTENSION, `+proj=pipeline` CRS, derived projected, temporal CS

Source, PROJ 9.9.0: `test/unit/test_io.cpp` (`wkt_parse.wkt1_geographic_with_PROJ4_extension` 790,
`wkt_parse.wkt2_krovak_modified_south_west` 1933, `wkt_parse.wkt2_krovak_modified_east_north` 2035,
`wkt_parse.invalid_EngineeingCRS` 9714), `test/unit/test_crs.cpp` (`crs.normalizeForVisualization_derivedprojected_operation`
7681, `createDerivedProjectedCRSNorthingEasting` 4817). Also read for the expected values: the C++ literals of those tests.

## What this lot is

Seven tests carried `#[ignore = "open: ..."]` with a cause written against an older state of the code.
Each ignore reason named a missing piece of the port. Run with `--ignored`, the seven tests pass on the
current code (HEAD `ead2c4c`). No library change was needed. The lot removes the seven attributes and
corrects the comments that still described them as open. The other ignored tests in the same files stay
as they are, with their reasons.

## Map of tests

| Test | File | Ignore reason before this lot | State found | Action |
|---|---|---|---|---|
| `wkt_parse_wkt1_geographic_with_proj4_extension` | `tests/test_io_wkt1.rs` | EXTENSION[PROJ4] dropped on import and export | passes: the model keeps `extension_proj4`, and the WKT1 GDAL export and the PROJ string write it | ignore removed |
| `wkt_parse_wkt1_geocentric_with_proj4_extension` | `tests/test_io_wkt1.rs` | same | passes | ignore removed |
| `wkt_parse_wkt1_projected_with_proj4_extension` | `tests/test_io_wkt1.rs` | same | passes | ignore removed |
| `wkt_parse_wkt2_krovak_modified_south_west` | `tests/test_io_wkt2.rs` | a CRS from `+proj=pipeline` is refused | passes: the `+type=crs +proj=pipeline` string imports as a projected CRS with the `Krovak Modified` method | ignore removed |
| `wkt_parse_wkt2_krovak_modified_east_north` | `tests/test_io_wkt2.rs` | same | passes | ignore removed |
| `crs_normalize_for_visualization_derivedprojected_operation` | `tests/unit_crs_unit_3.rs` | derived projected conversion export not ported | passes, export and normalized export match the C++ literals | ignore removed, doc comment rewritten |
| `wkt_parse_invalid_engineeingcrs` | `tests/unit_io_core_a.rs` | `buildCS` has no temporal CS, `CS[temporal,1]` in an ENGCRS refused | passes: `ENGCRS` with `EDATUM` and `CS[temporal,1]` parses, and the two refused inputs are refused | ignore removed |

The ignore reasons of the seven tests were not the cause of any failure at HEAD, so there is no
bug to describe. The two sheets that said these were open are corrected: `docs/PORTING-crs-unit-3.md`
(table row and the derived projected bullet now say the test passes).

## Checks against PROJ 9.9.0

- The EXTENSION case: `projinfo -q -o PROJ` on the test's GEOGCS text gives
  `+proj=longlat +foo=bar +wktext +type=crs`, the string the test expects. `projinfo -q -o WKT1_GDAL`
  starts with the same lines as the test's expected WKT1 (output cut after four lines, so the full text was
  not compared this way; the test itself compares it in full).
- The Krovak and temporal tests and the derived projected test use the C++ literals (copied by the test
  files; the C++ source above is the reference).

## Deviations

- **Database argument of the derived operation export.** The C++ test calls
  `op->exportToPROJString(PROJStringFormatter::create())`. The Rust test calls
  `export_to_proj_string_with_database(&db, ProjLines::Single)`, the only operation export the Rust API
  has (`src/iso19111/transform_export.rs`). The database is used to rename grids. The operations in this
  test have no grid, so the expected text is the same. The expected values are unchanged.

## Code changed

None under `src/`.

Test files, only the attributes and the comments that describe them:

- `tests/test_io_wkt1.rs`: three `#[ignore]` removed (the three EXTENSION tests).
- `tests/test_io_wkt2.rs`: two `#[ignore]` removed (the two Krovak `+proj=pipeline` tests).
- `tests/unit_io_core_a.rs`: one `#[ignore]` removed (`wkt_parse_invalid_engineeingcrs`).
- `tests/unit_crs_unit_3.rs`: one `#[ignore]` removed; the module doc and the test's doc comment no
  longer describe it as ignored.
- `docs/PORTING-crs-unit-3.md`: the derived projected test is now marked as passing.

## Commands run

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export PROJ_RS_LIBPROJ_DIR=/home/user/proj-rs/target/proj-reference/build
export PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data
export PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH
export PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES

# baseline: the seven tests with their ignore still set
cargo test --test test_io_wkt1 -- --ignored proj4
cargo test --test test_io_wkt2 -- --ignored krovak
cargo test --test unit_crs_unit_3 -- --ignored derivedprojected
cargo test --test unit_io_core_a -- --ignored engineeingcrs

# after the attributes were removed
cargo test --test test_io_wkt1 --test test_io_wkt2 --test unit_crs_unit_3 --test unit_io_core_a
cargo fmt --check
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
```

Results of the four targets after the change: `test_io_wkt1` 19 passed, 1 ignored (the datum alias test,
unchanged); `test_io_wkt2` 22 passed, 19 ignored (unchanged); `unit_crs_unit_3` 3 passed; `unit_io_core_a`
58 passed. `cargo fmt --check` and clippy 1.100 with `-D warnings`: clean. Full suite: see the commit message.
