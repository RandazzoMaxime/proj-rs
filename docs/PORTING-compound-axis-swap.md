# Port — `compound-axis-swap`

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `TEST(operation, compoundCRS_with_non_meter_horiz_and_vertical_to_geog)`: a compound CRS whose horizontal part is a UTM zone in US survey feet and whose vertical part is in US survey feet, to EPSG:4979. PROJ gives one operation whose vertical unit change is done once and whose final step is the lat/lon axis swap.

**Result: 1 case ported, passing.** The Rust test is `operation_compound_crs_with_non_meter_horiz_and_vertical_to_geog` in `tests/unit_compound_axis_swap.rs`. Its doc comment carries the C++ name, and the WKT and expected PROJ string are copied from the C++ file.

## C++ read

- `createOperationsCompoundToGeog` (`src/iso19111/operation/coordinateoperationfactory.cpp`, 9.9.0, line 6454). The derived branch (lines 6474-6502) takes a `DerivedCRS` horizontal part. A `ProjectedCRS` is one (`include/proj/crs.hpp`, `class ProjectedCRS final : public DerivedCRS`, line 641). So the result is `createComputeMetadata({opsFirst.front(), opLast})`, where `opsFirst` is `createOperations(source, base + vertical)` and `opLast` is `createOperations(base + vertical, target)`.
- `opLast` is a compound with a geographic base and a 3D geographic target. The vertical loop (lines 7149-7171) finds no interpolation CRS for a ballpark vertical change, so it calls `createHorizVerticalPROJBased` (line 2717) with a null horizontal part.
- `MyPROJStringExportableHorizVertical` (lines 2384-2417) writes the export of that object. It writes the horizontal operation, then the inverse of the target's axis steps, then the vertical step, then the target's `unitconvert` and `axisswap` with the Z unit conversion omitted. The inverse steps cancel the horizontal tail, which is what keeps the final swap after the merged vertical unit change.
- `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (`src/iso19111/crs.cpp`, 9.9.0, line 2446) writes `unitconvert` then `axisswap`.
- `PROJStringFormatter::toString` (`src/iso19111/io.cpp`, 9.9.0, lines 9366-9380) swaps an `axisswap` with the `unitconvert` before it when another `unitconvert` follows. This is the rule of `src/iso19111/pipeline.rs` at line 486. It is not changed.

## Root cause

The port's `projected_compound_to_geog` (`src/iso19111/compound.rs`, added in commit f322a55) did not follow the derived branch. It built the inverse projection by hand, on an east-north copy of the base, and chained it with the ballpark vertical change. The copy has no axis swap, so the export ends at the vertical step, and the final `axisswap +order=2,1` that a lat/lon target needs is lost. The output then gives longitude first for EPSG:4979.

## Change

`projected_compound_to_geog` in `src/iso19111/compound.rs`:

- The horizontal part is the first candidate of `create_candidates(projected, base)`, in place of the hand-built conversion. This is C++ `opsFirst.front()`.
- The vertical part is still `vertical_ballpark`, unchanged.
- The last part is `proj_based_horiz_vertical` with `null_offset(base, target)`, which is `createHorizVerticalPROJBased` with a null horizontal part, as C++ builds `opLast`.
- The two are chained with `chain`, as before.
- `longitude_first` is removed, since nothing uses it.

The base is no longer rewritten to east-north. The geographic base keeps its own axis order, as in the C++ compound.

## Deviations

1. The guards "different datum" and "registered vertical operation" in `projected_compound_to_geog` come from the earlier port. PROJ's derived branch has no such guard. They are kept, so such pairs stay on the general path. The test does not use them.
2. `opsFirst`: C++ takes the first of `createOperations(source, base + vertical)`, a compound-to-compound call with the same vertical, which reduces to the horizontal part. The Rust `compound_to_compound` refuses two equal vertical names, so the horizontal candidate is taken directly. For this pair the result is the same.
3. `opLast`: C++ reaches the proj-based object through the compound path. Here it is built directly. Its source is the compound source, not the intermediate `base + vertical` compound. This only changes the source and target fields, not the PROJ string.
4. The name of the concatenated operation is not checked by the test.

## Commands and results

- `cargo test --test unit_compound_axis_swap`, before the change: failed. The output had no final `+step +proj=axisswap +order=2,1`.
- `cargo test --test unit_compound_axis_swap`, after the change: 1 passed.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: exit 0, 1368 passed, 0 failed, 26 ignored. The docs of the previous lot state 1367 passed before this test was added.

## Left open

- `docs/PORTING-opfac-core-c.md` (under "Not ported (40)") and the ledger `docs/UNPORTED-UNIT.md` still list the case as open. They are not edited here. The lead should move the case and regenerate the generated files with `UPDATE_DOCS=1 cargo test --test docs_in_sync`.
