# Changelog

Versions follow [semver](https://semver.org). The stable surface is the one of
[docs/API-v1.md](docs/API-v1.md): the native facade, `proj-rs-georust` and the C API. The
rest is outside semver.

## Unreleased

- `+datum=` no longer hides `+R`, `+a`, `+b`, `+rf`, `+f`, `+e`, `+es` or an earlier `+ellps`.
  As in `pj_datum_set`, the datum's `ellps=` is appended at the end of the list, so an earlier
  `+ellps` wins and the size and shape keys override it: `+proj=merc +datum=WGS84 +R=6370000`
  is a sphere again (it was WGS 84, 23 km off at 45°N), and `+datum=WGS84 +a=6378137 +b=6356752.314`
  is the `unknown` datum PROJ 9.9.0 builds from the numbers, the SRS GMT writes for its grids
  (`grdproject/units`). Tests: `tests/datum_ellipsoid_override.rs`.
- R 4.5.0 with sf 1.1-3, terra 1.9-60 and lwgeom 0.2-17 runs as an eleventh real client
  (`tools/clients/r.sh`, `docs/CLIENTS-r.md`): the 25 test scripts of sf and terra's tinytest
  suite run once with the libproj of PROJ 9.9.0 and once with proj-rs, the dependencies pinned
  by a dated CRAN snapshot, and the script fails when the two passes differ. The 25 outputs of
  sf are identical and terra passes its 981 results on both sides; the passes still differ by
  the count of `Cannot open` lines in terra's log (95 against 154), since PROJ caches the
  answers of `lookForGridInfo` per context and proj-rs does not (`docs/CLIENTS-r.md`). Their paths through libproj found these gaps, each now compared with the reference libproj by
  `capi/tests/run.sh network_listing missing_input`, `tests/network_listing.rs` and a unit test
  of `src/crs.rs`:
  - the sort of `proj_create_operations` (`PROJ_GRID_AVAILABILITY_USED_FOR_SORTING`) took a CDN
    grid as available from its `proj.db` row, where PROJ opens it (`lookForGridInfo`): with the
    network on and no grid, `EPSG:4326` to `EPSG:5070` under `PJ_CRS_EXTENT_NONE` (terra's
    `proj_pipelines` default) listed grid operations ahead of the Helmerts of Hawaii and Alaska.
    `gridpath::available` now opens; the building of a pair, of a pipeline and of an
    instantiability mark uses `gridpath::known_available`, which does not, so
    `proj_create_crs_to_crs` still attempts no network open, as PROJ;
  - the `Cannot open <url>: <error>` line of a failed network open went nowhere: it now reaches
    the logger of the context of the call, once per open (47 and 49 lines for the two listings,
    as PROJ), through a per-thread channel the C library points at its context for the time of
    each entry point (`src/log_channel.rs`); it is not sent from inside the instantiation of a
    PROJ string by `proj_create`, where PROJ keeps only the last error line, nor twice when a
    failed point is replayed to find its message. A point whose grid does not open already gave
    infinity and `PROJ_ERR_OTHER_NETWORK_ERROR`; it now logs that line too;
  - `proj_list_get_count(NULL)` returned -1 instead of 0;
  - the NULL and wrongly typed arguments of `proj_create_projected_crs`,
    `proj_create_derived_projected_crs`, `proj_create_derived_geographic_crs`,
    `proj_create_geographic_crs_from_datum`, `proj_create_geocentric_crs_from_datum`,
    `proj_crs_create_bound_vertical_crs` and `proj_create_operations` give PROJ's object, errno
    and line (35 calls): `missing required input` with `PROJ_ERR_OTHER_API_MISUSE` for a NULL
    where PROJ says so, `Missing input datum_or_datum_ensemble` for the two datum constructors,
    NULL with nothing logged for a wrong type where PROJ logs nothing, and PROJ's own text
    otherwise (`vert_crs is not a VerticalCRS`, `source_crs is not a CRS or a
    CoordinateMetadata`...).
- GMT 6.5.0 runs as a tenth real client (`tools/clients/gmt.sh`, `docs/CLIENTS-gmt.md`): its 1180
  test scripts run once with the libproj of PROJ 9.9.0 and once with proj-rs. GMT reaches libproj
  through GDAL alone (`-J+proj=...` becomes an `OGRCreateCoordinateTransformation`, GeoTIFF SRS,
  raster extents); the two passes give the same verdict for each test and 0 file that differs out
  of the 3894 written, the ~1000 PostScript compared rendered at the RMS limit of GMT's own
  harness.
- Memory: the read-only `proj.db` embedded in proj-rs has no triggers (`data/proj-triggers.sql` holds
  them and writable copies replay them). Creating a transform peaks at about 2.5 MiB less, and starts
  faster. The embedded database is stored compressed, page by page (`data/proj.pages`): the binary
  is 7.6 MB smaller, creation takes about 9 ms more once per process, the conversions do not change.
  See [docs/PORTING-memory.md](docs/PORTING-memory.md).
- x86-64 has an AVX-512 batch path (8 lanes of `f64`, `src/simd/v8.rs`) behind the new `avx512`
  cargo feature (off by default; also relayed by `proj-rs-georust`). On an AVX-512 CPU,
  `batch_path()` gives `avx512` and every batch kernel runs 8 points at a time: Transverse
  Mercator/UTM, Lambert conic, Mercator/Web Mercator, `laea`, `somerc`, `sterea`, `cart` and
  Helmert, forward and inverse. The code is generic over the lane count (`Vector<N>` in
  `src/simd/vec.rs`), so one formula runs on AVX2 (4 lanes), AVX-512 (8) and NEON with the same
  bits per lane; only `asinh`/`atanh` differ between 4 and 8 lanes (their "all lanes small"
  branch depends on the group), within the same ULP bounds. The feature needs Rust >= 1.89 (the
  AVX-512 types of `std::arch`; `pulp/x86-v4` does not build below), so the MSRV (1.83) build
  never enables it and the CI `msrv` job lists the features explicitly without `avx512`. Measured
  on a Xeon 6985P-C with `proj-rs-bench`: batch conversions are 1.1x to 1.8x faster than the AVX2
  path (e.g. UTM 31 forward batch 19.3 -> 10.8 ns/point), one-point calls unchanged. Tests:
  digests per path in `src/simd/math.rs` and `tests/simd_bits.rs`/`tests/convert_array_golden.rs`
  (a `GOLDEN_AVX512` table), batch-vs-scalar in `tests/simd_batch.rs` over all lane positions of
  a group. See [docs/PORTING-simd.md](docs/PORTING-simd.md).

## 1.2.5

- aarch64 (Apple silicon, Linux ARM) has a batch path in lanes of four with NEON, as x86-64 has
  with AVX2: `batch_path()` gives `neon`. `V4` has a second realisation on two `float64x2_t`
  registers with the same API, built on the safe wrappers of `pulp`, so the vector math functions,
  the kernels and the formulas of the projections are the same code on both architectures and give
  the same bits (digests measured on AVX2 for the 11 functions of `math.rs`, checked on Apple
  silicon and Linux ARM, and for equatorial `laea`, `somerc`, `sterea`, `cart` and `helmert`,
  forward and inverse, checked on Linux ARM). Batches run 1.5 to 4 times faster than the portable
  path on an Apple M1 and a Neoverse N2. There is no `libmvec` and no Accelerate. The CI runs the whole suite on Linux ARM as well as on Apple silicon. See
  [docs/PORTING-simd.md](docs/PORTING-simd.md).

## 1.2.0

- Six more real clients run their own test suites once on libproj 9.9.0 and once on proj-rs, the
  scripts in `tools/clients/` and the results in `docs/CLIENTS-<name>.md`: SpatiaLite 5.1.0
  (104 of 108 tests on both sides, 0 logs that differ), georust/proj 0.31.0 (26 of 26 tests and 24
  of 24 examples on the C library; 24 of 26 on the facade, the two others explained),
  libgeotiff 1.7.4 (0 lines of `listgeo` and `geotifcp` that differ), PDAL 2.10.2 (140 of 142,
  the same two tests that read remote files fail on both), PostGIS 3.6.4 (247 of 247) and GRASS
  8.5.0 (the same result for each of 818 tests). With pyproj, QGIS and GDAL of 1.1.0 that makes
  nine.
- `proj_create` and `proj_create_from_wkt` follow PROJ more closely:
  - an operation or a CRS that `pj_init` refuses (`k <= 0`, `|lat_0| > 90`) is refused at creation,
    with PROJ's error code and text, and `proj_errno` is PROJ's (`capi/tests/run.sh`
    `errno_parity`);
  - a WKT that contains ` +proj=` is read as WKT, not as a PROJ string (PDAL);
  - `proj_create_from_wkt` adds the `validateParameters` warnings of PROJ, so GDAL caches the same
    CRS;
  - a `+proj=pipeline ... +type=crs` string that has no CRS shape is a PROJ-based operation, and
    `proj_pj_info` keeps an inverted one-step pipeline as a pipeline (GRASS);
  - a polar `stere` with `lat_ts` at the pole and `k != 1` is read as variant A (libgeotiff);
  - `proj_create_crs_to_crs` re-creates a side that fails with `proj_create` (SpatiaLite).
- The grammar errors of a WKT are PROJ's, byte for byte. The bison grammars of PROJ's WKT1 and WKT2
  parsers are ported (`tools/wkt-grammar/extract.py` writes the tables from the generated parsers
  of PROJ 9.9.0). On 21,342 WKT of the EPSG database and 64,703 malformed ones, there is no
  difference in the grammar errors. A grammar error that cuts a multi-byte character keeps PROJ's
  bytes in the list of errors, the log and the strict-mode failure (`WktParser::failure_bytes`,
  `projinfo::Report::warning_bytes`).
- CRS are read back as PROJ reads them: the axes of a polar `stere` or `laea` (`NORTH_POLE`,
  `SOUTH_POLE`), a CRS on the WGS 84 ensemble (`+datum=WGS84`), the orientation of a WKT1 Krovak,
  and the unit of a parameter in PROJJSON. The PROJ string of the WKT of all 13,719 EPSG CRS is
  libproj's.
- The 541 EPSG operations with a grid, read back from their WKT, export to a PROJ string: PROJ
  finds a parameter without an identifier by its name (`OperationParameter::getEPSGCode`), and so
  does `Parameter::epsg_code`. The 21,342 WKT of CRS and operations round-trip identically.
- A WKT1 projection that lacks a parameter is built as PROJ builds it: `UTM zone N` only with the
  four UTM parameters, and a polar stereographic with every parameter of its variant.
- Fixed: `projinfo` and the identification of a CRS panicked on a name with a multi-byte character
  (`Référence éé`), where a name was sliced at a byte index inside a character.

## 1.1.0

- The C library is tried against real clients, with their own test suites run once on libproj
  9.9.0 and once on proj-rs: pyproj 3.8.0 (`tools/clients/pyproj.sh`,
  [docs/CLIENTS-pyproj.md](docs/CLIENTS-pyproj.md)), and QGIS 3.34.4 and GDAL 3.13.3. Every pyproj
  test that passes on libproj passes; QGIS gives the same result test by test; GDAL's `osr` tests
  too, except three that point `PROJ_DATA` or the search path at another `proj.db`, which proj-rs
  embeds (a deliberate difference: [docs/PORTING-database-open.md](docs/PORTING-database-open.md)).
- About seventy fixes of the C API get there, each compared with libproj 9.9.0 by
  `capi/tests/run.sh` (33 comparisons): `proj_create` reads any object (URN, WKT and PROJJSON of
  operations, datums, ensembles, ellipsoids, coordinate systems) and every object exports to WKT
  and PROJJSON; a PROJ string without `+type=crs` is an operation; PROJ strings are cut and read as
  PROJ reads them (`pj_shrink` words, DMS angles, `+init=file:section`, `+init=AUTH:CODE` with other
  parameters, `+proj=latlon`, known ellipsoids, unread keys dropped); `proj_create_crs_to_crs`
  builds `ONLY_BEST`, `AUTHORITY`, the area of interest, two coordinate epochs and the `init=` rules
  of the context, and returns the single operation of a pair itself; `proj_pj_info` gives
  `pj_get_def`'s definition, in the authority axis order until normalized; `proj_trans` logs the
  `<step>: Invalid latitude` line of a refused point and keeps the error of PROJ's fallback; a
  PROJ-based operation has type 24; derived and temporal CRS, coordinate systems, grids and
  network availability answer as PROJ's do; WKT1 GDAL and ESRI details, CRS equivalence and
  `identify` follow PROJ.
- `set_allowed_intermediate_crs` no longer reads past the end of its array (it crashed pyproj).
- The C library archives carry `share/proj/proj.db`, for clients that look for the file (pyproj).

- The C library archives carry `lib/pkgconfig/proj.pc` and `lib/cmake/proj/proj-config.cmake`,
  as PROJ installs them: `pkg-config proj` and CMake's `find_package(PROJ 9 CONFIG)` (target
  `PROJ::proj`) find the library and report version 9.9.0. Both are relocatable.
- Two more release platforms: `linux-aarch64` (the tools and the C library, glibc 2.35) and
  `linux-x86_64-musl` (the tools only, one static executable for any Linux).
- The C library exports the geodesic API of PROJ: `geodesic.h` and its 21 `geod_*` functions
  (direct and inverse problems, lines, polygons), with the structures in the layout of PROJ 9.9.0.
  PROJ's `geodtest.c` passes against it. pyproj needs it to compile (`docs/CLIENTS-pyproj.md`).
- `Transformer` builds its candidates at their first use. A pair with several candidates on grids
  no longer reads every grid at `build()`: `EPSG:6364` to `EPSG:8801` with the parity grids builds
  in 0.55 s instead of 11.8 s in a dev build (the first transform then reads the grids it uses,
  3.4 s), and the peak memory of the build drops from 140 MB to 22 MB. On the eight points checked,
  the results are bit for bit those of the previous build, and the parity grids are unchanged. The
  release bench's median over its 214 cases is unchanged; one batch case moves by +3.9 % with no
  change to its code (see [docs/PORTING-transformer-build.md](docs/PORTING-transformer-build.md)).
- The AVX2 batch path has no `unsafe` left (57 gone from `src/simd/`): the intrinsics and the
  dispatch into code compiled with `avx2,fma` go through the safe API of `pulp` 0.22 (MIT, builds
  with Rust 1.83). Same instructions, same bits (`tests/simd_bits.rs`, 28 digests fixed before the
  change), and the batch cases of `proj-rs-bench` within -2.1 % to +1.3 % of the old code on their
  minimum: see [docs/PORTING-simd.md](docs/PORTING-simd.md).
- `proj-rs-bench` no longer calls libproj in-process (25 `unsafe` gone): `build.rs` compiles
  `src/bin/support/libproj_timer.c` against the libproj it finds, and the bench sends it the same
  points and reads back the same timings and images. Cases, names and history are unchanged; the
  libproj times agree with the old bench within the noise: see
  [docs/BENCHMARK.md](docs/BENCHMARK.md).
- No `unsafe` for the writability of `$HOME`: `access(2)` goes through `rustix::fs::access`, the
  same system call behind a safe API (Unix). The user directory is compared with PROJ's
  `projinfo` for a home without write permission, a home that is a file and a read-only mount:
  see [docs/PORTING-data-dirs.md](docs/PORTING-data-dirs.md).
- No `unsafe` left outside `capi/` for `atanh`: `projmath::atanh` no longer calls the C libm through
  `extern "C"`. Beyond the series domain it uses glibc's `__ieee754_atanh` formula over
  `f64::ln_1p`, bit for bit equal to glibc's `atanh` on 14 million inputs (edge cases included), and
  slightly faster (11.3 against 11.9 ns per call). On macOS and Windows it is no longer the
  platform's own `atanh`, within one ULP: see [docs/PORTING-simd.md](docs/PORTING-simd.md).
- `Transformer` is `Send + Sync`: several threads convert through one `&Transformer`, without a
  clone per thread. On the tested pairs and points, the results are bit for bit those of one thread.
  The state shared between points (the operation of the last point, and the pending warning of a
  failing candidate under the default `OnlyBest`) is atomic. Under several threads the order of the
  points is not defined: the documentation of `Transformer` and
  [docs/PORTING-transformer-sync.md](docs/PORTING-transformer-sync.md) say what that changes. Measured
  on one machine over 16 alternating series, `forward` and `forward_slice` stay within 1.7 % of the
  reference, on the median and the minimum. `clone` of a transformer with several candidates is about
  3 % slower on the minimum (4 % on the median); it is not a per-point path.
- Stable catalogue, database metadata and PROJ version on the native façade, so that an application
  no longer needs `proj_rs::unstable` for them. `Context::crs_catalog()` returns a `CrsCatalog`;
  `CrsCatalog::list(&CrsFilter)` lists the CRS of the database by `CatalogKind` (geographic 2D and
  3D, geocentric, projected, derived projected, vertical, compound, engineering), by authority, and
  with the deprecated ones only on `DeprecatedCrs::Include` (excluded by default). Each `CrsEntry`
  is one CRS and one of its areas of use, as `projinfo --list-crs` prints it: 17 EPSG CRS have two.
  `Context::database_metadata()` gives the layout version, `EPSG.VERSION`, `PROJ.VERSION` and
  `PROJ_DATA.VERSION` of `proj.db`; `proj_version()` gives the PROJ version ported (9.9.0). A
  database that cannot be read is `Error::Database`. The grid cache keeps `Cache`, already stable.
  The `unstable` module is unchanged. See [docs/API-v1.md](docs/API-v1.md) and
  `tests/stable_catalog.rs`.
- `Context::builder().hermetic()` builds a context that reads nothing from the machine: not
  `PROJ_NETWORK`, `PROJ_NETWORK_ENDPOINT` nor `PROJ_ONLY_BEST_DEFAULT`, no `proj.ini` wherever PROJ
  would look, no `HOME` for `~/` names, no working directory for a bare grid name. Its sources are
  its search path, its user directory and its own settings; the embedded `proj.db` is still used.
  Downloads and the chunk cache go only to the user directory the builder set. Open points:
  `PROJ_AUX_DB` and `PROJ_IGNORE_CELESTIAL_BODY` are still read by the database layer. See
  [docs/PORTING-context-hermetic.md](docs/PORTING-context-hermetic.md).
- A non-hermetic `Context` reads `proj.ini` from its search path and its user directory, as PROJ
  does once the search path is set, and not only from the default places.
- An unset `ONLY_BEST` (`OnlyBest::Default`) takes the context's default, as
  `proj_create_crs_to_crs` does: `PROJ_ONLY_BEST_DEFAULT=YES` (or `only_best_default = on` in
  `proj.ini`) makes a missing grid fail the transformer, as `cs2cs` 9.9.0 does. It was always the
  warning default.
- `Transformer` accepts a vertical CRS alone as source or target (`EPSG:4979` to `EPSG:3855` or
  `EPSG:5773`), as `proj_create_crs_to_crs` does: it refused the pair with `has no geodetic base to
  prepare its candidates`. Such a CRS has no horizontal coordinates, so its candidates are not
  reprojected, as for a geocentric CRS. With the grid the value is the one of `EPSG:4326+5773`;
  without it the ballpark operation is kept, as `cs2cs` 9.9.0 does.
- All of Karney's geodesic library is Rust, called by PROJ or not: the area `S12`, the polygons
  (`geod_polygon_*`, `geod_polygonarea`), `geod_inverseline`, `geod_directline`. The public
  `proj_rs::geodesic` module has `Geodesic::wgs84()`, `Geodesic::polygon_area(&[(lat, lon)])`
  returning the area and the perimeter, and `Polygon` for the incremental calls. Checked against the
  C library on 1 981 cases. See [docs/PORTING-geodesic.md](docs/PORTING-geodesic.md).
- `Transformer::last_used_operation()` names the single operation of a transformer (name,
  accuracy, ballpark flag) instead of returning it without a name, and `is_ballpark()` is true
  when a ballpark operation was the one chosen among several: it was always false. An application
  can refuse an approximate conversion by reading it after the conversion.
- `Transformer` finds the grids of its context's search path when one candidate operation is left
  (for instance `EPSG:4979` to `EPSG:4326+5773` with `Ballpark::Forbidden`): the operation was
  created outside the context and failed with `illegal value for +grids`.
- `Ballpark::Forbidden` no longer lets a ballpark operation through when the grid is missing: the
  `DiscardIfMissing` rerun re-added the operations the filter had dropped. The build now fails, as
  PROJ's `--no-ballpark` does.
- Minimum supported Rust version back to 1.83, the measured floor (it was declared 1.100 in
  1.0.0 because the code used newer standard-library methods; `src/compat.rs` now holds their
  equivalents). The release is still built with 1.100. A project on Rust 1.83 to 1.99 compiles
  the crate without `--ignore-rust-version`.
- The geodesics (`geod`, `invgeod`, the `aeqd` and `gnom` projections) are Rust: Charles Karney's C
  library is ported, and no longer compiled. `cargo build` needs a C compiler for SQLite only. The
  results are the same bits as the C code's on the same math library. See
  [docs/PORTING-geodesic.md](docs/PORTING-geodesic.md).
- `Proj::convert_array` and `Proj::convert_iter` convert fewer than 8 points one at a time instead
  of through the vector path, which costs more than it saves there: a lone point cost two to five
  times as much as `convert`, and now costs the same. From 8 points the vector path wins. The result
  is `convert`'s, bit for bit under 8. See
  [docs/PORTING-convert-iter.md](docs/PORTING-convert-iter.md).
- `Proj::convert_iter` converts the points of an iterator in blocks, through the vector path of
  `convert_array`: about twice as fast as `convert` one point at a time. The items are those of
  `convert`, errors included. See [docs/PORTING-convert-iter.md](docs/PORTING-convert-iter.md).

## 1.0.0

The first release. PROJ 9.9.0, ported to Rust.

- Native Rust API (`proj-rs`), drop-in for the georust `proj` 0.31 crate (`proj-rs-georust`) and
  the C API of `proj.h` (`proj-rs-capi`: 292 functions, as a replacement for libproj 9.9.0).
- Same results as PROJ 9.9.0 to 1 µm on linear coordinates and 1e-11 degree on angles, checked
  against PROJ's own `gie` suite, C++ unit tests and command-line cases.
- `proj.db` embedded: EPSG codes and the operation catalogue need no file. No TLS library
  unless the `network` feature is on.
- Command-line tools (`cct`, `cs2cs`, `proj`, `invproj`, `geod`, `invgeod`, `gie`, `projinfo`)
  in one executable, `proj-rs`.
- Release archives for Linux x86_64, macOS arm64 and x86_64, Windows x86_64: the tools, and the
  C library with its headers, with `SHA256SUMS`. See
  [docs/release-notes/v1.0.0.md](docs/release-notes/v1.0.0.md).
- Minimum supported Rust version: 1.100.
