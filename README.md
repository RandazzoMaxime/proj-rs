# PROJ-RS

Open source port of [PROJ](https://github.com/OSGeo/PROJ) **9.9.0** to Rust.
The goal is the same observable behavior as PROJ 9.9 on every ported
operation, with a Rust API (`Result`, enums, no error codes and no `NaN`
sentinel on the scalar path).

The target is a drop-in replacement for users of the
[`proj`](https://github.com/georust/proj) crate: same API, one line changed in
`Cargo.toml`, no C library to build. See [ROADMAP.md](ROADMAP.md).

The numeric oracle is the PROJ 9.9.0 `test/gie` suite, vendored under
`tests/gie/`. `builtins.gie` passes in full. `data/proj.db` is built from the
PROJ 9.9.0 SQL and opened with rusqlite. An EPSG code resolves from that
database when every step is a ported operation. A missing grid or an unknown
method is an error at creation, never a coordinate.

## Status

Ported so far: the classic projections exercised by `builtins.gie` (worldwide,
not a single country), pipelines, `cart`, Helmert, Molodensky, `unitconvert`,
`affine`, and the grid-shift and deformation operations when their grid file
is on disk. `cs2cs` and `cct` are thin binaries over the library.

`tests/gie/passing.txt` is the live count of passing `.gie` cases. The vendored
suite is 6856 cases. A `require_grid` case runs when that file is under
`data/grids/` or `tests/gie/tests/`. Those CDN grids are not stored in git.

### Progression

Counts are from green `cargo test --test gie_run` runs. `builtins.gie` is
2597 passed, 0 failed, 0 skipped from the second row on.

| Step | Passed | Failed | Skipped | Share |
|---|---:|---:|---:|---:|
| PROJ 9.9 builtins complete | 2743 | 0 | 4113 | 40.0% |
| proj.db, Adams, Guyou, Peirce, Spilhaus | 6301 | 0 | 555 | 91.9% |
| Conversions, test grids, deformation models | 6657 | 0 | 199 | 97.1% |
| CDN grids present on disk | 6678 | 0 | 178 | 97.4% |
| External grids fetched, suite closed | 6856 | 0 | 0 | 100% |

The last row is the vendored suite with nothing skipped. Those grids live in
`data/grids/`, which git ignores. A checkout without that directory does not
reproduce 6856.

The batch path (`forward_batch`) uses AVX2. On Linux glibc it calls libmvec.
On Windows it uses the C runtime math library one lane at a time. Other
targets run the scalar loop.

## Running

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

`tests/agree_cs2cs.rs` calls a `cs2cs` binary on `PATH` as a differential
check. The numbers that decide a pass are the vendored PROJ 9.9.0 `.gie`
expectations, not that install.

Rust 1.83 or newer. Lints come from
[BeautifulRust](https://github.com/morganlinton/BeautifulRust).

## Progress bench

`src/bin/bench.rs` times every ported operation, in nanoseconds per point,
on fixed grids. `bench/record.sh` runs it and appends one line to
`bench/history.jsonl` with the commit and the machine name, so every lot can
be measured against the previous ones.

The two rows below are different machines. Do not read them as a speedup.
`ratio` is our time divided by OSGeo4W libproj on the same grid. Below 1, this
port is faster. The full catalogue (about 160 operations) is in the JSONL
line, not in this table. The second row was recorded when the suite had
2743 passing cases. It has not been re-run since.

| Case | 2026-10-27 scalar | 2026-10-28 AVX2 | libproj | ratio |
|---|---:|---:|---:|---:|
| utm31 forward | 81.5 | 82.2 | 113.4 | 0.72 |
| utm31 inverse | 79.0 | 90.1 | | |
| Lambert-93 forward | 50.3 | 54.7 | 91.3 | 0.60 |
| Lambert-93 inverse | 138.5 | 163.3 | | |
| Web Mercator forward | 19.3 | 41.5 | 67.7 | 0.61 |
| Web Mercator inverse | 25.0 | 35.6 | | |
| UTM batch, zone center | 80.0 | 50.2 | | |
| UTM batch, zone edge | 75.0 | 44.3 | | |
| Lambert-93 batch | 47.5 | 34.2 | | |
| Helmert 7-parameter | 7.3 | 7.1 | | |

Nanoseconds per point, one million points. The 2026-10-27 row is
`maxime-win11-nosimd` at `b405008`. The 2026-10-28 row is `RANDAZZOPROD`,
`batch_path=avx2`, at dirty `16408a8`.

Slowest operations on that AVX2 run, same units, 128-point catalogue grid:

| Operation | ns/point |
|---|---:|
| gnom | 396.9 |
| aeqd | 394.5 |
| lsat | 356.3 |
| misrsom | 343.8 |
| dsea | 248.4 |
| isea | 194.5 |
| ivea | 193.0 |
| calcofi | 180.5 |
| tsea | 176.6 |
| mod_krovak | 157.0 |

Record another row with:

```bash
BENCH_MACHINE=my-box bench/record.sh
```

Compare only lines from the same machine, and pin the process
(`taskset -c 0`) for stable numbers.

## Contributing

Every change goes through a pull request. The procedure, the format of a lot
PR, and the merge rule are in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. Formulas taken from PROJ remain under PROJ's MIT license.
See [LICENSE](LICENSE).
