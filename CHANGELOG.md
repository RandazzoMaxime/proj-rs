# Changelog

Versions follow [semver](https://semver.org). The stable surface is the one of
[docs/API-v1.md](docs/API-v1.md): the native facade, `proj-rs-georust` and the C API. The
rest is outside semver.

## Unreleased

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
