# Port — planetocentric and west-positive geographic CRS (lot unit-niche-ocentric)

Source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp`, five `TEST()`s of the `niche`
class that stayed open in `docs/PORTING-niche-io-opfactory.md` (section 5, "Spherical
planetocentric / planetographic"). Rust side: `tests/unit_niche_ocentric.rs`.

## 0. Status (round 3)

- 4 tests in the tree, all expected to pass:
  `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west`,
  `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west`,
  `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric`,
  `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west`.
- Round-2 coordinator run (`coordinator-test-round2.txt`): three of the four passed. The
  failing one was the inverse half of `..._ographic_west_to_projected_of_ellipsoidal_ographic_west`.
  Its cause and fix are in section 1.1. The coordinator also reports that eleven other tests of
  the full suite fail on `main` on that machine; they are not in this lot.
- 1 test removed: `createOperation_bound_of_spherical_ocentric_to_same_type` (section 1.5).
- No `#[ignore]`. No expected value was changed: each expected string was checked against
  `test_operationfactory.cpp` (lines 10973-11157 for the four kept tests).
- Nothing was run by the author. Each code commit passed `cargo check -p proj-rs --tests`.

## 1. Failures seen, PROJ's output, cause

### 1.1 Round 2: inverse of `..._ographic_west_to_projected_of_ellipsoidal_ographic_west` (19911 -> 19901)

- Ours (`coordinator-test-round2.txt`, line 7):
  `+inv axisswap -1,2`, `+inv eqc ...`, `unitconvert rad->deg`, `+inv axisswap -2,1`.
- PROJ (test lines 11051-11057): `+inv axisswap -1,2`, `+inv eqc ...`, `unitconvert rad->deg`,
  `axisswap 2,-1`.
- Trace.
  - `createOperation(19911, 19901)` is an `InverseConversion` of the forward conversion
    19901 -> 19911. Its PROJ export is `startInversion(); forward->_exportToPROJString();
    stopInversion()` (`singleoperation.cpp:5415-5420`).
  - The forward export of a geographic source writes that source inverted: `startInversion();
    srcGeodCRS->_exportToPROJString(); stopInversion()` (`conversion.cpp:3908-3915`). The
    geographic export is `unitconvert rad->deg`, `axisswap 2,-1` (`crs.cpp:2446-2475`,
    `addAxisSwap` at `crs.cpp:2416-2440`). Inverted, it is `axisswap 2,-1` stored as inverted.
  - The normal form of an inverted `axisswap 2,-1` is `axisswap -2,1` (`io.cpp:8947-8952`). It
    is applied only when the pipeline is printed, after the inverse has inverted the steps again.
  - Round 1 wrote `axisswap -2,1` directly as a plain step (the shortcut of commit 18c61d7). The
    inverse then gave `+inv axisswap -2,1`, which the normal form rules cannot turn into `2,-1`.
- Fix (commit 510ee50): `projstring.rs`, `source_axis_swap`. The source axis swap is stored as
  `+inv +proj=axisswap +order=2,-1`. The helper writes a plain swap only for the self-inverse
  orders `2,1` and `1,-2`, where the normal form is the same (`io.cpp:8938-8941`). The
  `operation_proj_string` path prints through the formatter when an inverted swap is present, so
  the normal form is applied there too (`projstring.rs`, comment at `operation_proj_string`).
- Hand trace of the final code: stored steps `[+inv axisswap 2,-1, unitconvert deg->rad, eqc,
  axisswap -1,2]`. `stop_inversion` gives `[+inv axisswap -1,2, +inv eqc, unitconvert rad->deg,
  axisswap 2,-1]`. Normalisation and merging leave that list unchanged. This matches PROJ's
  expected string. Not run.

### 1.2 Round 1: forward of 19901 -> 19911 (test 2 of the file)

- Ours: `axisswap +order=2,-1`. PROJ: `axisswap +order=-2,1` (test lines 11038-11044).
- Cause: the same inverted source as 1.1 (`conversion.cpp:3908-3915`).
- Fixed in 18c61d7, and the stored form is now the one of 510ee50.

### 1.3 Round 1: forward of 19901 -> 19912 (test 3 of the file)

- Ours: `Format("the source CRS is not geographic")`. PROJ: `axisswap -2,1`, `unitconvert deg->rad`,
  `eqc +a=2440530 +b=2438260` (test lines 11071-11076).
- Trace. `createOperations(19901, 19912)` goes to `derivedDst` (`coordinateoperationfactory.cpp:3704`),
  the inverse of `createOperations(19912, 19901)`. That runs `createOperationsDerivedTo`
  (5523-5574), which uses the base 19902 (spherical). `createOperationsFromSphericalPlanetocentric`
  (5410-5463) gives `geoc` 19902 -> 19901, and the chain's pairs cancel in the cleanup.
- Fixed in 18c61d7 (`spherical_source_projection`, `inverted_spherical_steps`).

### 1.4 Round 1: `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west`

- Ours: `axisswap 2,1`, `unitconvert deg->rad`, `inv geoc +R=2440530`, `unitconvert rad->deg`,
  `axisswap 2,-1`. PROJ: `axisswap +order=1,-2` (test lines 11014-11015), both directions.
- Trace. The datums are named `Mercury (2015) - Sphere` and `Mercury (2015)`. The non-strict
  name comparison (`common.cpp:930-949`) fails, so the intermediate path is taken
  (`createOperationsFromSphericalPlanetocentric`, 5440-5462). Its first leg is exported with
  `omitProjLongLatIfPossible` (`conversion.cpp:3868`), and `GeodeticCRS::_exportToPROJString` then
  omits `geoc` for a sphere on the zero meridian without TOWGS84 (`crs.cpp:2504-2511`).
- Fixed in 85af950 (`transform_export.rs`, `geographic_geocentric_end`, with
  `projstring::spherical_omits_geoc`).

### 1.5 Round 1: `createOperation_bound_of_spherical_ocentric_to_same_type` (removed)

- Ours: `Format("Invalid nature of source and/or targetCRS for Geographic/Geocentric conversion")`.
- PROJ (test lines 10910-10920): one `helmert +x=-3 +y=-3 +z=-3`, between `push +v_3` and `pop +v_3`,
  with `geoc +ellps=GRS80` before and `geoc +ellps=clrk66` after.
- Cause found, not fixed: the first leg's source is a bound CRS. `Conversion::_exportToPROJString`
  takes the base of a bound CRS (`conversion.cpp:3838-3845`), and the port reads only geodetic
  endpoints (`geographic_geocentric`).
- Why removed. After the base is taken, the expected output still needs a single
  `helmert +x=-3`, from `1 - 4` of the two `+towgs84` values. `createOperationsBoundToBound` with
  equal hubs (`coordinateoperationfactory.cpp:6219-6244`) gives two Helmert steps, and I did not
  find the PROJ step that merges them. The port has no merge either (`pipeline.rs` drops only
  zero Helmert steps). I had no reason to believe the test passes, so the test and its branch
  (`bound_of_spherical_planetocentric_operations`, commit 9ccd5a8, and its dispatch in
  `factory.rs`) are removed (commit cf02b79).
- Next step for a later lot: trace `createOperationsBoundToBound` (6207-6281) and the Helmert
  composition in `io.cpp` (`PROJStringFormatter::toString`). Then port the bound endpoint in
  `geographic_geocentric` (`conversion.cpp:3838-3845`) and the branch.

## 2. Commits on the branch

| Commit | Change | PROJ reference |
|---|---|---|
| 9ccd5a8 | (reverted by cf02b79) bound CRS of a spherical base | `coordinateoperationfactory.cpp:5467-5519` |
| 18c61d7 | `projection_steps`: spherical planetocentric source on a spherical base (`spherical_source_projection`, `inverted_spherical_steps`, `spherical_omits_geoc`); the source axis swap was first written as a plain inverse (superseded by 510ee50) | `conversion.cpp:3908-3915`, `crs.cpp:2416-2440`, `crs.cpp:2504-2511` |
| 85af950 | `geographic_geocentric_end`: a sphere end that `spherical_omits_geoc` writes its angular unit and axis order only | `conversion.cpp:3865-3876`, `crs.cpp:2504-2511` |
| cf02b79 | drop the bound-of-spherical case and revert its branch | section 1.5 |
| 510ee50 | source axis swap stored inverted (`source_axis_swap`), normal form printed | `conversion.cpp:3908-3915`, `singleoperation.cpp:5415-5420`, `io.cpp:8938-8952` |
| (this commit) | sheet | |

## 3. Map PROJ function -> Rust function (current tree)

| PROJ | Rust | State |
|---|---|---|
| `createOperations`, spherical source (3659) | `factory.rs` `create_candidates_at`, `geodetic_ops::spherical_planetocentric_operations` | ported before this lot |
| `createOperations`, spherical target (3664) | `factory.rs` `create_candidates_at`, inverse | ported before this lot |
| `createOperationsDerivedTo` (5523) | `derived.rs` `derived_candidates` | ported before this lot, not changed |
| `Conversion::_exportToPROJString`, source written inverted (3908-3915) | `projstring.rs` `projection_steps`, `source_axis_swap`, `inverted_spherical_steps` | **ported here** (18c61d7, 510ee50) |
| `InverseCoordinateOperation::_exportToPROJString` (singleoperation.cpp:5415) | `conversion_export.rs` `export_steps` (`start_inversion` / `stop_inversion`) | ported before this lot, used as is |
| `GeodeticCRS::_exportToPROJString`, sphere omits geoc (crs.cpp:2504-2511) | `projstring.rs` `spherical_omits_geoc`; `transform_export.rs` `geographic_geocentric_end` | **ported here** (18c61d7, 85af950) |
| `createOperationsFromBoundOfSphericalPlanetocentric` (5467) | none | removed (section 1.5) |
| `GeodeticCRS::addAngularUnitConvertAndAxisSwap` (2446) | `transform_export.rs` `angular_unit_and_axis_swap` | ported before this lot, not changed |

## 4. Map PROJ `TEST()` -> Rust test (current tree)

| PROJ `TEST()` (test_operationfactory.cpp) | Rust test (`tests/unit_niche_ocentric.rs`) | Status |
|---|---|---|
| `createOperation_spherical_ocentric_spherical_to_ellipsoidal_north_west` (10973) | `operation_spherical_ocentric_spherical_to_ellipsoidal_north_west` | expected to pass (85af950, section 1.4) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` (11031) | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ographic_west` | expected to pass (18c61d7, 510ee50, sections 1.1, 1.2) |
| `createOperation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` (11064) | `operation_ellipsoidal_ographic_west_to_projected_of_ellipsoidal_ocentric` | expected to pass (18c61d7, 510ee50, section 1.3) |
| `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` (11128) | `operation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ographic_west` | passed in the round-2 run |
| `createOperation_bound_of_spherical_ocentric_to_same_type` (10896) | none (removed) | open, section 1.5 |

Each kept test checks both directions, as its C++ test does. Expected strings are the C++ strings.
Only the first of them is run by the coordinator's round-2 run; "expected to pass" for the others
means the fixes above, which the coordinator must run.

## 5. Deviations

- `spherical_source_projection` takes the ellipsoid and prime meridian of the projected base, and
  writes no `ob_tran` pole step or `+inv +proj=longlat` clause for a spherical source. C++ writes
  those for a geographic source only (`crs.cpp:2481-2513`, `conversion.cpp:3886-3915`).
- `spherical_omits_geoc` needs a datum. Without one the inverted export returns a format error
  (`a spherical CRS has no datum`). C++ reads the ellipsoid through `datumNonNull`.
- `geographic_geocentric_end` applies the `geoc` omission to every EPSG:9602 conversion with a
  spherical end, since the port builds the latitude conversion as 9602
  (`geodetic_ops::geographic_geocentric_conversion`). C++ has a separate latitude method whose
  export sets the omit flag (`conversion.cpp:3813-3868`). Within this lot both coincide.
- `source_axis_swap` writes a plain `2,1` or `1,-2`, not the inverted form. C++ writes the inverted
  form and normalises it to the same text (`io.cpp:8938-8941`), so the output is identical.
- `operation_proj_string` prints through the formatter when a source axis swap is inverted. This
  is the only projected-target path whose output changes (section 7).

## 6. Open items

- Section 1.5: bound CRS of a spherical base (removed test, with the Helmert merge trace).
- Sections 1.1 to 1.4 rely on the pipeline cleanup matching PROJ's `PROJStringFormatter::toString`
  (normal forms, then the merges). The traces were done by reading the code, not by a run.
- The planetographic and planetocentric derived chain (`createOperationsDerivedTo` with a
  spherical base) is matched for the cases of this lot only.

## 7. Callers at risk

- `projstring.rs` `projection_steps`, geographic source: every projection from a geographic CRS
  whose axes are not east, north. Such sources now store the inverted swap. Callers whose output
  changes are those with a west or south first axis; `2,1` is written plain as before. Grep of
  `tests/` and `src/` found no expectation of the old `order=2,-1`/`-2,1` pipelines outside this
  lot's test file and `pipeline/tests.rs` (which tests the cleanup, not this export).
- `operation_proj_string` (used by `select.rs` for a bare conversion, and by `create_operation_proj_string`):
  for a projected target and a geographic source with a west or south axis, the output goes through
  the formatter, so it is normalised. Before, it was the raw join. Not checked by a run.
- `projection_steps` for a spherical source: before, an error (`the source CRS is not geographic`);
  now steps. Any candidate dropped for that reason can now appear.
- `geographic_geocentric_end`: a 9602 conversion with a sphere end on the zero meridian omits
  `geoc`. No test expects a spherical `geoc`; the WGS 84 spherical tests use a non-sphere ellipsoid.

## 8. Verification

```text
cargo fmt -- <touched files>
node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests
  -> job 3841 (18c61d7): succeeded
  -> job 3848 (85af950): succeeded
  -> job 3849 (cf02b79 tree): succeeded
  -> job 3855 and 3856 (510ee50): succeeded, no warning
```

Tests written, not run by the author: the coordinator runs them.
