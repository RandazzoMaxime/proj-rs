# Port — planetocentric and west-positive geographic CRS (lot unit-niche-ocentric)

Source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp`, five `TEST()`s of the `niche`
class that stayed open in `docs/PORTING-niche-io-opfactory.md` (section 5, "Spherical
planetocentric / planetographic"). Rust side: `tests/unit_niche_ocentric.rs`.

Status: one branch ported (test 1), four tests written and not traced to a fix (tests 2 to 5).
Nothing was measured by the author: every test below is "unverified" until the coordinator
runs it.

## 1. PROJ functions read

| PROJ file | Lines | Role |
|---|---|---|
| `test/unit/test_operationfactory.cpp` | 10896-10921 | `createOperation_bound_of_spherical_ocentric_to_same_type` |
| same | 10973-11025 | `createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` |
| same | 11031-11058 | `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` |
| same | 11064-11089 | `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` |
| same | 11126-11157 | `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` |
| `src/iso19111/operation/coordinateoperationfactory.cpp` | 3640-3700 | `createOperations`: registry, spherical source and target, bound source and target |
| same | 3704-3707 | `derivedDst`: inverse of the reverse call |
| same | 5410-5463 | `createOperationsFromSphericalPlanetocentric` |
| same | 5467-5519 | `createOperationsFromBoundOfSphericalPlanetocentric` |
| same | 5523-5550 | `createOperationsDerivedTo` (base of a projected source) |
| same | 6207-6244 | `createOperationsBoundToBound`, same geographic hub |
| `src/iso19111/crs.cpp` | 1937-1950 | `GeodeticCRS::isSphericalPlanetocentric` |
| same | 2446-2475 | `GeodeticCRS::addAngularUnitConvertAndAxisSwap` |
| same | 2481-2532 | `GeodeticCRS::_exportToPROJString`, spherical branch (`omitProjLongLatIfPossible`) |

## 2. Map PROJ function -> Rust function

| PROJ | Rust | State |
|---|---|---|
| `createOperations`, spherical source (3659) | `factory.rs` `create_candidates_at`, `geodetic_ops::spherical_planetocentric_operations` | ported before this lot |
| `createOperations`, spherical target (3664) | `factory.rs` `create_candidates_at`, inverse | ported before this lot |
| `createOperationsFromBoundOfSphericalPlanetocentric` (5467) | `geodetic_ops.rs` `bound_of_spherical_planetocentric_operations` | **ported here** |
| `createOperations`, boundSrc with spherical base (3676-3684) | `factory.rs` `create_candidates_at`, call to the function above | **ported here** |
| `createOperations`, boundDst with spherical base, source not bound (3685-3692) | `factory.rs` `create_candidates_at`, inverse of `create_candidates(target, source)` | **ported here** |
| `createOperationsBoundToBound`, same hub (6219-6244) | `bound_ops.rs` `bound_to_bound` | ported before this lot, not changed |
| `createOperationsDerivedTo` (5523) | `derived.rs` `derived_candidates` | ported before this lot, not changed |
| `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (2446) | `transform_export.rs` `angular_unit_and_axis_swap` | ported before this lot, not changed |

## 3. Map PROJ `TEST()` -> Rust test

| PROJ `TEST()` | Rust test (`tests/unit_niche_ocentric.rs`) | Status |
|---|---|---|
| `createOperation_bound_of_spherical_ocentric_to_same_type` | `operation_bound_of_spherical_ocentric_to_same_type` | unverified (see 5.1) |
| `createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | unverified, open (see 5.2) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | unverified, open (see 5.3) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | unverified, open (see 5.3) |
| `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | unverified, open (see 5.3) |

The expected strings and WKT are copied from the C++ file. Each test checks both directions,
as the C++ test does, except test 1, which checks the forward direction only, as its C++ test.

## 4. Changes

- `geodetic_ops.rs`: `bound_of_spherical_planetocentric_operations`. It returns `None` unless
  the source is a bound CRS whose base is a spherical planetocentric CRS. The steps follow
  the C++ function:
  1. the intermediate geographic CRS `<base name> (geographic)`, on the datum of the base,
     in latitude/longitude degrees, with the domains of the base (`addDomains`);
  2. the binding transformation, cloned, with its source set to the intermediate CRS
     (`setCRSs(transf, intermGeog, transfTarget)`), and the bound CRS on the intermediate
     CRS and the hub of the source;
  3. the first step `geographic_geocentric_conversion(base, intermediate)` from the source
     bound CRS to the intermediate bound CRS;
  4. the candidates from the intermediate bound CRS to the target, each one with its
     endpoints set (`setCRSs(opSecondClone, intermBoundCRS, targetCRS)`), concatenated after
     the first step.
- `factory.rs` `create_candidates_at`: the call above, placed after the bare spherical checks
  and before `bound_candidates`, as C++ places it before its bound logic. The mirror for a
  bound target with a spherical base takes the inverse of the reverse call, and runs only when
  the source is not a bound CRS (C++ `else if (boundDst)`).
- `tests/unit_niche_ocentric.rs`: five tests, one per open `TEST()`.

## 5. Open items

### 5.1 `createOperation_bound_of_spherical_ocentric_to_same_type` (ported, unverified)

The C++ result is one `helmert +x=-3 +y=-3 +z=-3`. The two bound CRS have 1,2,3 and 4,5,6 to
WGS 84, so `-3 = 1 - 4` is a single Helmert with a push/pop around it. In C++ the same-hub branch
of `createOperationsBoundToBound` builds two operations, the source to WGS 84 and WGS 84 to the
target, and concatenates them. The expected string therefore assumes that the two Helmert
steps are merged when the pipeline is written. This is not traced: the Rust export of a
concatenation of two Helmert steps has to be checked. If it does not merge them, the test
fails on the step list, and the cause is that merge, not the new branch.

Deviation: the target of the binding transformation is the target of the transformation
(`transfTarget`), with the hub as fallback. The intermediate CRS has the domains of the base,
as `addDomains` gives it. The spherical branch (`spherical_planetocentric_operations`) does not
copy them, as its sheet records.

### 5.2 `createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` (open)

The expected string is only `+proj=axisswap +order=1,-2`, in both directions. The source and
target are on the same body with different ellipsoids: a sphere of radius 2440530 (`Mercury
(2015) - Sphere`) and the IAU ellipsoid with inverse flattening 1075.12 (`Mercury (2015)`).
PROJ reaches the axis swap alone, so every other step of the path cancels or is omitted.

The Rust path is `spherical_planetocentric_operations`, with `same_geodetic_datum` as the
C++ `IsSameDatum`. The Rust export of a spherical end writes `geoc` unconditionally
(`transform_export.rs`, `geodetic_export`, the `Geodetic::Spherical` arm). The C++ spherical
export omits `geoc` for a sphere with zero prime meridian and no `towgs84` (`crs.cpp:2504-2511`,
`omitProjLongLatIfPossible`). Whether that C++ rule also applies to the conversion's end is not
traced here. This is the first thing to check. Next step: print the Rust steps for both
directions, then compare with the steps of the C++ `createOperationsFromSphericalPlanetocentric`
on the same pair.

### 5.3 The four IAU planetographic/planetocentric projected cases (open)

IAU_2015 `19901` is ellipsoidal with latitude and west-positive longitude. `19902` is
spherical planetocentric (`GEODCRS`, no PROJ string export: `crs.cpp:2515`). `19911` is an
equirectangular projection on `19901`. `19912` is the same projection on `19902`.

- `19901 -> 19911` (test 3) is the deriving conversion of the target, with the source equal to
  its base. The expected string has the west swap `-2,1` before and `-1,2` after the projection.
- `19901 -> 19912` (test 4) and `19902 -> 19911` (test 5): the target base is not the source,
  so PROJ goes through `derivedDst` (3704), the inverse of the reverse call, and then
  `createOperationsDerivedTo` (5523), with the base of the projected CRS. The expected strings
  show that the planetocentric base of `19912` adds no geocentric step on the ographic side
  (test 4), and that the ocentric side of test 5 does add `inv geoc`.

What is not known: whether the Rust derived path (`derived.rs`, `derived_candidates`) takes
the spherical base of a projected CRS through `spherical_planetocentric_operations` as the
C++ `createOperationsDerivedTo` does, and whether the west axis of the base gives the same
`axisswap` steps. Neither was traced, and no change was made for these tests.

Existing test `operation_create_operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric`
(`tests/unit_opfac_core_c_bound3.rs`) covers the `19902 -> 19912` pair, which passes per its
sheet (`docs/PORTING-lotc-bound3.md`). Its path is the one to compare against for tests 4 and 5.

## 6. Callers at risk

- `factory.rs` `create_candidates_at`: the new call applies to any source that is a bound CRS
  with a spherical planetocentric base, and the mirror to any non-bound source with a bound
  target of that kind. Grep of `tests/` and `src/iso19111/` finds no existing test with a bound
  CRS on a spherical base: the only spherical PROJ strings in the tests are
  `tests/unit_op_spherical_ocentric.rs` (no bound CRS) and `tests/unit_opfac_core_c_bound3.rs`
  (projected CRS on a spherical base, no bound CRS). So no passing test should change, but
  this is by grep, not by run.
- `bound_ops.rs` `bound_candidates`: no longer reached for a bound CRS with a spherical base.
- `geodetic_ops.rs` `spherical_planetocentric_operations`: unchanged. It is the one that the
  intermediate of test 1 is built like.

## 7. Verification

```text
cargo fmt -- src/iso19111/geodetic_ops.rs src/iso19111/factory.rs tests/unit_niche_ocentric.rs
node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests
  -> job 3823: succeeded in 23 s, no warning in the log
```

Tests written, not run by the author: the coordinator runs them.
