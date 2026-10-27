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
result. Lots 8, 12, 13 and 14 build that surface.

## Yardstick: PROJ's own test suite

proj-rs works against PROJ's suite, not against tests written for it.
PROJ 9.4 describes its behavior in `test/gie/*.gie`: declarative files
(`operation +proj=…`, `accept`, `expect`, `tolerance`, `roundtrip`,
`expect failure errno …`) that `gie` runs. They are vendored unchanged under
`tests/gie/` (MIT, with PROJ's notice) and run by a Rust runner.

- Every `operation` proj-rs cannot build yet counts as **skipped**, never as
  passed. The runner prints passed / failed / skipped per file.
- **Ratchet.** `tests/gie/passing.txt` holds the number of passing cases.
  CI fails if the count drops, and also if it rises without the file being
  updated, so every lot commits its progress.
- **Failed is not skipped.** A case whose operation builds but whose result is
  off is a failure and blocks the PR. Tolerances are the ones in the `.gie`
  file, never loosened.
- `bench/record.sh` also writes the pass count to `bench/history.jsonl`, so
  speed and coverage are tracked together.
- Not ported: `test/unit` (gtest on the C API, which proj-rs does not have;
  the behavior it checks is mostly also in the `.gie` files) and `test/cli`
  (the `cs2cs` / `cct` executables, out of scope).

After lot 9, the next lots are picked from the runner's output: the
`operation` with the most skipped cases goes first, one projection or
conversion per lot, until `builtins.gie` passes in full.

## Next, in order

| Lot | Topic | Source | Why here |
|---|---|---|---|
| 8 | PROJ string parser | PROJ `src/init.cpp`, `src/param.cpp` | `+proj=utm +zone=31 +ellps=WGS84` → a ported operation. Unknown `+proj` or parameter is a creation error. Every `.gie` case and `Proj::new` start with it. |
| 9 | `gie` runner and vendored PROJ 9.4 suite | PROJ `test/gie/`, `src/apps/gie.cpp` | The yardstick above, with its ratchet. Lands before any new projection so each later lot is measured by it. |
| 10 | Geodetic ↔ geocentric cartesian | PROJ `src/conversions/cart.cpp` | Helmert (lot 7) works on X/Y/Z. Without `cart` it cannot transform a lon/lat point. |
| 11 | libproj baseline in the bench | `proj_trans_generic` via FFI, bench binary only | Times PROJ on the same grids, so each case reads as a ratio to PROJ, not just a number. |
| 12 | Pipeline engine (`+proj=pipeline`, `+inv`, `+step`) | PROJ `src/pipeline.cpp` | Introduces the shared operation trait. `forward` takes a `Geographic` there, which removes the two bare `f64` and the lon/lat swap risk. Unlocks many `.gie` cases. |
| 13 | georust `proj` facade | georust `proj` 0.31 public API | `Proj`, `ProjBuilder`, `Coord`, `Transform`, errors, `geo-types` feature, same signatures. Its own tests and examples run against proj-rs in CI, and a differential test compares both crates on Linux. `ProjBuilder` network and grid methods return an error. |
| 14 | Built-in CRS table for `new_known_crs` | EPSG registry, ported CRSs only | EPSG:4326, 4258, 4978, 2154, 3857, 326xx and 327xx, with the axis order `proj` gives after `proj_normalize_for_visualization` (lon, lat). No SQLite database. |
| 15 | Approximate Transverse Mercator (Evenden / Snyder) | `approx` branch of `tmerc.cpp` | Also the sphere path that lot 2 refuses. |
| 16 | Lambert Azimuthal Equal Area | `src/projections/laea.cpp` | |
| 17 | Portable batch path | AVX2 batch without glibc libmvec | Today the AVX2 path only builds on Linux glibc. Windows and macOS run the scalar loop. |
| 18+ | The rest of PROJ, one operation per lot | `test/gie/*.gie`, most skipped first | Ordered by the runner, not by this table. |

Lots 15 and later are open to the community. Claim one in an issue before
opening the PR, stay on PROJ 9.4 behavior, ship a `docs/PORTING-*.md`, and
bump `tests/gie/passing.txt`.

Out of scope until a lot claims it explicitly: the full EPSG database (lot 13
only embeds the ported CRSs), NTv2 grids,
WKT / PROJJSON parsing, the remaining projections. A lot does not add them
"while we're at it".
