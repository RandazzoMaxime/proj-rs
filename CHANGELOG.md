# Changelog

Versions follow [semver](https://semver.org). The stable surface is the one of
[docs/API-v1.md](docs/API-v1.md): the native facade, `proj-rs-georust` and the C API. The
rest is outside semver.

## Unreleased

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
