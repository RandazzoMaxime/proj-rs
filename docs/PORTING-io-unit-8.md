# Port — PROJ `test/unit/test_io.cpp`, `io.projstringformatter*`

Sources, PROJ 9.9.0:

- `test/unit/test_io.cpp`: the tests (line ~10115 to ~10940).
- `src/iso19111/io.cpp`: `PROJStringFormatter` (`addStep`, `addParam`, `startInversion`, `stopInversion`, `ingestPROJString`, line ~10298 to ~10440) and `PROJStringSyntaxParser` (line ~10163 to ~10294).

## Tests ported (16)

Each test is one function in `src/iso19111/pipeline/tests.rs`, named `io_projstringformatter_<C++ name>`. The formatter is crate-private (`PipelineFormatter` in `src/iso19111/pipeline.rs`), so the tests live beside it and no public API is added.

| C++ TEST | Rust test |
|---|---|
| `projstringformatter` | `io_projstringformatter` |
| `projstringformatter_helmert_3_param_noop` | `io_projstringformatter_helmert_3_param_noop` |
| `projstringformatter_helmert_7_param_noop` | `io_projstringformatter_helmert_7_param_noop` |
| `projstringformatter_merge_consecutive_helmert_3_param` | `io_projstringformatter_merge_consecutive_helmert_3_param` |
| `projstringformatter_merge_consecutive_helmert_3_param_noop` | `io_projstringformatter_merge_consecutive_helmert_3_param_noop` |
| `projstringformatter_merge_consecutive_helmert_7_param_noop` | `io_projstringformatter_merge_consecutive_helmert_7_param_noop` |
| `projstringformatter_merge_inverted_helmert_with_opposite_conventions` | `io_projstringformatter_merge_inverted_helmert_with_opposite_conventions` |
| `projstringformatter_cart_grs80_wgs84` | `io_projstringformatter_cart_grs80_wgs84` |
| `projstringformatter_axisswap_unitconvert_axisswap` | `io_projstringformatter_axisswap_unitconvert_axisswap` |
| `projstringformatter_axisswap_one_minus_two_inv` | `io_projstringformatter_axisswap_one_minus_two_inv` |
| `projstringformatter_axisswap_two_one_followed_two_minus_one` | `io_projstringformatter_axisswap_two_one_followed_two_minus_one` |
| `projstringformatter_axisswap_minus_two_one_followed_two_one` | `io_projstringformatter_axisswap_minus_two_one_followed_two_one` |
| `projstringformatter_axisswap_two_minus_one_followed_minus_two_one` | `io_projstringformatter_axisswap_two_minus_one_followed_minus_two_one` |
| `projstringformatter_axisswap_two_minus_one_followed_one_minus_two` | `io_projstringformatter_axisswap_two_minus_one_followed_one_minus_two` |
| `projstringformatter_unitconvert` | `io_projstringformatter_unitconvert` |
| `projstringformatter_unmodified` | `io_projstringformatter_unmodified` |

## Not ported (still in `docs/UNPORTED-UNIT.md`)

- `projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` and `projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc`: they expect PROJ's grid push/pop rewriting (`+proj=push`, `+proj=pop`, `omit_fwd`/`omit_inv`). `src/iso19111/pipeline.rs` documents that rewriting as not ported, so these are expected to fail. They were not run.
- `projstringformatter_krovak_to_krovak_east_north` and `projstringformatter_krovak_east_north_to_krovak`: they expect the Krovak `axis=swu` merge to `+proj=axisswap +order=-2,-1`, which is documented as not ported. Not run.

## Source changes

- `src/iso19111/pipeline.rs`: `PipelineFormatter::ingest_proj_string` (crate-private) and `parse_proj_string`, a port of `PROJStringSyntaxParser` for the pipeline and single-step forms. Tokens are split on whitespace with the leading `+` removed.
  - Dropped: global parameters (before the first step) and `title=`. PROJ keeps globals apart from the steps. No test in this lot has them.
  - `init=` is read as a parameter, so it prints differently from PROJ's `+init=`.
  - Quoted values are not handled, since PROJ's `pj_trim_argc` quoting is not ported.
- `src/iso19111/pipeline.rs`: `#[cfg(test)] mod tests;`, and the module doc now names the parser's gaps.
- `src/iso19111/pipeline/tests.rs`: the 16 tests.

No new public API.

## Verification

- `cargo test --lib io_projstringformatter`: 16 passed.
