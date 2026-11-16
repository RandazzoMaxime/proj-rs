# proj-rs-georust

A drop-in replacement for the georust [`proj`](https://crates.io/crates/proj) 0.31 crate,
on the proj-rs engine: no libproj, no C toolchain at build time.

```toml
[dependencies]
proj = { package = "proj-rs-georust", version = "1" }
```

The library is named `proj`, so `use proj::Proj;` keeps working. The surface is the one of
`proj` 0.31: `Proj`, `ProjBuilder`, `Coord`, `Transform`, `ProjError`, `ProjCreateError`,
`Area`, `Info`, `ProjInfo`, the `wkt` module and the `geo-types` feature. The examples of the
`proj` 0.31 documentation run as tests in `tests/`.
