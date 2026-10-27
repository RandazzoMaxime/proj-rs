# Roadmap

Each row is a lot: one pull request, one topic, one `docs/PORTING-*.md` sheet
when the lot translates C or C++. Lots merge in table order. Only the
maintainer merges (see [CONTRIBUTING.md](CONTRIBUTING.md)).

Every lot adds its cases to the progress bench (`src/bin/bench.rs`) and
appends a run to `bench/history.jsonl` with `bench/record.sh`. A lot that
slows an existing case says so in its PR.

## Landed (PR #1)

| Lot | Topic | PROJ 9.4 source |
|---|---|---|
| 0 | Governance, CI, lints | — |
| 1 | Ellipsoid, errors, longitude reduction | `a`, `rf` parameters |
| 2 | Exact Transverse Mercator and UTM | `src/projections/tmerc.cpp` (`PODER_ENGSAGER`) |
| 3 | Lambert Conformal Conic and Lambert-93 | `src/projections/lcc.cpp`, `tsfn.cpp`, `phi2.cpp` |
| 4 | AVX2 batch Transverse Mercator, progress bench | `tmerc.cpp`, 4 points per call |
| 5 | AVX2 batch Lambert-93 | `lcc_e_forward`, 4 points per call |
| 6 | Mercator and Web Mercator | `src/projections/merc.cpp` |
| 7 | 7-parameter Helmert | `+proj=helmert` |

## Goal: drop-in for `georust/proj` users

A project that uses the [`proj`](https://github.com/georust/proj) crate (the
libproj bindings) switches by changing one line of `Cargo.toml`, and no Rust
code:

```toml
proj = { package = "proj-rs", git = "https://github.com/RandazzoMaxime/proj-rs" }
```

So the crate exposes the same surface as `proj` 0.31 (libproj 9.6):
`Proj::new`, `Proj::new_known_crs`, `convert`, `project`, `convert_array`,
`project_array`, the `Coord` trait, the `Transform` trait and the `geo-types`
feature, `ProjBuilder`, `ProjError`, `ProjCreateError`, `Area`. What
proj-rs cannot do yet returns an error at creation time, never a wrong
result. Lots 10 to 13 build that surface.

## Next, in order

| Lot | Topic | Source | Why here |
|---|---|---|---|
| 8 | Geodetic ↔ geocentric cartesian | PROJ `src/conversions/cart.cpp` | Helmert (lot 7) works on X/Y/Z. Without `cart` it cannot transform a lon/lat point. |
| 9 | libproj baseline in the bench | `proj_trans_generic` via FFI, bench binary only | Times PROJ on the same grids, so each case reads as a ratio to PROJ, not just a number. |
| 10 | PROJ string parser | PROJ `src/init.cpp`, `src/param.cpp` | `+proj=utm +zone=31 +ellps=WGS84` → a ported operation. Unknown `+proj` or parameter is a creation error. Input of `Proj::new`. |
| 11 | Pipeline engine (`+proj=pipeline`, `+inv`, `+step`) | PROJ `src/pipeline.cpp` | Introduces the shared operation trait. `forward` takes a `Geographic` there, which removes the two bare `f64` and the lon/lat swap risk. |
| 12 | georust `proj` facade | georust `proj` 0.31 public API | `Proj`, `ProjBuilder`, `Coord`, `Transform`, errors, `geo-types` feature, same signatures. Its own tests and examples run against proj-rs in CI, and a differential test compares both crates on Linux. `ProjBuilder` network and grid methods return an error. |
| 13 | Built-in CRS table for `new_known_crs` | EPSG registry, ported CRSs only | EPSG:4326, 4258, 4978, 2154, 3857, 326xx and 327xx, with the axis order `proj` gives after `proj_normalize_for_visualization` (lon, lat). No SQLite database. |
| 14 | Approximate Transverse Mercator (Evenden / Snyder) | `approx` branch of `tmerc.cpp` | Also the sphere path that lot 2 refuses. |
| 15 | Lambert Azimuthal Equal Area | `src/projections/laea.cpp` | |
| 16 | Portable batch path | AVX2 batch without glibc libmvec | Today the AVX2 path only builds on Linux glibc. Windows and macOS run the scalar loop. |

Lots 14 to 16 are open to the community. Claim one in an issue before
opening the PR, stay on PROJ 9.4 behavior, and ship a `docs/PORTING-*.md`.

Out of scope until a lot claims it explicitly: the full EPSG database (lot 13
only embeds the ported CRSs), NTv2 grids,
WKT / PROJJSON parsing, the remaining projections. A lot does not add them
"while we're at it".
