# projinfo

The binary is the Cargo target `projinfo` (`src/bin/projinfo.rs`). It reads the
embedded `data/proj.db` through the library's ISO 19111 layer.

## How it describes an object

`projinfo <object>` calls `create_from_user_input` (the same entry point as
`projinfo_lib.cpp`), then renders the requested formats with the library:

| `-o` value | Library call |
|---|---|
| default, `PROJ` | `Crs::export_to_proj_string` |
| `WKT2:2019`, `WKT2_2019` | `Crs::export_to_wkt` with `WktConvention::Wkt2_2019` |
| `WKT2:2015`, `WKT2_2015` | `Crs::export_to_wkt` with `WktConvention::Wkt2_2015` |
| `WKT1:GDAL`, `WKT1:ESRI` | `Crs::export_to_wkt` with the WKT1 conventions |
| `PROJJSON` | `Crs::export_to_json` |
| `all` | every format above, in PROJ's order |

A deprecated object prints `Warning: object is deprecated` first.
`-o SQL` and the simplified WKT forms exit 1 with `cannot write dialect ...`.

## Library fixes this lot needed (`src/iso19111/`)

- `Crs::export_to_proj_string` (`projstring.rs`): one entry point for geographic,
  geocentric, and projected CRS. It did not exist for the `Crs` enum.
- `ENSEMBLEACCURACY` and the PROJJSON `accuracy` are the text stored in `proj.db`:
  `2.0`, not `2`. The text is read with `CAST(... AS TEXT)`, because the
  column is REAL and a Rust `f64` drops the `.0`. A built ensemble keeps the
  number formatting (`100`), as PROJ does for a number it was given.
- PROJJSON ensembles: members are `name` and `id` only; the ensemble carries an
  `ellipsoid` without an `id`; `accuracy` is a string; the axis `unit` is a name
  for `degree` and `metre`, an object otherwise; a CRS writes its first usage as
  `scope`, `area`, and `bbox`.

## Covered by the yaml suite

`tests/cli_yaml/test_projinfo.yaml`: 47 of 111 cases match
(`docs/UNPORTED-CLI.md` lists the other 65, with the reason for each).
The list, kind, completion, database-structure, search-path and remote-data
options are in `docs/PORTING-projinfo-lists.md`.

## Not built yet, by family

- **Name lookup**: `NAD27`, `WGS 84 + EGM96 height`, `KKJ`, `GDA94`, and other
  object names. `create_from_user_input` only takes codes, PROJ strings, WKT,
  and PROJJSON. PROJ also searches the names.
- **Operation search** (`-s` / `-t`): the candidate list and its per-operation
  output. The factory exists (`CoordinateOperationFactory::create_operations`);
  the formatting, the `--spatial-test`, `--area`, `--bbox`, and `--summary`
  options, and PROJ's ordering are not all matched yet.
- **Operation names and `-k operation`**: transformations and concatenated operations now have a PROJ string (`docs/PORTING-transformation-export.md`). `-k operation` is still not parsed, and the names are not searched.
- **Implicit compounds** written with `+` (`EPSG:4326+3855`).
- **`-o SQL`** and `--dump-db-structure <object>`: SQL export is not ported.
- **WKT1 input** without a `PRIMEM` node (`GEOGCS should have a PRIMEM node`).

## Preserved quirks

- `projinfo` writes exit 1 with the usage text on an unknown option, as PROJ does.
- `Too many parameters: <arg>` names the third positional argument.
