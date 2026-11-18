# Changelog

Versions follow [semver](https://semver.org). The stable surface is the one of
[docs/API-v1.md](docs/API-v1.md): the native facade, `proj-rs-georust` and the C API. The
rest is outside semver.

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
