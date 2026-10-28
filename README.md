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

`tests/gie/passing.txt` is the live count of passing `.gie` cases. Cases
tagged `require_grid` stay skipped until that file is present. The PROJ-data
CDN grids are not stored in this repository.

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
be measured against the previous ones:

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
