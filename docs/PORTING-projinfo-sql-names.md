# projinfo: SQL output and name search without `-k`

Port of the parts of `projinfo` that write SQL and that find an object by name,
from PROJ 9.9.0: `projinfo_lib.cpp` (`buildObject`, `outputObject`, the `SQL`
and `--output-id` option handling), `iso19111/io.cpp` (`createFromUserInput`,
its name search `searchObject`) and `iso19111/factory.cpp`
(`getInsertStatementsFor`, `identifyOrInsert`, `identifyOrInsertUsages`,
`findFreeCode`, `formatStatement`). The PROJ string import for a projected CRS
is from `iso19111/io.cpp` (`PROJStringParser::createFromPROJString`).

Cases closed in `tests/cli_yaml/test_projinfo.yaml`: 6, 7 (`-o SQL` with
`--output-id`), 46 (`D_WGS_1984`), 67 (`--dump-db-structure --output-id`), and
109 (`EGM2008`).

## API mapping

| PROJ (C++) | Rust |
|---|---|
| `DatabaseContext::getInsertStatementsFor` (CRS, ellipsoid, prime meridian, datum) | `insert_statements` (`src/iso19111/insert.rs`), with `Writer::crs`, `geodetic`, `projected`, `ellipsoid`, `prime_meridian`, `geodetic_datum` |
| `identifyOrInsert(UnitOfMeasure)` | `Writer::unit` |
| `identifyOrInsert(CoordinateSystem)` | `Writer::coordinate_system`, `same_system` |
| `identifyFromNameOrCode` | `Writer::existing` |
| `identifyOrInsertUsages` | `Writer::usages` |
| `findFreeCode` | `Writer::free_code` |
| `getInsertStatementsFor(Conversion)`, reached from the projected CRS path | `Writer::conversion` |
| `formatStatement` (`%q`, `%Q`, `%f`, `%s`, `%d`) | `quote`, `number` (`toString(double)`, via `format_g`), and `format!` |
| `DatabaseContext::text_rows` (new, `pub(crate)`) | the `query` helper of `factory.rs`, read as text |
| `PROJStringParser` for `+proj=` naming a projected method | `projected_from_proj_string` (`src/iso19111/projstring.rs`) |
| `create_from_user_input` PROJ string branch | `create_from_user_input` (`src/iso19111/json.rs`) |
| `buildObject`, name search through `searchObject` | `create_default`, `search_default`, `settle`, `named_object` (`src/bin/projinfo.rs`) |
| `outputObject`, SQL block | `Dialect::Sql`, `sql_text`, `render` |

Public items: `insert_statements` (re-exported at the crate root). The
`Dialect` enum and `Options` are private to the binary.

## Test mapping

| Yaml case | Rust side |
|---|---|
| `test_projinfo.yaml#6` `+proj=merc ... --output-id HOBU:MY_CRS -o SQL -q` | `projected_from_proj_string`, `insert_statements`, `Writer::conversion` |
| `#7` the same with `--authority HOBU` | `Writer::geodetic_datum`, `Writer::ellipsoid`, `Writer::prime_meridian` |
| `#46` `D_WGS_1984` | `search_default` (CRS pass, then the other types) |
| `#67` `--dump-db-structure --output-id HOBU:XXXX EPSG:4326` | `run` (implicit SQL and quiet), `insert_statements` |
| `#109` `EGM2008` | `search_default` (approximate pass, `Datum` table) |

No Rust unit test covers the SQL writer yet. The yaml cases above are the test.

## Deliberate differences

- The writer starts from the database as it is. PROJ also sees the rows it
  inserted earlier in the same session, through its in-memory database. So a
  scope or extent that an earlier insert created is inserted again, not reused.
- A datum ensemble that the database does not know is an error. The insert of
  an ensemble is not ported (`projinfo -o SQL` on `EPSG:4326` with `--authority`
  errors).
- A vertical CRS, a compound CRS, a bound CRS, and a derived CRS are errors.
  `Error when exporting to SQL: ...` goes to stderr and the SQL block is
  dropped. PROJ writes them.
- `Identifier` and `CoordinateSystem` matching for `identify` uses the
  database only.
- `--output-id` values are split on `:` and must give two parts, as in PROJ.
- PROJ strings: only `+proj=longlat` and the projected methods of
  `parammap.rs` whose PROJ parameters cover every key are imported. `+proj=utm`
  is not (PROJ turns it into a Transverse Mercator named `UTM zone NN`).
  Unknown keys are an error, not ignored.
- A parameter with a DB unit `9102` is written as `9102`. PROJ writes `9122`
  (the canonical degree) for the same parameter of `EPSG:2154`. Not fixed here.
- A name that matches nothing: `input string: parsing of 'X' failed:
  unrecognized format / unknown name`, as PROJ prints it. With `-k crs`, PROJ
  prints nothing and exits 0; this port exits 1 with the same message.
- Error messages go to stderr with no `projinfo: ` prefix, as `projinfo_lib.cpp`
  writes them (`strm.cerr`). Before this lot, `main` added the prefix.

## Quirks kept

- The usage name for a derived usage code is built by appending `_N` for each
  domain to the same string, so a second domain's code carries both suffixes.
  This is `identifyOrInsertUsages`, kept as written.
- The name search of the default path uses the first match whose name is
  equivalent when several match, and the 2D then the 3D geographic CRS first.
  PROJ's second equivalence pass (bigger differences allowed) is not ported.

## Verification

- `cargo test --test cli_yaml` with the reference on `PATH`: projinfo open
  cases 65 → 60, no regression reported by the runner.
- Differential runs against `proj-ref/bin/projinfo` on `EPSG:4326`,
  `EPSG:4979`, `EPSG:4269`, `EPSG:4978`, `EPSG:2154`, `EPSG:32631`,
  `EPSG:3855`, and four PROJ strings (`longlat` with `+datum`, `+ellps`,
  `+a`/`+rf`, and `tmerc`). Without `--authority`, all match except
  `EPSG:2154` (unit `9102`), `EPSG:3855` (vertical, not ported), and
  `+proj=utm` (not imported). With `--authority HOBU`, the PROJ strings and
  `EPSG:4979` and `EPSG:32631` differ only where the datum or ensemble is
  inserted, and `EPSG:4269` differs in the CRS-level extent (see deliberate
  differences).
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
