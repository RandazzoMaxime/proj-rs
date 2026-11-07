# Port — bound CRS to compound CRS (`createOperationsBoundToCompound`)

Lot `bound-to-compound`. Cause: an operation from a bound CRS to a compound CRS was not built, and
`operation.boundCRS_to_compoundCRS` returned no operation. Two ledger lines are closed:
`operation.boundCRS_to_compoundCRS` and `operation.boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS`.

Sources, PROJ 9.9.0 (`src/iso19111/operation/coordinateoperationfactory.cpp`):

- `CoordinateOperationFactory::Private::createOperations`, the dispatch: `boundSrc && compoundDst` (line 3797) and its reverse `boundDst && compoundSrc` (3803).
- `createOperationsBoundToCompound`, the function (7852-8041): block 1 (7864-7911), block 2 (7913-7991), block 3 (7993-8034), fallback (8036-8040).
- `createOperationsBoundToGeog` (5578): the hub-is-target branch (5627-5661), which returns the binding transformation itself, so its target is the 2D hub.
- `ConcatenatedOperation::createComputeMetadata` with `disallowEmptyIntersection` (`Private::Context`, 589-592).
- `CoordinateOperationFactory::createOperation(src, dst)` (836-845): the context is `create(nullptr, nullptr, 0.0)`.

## What is written

| Rust (`src/iso19111/`) | PROJ |
|---|---|
| `bound_ops::bound_candidates`, arm `(Crs::Bound, Crs::Compound)` | the dispatch at 3797 |
| `factory::create_candidates`, last arm `(Compound, Bound)`: the inverse of `create_candidates(bound, compound)` | the reverse at 3803, `applyInverse(createOperations(...))` |
| `bound_ops::bound_to_compound`, block 1 (first `if let`) | block 1, 7864-7911 |
| `bound_ops::bound_to_compound`, block 2 (`if let Crs::Bound(comp0)`) | block 2, 7913-7991 |
| `bound_ops::bound_to_compound`, block 3 (after block 2) | block 3, 7993-8033 |
| the last `factory.create_candidates(bound_src.source_crs(), target, ..)` | the fallback, 8036-8040 |
| `bound_ops::set_endpoints` | `setCRSs` |
| `compound::compute_metadata` | `ConcatenatedOperation::createComputeMetadata` |
| `bound_ops::extract_geodetic`, `extract_geographic` | `CRS::extractGeodeticCRS`, `extractGeographicCRS` |
| `bound_ops::datums_equivalent` | `datumNonNull(...)->isEquivalentTo(..., EQUIVALENT)` |
| `bound_ops::single_axis_count` | `coordinateSystem()->axisList().size()` |
| `bound_ops::crs_equivalent` (existing) | `_isEquivalentTo(..., EQUIVALENT)` on the CRS |

The blocks keep PROJ's order. The adapter of block 2 (used when the two steps of a pair do not meet,
for example 2D against 3D) is `createOperations(opSrc->targetCRS(), opDst->sourceCRS())` with its
first result, as in PROJ.

No new public method was added. `CoordinateOperation::is_equivalent_to` already exists and is used
by the test (see the deviations).

## Tests

`tests/unit_op_bound_to_compound.rs`, both passing (they failed before the change):

| PROJ 9.9.0 `test_operationfactory.cpp` | Rust test | Checks |
|---|---|---|
| `TEST(operation, boundCRS_to_compoundCRS)` (9609) | `operation_bound_crs_to_compound` | the forward pipeline (`hgridshift @foo.gsb`, `inv hgridshift @bar.gsb`, `inv vgridshift @bar.gtx`), and that the inverse of `dst -> src` is equivalent to it |
| `TEST(operation, boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS)` (9640) | `operation_bound_crs_to_compound_with_hub_crs_same_as_compound_geographic_crs` | the first operation of the list (`somerc`, `helmert`, `vgridshift us_nga_egm08_25.tif`) for `CH1903 / LV03` to `EPSG:9518` |

The expected strings are copied from `test_operationfactory.cpp` in the PROJ 9.9.0 tree, not from
this crate. The WKT of the hub test is the one of PROJ's test, extracted from that file.

## Deviations

1. **Intermediate CRS without the WORLD domain.** PROJ gives the 3D geographic intermediate of block 2 the WORLD domain of validity (7934-7942). It is not set here, as in `bound_to_geod` (same reason, see its doc comment): with it, the datum pivot of this port changes the chain. Not measured with the domain set.
2. **Empty intersection is a skip, not an exception.** PROJ's `createComputeMetadata` in block 2 (7970-7984) has no `try`, so an empty intersection throws and aborts the search; block 3 catches it (8021-8027). This port drops the candidate where `compute_metadata` returns `None`, as the rest of the port does. No test here reaches that path.
3. **No registry lookup before the case.** PROJ runs `createOperationsFromDatabase` before the bound cases (3652). `bound_candidates` runs before the registry, as for the other bound cases of this module. The two tests have no authority codes on the bound side.
4. **Reverse dispatch is extended, not returned.** PROJ returns at `boundDst && compoundSrc`. The Rust `create_candidates` adds to `found`, the existing convention of its last arms.
5. **Only the single CRS kinds this port binds count as 3D in block 3.** `single_axis_count` gives the axis count of geographic, geocentric, projected and vertical CRS; the other kinds count as not 3D, so block 3 is skipped for them.
6. **Strict equivalence in the test.** PROJ's `_isEquivalentTo(op)` uses the EQUIVALENT criterion. The test uses `CoordinateOperation::is_equivalent_to`, which is the strict one, so it asserts a stronger property than PROJ's assertion. It holds here.

## Checks against PROJ 9.9.0 (`projinfo` of the reference build, `PROJ_DATA` of the oracle)

| Pair | PROJ 9.9.0 | This crate |
|---|---|---|
| test pair, `dst -> src` (`+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@bar.gtx +geoid_crs=horizontal_crs` to `+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb`) | `unitconvert`, `vgridshift @bar.gtx`, `hgridshift @bar.gsb`, `inv hgridshift @foo.gsb`, `unitconvert` | same |
| `+proj=longlat +ellps=GRS67 +towgs84=1,2,3,0,0,0,0` to the compound of the test (`+nadgrids=@bar.gsb +geoidgrids=@bar.gtx`) | `unitconvert`, `push v_3`, `cart GRS67`, `helmert`, `inv cart WGS84`, `pop v_3`, `inv hgridshift @bar.gsb`, `inv vgridshift @bar.gtx`, `unitconvert` | same |
| `+proj=longlat +ellps=GRS67 +nadgrids=@foo.gsb` to `EPSG:4326+5773`, and the reverse | `axisswap +order=2,1` | same |

## Test totals and gates

Before the change (base `3ccfd8a`, the new test file present): `cargo test --all-features --no-fail-fast`, 1651 passed, 2 failed (the two new tests), 32 ignored. `cli_yaml` passed.

After the change: 1653 passed, 0 failed, 32 ignored (the same run, with the two tests closed). `cli_yaml` passed. The ignored tests (`-- --ignored`) give the same results before and after: 26 pass and 6 fail, the failing ones being `wkt_parse_boundcrs_with_interpolation_as_parameter` and five `operation_compound_to_compound_*` cases, all already open. `operation_compound_to_compound_issue_3328` passes in both runs; it is ignored and is left as it is, as it belongs to the compound-to-compound lot.

`cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass. `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` regenerated `README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md` and `docs/progress.svg`, and the counts match the ledgers (`PROJ C++ unit tests`: 1237 ported, 115 open; `test_operationfactory.cpp`: 221 ported, 53 open).

The ledger change is two lines removed from `docs/UNPORTED-UNIT.md` (lines 96-97).
