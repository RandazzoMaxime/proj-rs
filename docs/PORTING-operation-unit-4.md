# Port — PROJ `test/unit/test_operation.cpp`, conversion method changes (Mercator, LCC)

Sources, PROJ 9.9.0:

- `test/unit/test_operation.cpp`: the tests.
- `src/iso19111/operation/conversion.cpp`: `Conversion::convertToOtherMethod` (line ~2860 to ~3030), the Mercator A/B and LCC 1SP/2SP branches.
- `src/iso19111/operation/singleoperation.cpp`: `SingleOperation::_isEquivalentTo` (line ~1489 to ~1750), the cross-method rules used under `EQUIVALENT`.

## Tests ported (21)

Each test is one function in `tests/test_operation_conv.rs`, named `operation_<C++ name>` in snake case. A PROJ `nullptr` is `None`.

| C++ TEST | Rust test |
|---|---|
| `mercator_variant_A_to_variant_B` | `operation_mercator_variant_a_to_variant_b` |
| `mercator_variant_A_to_variant_B_scale_1` | `operation_mercator_variant_a_to_variant_b_scale_1` |
| `mercator_variant_A_to_variant_B_invalid_scale` | `operation_mercator_variant_a_to_variant_b_invalid_scale` |
| `mercator_variant_A_to_variant_B_invalid_eccentricity` | `operation_mercator_variant_a_to_variant_b_invalid_eccentricity` |
| `mercator_variant_B_to_variant_A` | `operation_mercator_variant_b_to_variant_a` |
| `mercator_variant_B_to_variant_A_invalid_std1` | `operation_mercator_variant_b_to_variant_a_invalid_std1` |
| `mercator_variant_B_to_variant_A_invalid_eccentricity` | `operation_mercator_variant_b_to_variant_a_invalid_eccentricity` |
| `lcc2sp_to_lcc1sp` | `operation_lcc2sp_to_lcc1sp` |
| `lcc2sp_to_lcc1sp_phi0_eq_phi1_eq_phi2` | `operation_lcc2sp_to_lcc1sp_phi0_eq_phi1_eq_phi2` |
| `lcc2sp_to_lcc1sp_phi0_diff_phi1_and_phi1_eq_phi2` | `operation_lcc2sp_to_lcc1sp_phi0_diff_phi1_and_phi1_eq_phi2` |
| `lcc1sp_to_lcc2sp_invalid_eccentricity` | `operation_lcc1sp_to_lcc2sp_invalid_eccentricity` |
| `lcc1sp_to_lcc2sp_invalid_scale` | `operation_lcc1sp_to_lcc2sp_invalid_scale` |
| `lcc1sp_to_lcc2sp_invalid_lat0` | `operation_lcc1sp_to_lcc2sp_invalid_lat0` |
| `lcc1sp_to_lcc2sp_null_lat0` | `operation_lcc1sp_to_lcc2sp_null_lat0` |
| `lcc2sp_to_lcc1sp_invalid_lat0` | `operation_lcc2sp_to_lcc1sp_invalid_lat0` |
| `lcc2sp_to_lcc1sp_invalid_lat1` | `operation_lcc2sp_to_lcc1sp_invalid_lat1` |
| `lcc2sp_to_lcc1sp_invalid_lat2` | `operation_lcc2sp_to_lcc1sp_invalid_lat2` |
| `lcc2sp_to_lcc1sp_invalid_lat1_opposite_lat2` | `operation_lcc2sp_to_lcc1sp_invalid_lat1_opposite_lat2` |
| `lcc2sp_to_lcc1sp_invalid_lat1_and_lat2_close_to_zero` | `operation_lcc2sp_to_lcc1sp_invalid_lat1_and_lat2_close_to_zero` |
| `lcc2sp_to_lcc1sp_invalid_eccentricity` | `operation_lcc2sp_to_lcc1sp_invalid_eccentricity` |
| `conversion_missing_parameter_scale` | `operation_conversion_missing_parameter_scale` |

## Not ported (still in `docs/UNPORTED-UNIT.md`)

- `mercator_variant_A_to_variant_B_no_crs`: PROJ returns `nullptr` when the conversion has no source CRS. The Rust method takes the base CRS as an argument (`DerivedConversion::convert_to_other_method`), so a conversion without one cannot be written.
- `conversion_missing_parameter`: the WKT import does not fill the default `latitude_of_origin = 0` for a Transverse Mercator, so `values_equivalent` in `src/iso19111/buildop.rs` sees parameter lists of different length. Needs default filling for absent optional parameters.
- `three_param_equivalent_to_seven_param`, `position_vector_equivalent_coordinate_frame`: `Transformation::createGeocentricTranslations`, `createPositionVector` and `createCoordinateFrameRotation` do not exist here, and equivalence across the TOWGS84 conventions is not implemented.

## Source changes

- `src/iso19111/convmethod.rs` (new module): a direct translation of PROJ's arithmetic for `convertToOtherMethod` on Mercator 9804 to 9805 and back, and LCC 1SP 9801 to 2SP 9802 and back. Same operation order as PROJ, and the thousandth-of-a-degree rounding for `phi1`, `phi2` and the latitude of origin.
- `src/iso19111/mod.rs`: `mod convmethod;`.
- `src/iso19111/buildop.rs`: `from_code` and `measure_of_code` changed from private to `pub(crate)`. Nothing else.

New public API:

- `impl DerivedConversion<'_> { pub fn convert_to_other_method(&self, method_code: i32) -> Option<Conversion> }`: PROJ `Conversion::convertToOtherMethod` on the base CRS of the projected CRS. `None` is PROJ's `nullptr`.
- `impl Conversion { pub fn is_equivalent_to_with_source(&self, other: &Self, source: &GeodeticCRS, criterion: Equivalence) -> bool }`: PROJ `SingleOperation::_isEquivalentTo` for two conversions sharing `source` as base CRS.

## Equivalence rule, checked against PROJ 9.9.0

`is_equivalent_to_with_source` follows `SingleOperation::_isEquivalentTo` in `singleoperation.cpp`:

- Under `Strict`, the result is `is_equivalent_to_with`.
- Under `Equivalent`, the parameters are compared first. If they are not equivalent, PROJ's cross-method rules apply, in this order:
  - Two LCC 2SP: compare the 1SP forms of both (PROJ's fallback at line ~1734).
  - LCC 1SP against LCC 2SP: the reverse call, so the 2SP side is converted to 1SP (line ~1592).
  - Mercator A to B, Mercator B to A, and LCC 2SP to 1SP: `self` is converted to the other's method, and the result is compared with `other` (line ~1603).
  - Anything else: not equivalent.

An earlier version also converted the other side, and converted 1SP to 2SP. PROJ does neither, so those were removed.

## Float comparisons

PROJ's tests compare `lat_1 == 25.917499691810534` and `k_0 == 0.9` with `EXPECT_EQ`, which is exact. Both are compared here with a relative tolerance of `1e-14`, because the last digit of `acos`, `sqrt` and `sin` differs between glibc and the Windows CRT. Every other expected value is compared exactly, as in PROJ.

## Verification

- `cargo fmt`: run before the tests.
- `cargo test --all-features --test test_operation_conv`: 21 passed.
