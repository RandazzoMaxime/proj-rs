# Porting sheet: CRS identification against the database (unit lot `identify`)

Branch `unit-fix-identify`, base `unit-core-passing-2` (3ccfd8a). Tests of PROJ 9.9.0
`test/unit/test_crs.cpp` that identify a CRS with an authority factory:
`crs.compoundCRS_identify_db`, `crs.projectedCRS_identify_db`, `crs.boundCRS_identify_db`.
Expected values come from that file, and each was checked against PROJ 9.9.0
(`projinfo --identify` on the reference build).

## Function map (PROJ 9.9.0 -> Rust)

| PROJ | Rust |
|---|---|
| `CRS::identify` / `CRS::_identify` (crs.cpp:1224, 1562) | `Crs::identify_with` (src/iso19111/identify.rs) |
| `CompoundCRS::identify` (crs.cpp:5595) | `CompoundCRS::identify_with` |
| `ProjectedCRS::identify` (crs.cpp:4858) | `ProjectedCRS::identify_with`, `identify_base`, `identify_utm_zone`, `identify_projected_general`, `add_projected_candidate` |
| `BoundCRS::_identify` (crs.cpp:6236) | `BoundCRS::identify_with` |
| `createCompoundCRSFromExisting` (factory.cpp:10490) | `AuthorityFactory::create_compound_crs_from_existing` |
| `createProjectedCRSFromExisting` (factory.cpp:10149) | `AuthorityFactory::create_projected_crs_from_existing` |
| `buildSqlLookForAuthNameCode` (factory.cpp:10098) | `auth_name_code_in` |
| `createObjectsFromNameEx` (factory.cpp:9363), with the matched name | `objects_from_name_matched`, `create_projected_crs_by_name_matched`, `create_compound_crs_by_name` |
| `Identifier::canonicalizeName` (metadata.cpp:1285) | `canonical_search_name` |
| `Identifier::isEquivalentName(a, b)`, default `biggerDifferencesAllowed = true` | `equivalent_name_bigger_differences` |
| `SingleCRS::baseIsEquivalentTo`, `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (datum.cpp:1602) | `frame_names_equivalent_by_db`, `ProjectedCRS::is_equivalent_to_with_database` (equiv.rs) |
| `Ellipsoid::_isEquivalentTo` non-strict (datum.cpp:1102) | `ellipsoid_equivalent_tolerant`, `ellipsoid_parameters_match` |
| `Transformation::createTOWGS84` / `BoundCRS::createFromTOWGS84` (crs.cpp:5973) | `bind_towgs84` (wkt_import.rs) |

## Commit: compound CRS identification

`CompoundCRS::identify` had no arm in `Crs::identify_with`. Ported whole: the identifier
lookup, the two name passes, the `createCompoundCRSFromExisting` fallback, the promotion of a
single 90 to 100, and the horizontal plus vertical combination.

Test: `tests/unit_crs_identify_core.rs`, `crs.compoundCRS_identify_db`.

## Commit: projected CRS identification

`ProjectedCRS::identify` was a partial port (a UTM branch that PROJ does not have, and a name
search with no `createProjectedCRSFromExisting` fallback). Ported whole, with the UTM branch
of PROJ only (the hardcoded WGS 84, NAD27, and NAD83 zones; a UTM zone on another base goes
through the general search).

Supporting pieces, each because a block of the test needs it:

- `IMPLICIT_CS`: `ProjectedCRS` carries `implicit_cs`, set for a WKT1 `PROJCS` without `AXIS`
  (io.cpp:4800) and for a PROJ string projected CRS (io.cpp:12411).
- `createProjectedCRSFromExisting`, both SQL statements, with the parameter loop that stops at
  the first non-measure value (PROJ's `break`), and the 200-row limits.
- Name search: `canonical_search_name` is PROJ's `canonicalizeName` (the `_IntlFeet` and
  `19dd` rules included, bigger differences on), so `..._IntlFeet` matches its `_Feet_Intl`
  alias as PROJ does.
- Datum equivalence with the database: `frame_names_equivalent_by_db` (the `D_` prefix, the
  official name and aliases of the datum identifier, the aliases by name).
- Tolerances as PROJ has them: ellipsoid parameters to a relative `1e-10` for the non-strict
  criteria, and the lax `Clarke 1880 mod.` rule; conversion parameters to `2e-10`
  (`ParameterValue::_isEquivalentTo`), including the Lambert 2SP standard-parallel exchange.
- Spherical and geocentric bases compare their axes with the criterion (direction and unit
  only for the non-strict criteria), as `CoordinateSystem::_isEquivalentTo` does.
- WKT import, three cases of the test that the identification depends on:
  - a `BASEGEODCRS` without `CS` takes the CS of the database CRS its `ID` names, spherical
    included (io.cpp:3300-3318);
  - an ESRI `_FIPS_` name in US feet is the name of its `_Feet` alias (io.cpp:4736-4757);
  - the angular unit of a projected CRS whose base is a bound CRS is the base's axis unit
    (io.cpp:4800 uses `baseGeodCRS`, not degrees).

Test: `tests/unit_crs_identify_projected.rs`, `crs.projectedCRS_identify_db`. The C++ test
has 37 blocks; the Rust test runs each through `block!` and reports every failing block.

## Commit: bound CRS identification

`BoundCRS::_identify` ported whole (crs.cpp:6236): the hub is EPSG:4326, each identified
base is bound through the operation the database has from it to EPSG:4326 (the
`PARTIAL_INTERSECTION` context), matched by PROJ string or by equivalence, with the
concatenated `longitude rotation + Helmert` case and the `Null geographic offset` case.

Supporting pieces:

- `CoordinateOperation::is_equivalent_to_with`: the non-strict criteria compare a concatenated
  operation step by step, each step as the operation it runs.
- A WKT1 `TOWGS84` is a position vector (EPSG:9606) for every value list: PROJ pads the values
  to seven and `createFromTOWGS84` uses seven values (io.cpp:2486-2505, crs.cpp:5973). The
  import chose geocentric translations (9603) for zero rotations.

Test: `tests/unit_crs_identify_core.rs`, `crs.boundCRS_identify_db`.
