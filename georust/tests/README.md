# Examples of the `proj` 0.31 documentation

`proj_031_docs.rs` holds every example of the documentation of `proj` 0.31.0, copied
verbatim from the crate downloaded from crates.io
(`https://crates.io/api/v1/crates/proj/0.31.0/download`). Each one is a test of this crate: it
compiles against the `proj` library of `proj-rs-georust` and runs.

Taken: all 30 code blocks, from the doc comments of `src/lib.rs`, `src/proj.rs`,
`src/geo_types.rs` and `src/transform.rs`, and from `README.md` where it differs. The tests that
use `geo-types` are behind the `geo-types` feature, as in `proj`.

Two changes, both in the README examples, which `proj` does not run as tests:

- one example writes `epsiolon=1e-5` (a typo, it does not compile); the test spells it `epsilon`;
- four assertions compare at machine precision against values written by libproj 9.6; the port,
  which is PROJ 9.9, is 3e-9 m away on 1.4e6 m. They take an `epsilon = 1e-6`, a micrometre.

The examples of the doc comments are unchanged.

| Source (`proj` 0.31.0) | Line | Test |
|---|---|---|
| `src/lib.rs` | 32 | `doc_src_lib_rs_32` |
| `src/lib.rs` | 57 | `doc_src_lib_rs_57` |
| `src/lib.rs` | 146 | `doc_src_lib_rs_146` |
| `src/lib.rs` | 186 | `doc_src_lib_rs_186` |
| `src/lib.rs` | 206 | `doc_src_lib_rs_206` |
| `src/proj.rs` | 498 | `doc_src_proj_rs_498` |
| `src/proj.rs` | 552 | `doc_src_proj_rs_552` |
| `src/proj.rs` | 635 | `doc_src_proj_rs_635` |
| `src/proj.rs` | 643 | `doc_src_proj_rs_643` |
| `src/proj.rs` | 688 | `doc_src_proj_rs_688` |
| `src/proj.rs` | 698 | `doc_src_proj_rs_698` |
| `src/proj.rs` | 749 | `doc_src_proj_rs_749` |
| `src/proj.rs` | 974 | `doc_src_proj_rs_974` |
| `src/proj.rs` | 1038 | `doc_src_proj_rs_1038` |
| `src/proj.rs` | 1072 | `doc_src_proj_rs_1072` |
| `src/proj.rs` | 1120 | `doc_src_proj_rs_1120` |
| `src/proj.rs` | 1439 | `doc_src_proj_rs_1439` |
| `src/proj.rs` | 1458 | `doc_src_proj_rs_1458` |
| `src/geo_types.rs` | 5 | `doc_src_geo_types_rs_5` |
| `src/geo_types.rs` | 32 | `doc_src_geo_types_rs_32` |
| `src/transform.rs` | 19 | `doc_src_transform_rs_19` |
| `src/transform.rs` | 46 | `doc_src_transform_rs_46` |
| `src/transform.rs` | 76 | `doc_src_transform_rs_76` |
| `src/transform.rs` | 104 | `doc_src_transform_rs_104` |
| `README.md` | 34 | `doc_README_md_34` |
| `README.md` | 58 | `doc_README_md_58` |
| `README.md` | 80 | `doc_README_md_80` |
| `README.md` | 162 | `doc_README_md_162` |
| `README.md` | 199 | `doc_README_md_199` |
| `README.md` | 219 | `doc_README_md_219` |
