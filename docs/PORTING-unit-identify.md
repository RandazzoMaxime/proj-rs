# Porting sheet: CRS identification against the database (unit lot `identify`)

Branch `unit-fix-identify`, base `unit-core-passing-2` (3ccfd8a). Tests of
PROJ 9.9.0 `test/unit/test_crs.cpp` that identify a CRS with an authority factory.
Expected values come from that file; each one was checked against PROJ 9.9.0.

## Function map (PROJ 9.9.0 `src/iso19111/crs.cpp` -> Rust)

| PROJ | Rust |
|---|---|
| `CompoundCRS::identify` (crs.cpp:5595) | `CompoundCRS::identify_with` (src/iso19111/identify.rs) |
| `CompoundCRS::_identify` (crs.cpp:5789) | `Crs::identify_with` arm `Compound` |
| `createCompoundCRSFromExisting` (factory.cpp:10490) | `AuthorityFactory::create_compound_crs_from_existing` (src/iso19111/factory.rs) |
| `buildSqlLookForAuthNameCode` (factory.cpp:10098) | `auth_name_code_in` (src/iso19111/factory.rs) |
| `createObjectsFromName` for `COMPOUND_CRS` | `AuthorityFactory::create_compound_crs_by_name` |
| `Identifier::isEquivalentName(a, b)` (default `biggerDifferencesAllowed = true`, metadata.cpp:1518) | `equivalent_name_bigger_differences` |
| `CompoundCRS::_isEquivalentTo` (non-strict criterion) | `CompoundCRS::is_equivalent_to_with` (src/iso19111/equiv.rs) |

## Commit 1: compound CRS identification

The compound CRS identify had no arm in `Crs::identify_with` (the result was empty
for every compound). Ported whole: the code match, the name search with its two passes,
the `createCompoundCRSFromExisting` fallback when the CRS has no identifier and no
equivalent name was found, the promotion of a single 90 to 100, and the horizontal and
vertical combination.

Gap (not a deviation of the algorithm): the implicit axis order criterion (crs.cpp:5602)
reads `CRS::hasImplicitCS()` of the first component. This port keeps the flag on
`GeographicCRS::implicit_cs` only, so a projected component reads the flag of its base CRS.
PROJ also reads the projected CRS's own `IMPLICIT_CS` flag, which this port does not model
yet (the projected commit below).

Tests: `tests/unit_crs_identify_core.rs`, `crs.compoundCRS_identify_db` (all blocks pass).
