# Roadmap

The reference is the latest PROJ release, **PROJ 9.9.0**. A new PROJ release
opens a lot that diffs the ported sources and moves the reference again.

Each row is a lot: one pull request, one topic, one `docs/PORTING-*.md` sheet
when the lot translates C or C++. Only the maintainer merges (see
[CONTRIBUTING.md](CONTRIBUTING.md)).

Every lot adds its cases to the progress bench (`src/bin/bench.rs`) and
appends a run to `bench/history.jsonl` with `bench/record.sh`. A lot that
slows an existing case says so in its PR.

✅ done and verified &nbsp;·&nbsp; 🟡 done with a known gap &nbsp;·&nbsp; ⬜ open

## Landed

| Lot | Topic | | Notes |
|---|---|:-:|---|
| 0 | Governance, CI, lints | ✅ | |
| 1 | Ellipsoid, errors, longitude reduction | ✅ | |
| 2 | Exact Transverse Mercator and UTM | ✅ | `tmerc.cpp`, `PODER_ENGSAGER` |
| 3 | Lambert Conformal Conic and Lambert-93 | ✅ | `lcc.cpp`, `tsfn.cpp`, `phi2.cpp` |
| 4 | AVX2 batch Transverse Mercator, progress bench | ✅ | |
| 5 | AVX2 batch Lambert-93 | ✅ | |
| 6 | Mercator and Web Mercator | ✅ | |
| 7 | 7-parameter Helmert | ✅ | |
| 8 | Move the reference to PROJ 9.9 | 🟡 | Behavior is checked against the PROJ 9.9.0 `gie` suite and the CLIs report 9.9.0. `PORTING-tmerc`, `-lcc`, `-merc` and `-helmert` still cite PROJ 9.4 |
| 9 | PROJ string parser | ✅ | `Operation::create` in `src/engine.rs`. An unknown `+proj` is a creation error |
| 10 | `gie` runner, vendored PROJ 9.9 suite, ratchet | ✅ | `tests/gie_run.rs`, `tests/gie/passing.txt` |
| 11 | Geodetic ↔ geocentric cartesian (`cart`) | ✅ | |
| 12 | libproj baseline in the bench | 🟡 | `build.rs` links libproj only when an import library sits at one fixed Windows path, so the `ratio` column is not reproducible elsewhere |
| 13 | Pipeline engine (`+proj=pipeline`, `+inv`, `+step`) | ✅ | |
| 14 | georust `proj` facade | 🟡 | The API surface and `geo-types` feature exist (`src/proj.rs`, `tests/proj_facade.rs`). georust's own tests and examples are not yet run against it, and no differential test compares the two crates |
| 15 | Built-in CRS table for `new_known_crs` | ✅ | Replaced by something larger: the full PROJ 9.9 `proj.db` is embedded and opened with `rusqlite` |
| 16 | Approximate Transverse Mercator | ✅ | `src/tmerc_approx.rs` |
| 17 | Lambert Azimuthal Equal Area | ✅ | `src/projections/laea.rs` |

Landed outside the lot sequence, in the PROJ 9.9 port: the rest of the
projection catalogue (**189 operations** build and run), grid shifts, `tinshift`,
deformation models, the ISO 19111 model with WKT import and export, on-demand
grid download, and the `cct`, `cs2cs`, `proj` and `projinfo` binaries.

## What is left

| Order | Topic | Where it stands |
|---|---|---|
| 1 | **A fresh checkout that is green** | Done for `gie`: 6,856 / 6,856 with `PROJ_NETWORK=ON`, 6,811 / 6,811 offline. CI checks against PROJ 9.9.0 from conda-forge. The unit and CLI suites are still open, see 2 and 3 |
| 2 | **ISO 19111 and the PROJ unit tests** | 243 ported, **1,136** named `TEST()`s open (2026-10-28). Largest: `test_io` (419), `test_operationfactory` (274), `test_operation` (144), `test_crs` (120), `test_c_api` (109). List: [`docs/UNPORTED-UNIT.md`](docs/UNPORTED-UNIT.md) |
| 3 | **CLI parity** | 155 of 276 PROJ CLI cases do not match yet: `projinfo` 96, `cs2cs` 57, `proj` 2. In `cs2cs` the biggest groups are CRS names that do not resolve and numeric differences; `+proj=sch` is not ported. List: [`docs/UNPORTED-CLI.md`](docs/UNPORTED-CLI.md) |
| 4 | **Portable batch path** | The AVX2 path covers x86-64 on Linux glibc (libmvec) and Windows (CRT). Other targets run the scalar loop |
| 5 | **georust facade parity** | Run georust's own tests against the facade in CI, add the differential test, then finish the drop-in claim |
| 6 | **Grid cache in CI** | Done: the network step caches the CDN grids (about 81 MB, 10 files) in `actions/cache`. Re-run on a cache miss only |

Anything under "What is left" is open to the community. Claim it in an issue
before opening the PR, stay on PROJ 9.9 behavior, ship a `docs/PORTING-*.md`,
and update the matching ledger (`tests/gie/passing.txt`,
`docs/UNPORTED-UNIT.md` or `docs/UNPORTED-CLI.md`).

## Goal: drop-in for `georust/proj` users

A project that uses the [`proj`](https://github.com/georust/proj) crate (the
libproj bindings) switches by changing one line of `Cargo.toml`, and no Rust
code:

```toml
proj = { package = "proj-rs", git = "https://github.com/RandazzoMaxime/proj-rs" }
```

The crate exposes the same surface as `proj` 0.31 (built on libproj 9.6;
proj-rs follows PROJ 9.9 underneath): `Proj::new`, `Proj::new_known_crs`,
`convert`, `project`, `convert_array`, `project_array`, the `Coord` trait, the
`Transform` trait and the `geo-types` feature, `ProjBuilder`, `ProjError`,
`ProjCreateError`, `Area`. What proj-rs cannot do returns an error at creation
time, never a wrong result.

## Yardstick: PROJ's own test suite

proj-rs works against PROJ's suite, not against tests written for it.
PROJ 9.9 describes its behavior in `test/gie/*.gie`: declarative files
(`operation +proj=…`, `accept`, `expect`, `tolerance`, `roundtrip`,
`expect failure errno …`) that `gie` runs. They are vendored unchanged under
`tests/gie/` (MIT, with PROJ's notice) and run by a Rust runner.

- Every `operation` proj-rs cannot build counts as **skipped**, never as
  passed. The runner prints passed / failed / skipped per file.
- **Ratchet.** `tests/gie/passing.txt` holds the number of passing cases. The
  test fails if the count drops, and also if it rises without the file being
  updated, so every lot commits its progress.
- **Failed is not skipped.** A case whose operation builds but whose result is
  off is a failure and blocks the PR. Tolerances are the ones in the `.gie`
  file, never loosened.
- `bench/record.sh` also writes the pass count to `bench/history.jsonl`, so
  speed and coverage are tracked together.
- `builtins.gie` passes in full: 2,597 / 2,597.
- **Two modes.** Offline, the 3 files that need the CDN are not run, as PROJ's
  `ctest` does. With `PROJ_NETWORK=ON` all 26 files run. `passing.txt` holds
  one count per mode.

The PROJ C++ unit tests and CLI cases are tracked by name, in
`docs/UNPORTED-UNIT.md` and `docs/UNPORTED-CLI.md`. A case leaves its ledger
when it passes. For the CLI cases the test enforces it: a case that used to
pass and now fails makes `tests/cli_yaml.rs` fail.

A lot does not add anything "while we're at it". If it is not in the table, it
gets its own lot.
