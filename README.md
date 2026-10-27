# PROJ-RS

Open source port of [PROJ](https://github.com/OSGeo/PROJ) to Rust.
The goal is the same observable behavior as the latest PROJ (9.9) on the
ported conversions, with a Rust API (`Result`, enums, no error codes and no `NaN`
sentinel on the scalar path).

The target is a drop-in replacement for users of the
[`proj`](https://github.com/georust/proj) crate: same API, one line changed in
`Cargo.toml`, no C library to build. See [ROADMAP.md](ROADMAP.md).

The first scope is what metropolitan France needs:

- **UTM** — exact Transverse Mercator by Poder / Engsager, PROJ's default
  `tmerc_default_algo = poder_engsager`.
- **Lambert-93** — ellipsoidal Lambert Conformal Conic on GRS80.

Everything else (EPSG database, NTv2 grids, WKT, pipelines, other projections)
is listed in [ROADMAP.md](ROADMAP.md) and lands through pull requests.

## Status

Ported: exact Transverse Mercator / UTM, Lambert-93, Mercator and Web
Mercator, 7-parameter Helmert. Each module has a porting sheet in
[docs/](docs/). The next lots are in [ROADMAP.md](ROADMAP.md).

The batch path (`forward_batch`) uses AVX2 and glibc's libmvec on Linux
x86_64. Every other target builds and runs the scalar loop.

## Running

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

`tests/agree_cs2cs.rs` calls PROJ's `cs2cs`, so the tests need PROJ on the
`PATH` (9.4 today, 9.9 once lot 8 lands; see [ROADMAP.md](ROADMAP.md)) (`proj-bin` on Debian / Ubuntu).

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
