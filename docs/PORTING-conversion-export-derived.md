# Port — `conversion-export-derived`

PROJ 9.9.0 unit tests closed in this lot, in two files:

- `test/unit/test_crs.cpp`, `TEST(crs, normalizeForVisualization_derivedprojected_operation)`.
- `test/unit/test_operationfactory.cpp`, `TEST(operation, compoundCRS_with_horizontal_boundCRS_to_compoundCRS_with_derived_vertical)`.

**Result: 2 cases closed.** Rust tests:

- `crs_normalize_for_visualization_derivedprojected_operation` in `tests/test_crs.rs`.
- `operation_compound_with_horizontal_bound_crs_to_compound_with_derived_vertical` in `tests/unit_op_compound_to_compound_3.rs`.

Both carry the C++ name in their doc comment. The WKT and the expected PROJ strings are copied from the C++ files, not from the port.

## C++ read

Source: `src/iso19111/operation/conversion.cpp` (PROJ 9.9.0).

- `Conversion::_exportToPROJString`, method branch `starts_with(methodName, "PROJ ")` (line 4127), which calls `createPROJExtensionFromCustomProj` (lines 3650-3680). That function writes the step from the second token of the name, the `k=v` tokens of the name as parameters, then each parameter value of the conversion. A linear measure is written in SI (lines 3680-3685).
- Source modifiers (lines 3813-3880, under `applySourceCRSModifiers`). A projected source runs `addUnitConvertAndAxisSwap(false)` inside `startInversion` (lines 3906-3922).
- Target modifiers (lines 4422-4475). A derived projected target runs `DerivedProjectedCRS::addUnitConvertAndAxisSwap` (`crs.cpp` 6970), which is `ProjectedCRS::addUnitConvertAndAxisSwap` on the axis list of the derived CRS. A projected target runs the same on its own axes.

## Map

Rust, in the worktree:

| Function | File | Role |
|---|---|---|
| `export_proj_method` | `src/iso19111/conversion_export.rs` | New. A `PROJ <step>` conversion from a projected or vertical source to a derived projected or vertical target. Writes the C++ sequence above. Any other pair returns `false` and keeps the projection path. |
| `export_steps` | `src/iso19111/conversion_export.rs` | Calls `export_proj_method` before `export_projection`. |
| `axes_unit_and_axis_swap` | `src/iso19111/transform_export.rs` | Was `projected_unit_and_axis_swap(&ProjectedCRS)`. The body now takes an axis slice, so a derived CRS can use it. The projected wrapper is unchanged. |
| `axes_order` | `src/iso19111/projstring.rs` | Was the body of `projected_axis_order(&ProjectedCRS)`, now on an axis slice. `projected_axis_order` calls it. |
| `must_axis_order_be_switched`, `reversed` | `src/iso19111/normalize.rs` | Now handle `Crs::DerivedProjected`. A north-then-east derived CRS is reversed by `normalizeForVisualization`, as `DerivedProjectedCRS::normalize_for_visualization` in `projstring.rs` already does for the CRS itself. |

## Cause

`export_projection` builds the projection from the base of the target. For a `PROJ unimplemented` conversion from a projected CRS to a derived projected CRS, the target has no base projection that can be inverted, so `projection_steps(derived, projected)` failed with "the source CRS is not geographic". The same failure hit a `PROJ affine` conversion from a vertical CRS to a derived vertical CRS ("this operation is not a projection of the source CRS").

Normalization did not reverse a derived projected CRS, so the target axis swap of the normalized operation was kept.

## Deviations from PROJ

- A `PROJ` method whose parameter value is not a measure, or is an angular measure, returns an error here. PROJ writes these values through `createPROJExtensionFromCustomProj`. Neither case is in the closed tests, and writing them is left for a later lot.
- A `PROJ` method whose source is not projected or vertical, or whose target is not derived projected or vertical, keeps the existing path. Geographic sources of a `PROJ` method (source modifiers of `srcGeodCRS->_exportToPROJString`, C++ 3868-3880) are not written here.
- Target modifiers of a projected target under a `PROJ` method are not written, because the projected target keeps the projection path. That is the existing behaviour.

## Still open, with reasons

- `operation.compoundCRS_to_compoundCRS_with_vertical_transform` (`test_operationfactory.cpp:5451`). Its last two blocks call `setUseApproxTMerc(true)` on an operation export, then compare the `+approx` strings. This crate has the approximate tmerc only on a single `Conversion` (`export_to_proj_string_approx_tmerc`, `src/iso19111/buildop.rs:1266`). A concatenated operation export does not carry that flag, so these two checks cannot be made in Rust. Closing the test needs a public approximate export of operations, which is a different lot.
- `operation.boundCRS_to_derived_geog_with_transformation_with_source_crs_being_base_crs_of_source_crs` (`test_operationfactory.cpp:3465`). Its input is PROJJSON with a derived geographic source and a bound CRS. Not attempted in this lot.
- `crs.normalizeForVisualization_derivedprojected_operation` and the compound test above are the only two closed. `operation.compoundCRS_with_derived_vertical_CRS` (`test_operationfactory.cpp:10210`) uses the `Vertical Offset and Slope` method (`vertoffset`), another export branch. Not attempted.

## Commands

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH
export PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data

cargo test --test test_crs crs_normalize_for_visualization_derivedprojected
cargo test --test unit_op_compound_to_compound_3 derived_vertical
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
```

`docs_in_sync` fails in the full run because `UPDATE_DOCS=1` was not run after removing the two lines of `docs/UNPORTED-UNIT.md`. That run is the maintainer's step, as the brief asked.
