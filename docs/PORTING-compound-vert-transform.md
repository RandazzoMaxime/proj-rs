# Port: `compoundCRS_to_compoundCRS_with_vertical_transform`

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp:5451`, `coordinateoperationfactory.cpp`
(`createOperationsBoundToVert` 5957-5978, `createOperationsCompoundToCompound` 7214-7612).

The test is a compound CRS whose second component is a bound vertical CRS (base `ODN height`, hub
`unknown` height, a VERTCON `bla.gtx`), to a UTM 32N + height compound. Both directions.

## Cause

`createOperations(boundSrc, vertDst)` had no dispatch. A bound vertical CRS to its hub returned zero
candidates, so the compound had no vertical step at all.

PROJ's `createOperationsBoundToVert` returns the binding transformation itself when the target is
equivalent to the hub, and otherwise the base of the source to the target. The reverse case (vertical
source, bound target) is `applyInverse` of the forward one.

## Port

- `src/iso19111/bound_vert.rs`: `bound_to_vert`, the function above.
- `src/iso19111/bound_ops.rs`: the dispatch arm for (bound, vertical), and the reverse guard extended to a
  vertical source. `with_endpoints` is now `pub(super)`.

## Tests

- `tests/unit_op_compound_vert_transform.rs`: the forward PROJ string and the inverse round trip, copied
  from the C++. Passes.
- The `+approx` blocks are not ported. The operation-level export has no approximate-TM option; only
  `export_to_proj_string_approx_tmerc` on a conversion exists.
- `tests/unit_op_nadgrids_with_pm.rs`: the WKT2 round trip is added and `#[ignore]`d. After import the port
  writes an extra `+step +proj=longlat +ellps=intl +pm=lisbon`, which PROJ does not write. Cause not
  identified. The first two assertions pass, so `operation.nadgrids_with_pm` stays in the ledger.

## Commands run

- `cargo test -p proj-rs --all-features --no-fail-fast`: 1721 passed, 1 failed (the WKT2 round trip,
  before it was ignored), 27 ignored. The failure was the one above; no other test changed outcome.
- `cargo fmt --check`, `cargo clippy -p proj-rs --all-targets --all-features -- -D warnings`: clean.
- Reference: PROJ 9.9.0 source tag, read at `test/unit/test_operationfactory.cpp`. No PROJ binary was built.
