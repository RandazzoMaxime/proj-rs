# Port: `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound`

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp:6460`, `coordinateoperationfactory.cpp`
(`createOperationsCompoundToCompound` 7214-7612, `createOperationsCompoundToGeog` 6454-7172,
`buildCRSIds` 1776-1825), `crs.cpp` (`GeographicCRS::_isEquivalentTo`, `SingleCRS::baseIsEquivalentTo`).

The test is a compound CRS of a bound CRS (`CH1903 / LV03` with a Helmert to WGS 84) and `LN02 height`,
to `EPSG:9518` (`WGS 84 + EGM2008 height`), in both directions. PROJ's first candidate is `Inverse of Map
projection of CH1903 / LV03 + MyTransformation from CH1903 to WGS84 + Inverse of ETRS89 to WGS 84 (1) +
Inverse of ETRS89 to LN02 height + ETRS89 to WGS 84 (1) + WGS 84 to EGM2008 height (1)`: the Helmert of the
bound CRS, then the vertical grid of `LN02` through ETRS89.

## Four causes

1. **The test's context was not the C++ one.** `TEST()` sets `SpatialCriterion::PARTIAL_INTERSECTION`; the
   Rust test used the default (strict containment), so `findOpsInRegistryDirectTo` dropped
   `ETRS89 to LN02 height` (its extent, Switzerland, does not contain the world). The test now uses
   `partial_ignore_grids`, as the C++ does. This was the main reason earlier attempts did not converge.
2. **`GeographicCRS::_isEquivalentTo` under `EQUIVALENT`** (`equiv.rs`, `Crs::is_equivalent_to_with`). The
   bound hub `WGS 84` (axes `Latitude`/`Longitude`, a datum) was not equivalent to `EPSG:4326` (axes
   `Geodetic latitude`/`Geodetic longitude`, an ensemble): a geographic pair fell to the strict comparison,
   which compares CRS and axis names. Outside `Strict` the names are not compared, only the datum or
   ensemble and the axes' directions and units. Without it the bound branch of `createOperationsCompoundToCompound`
   (7419-7446) was skipped.
3. **`VerticalCRS::_isEquivalentTo` under `EQUIVALENT`** (same place). The `LN02 height` read from WKT has
   no identifier and the axis name `gravity-related height`; the registered one has `Gravity-related
   height` and an abbreviation. Outside `Strict` the names compare as `Identifier::isEquivalentName` does
   and the axis by direction and unit.
4. **`buildCRSIds` in `findOpsInRegistryDirectTo`** (`factory.rs`). A CRS with no identifier is looked up
   by name and identified when it is equivalent to the registered one (`registry_identity`). The finder
   ported in #169 took the identifiers only.

The dispatch of `compound_to_geog3d` also changes: a bound horizontal part no longer takes
`bound_compound_to_geog3d` (an approximation that gave a ballpark horizontal step) but the same search as
any other horizontal part, which already handled the derived and the bound-and-bound branches of
`createOperationsCompoundToGeog` (6473-6685). A bound vertical part keeps the approximation.

## Tests

`operation_compound_to_compound_issue_3152_ch1903lv03_ln02_bound` (`tests/unit_op_compound_to_compound_2.rs`)
passes, with its first candidate's name and PROJ string, and the name of the first inverse candidate. It
takes about two minutes in debug mode: PROJ's context gives 129 candidates.

## Still open

`compoundCRS_to_compoundCRS_context` stays `#[ignore]`d with a corrected reason. Its first candidate is
right (name and PROJ string); the failing assertion is the WKT2 round trip: the PROJ string of the
re-imported operation has the vertical unit change and the `vgridshift` before the axis swap and the
degree conversion.

## Commands run

On a Windows workstation, without the reference `cs2cs`: `cargo fmt --check`,
`cargo clippy -p proj-rs --all-targets --all-features -- -D warnings` (clean),
`cargo test -p proj-rs --all-features --no-fail-fast` (1707 passed, 11 failed, 28 ignored; the same 11
fail on `main` on that machine), the seven test files of the bound-compound family and
`unit_op_compound_to_compound_2` with `--include-ignored`, `UPDATE_DOCS=1 cargo test --test docs_in_sync
--test progress_svg`, then without. The causes were found with a build of PROJ 9.9.0 compiled with
`-DTRACE_CREATE_OPERATIONS -DENABLE_TRACING`.
