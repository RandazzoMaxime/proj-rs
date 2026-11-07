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

The order follows the goal: a project that uses georust `proj` switches with
one line. What proves that comes first; matching PROJ's text output to the
letter comes last.

| Order | Topic | Where it stands |
|---|---|---|
| 1 | **A fresh checkout that is green** | Done for `gie`: 6,856 / 6,856 with `PROJ_NETWORK=ON`, 6,811 / 6,811 offline. CI checks against PROJ 9.9.0 from conda-forge |
| 2 | **georust facade parity** | <!--gen:georust_pass-->25<!--/gen--> of <!--gen:georust_total-->25<!--/gen--> is the generated count. The suite [`tests/georust_parity.rs`](tests/georust_parity.rs) passes with zero `#[ignore]`. `new_known_crs` and `proj_known_crs` choose the candidates as `proj_create_crs_to_crs` does (the default warning policy, the rerun without the grid operations) and transform with the candidate `proj_trans` picks for each point, both directions. A pair the selection does not write is an error at creation; there is no other path. They use the georust axis order. The facade matches the PROJ 9.9.0 values of the named pairs in `tests/iso_pipeline_oracle.rs`, `EPSG:4326` to `EPSG:5070` and `EPSG:4326` to `EPSG:2230` included. Done since: area of use, `def()` and `Debug` text, WKT and PROJJSON of a CRS and of the operation behind a pair, PROJJSON import of a projected CRS, compound names such as `EPSG:4326+3855` (a forward conversion to a compound target takes the height 0 of the null offset, as libproj does), one coordinate epoch on a transformation (`proj=set`), a missing optional `@` grid as an identity step, and the axis order of a polar CRS (`EPSG:3413`). Still open: the factory's discard rerun for compound pairs, which keeps `EPSG:4326+5773` to `EPSG:4326+5782` a creation error where libproj creates it. Sheet of this part: [`docs/PORTING-facade-libproj-parity.md`](docs/PORTING-facade-libproj-parity.md). Sheet: [`docs/PORTING-facade-parity.md`](docs/PORTING-facade-parity.md). A differential check against the reference `cs2cs` on many points and EPSG pairs runs in CI (`tests/agree_cs2cs.rs`) |
| 3 | **`cs2cs` real gaps** | Wrong or missing results, not wording. Fixed: a chain that holds a projection (a conversion, a Lambert or UTM step, and a transformation) is written as a PROJ pipeline, so `cs2cs` matches PROJ 9.9.0 on `EPSG:4326` to `EPSG:2154`, `EPSG:4326` to `EPSG:5070` (the wrong E = 16524808 is gone), `EPSG:4326` to `EPSG:2230`, `EPSG:2154` to `EPSG:32631`, and `EPSG:4326` to `EPSG:3035` at one checked point. Also fixed since: compound CRS names (`EPSG:3912+5779`), `--accuracy` and `--no-ballpark`, `ob_tran` with `o_proj=longlat`, the datum shift of `+init=epsg:`, grids found under their old name (`BETA2007.gsb`, `egm96_15.gtx`), the ranking of an operation whose grid is missing (`isPROJExportable`), the per-point retry over alternative operations (`proj_trans`), `--only-best` and `PROJ_ONLY_BEST_DEFAULT`, the 3D promotion of a CRS given by name, longitude-first sources (`OGC:CRS84`), the accuracy of a concatenation (a sum, as PROJ). The CLI harness now compares heights and axis order too. Still open, by cause: the pivot candidate set is larger than PROJ's (about 100 against 11 for `EPSG:26915` to `EPSG:26715`), heights lost on `+proj=sch` pairs, the output axis order and format of some targets, `PROJ_DEBUG` messages, one time-dependent operation. <!--gen:cli_cs2cs_open-->0<!--/gen--> `cs2cs` cases open. List: [`docs/UNPORTED-CLI.md`](docs/UNPORTED-CLI.md) |
| 4 | **PROJ unit tests, by value** | <!--gen:unit_ported-->1247<!--/gen--> ported, **<!--gen:unit_open-->105<!--/gen-->** named `TEST()`s open. Largest: <!--gen:unit_largest-->`test_operationfactory` (43), `test_factory` (26), `test_c_api` (24), `test_crs` (9), `test_io` (2)<!--/gen-->. Sorted by name in [`docs/UNIT-TRIAGE.md`](docs/UNIT-TRIAGE.md): <!--gen:unit_core-->50<!--/gen--> `core` (a result a user sees: WKT, PROJ string, chosen operation, identification) go first, <!--gen:unit_niche-->14<!--/gen--> `niche` (rare CRS kinds) next, <!--gen:unit_shape-->41<!--/gen--> `shape` (the C++ object model and the C ABI) last, and may stay unported when the Rust API differs on purpose. List: [`docs/UNPORTED-UNIT.md`](docs/UNPORTED-UNIT.md) |
| 5 | **`projinfo` text parity** | <!--gen:cli_projinfo_open-->0<!--/gen--> `projinfo` cases open (and <!--gen:cli_proj_open-->0<!--/gen--> `proj`). Mostly the exact text of the candidate operation list. Lower value than 2 and 3: the numbers are right when the text differs |
| 6 | **Portable batch path** | The AVX2 path covers x86-64 on Linux glibc (libmvec) and Windows (CRT). Other targets run the scalar loop |
| 7 | **Grid cache in CI** | Done: the network step caches the CDN grids (about 81 MB, 10 files) in `actions/cache`. Re-run on a cache miss only |

The numbers in this table are computed from the ledgers by
`tests/docs_in_sync.rs`: `UPDATE_DOCS=1 cargo test --test docs_in_sync`
refreshes them after a lot.

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
time: a pair it does not build is refused, not answered by another operation.

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
