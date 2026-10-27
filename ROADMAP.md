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

## Next, in order

| Lot | Topic | PROJ source | Why here |
|---|---|---|---|
| 8 | Geodetic ↔ geocentric cartesian | `src/conversions/cart.cpp` | Helmert (lot 7) works on X/Y/Z. Without `cart` it cannot transform a lon/lat point. |
| 9 | libproj baseline in the bench | `proj_trans_generic` via FFI, bench binary only | Times PROJ on the same grids, so each case reads as a ratio to PROJ, not just a number. |
| 10 | Pipeline engine (`+proj=pipeline`, one step at a time) | `src/pipeline.cpp` | Introduces the shared operation trait. `forward` takes a `Geographic` there, which removes the two bare `f64` and the lon/lat swap risk. |
| 11 | Approximate Transverse Mercator (Evenden / Snyder) | `approx` branch of `tmerc.cpp` | Also the sphere path that lot 2 refuses. |
| 12 | Lambert Azimuthal Equal Area | `src/projections/laea.cpp` | |
| 13 | Portable batch path | AVX2 batch without glibc libmvec | Today the AVX2 path only builds on Linux glibc. Windows and macOS run the scalar loop. |

Lots 11 to 13 are open to the community. Claim one in an issue before
opening the PR, stay on PROJ 9.4 behavior, and ship a `docs/PORTING-*.md`.

Out of scope until a lot claims it explicitly: EPSG database, NTv2 grids,
WKT / PROJJSON parsing, the remaining projections. A lot does not add them
"while we're at it".
