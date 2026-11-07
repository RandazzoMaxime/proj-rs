# projinfo: misc parity (lot E)

Two `test_projinfo.yaml` cases, and the `projinfo` output they showed to be wrong
in the same code path. Oracle: PROJ 9.9.0 `projinfo` (tag 9.9.0 sources).

## Cases closed

| Case | Before | After |
|---|---|---|
| `test_projinfo.yaml#23` Testing non compliant WKT1 | exit 1, `GEOGCS should have a PRIMEM node` | exit 0; stdout, stderr and exit code equal to PROJ |
| `test_projinfo.yaml#28` `EPSG:32631 --3d` | text mismatch in WKT2 (scope, 3D base CS) | equal to PROJ |

The ledger `docs/UNPORTED-CLI.md` lists 32 projinfo rows (34 before). Nothing reopened.

## PROJ read

- `src/apps/projinfo_lib.cpp`
  - `buildObject`, WKT branch: non-strict `WKTParser`, `Warning: ` and `Grammar error: ` lines to the WARN stream, unless `-q` (389-406).
  - `outputOperationSummary`: the grid suffix, `getenv("PROJINFO_NO_GRID_CHECK")` (898-962, grid check 945).
  - `outputOperations`: summary and full listing use the same line (1156-1176).
  - `outputObject`: each block, its header, its export error caught and reported on the WARN stream; WKT1 blocks end with an extra blank line (610-828, WKT1 752-805).
  - `--3d` calls `CRS::promoteTo3D` on any CRS (543-550).
- `src/apps/projinfo.cpp`: WARN and ERR go to `std::cerr`, INFO to `std::cout` (45-54).
- `src/iso19111/io.cpp`
  - `WKTParser::Private::emitRecoverableWarning` and `emitGrammarError` (1532-1550): strict throws, lax records.
  - `createFromWKT`: the object is built, then the WKT1 text goes through `pj_wkt1_parse` (8506-8520).
  - `buildCS` (2859-3096): the VERT_CS unit (3081-3089); `emitRecoverableMissingUNIT` (2845-2856).
  - Projected CRS import promotes a 2D base of a 3D CS (4870-4874); derived geographic does too (3478-3481).
- `src/iso19111/crs.cpp`
  - `createProperties` of `promoteTo3D`: the extent of each domain without its scope; `Promoted to 3D from AUTH:CODE` then the source remarks (1341-1378).
  - `ProjectedCRS::_exportToWKT` writes the WKT2 base by hand, with no CS (4536-4557).
  - `GeodeticCRS` WKT1 3D error text: `WKT1 does not support Geographic 3D CRS.`
- `src/wkt1_parser.cpp` (`pj_wkt1_lex`, `pj_wkt1_parse`), `src/wkt_parser.cpp` (`pj_wkt_error`), `src/wkt1_grammar.y`, `src/wkt1_generated_parser.c` (bison 3.5.1: `yytnamerr` 1093, `yysyntax_error` 1144-1282, `yyparse` 1305).
- `src/iso19111/c_api.cpp:742-758`: `validateParameters` notes go to the C API's `out_warnings` only.
- `src/iso19111/common.cpp:305`: `UnitOfMeasure::operator==` compares names.
- `test/cli/run_cli_test.py:521-522`: a yaml `out` is run with `stderr=subprocess.STDOUT`, one merged stream.

## Port

| Area | PROJ | Port |
|---|---|---|
| WKT1 syntax check | `pj_wkt1_parse`, bison tables | `src/iso19111/wkt1_syntax.rs` (lexer, `yyparse`, `yysyntax_error`, `pj_wkt_error`); `src/iso19111/wkt1_syntax/tables.rs` extracted from `wkt1_generated_parser.c` by a script (not committed) |
| Grammar check in `createFromWKT` | io.cpp:8506-8520 | `WktParser::create_from_wkt` (`src/iso19111/wkt_import.rs:636`): after `build_from_wkt`, strict fails, lax records in `grammar_errors()` |
| Parser warnings | `WKTParser::warningList()` | no validation notes (`warnings()` holds the recoverable ones only) |
| Lax WKT in projinfo | projinfo_lib.cpp:389-406 | `read_wkt` (`src/bin/projinfo.rs:1216`), used by `load_named` and `build_object` |
| Missing UNIT text | io.cpp:2845-2856 | `missing_unit` (`wkt_import.rs:2108`) |
| VERT_CS without UNIT | io.cpp:3081-3089 | `wkt_import.rs:1995-2003`: metre, warning on the VERT_CS |
| Promoted CRS properties | crs.cpp:1341-1378 | `promoted_properties` (`src/iso19111/identify.rs:1016`) |
| Base of a 3D projected CRS | crs.cpp:4541-4557 (no CS), io.cpp:4870-4874 (import promotes) | `src/iso19111/wkt.rs` writes no base CS; `wkt_import.rs:2579` promotes a 2D base |
| Summary line | projinfo_lib.cpp:898-962 | `operation_heading` (`projinfo.rs:1326`) with `summary_grid_check` (1316) |
| Trailing newline of candidates | `println` of PROJ's lines | `without_final_newline` (1069) on candidate text |
| Export errors per block | projinfo_lib.cpp:640-828 | `render` (`projinfo.rs:1698`) prints the header, then `Error when exporting to <dialect>: ...` on stderr |
| `--3d` | `CRS::promoteTo3D` | `promote` (1310) calls `Crs::promote_to_3d` |
| yaml combined output | run_cli_test.py:521-522 | `tests/cli_yaml.rs`: one open file for stdout and stderr (dup'd descriptors) |

## Deliberate differences and notes

- `WktParser::set_strictness` takes the `WktStrictness` enum of the WKT1 export (`Strict`, `Lenient`), and the lax parse passes `Lenient`. The name is a conflation, kept because the API is public.
- WKT2 grammar errors (`pj_wkt2_parse`, `wkt2_grammar.y`) are not ported. A WKT2 text gets no `Grammar error:` line.
- The AXIS branch (`wkt_import.rs`, `build_axis`) still warns `missing UNIT in AXIS node`. PROJ throws `buildCS: missing UNIT` for an AXIS with no unit and no fallback (io.cpp:2758-2762). No oracle case here shows it, so it is not changed.
- Two unit fixtures in `tests/test_io.rs` (polar stereographic, 5 cases) ended their parameter list with a comma, which PROJ's grammar rejects. PROJ's own text (test_io.cpp) has one comma. The expected strings are unchanged.
- The yaml harness compares combined output by content lines, which trims trailing newlines. The output newlines are therefore checked by `tests/cli_projinfo_text.rs` instead.
- The 3D base CS rule that wrote the CS of a 3D geographic base (`PROJ issue 2122`, in `docs/PORTING-crs-core-2.md`) is removed: PROJ 9.9.0 writes no base CS.

## Oracle table

Each invocation was run on PROJ 9.9.0 `projinfo` and on this build with the yaml environment (`PROJINFO_NO_GRID_CHECK=YES`, `PROJ_DATA` as the yaml runner sets it), and stdout, stderr and the exit code were compared byte for byte. The captures are not committed; the expected texts they produced are copied into the tests named under Commands.

| # | Invocation | Exit | stdout | stderr |
|---|---|---|---|---|
| 1 | WKT1 `GEOGCS` without PRIMEM (`test_projinfo.yaml#23`) | = | = | = |
| 2 | WKT1 with `,,` before UNIT (polar stereographic) | = | differs (MERIDIAN, below) | = |
| 3 | WKT1 polar stereographic, one comma | = | differs (MERIDIAN, below) | = |
| 4 | WKT1 text followed by `x` | = | = | = |
| 5 | WKT1 unterminated string | = | = | = |
| 6 | WKT1 unknown node `FOO[...]` where UNIT is expected | = | = | = |
| 7 | WKT1 with a non-ASCII byte after the root | = | = | = |
| 8 | WKT1 lowercase keywords | = | = | = |
| 9 | ESRI `GCS_WGS_1984` | = | = | = |
| 10 | `EPSG:32631 --3d` (`test_projinfo.yaml#28`) | = | = | = |
| 11 | `-o PROJJSON EPSG:2154` | = | = | = |
| 12 | `-o WKT1_GDAL EPSG:32631` (blank line after the block) | = | = | = |
| 13 | `-o ALL EPSG:4979` (WKT1 3D export error, exit 0) | = | = | = |
| 14 | `-o PROJ EPSG:25832` | = | = | = |
| 15 | `-o WKT2:2019 EPSG:6349` | = | = | = |
| 16 | `-o ALL EPSG:2154` | = | = | = |
| 17 | `-o PROJJSON EPSG:3035` | = | = | = |
| 18 | `-o PROJ EPSG:4326 --3d` | = | = | = |
| 19 | `EPSG:2154 --3d` | = | differs (`+towgs84` term, below) | = |
| 20 | `EPSG:2154`, `-o PROJ` (2D) | = | = | = |
| 21 | VERT_CS without UNIT, one AXIS | = | = | = |
| 22 | `--3d` of a PROJ string with towgs84 (source) | = | = | = |
| 23 | `--3d` of a PROJ string with towgs84 (target) | = | = | = |
| 24 | `EPSG:4326` | = | = | = |
| 25 | `-o ALL EPSG:4326` | = | = | = |
| 26 | `-s EPSG:4326 -t EPSG:32631 --single-line` | = | = | = |
| 27 | `-s EPSG:4326 -t EPSG:4258 -o PROJ` | = | = | = |
| 28 | summary `EPSG:4230` to `EPSG:4258`, `--bbox` (`#15`) | = | = | = |
| 29 | summary `EPSG:23031` to `EPSG:4326` (`#16`) | = | = | = |
| 30 | summary `EPSG:4230` to `EPSG:4258`, `--area EPSG:3237` (`#17`) | = | = | = |
| 31 | summary `--area 'Denmark - onshore'` (`#18`) | = | = | = |
| 32 | summary `NAD27` to `NAD83` (`#13` region) | = | differs (DERIVED_FROM identifiers, below) | = |
| 33 | summary `EPSG:31467` to `ETRS89`, `--bbox` | = | differs (candidate order, below) | = |
| 34 | summary `AGD66` to `WGS 84 (G1762)` | = | differs (candidate count, below) | = |
| 35 | summary `EPSG:4936` to `EPSG:4978`, `--bbox` | = | = | = |
| 36 | summary `EPSG:23030` to `EPSG:25830`, `known_available` (grid check off) | = | = | = |
| 37 | summary `EPSG:23030` to `EPSG:25830`, `known_available` (grid check on, `at least one grid missing`) | = | = | = |
| 38 | summary `EPSG:7789` to `EPSG:4936`, `--area EPSG:1080` | = | differs (candidate set and NKG grid flag, below) | = |
| 39 | `-q -o PROJ --single-line` with `--normalize-axis-order` (candidates) | = | = | = |
| 40 | `-q -o PROJ --area EPSG:3237` (candidates) | = | = | = |

Totals: 40 invocations, 33 byte-identical, 7 differ.

Rows 36 and 37 are one pair, run with the grid check off and on. Only the `, at least one grid missing` suffix differs between them.

## Open items

| Item | Cause | Where to look |
|---|---|---|
| #2, #3 MERIDIAN nodes | WKT2 export of a polar stereographic CRS with `latitude_of_origin`: PROJ writes `MERIDIAN[90, ...]` and `MERIDIAN[180, ...]` on the conversion; the port writes none | conversion export of `Polar_Stereographic` (`src/iso19111/wkt.rs`) |
| #19 `+towgs84` of a 3D promoted projected CRS | PROJ finds a towgs84 (zero) transformation for the promoted 3D base of EPSG:2154; the port finds none | bound-CRS lookup of a 3D CRS (`create_bound_crs_to_wgs84_if_possible`) |
| #32 `DERIVED_FROM(EPSG)` identifiers | identifiers of derived operations | another lot (projinfo derived identifiers) |
| #33 candidate order | sorting of equal candidates | engine (`select.rs`) |
| #34 candidate count (20 against 19) | an extra candidate | engine |
| #38 candidate set, NKG grid flag | the candidate `EPSG:10894` is missing, and `NKG:ITRF2014_TO_DK` does not report its missing grid | engine and `grids_needed` |
| WKT2 grammar errors | `pj_wkt2_parse` is not ported | `wkt2_grammar.y`, a second table set |
| AXIS without UNIT | warning text and behaviour differ (io.cpp:2758-2762) | `wkt_import.rs`, `build_axis` |

## Commands

```bash
cargo test --test cli_yaml            # ledger: 32 projinfo rows
cargo test --test test_wkt1_syntax    # 10 tests, expected texts from projinfo 9.9.0 stderr
cargo test --test cli_projinfo_text   # 3 tests: summary bytes, grid suffix, WKT1 blank line
cargo test --test test_io --test test_io_wkt1 --test test_io_wkt2 --test test_c_api
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<oracle bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```
