# projinfo

New files:

- `src/bin/projinfo.rs`
- `PORT-NOTES-projinfo.md`

The binary is the Cargo target `projinfo`. It reads the embedded `data/proj.db` (already in the tree; not a new file). `src/lib.rs` is unchanged.

Covered, for a geographic EPSG code the database can describe (checked against PROJ 9.9.0 `test/cli/test_projinfo.yaml` for EPSG:4326):

- `--version` prints `projinfo: Rel. 9.9.0, September 15th, 2026`
- no `-o`, or `-o default`: PROJ string and WKT2:2019
- `-o PROJ`
- `-o WKT2:2019` (also `WKT2_2019`, `WKT2-2019`, and the WKT2:2018 aliases)
- `-o PROJJSON`

`-o` values this port does not write (`WKT1:GDAL`, `WKT1:ESRI`, `WKT2:2015`, simplified WKT, `SQL`, `all`, `WKT_ALL`) exit 1 with `cannot write dialect ...`. An unknown `-o` token is an unrecognized value.

Not built: CRS pairs (`-s`/`-t`), name lookup, WKT1, WKT2:2015, SQL, operations, grids, completion, and non-geographic objects. A projected or vertical code is an error, not a guessed coordinate.

C++ read first: `src/apps/projinfo.cpp`, `src/apps/projinfo_lib.cpp`, and the WKT/JSON exporters under `src/iso19111/` (`crs.cpp`, `datum.cpp`, `coordinatesystem.cpp`, `common.cpp`, `io.cpp`, `factory.cpp`). EPSG:4326 still uses extent 1262 (`World.`) rather than the later country-list extent, as `AuthorityFactory` does.
