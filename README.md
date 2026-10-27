# PROJ-RS

Open source port of [PROJ](https://github.com/OSGeo/PROJ) to Rust.
The goal is the same observable behavior as PROJ 9.4 on the ported
conversions, with a Rust API (`Result`, enums, no error codes and no `NaN`
sentinel on the scalar path).

The first scope is what metropolitan France needs:

- **UTM** — exact Transverse Mercator by Poder / Engsager, PROJ 9.4's default
  `tmerc_default_algo = poder_engsager`.
- **Lambert-93** — ellipsoidal Lambert Conformal Conic on GRS80.

Everything else (EPSG database, NTv2 grids, WKT, pipelines, other projections)
is listed in [ROADMAP.md](ROADMAP.md) and lands through pull requests.

## Status

| Lot | Content | Branch |
|---|---|---|
| 0 | Governance, license, CI, clippy | `cursor/lot-0-gouvernance-a775` |
| 1 | Ellipsoid, errors, angles | `cursor/lot-1-ellipsoide-a775` |
| 2 | Transverse Mercator and UTM | `cursor/lot-2-tmerc-a775` |
| 3 | Lambert Conformal Conic and Lambert-93 | `cursor/lot-3-lambert-a775` |
| 4 | AVX2 batch Transverse Mercator and timing | `cursor/lot-4-simd-bench-a775` |
| 5 | AVX2 batch Lambert-93 | `cursor/lot-5-lambert-avx-a775` |
| 6 | Mercator and Web Mercator | `cursor/lot-6-mercator-a775` |
| 7 | 7-parameter Helmert | `cursor/lot-7-helmert-a775` |

Lots 1 to 7 are stacked pull requests. Review them in order.
**Only the maintainer merges.** The PR descriptions are in
[docs/LOTS.md](docs/LOTS.md): the agent's temporary forge cannot open
them, and the session's GitHub token is not allowed to create
`RandazzoMaxime/proj-rs`.

## Running

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Lot 4 compares the crate against `libproj` (`libproj-dev`, `proj-bin`):

```bash
cargo test --all-features
cargo run --release --features bench --bin proj-rs-bench
```

Rust 1.83 or newer. Lints come from
[BeautifulRust](https://github.com/morganlinton/BeautifulRust).

## Contributing

Every change goes through a pull request. The procedure, the format of a lot
PR, and the merge rule are in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. Formulas taken from PROJ remain under PROJ's MIT license.
See [LICENSE](LICENSE).
