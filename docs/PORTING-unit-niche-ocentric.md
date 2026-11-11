# Port — planetocentric and west-positive geographic CRS (lot unit-niche-ocentric)

Source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp`, five `TEST()`s of the `niche`
class that stayed open in `docs/PORTING-niche-io-opfactory.md` (section 5, "Spherical
planetocentric / planetographic"). Rust side: `tests/unit_niche_ocentric.rs`.

Status after round 2 (coordinator run of round 1 is the reference for the failures below):

- 4 tests in the tree: `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west`,
  `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west`,
  `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric`,
  `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west`.
- 1 test removed: `createOperation_bound_of_spherical_ocentric_to_same_type` (section 4).
  Its branch, `bound_of_spherical_planetocentric_operations`, is reverted.
- No test is `#[ignore]`d. Expected strings are the C++ strings (no change needed: the
  copies were checked against `test_operationfactory.cpp` lines 10973-11157).

Nothing was run by the author: every test below is unverified until the coordinator runs it.

## 1. Round 1 failures: outputs, PROJ, cause

Output of the coordinator run (`coordinator-test-round1.txt`), PROJ's expected output, and
the trace in the PROJ 9.9.0 sources. Paths are under `src/iso19111/` of PROJ.

### 1.1 `..._ographic_west_to_projected_of_ellipsoidal_ographic_west` (19901 -> 19911, forward)

- Ours: `axisswap +order=2,-1`, `unitconvert deg->rad`, `eqc`, `axisswap -1,2`.
- PROJ (test line 11038-11044): `axisswap +order=-2,1`, `unitconvert deg->rad`, `eqc`, `axisswap -1,2`.
- Cause: `Conversion::_exportToPROJString` writes the source geographic CRS before the
  projection, inverted (`formatter->startInversion(); srcGeodCRS->_exportToPROJString();
  stopInversion()`, `operation/conversion.cpp:3908-3915`). The geographic export is
  `unitconvert rad->deg` then `axisswap 2,-1` (`crs.cpp:2446-2475`, `addAxisSwap` at
  `crs.cpp:2416-2440`). Reversed and inverted, it is `axisswap -2,1` then `unitconvert deg->rad`.
  The inverse of `2,-1` is `-2,1`, which the port did not compute.
- Fixed in 18c61d7 (`projstring.rs`, `projection_steps`, `inverse_axis_order`).

### 1.2 `..._ographic_west_to_projected_of_ellipsoidal_ocentric` (19901 -> 19912, forward)

- Ours: `Format("the source CRS is not geographic")` from `projection_steps`.
- PROJ (test lines 11071-11076): `axisswap -2,1`, `unitconvert deg->rad`, `eqc +a=2440530 +b=2438260`.
- Trace. `createOperations(19901, 19912)` goes to `derivedDst` (`coordinateoperationfactory.cpp:3704`):
  inverse of `createOperations(19912, 19901)`. That is `createOperationsDerivedTo` (5523-5574),
  which adds the inverse deriving conversion (19912 -> 19902) and the operations from the base
  19902 (spherical, planetocentric) to 19901. The spherical branch
  (`createOperationsFromSphericalPlanetocentric`, 5410-5463) gives `geoc` 19902 -> 19901, since
  both datums are `Mercury (2015)` (`iau.sql:128-129`). `applyInverse` then gives
  `[inv geoc, eqc (source 19902)]`. The eqc export takes its source as a spherical CRS:
  inverted spherical export with `geoc +a +b` (`conversion.cpp:3908-3915`). The chain's
  pairs cancel in the pipeline cleanup: `geoc`/`inv geoc`, `unitconvert rad->deg` /
  `unitconvert deg->rad`, `axisswap 2,1` / `axisswap 2,1`. What remains is PROJ's string.
- Fixed in 18c61d7 (spherical source branch: `spherical_source_projection`,
  `inverted_spherical_steps`). The cancellation was not checked by a run.

### 1.3 `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west` (forward)

- Ours: `axisswap 2,1`, `unitconvert deg->rad`, `inv geoc +R=2440530`, `unitconvert rad->deg`, `axisswap 2,-1`.
- PROJ (test lines 11014-11015): `axisswap +order=1,-2` (both directions).
- Trace. Datums differ by name: `Mercury (2015) - Sphere` and `Mercury (2015)`. The non-strict
  name comparison (`common.cpp:930-949`) fails, so `IsSameDatum` is false and the path is the
  intermediate one (`createOperationsFromSphericalPlanetocentric`, 5440-5462). The first leg,
  the latitude conversion spherical -> interm, is exported with `omitProjLongLatIfPossible`
  set (`conversion.cpp:3868`), and `GeodeticCRS::_exportToPROJString` then omits `geoc` for a
  sphere on the zero meridian without TOWGS84 (`crs.cpp:2504-2511`). Our first leg wrote
  `geoc +R`. With that step omitted, the cleanup merges the axis swaps and cancels the unit
  pairs, which gives `axisswap 1,-2`.
- Fixed in 85af950 (`transform_export.rs`, `geographic_geocentric_end`, with
  `projstring::spherical_omits_geoc`).

### 1.4 `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` (19902 -> 19911)

Passed in round 1. Commits 18c61d7 and 85af950 change code on its path (projection source
steps for a west source, spherical omission). Its strings are unchanged in the cases the
coordinator saw, but it must be run again.

### 1.5 `createOperation_bound_of_spherical_ocentric_to_same_type` (removed)

- Ours: `Format("Invalid nature of source and/or targetCRS for Geographic/Geocentric conversion")`.
- PROJ (test lines 10910-10920): one `helmert +x=-3 +y=-3 +z=-3` between `push +v_3`/`pop +v_3`,
  with `geoc +ellps=GRS80` before and `geoc +ellps=clrk66` after.
- Cause found, not fixed: the first leg's source is a bound CRS. `Conversion::_exportToPROJString`
  takes the base of a bound CRS (`conversion.cpp:3838-3845`), and the port does not
  (`geographic_geocentric` only reads geodetic endpoints). The port had the branch
  `createOperationsFromBoundOfSphericalPlanetocentric` (`coordinateoperationfactory.cpp:5467-5519`).
- Why removed. After the base is taken, the expected output still needs one `helmert +x=-3`
  (1 - 4 from the two `+towgs84`). The path `createOperationsBoundToBound` with equal hubs
  (`coordinateoperationfactory.cpp:6219-6244`) gives two Helmert steps, `+x=1 ...` then
  `+inv +x=4 ...`, and this lot did not find the PROJ step that merges them. The port has no
  merge either (`pipeline.rs`: only zero Helmert steps are dropped). The reason to believe
  the test passes is missing, so the test and its branch (`bound_of_spherical_planetocentric_operations`,
  and its dispatch in `factory.rs`, both from commit 9ccd5a8) are removed.
- Next step, for a later lot: trace `createOperationsBoundToBound` (6207-6281) and the
  `helmert` composition (if any) in `io.cpp` `PROJStringFormatter::toString`. Then port the
  bound endpoint in `geographic_geocentric` (`conversion.cpp:3838-3845`) and the branch.

## 2. Changes in this branch

| Commit | Change | PROJ reference |
|---|---|---|
| 18c61d7 | `projection_steps`: the geographic source is written inverted (`inverse_axis_order`), and a spherical planetocentric source on a spherical base is accepted (`spherical_source_projection`, `inverted_spherical_steps`, `spherical_omits_geoc`) | `conversion.cpp:3908-3915`, `crs.cpp:2416-2440`, `crs.cpp:2504-2511`, `io.cpp:10317-10355` |
| 85af950 | `geographic_geocentric_end`: a spherical end that `spherical_omits_geoc` writes its angular unit and axis order only | `conversion.cpp:3865-3876`, `crs.cpp:2504-2511` |
| (this commit) | removal of the bound-of-spherical case: the function and its dispatch are reverted to the state before 9ccd5a8; the test is removed | section 1.5 |

Earlier commit `922d23f` (sheet) and `9ccd5a8` (branch, now reverted) stay in history.

## 3. Map PROJ function -> Rust function (current tree)

| PROJ | Rust | State |
|---|---|---|
| `createOperations`, spherical source (3659) | `factory.rs` `create_candidates_at`, `geodetic_ops::spherical_planetocentric_operations` | ported before this lot |
| `createOperations`, spherical target (3664) | `factory.rs` `create_candidates_at`, inverse | ported before this lot |
| `createOperationsDerivedTo` (5523) | `derived.rs` `derived_candidates` | ported before this lot, not changed |
| `Conversion::_exportToPROJString`, source geodetic export inverted (3908-3915) | `projstring.rs` `projection_steps`, `inverted_spherical_steps` | **ported here** (18c61d7) |
| `GeodeticCRS::addAxisSwap` inverted (crs.cpp:2416) | `projstring.rs` `inverse_axis_order` | **ported here** (18c61d7) |
| `GeodeticCRS::_exportToPROJString`, sphere omits geoc (crs.cpp:2504-2511) | `projstring.rs` `spherical_omits_geoc`; `transform_export.rs` `geographic_geocentric_end` | **ported here** (18c61d7, 85af950) |
| `createOperationsFromBoundOfSphericalPlanetocentric` (5467) | none | removed (section 1.5) |
| `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (2446) | `transform_export.rs` `angular_unit_and_axis_swap` | ported before this lot, not changed |

## 4. Map PROJ `TEST()` -> Rust test (current tree)

| PROJ `TEST()` | Rust test (`tests/unit_niche_ocentric.rs`) | Status |
|---|---|---|
| `createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | unverified; fix 85af950 (section 1.3) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | unverified; fix 18c61d7 (section 1.1) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | unverified; fix 18c61d7 (section 1.2) |
| `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | passed in round 1; re-run needed (section 1.4) |
| `createOperation_bound_of_spherical_ocentric_to_same_type` | none (removed) | open, section 1.5 |

Each kept test checks both directions, as its C++ test does. Expected strings are C++ strings.

## 5. Deviations

- `spherical_source_projection`: a spherical source takes the ellipsoid and prime meridian of
  the projected base (C++ takes the base's own export). The `ob_tran` pole step and the
  `+inv +proj=longlat` clause for a source on another meridian are not written for a
  spherical source. Only the geographic source takes them.
- `spherical_omits_geoc` needs a datum. Without one the `geoc` step is written, and the
  inverted export returns a format error (`a spherical CRS has no datum`). C++ reads the
  ellipsoid through `datumNonNull`, which the port does not model.
- Omission of `geoc` in `geographic_geocentric_end` applies to every 9602 conversion with a
  spherical planetocentric end, since the port builds the latitude conversion as EPSG:9602
  (`geodetic_ops::geographic_geocentric_conversion`). C++ has a separate latitude method
  whose export sets the omit flag (`conversion.cpp:3813-3868`). Within this lot the only
  spherical ends are built by the planetocentric path, so the two coincide.
- The projection source of a geographic CRS is inverted in every projection export
  (`projection_steps`). PROJ does the same at `conversion.cpp:3908-3915`, so the change is
  faithful, not specific to this lot.

## 6. Open items

- Section 1.5: bound CRS of a spherical base (removed test, with the trace of the Helmert merge).
- Section 1.2 and 1.3 rely on the cleanup of the pipeline cancelling the same pairs as PROJ's
  `PROJStringFormatter::toString`. Not checked by a run.
- The planetographic/planetocentric derived chain (`createOperationsDerivedTo` with a
  spherical base) was matched for one test each, not in general.

## 7. Callers at risk

- `projstring.rs` `projection_steps`: every projection with a geographic source. Only sources
  whose inverted axis order differs from the order (a west or south first axis, such as
  `2,-1` or `-1,2`) change; `2,1` and `1,2` are self-inverse. Grep of `tests/` and `src/`
  finds no expectation of the old order (`order=2,-1` appears only in the new test file and
  in `pipeline/tests.rs`, which tests the cleanup, not this export).
- `projection_steps` for a spherical source: previously an error (`the source CRS is not
  geographic`), now steps. Any candidate that was dropped because of this error can now
  appear. Not checked by a run; the coordinator's full run of `cargo test --all-features`
  is the check.
- `geographic_geocentric_end`: a 9602 conversion with a sphere end on the zero meridian now
  omits `geoc`. Grep of `tests/` finds no expectation of a spherical `geoc`; the WGS 84
  spherical tests use a non-sphere ellipsoid and are unchanged.

## 8. Verification

```text
cargo fmt -- <touched files>
node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests
  -> job 3841 (18c61d7): succeeded, no warning
  -> job 3848 (85af950): succeeded, no warning
  -> job 3849 (final tree, test removed, code reverted): succeeded, no warning
```

Tests written, not run by the author: the coordinator runs them.
