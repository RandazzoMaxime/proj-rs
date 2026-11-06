# cs2cs: axis order and format of the output

Lot `cs2cs-output-axes`. Three defects were briefed (ledger row `#129`, `EPSG:4326 OGC:CRS84`, and
the compound target of `#126`/`#127`). Checking the output against PROJ 9.9.0 over 52 pairs showed two
more: the export of a bare axis swap when the other CRS is compound (`OGC:CRS84 EPSG:4326+5782`), and
the reversed output `-s` (`2dN 45dE` for `2dE 45dN`). All five are fixed in code, one commit each.

The ledger `docs/UNPORTED-CLI.md` goes from 7 open `cs2cs` rows to 6: `#129` closes. `#126` now
prints the exact stdout of PROJ but keeps its stderr mismatch, so it stays open. No previously passing
case reopened.

| commit | defect | proof |
|---|---|---|
| `37ef49b` | the facade answers longitude first; a latitude-first target is printed unswapped (`#129`) | ledger `#129` closed |
| `3578b0b` | `EPSG:4326 OGC:CRS84`: the inverse of the bare axis swap is refused, and `OGC:CRS84` (a text code) is read as an unknown CRS | `tests/cli.rs`, `tests/lonlat_source_axis.rs`, unit tests |
| `8b1a88f` | a compound target (`EPSG:4326+5782`) is printed in DMS, where PROJ prints `%.2f` numbers | `tests/cli.rs`, unit test; `#126` stdout exact |
| `650b50d` | a bare axis swap is refused when the other side is a compound CRS (`OGC:CRS84 EPSG:4326+5782`) | `tests/lonlat_source_axis.rs` |
| `366378a` | `-s`: the hemisphere letters of a latitude-first target are on the wrong value | `tests/cli.rs`; oracle rows 3, 4 below |

## PROJ 9.9.0 behaviour, with file:line

- `src/apps/cs2cs.cpp:274-313` `instantiate_crs`: for a bound, geographic, geodetic CRS it sets
  `isLongLatCS` and `isLatFirst` from the first axis name (`latitude`, `longitude`). A compound CRS is
  none of these, so its side is neither longitude-latitude nor projected.
- `src/apps/cs2cs.cpp:843-861`: the source and the target go through `instantiate_crs`; the target
  gives `destIsLongLat` and `destIsLatLong`.
- `src/apps/cs2cs.cpp:1014-1021`: `informat = dmstor` only for a degree source; `oform = "%.2f"` when
  the target is not `destIsLongLat` and no `-f` is given.
- `src/apps/cs2cs.cpp:211-237` (in `process`): a DMS line is printed when `destIsLongLat && !oform`. With
  `destIsLatLong` the first value is latitude (N/S) unless `reverseout`, when the first value is the
  longitude (E/W), then latitude (N/S). Without `destIsLatLong` the letters follow the axes the same way.
- `src/iso19111/operation/singleoperation.cpp:3479-3500`: an axis-order change (EPSG:9843, 9844) writes
  `axisswap order=2,1`, and a `unitconvert` only when both CRS are geographic with different units. With
  no CRS pair it is the swap alone.
- `src/iso19111/io.cpp:8938`: `axisswap order=2,1` is its own inverse, so the inverse writes the same
  string once the `+inv` is dropped.
- `src/iso19111/operation/conversion.cpp:2573` `createAxisOrderReversal`: the registered conversion
  (EPSG:15498 for two axes) is the one the factory builds for `OGC:CRS84` against `EPSG:4326`.

## Port

- `src/bin/cs2cs.rs` `Catalog::layout` and `layout_of_type`: a side is read from `proj.db` through its
  CRS type, as `instantiate_crs` does. A geographic CRS is `DegreeLongLat`; a compound CRS is `Other`,
  whatever its horizontal part; its horizontal part keeps the axis order (`lat_first`).
- `compound_notation`: the `AUTH:CODE+VERTICAL` text is recognised before `first_auth`, which cut it at
  `+` and read the horizontal part as the whole CRS.
- `first_axis_north_or_south`: the axis table of the CRS's coordinate system, the first axis
  orientation north or south. This is `lat_or_northing_first` of the library (`select.rs:808`).
- `Catalog` codes are text (`cell_code`, `authority(auth, &str)`): `proj.db` stores `INTEGER_OR_TEXT`
  codes, and `OGC:CRS84` is text. SQLite compares a text parameter with an integer or a text column the
  same way, so `EPSG:4326` and `OGC:CRS84` are looked up alike.
- `FacadeAxes` (`Engine::Facade`): the facade reads and writes longitude or easting first
  (`src/proj_iso.rs`, the georust boundary). `swap_in` and `swap_out` put the CRS's own axis order
  around `convert_xyz`. `Engine::lat_first` reports `swap_out` for it.
- `render_point`: the match of `(lat_first, reverse_out)` gives each DMS value the letters of its axis.
- `src/iso19111/select.rs` `export` and `export_operation`: a bare axis-order change (9843, 9844) is
  written with `transform_export::axis_order_reversal`, as PROJ writes it. In `export`, with the
  caller's pair, an inverse is wrapped in an inversion the formatter cancels.

## Deviations from PROJ

1. The facade boundary. PROJ's `proj_trans` returns the target's own axis order, and `cs2cs` reads the
   source in its own order. The georust facade of this port is longitude first on both sides. `cs2cs`
   therefore swaps on the facade boundary using the CRS axis table (`swap_in`, `swap_out`), where PROJ
   has no such step. Pipelines (`Engine::Pipe`) and PROJ strings (`Engine::Pair`) need no swap, since
   their order is already the CRS's.
2. The axis test reads the `axis` table's orientation (`north` or `south`), not the axis name
   `latitude` of `instantiate_crs`. The two agree on every EPSG CRS checked (for example EPSG:4326 and
   EPSG:4979 north, EPSG:7034 east, EPSG:32661 `South along 180°E`).
3. A bare axis-order change exported without a CRS pair writes the swap alone, as PROJ does with null
   CRS. With the caller's pair it writes the unit step too, when the two ends are geographic with
   different units. No such pair occurs in the matrix.
4. `lat_or_northing_first` (`src/iso19111/select.rs:808`) reads the first axis of a geographic or
   projected CRS only. For a compound target it returns `false` where the horizontal part is
   latitude-first. This is not visible: `cs2cs` reads the flag only for DMS, and a compound target is
   never printed in DMS. It is left as an open item rather than changed here.

## Oracle comparison

PROJ 9.9.0 `cs2cs` (the reference build) against this port, the stdout text compared exactly, with the
exit code. Each row is one invocation with the input on stdin. The 52 rows cover: geographic 2D and 3D,
both axis orders; `OGC:CRS84` and `OGC:CRS83`; compound sources and targets; geocentric; projected
northing-first (UPS, EPSG:32661) and easting-first; PROJ-string targets after `+to`; and the
options `-r`, `-s`, `-f`, `-d`, `-W`, `-w`, `-E`, `--only-best=no`.

Result: 52 of 52 identical, all with exit code 0.

Before the fixes of this lot, six rows differed, each observed with the same command:

- row 8 (`#129`): `2dE\t49dN 0.000`;
- row 11 (`EPSG:4326 OGC:CRS84`): `10.00\t45.00 0.00`;
- row 17 (`EPSG:4326+5773 EPSG:4326+5782`): `39dE\t3dS 0.000`;
- row 21 (`OGC:CRS84 EPSG:4326+5782`): `*\t* inf`;
- rows 34 and 37 (`-s`): `2dN\t45dE 0.000` and `45dN\t2dE 0.000`.

| # | input | arguments | PROJ 9.9.0 `cs2cs` stdout | this port |
|---|---|---|---|---|
| 1 | `45 2 0` | `EPSG:4326 EPSG:4979` | `45dN\t2dE 0.000` | same |
| 2 | `45 2 0` | `EPSG:4326 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 3 | `45 2 0` | `EPSG:4979 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 4 | `45 2 0` | `EPSG:4326 EPSG:4269` | `45dN\t2dE 0.000` | same |
| 5 | `45 2 0` | `EPSG:4326 EPSG:7034` | `2dE\t45dN 0.000` | same |
| 6 | `2 45 0` | `EPSG:7034 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 7 | `2 45 0` | `EPSG:7034 EPSG:4979` | `45dN\t2dE 0.000` | same |
| 8 | `2 49 0` | `+proj=longlat +datum=WGS84 +to EPSG:4979` | `49dN\t2dE 0.000` | same |
| 9 | `2 49 0` | `+proj=longlat +datum=WGS84 +to EPSG:4326` | `49dN\t2dE 0.000` | same |
| 10 | `2 49 0` | `+proj=longlat +datum=WGS84 +to +proj=longlat +datum=NAD83` | `2dE\t49dN 0.000` | same |
| 11 | `10 45 0` | `EPSG:4326 OGC:CRS84` | `45dE\t10dN 0.000` | same |
| 12 | `2 45 0` | `OGC:CRS84 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 13 | `2 45 0` | `OGC:CRS84 EPSG:25831` | `421184.70\t4983436.77 0.00` | same |
| 14 | `2 45 0` | `OGC:CRS84 EPSG:4979` | `45dN\t2dE 0.000` | same |
| 15 | `10 45 0` | `EPSG:4326 OGC:CRS83` | `45dE\t10dN 0.000` | same |
| 16 | `2 45 0` | `OGC:CRS83 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 17 | `39 -3 0` | `EPSG:4326+5773 EPSG:4326+5782` | `39.00\t-3.00 0.00` | same |
| 18 | `45 2 0` | `EPSG:4326 EPSG:4326+5782` | `45.00\t2.00 0.00` | same |
| 19 | `45 2 0` | `EPSG:4326+5773 EPSG:4326` | `45dN\t2dE 0.000` | same |
| 20 | `4500000 300000 0` | `EPSG:25831 EPSG:4326+5782` | `2.25\t36.81 0.00` | same |
| 21 | `2 45 0` | `OGC:CRS84 EPSG:4326+5782` | `45.00\t2.00 0.00` | same |
| 22 | `4611000 1000000 4300000` | `EPSG:4978 EPSG:4326` | `42d32'10.274"N\t12d14'11.031"E 15268.581` | same |
| 23 | `45 2 0` | `EPSG:4326 EPSG:4978` | `4514838.88\t157661.65 4487348.41` | same |
| 24 | `4500000 300000 0` | `EPSG:25831 EPSG:4326` | `2d15'8.017"N\t36d48'44.695"E 0.000` | same |
| 25 | `300000 4500000 0` | `EPSG:25831 EPSG:4326` | `40d37'35.903"N\t0d38'7.149"E 0.000` | same |
| 26 | `45 2 0` | `EPSG:4326 EPSG:25831` | `421184.70\t4983436.77 0.00` | same |
| 27 | `45 2 0` | `EPSG:4326 EPSG:32661` | `-3256209.87\t2183550.89 0.00` | same |
| 28 | `2000000 2000000 0` | `EPSG:32661 EPSG:4326` | `90dN\t0dE 0.000` | same |
| 29 | `2 45 0` | `+proj=longlat +datum=WGS84 +to +proj=utm +zone=31 +datum=WGS84` | `421184.70\t4983436.77 0.00` | same |
| 30 | `45 2 0` | `EPSG:4326 +to +proj=utm +zone=31 +datum=WGS84` | `421184.70\t4983436.77 0.00` | same |
| 31 | `4500000 300000 0` | `EPSG:25831 +to +proj=longlat +datum=WGS84` | `36d48'44.695"E\t2d15'8.017"N 0.000` | same |
| 32 | `45 2 0` | `EPSG:4326 +to +proj=merc +ellps=WGS84` | `222638.98\t5591295.92 0.00` | same |
| 33 | `45 2 0` | `-r EPSG:4326 EPSG:4979` | `2dN\t45dE 0.000` | same |
| 34 | `45 2 0` | `-s EPSG:4326 EPSG:4979` | `2dE\t45dN 0.000` | same |
| 35 | `45 2 0` | `-s EPSG:4326 OGC:CRS84` | `45dN\t2dE 0.000` | same |
| 36 | `45 2 0` | `-r EPSG:4326 OGC:CRS84` | `45dE\t2dN 0.000` | same |
| 37 | `45 2 0` | `-s OGC:CRS84 EPSG:4326` | `45dE\t2dN 0.000` | same |
| 38 | `45 2 0` | `-f %.4f EPSG:4326 EPSG:4979` | `45.0000\t2.0000 0.0000` | same |
| 39 | `10 45 0` | `-f %.6f EPSG:4326 OGC:CRS84` | `45.000000\t10.000000 0.000000` | same |
| 40 | `45 2 0` | `-f %.6f EPSG:4326+5773 EPSG:4326+5782` | `45.000000\t2.000000 0.000000` | same |
| 41 | `39 -3 0` | `--only-best=no EPSG:4326+5773 EPSG:4326+5782` | `39.00\t-3.00 0.00` | same |
| 42 | `39 -3 0` | `-d 4 EPSG:4326+5773 EPSG:4326+5782` | `39.0000\t-3.0000 0.0000` | same |
| 43 | `45 2 0` | `-W5 EPSG:4326 EPSG:4979` | `45d00'00.00000"N\t2d00'00.00000"E 0.000` | same |
| 44 | `45 2 0` | `-w3 EPSG:4326 EPSG:4979` | `45dN\t2dE 0.000` | same |
| 45 | `10 45 0` | `-w3 EPSG:4326 OGC:CRS84` | `45dE\t10dN 0.000` | same |
| 46 | `45 2 0` | `-E EPSG:4326 EPSG:4979` | `45 2 0\t45dN\t2dE 0.000` | same |
| 47 | `2 45 0` | `OGC:CRS84 EPSG:4326+5773` | `45.00\t2.00 0.00` | same |
| 48 | `45 2 0` | `-s EPSG:4326 EPSG:4326+5782` | `2.00\t45.00 0.00` | same |
| 49 | `45 2 0` | `-s EPSG:4326+5773 EPSG:4326+5782` | `2.00\t45.00 0.00` | same |
| 50 | `4500000 300000 0` | `-f %.3f EPSG:25831 EPSG:4326+5773` | `2.252\t36.812 0.000` | same |
| 51 | `2 45 0` | `EPSG:4326+5773 OGC:CRS84` | `45dE\t2dN 0.000` | same |
| 52 | `45 2 0` | `-f %.6f EPSG:4326 EPSG:4269` | `45.000000\t2.000000 0.000000` | same |

Where the text differs from the numbers the ledger compares, the text is the criterion. The yaml
harness (`tests/cli_yaml.rs`, `compare_numbers`) reads numbers only, so the hemisphere letters and the
`%.2f` form of a compound target are not checked by a ledger row; `tests/cli.rs` pins them.

Commands (the matrix is a shell script that runs both binaries on each line of a case list; the 52
calls are these, with the input given on stdin):

```bash
# PROJ 9.9.0 (oracle): the reference build of cs2cs
PROJ_DATA=<scratch>/reftest/build/data:<worktree>/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  <scratch>/reftest/build/bin/cs2cs -s EPSG:4326 EPSG:4979 <<< "45 2 0"
# this port
PROJ_DATA=<worktree>/data:<worktree>/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs -s EPSG:4326 EPSG:4979 <<< "45 2 0"
# the path a pair takes (PROJ_RS_TRACE_PATH names the builder; `iso` hides an earlier refusal)
PROJ_RS_TRACE_PATH=1 ... target/debug/cs2cs OGC:CRS84 EPSG:4326+5782
# the ledger, the lot's tests, and the generated counts
CARGO_INCREMENTAL=0 cargo test --test cli_yaml
cargo test --bin cs2cs --test cli --test lonlat_source_axis
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
```

## Open items

1. `#126`: PROJ prints, before the result, the warning that `EPSG:4326+5773` to `EPSG:4326+5782`
   could not use the Inverse of WGS 84 to EGM96 height operation for lack of its grids, and the
   ballpark note. This port prints neither (the `Alternatives` path under the default policy). The
   stdout is exact; the stderr keeps the row open.
2. The other open `cs2cs` rows are untouched: `#71` and `#72` (SCH), `#96` (authority DMS precision),
   `#125` and `#139` (grids).
3. `+proj=longlat +datum=WGS84 +geoidgrids=@i_dont_exist.tif +vunits=m +no_defs +type=crs +to EPSG:4979`
   (the `#129` command): the ISO selection finds no operation (`cs2cs: path=facade iso=failed: no
   operation from ...`), and the facade answers. The output is right; the candidate search is the
   factory's, and this lot does not change it.
4. `lat_or_northing_first` (`src/iso19111/select.rs:808`) does not look into a compound CRS (deviation
   4). No output depends on it today.
5. An invocation without `+to` (`cs2cs EPSG:4326 +proj=merc +ellps=WGS84`) prints a value and exits 0;
   PROJ prints `missing target CRS and source CRS is not a projected CRS` and exits 3. This is argument
   parsing, not the output format, and is not changed here.

## Oracle check after rebase on main

25 `cs2cs` invocations compared as exact text with PROJ 9.9.0 (`tools/build-reference.sh`):
geographic 2D/3D, compound, geocentric, projected, `OGC:CRS84`, PROJ strings, `-r`, `-s`,
`-f`, `-w`. 24 identical. The one difference predates this lot (same output on `main`):
`EPSG:4326 EPSG:4807` on `49 2`, PROJ `49d0'0.26"N 0d20'11.363"W 0.000`, the port
`54.44 -0.37 0.00`. The target's grad unit is printed as plain numbers instead of being
converted to degrees and written in DMS. Open item.
