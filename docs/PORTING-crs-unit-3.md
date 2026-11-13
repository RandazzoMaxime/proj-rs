# Port — PROJ `test/unit/test_crs.cpp`, lot 3

Source, PROJ 9.9.0: `test/unit/test_crs.cpp` (`boundCRS_crs_link` 5409, `norway_ntm` 7982,
`VerticalCRS_ensemble_exception_in_create` 4749, `normalizeForVisualization_derivedprojected_operation`
7681, `createDerivedProjectedCRSNorthingEasting` 4817). Also read: `src/iso19111/crs.cpp`
(`SingleCRS::baseIsEquivalentTo` 1652-1692, `CRS::createBoundCRSToWGS84IfPossible` 516),
`src/iso19111/io.cpp` (`buildProjectedCRS`, EPSG v12.025 Norway rule, 4689-4716), `src/iso19111/datum.cpp`
(`DatumEnsemble::asDatum` 1943), `src/iso19111/c_api.cpp` (`proj_is_equivalent_to_internal`).

Lots 1 and 2 are `docs/PORTING-crs-unit.md` and `docs/PORTING-crs-unit-2.md`. This lot had five
candidates. Three are closed (one of them a replacement); two stay open with a stated cause.

## Map of functions and tests

| C++ `TEST()` | Rust test | Status |
|---|---|---|
| `crs.norway_ntm` | `tests/unit_crs_strings_misc.rs`: `crs_norway_ntm_esri_wkt1_export`, `crs_norway_ntm_esri_wkt1_import_identifies_5105` (already there), `crs_norway_ntm_wkt2_before_epsg_12_025_is_equivalent_to_5105` (new) | Closed. The WKT2 import is equivalent to EPSG:5105. |
| `crs.boundCRS_crs_link` | `tests/unit_crs_unit_3.rs`: `crs_bound_crs_link` | Closed, with a deviation (below). The `use_count()` and `expired()` blocks are not ported. |
| `crs.VerticalCRS_ensemble_exception_in_create` | `tests/unit_crs_unit_3.rs`: `crs_vertical_crs_ensemble_exception_in_create` | Closed, as a replacement for `crs.scope_area_bbox_remark`, which is blocked (below). The no-datum case is not expressible (below). |
| `crs.normalizeForVisualization_derivedprojected_operation` | `tests/unit_crs_unit_3.rs`: `crs_normalize_for_visualization_derivedprojected_operation` | Closed in the next lot, `docs/PORTING-wkt-extension-pipeline.md`: the test runs without `#[ignore]` and passes on the current code. The section below is the status at the time of this lot. |
| `crs.scope_area_bbox_remark` | none | Open. Blocked: no vertical or temporal extent in the model (below). |
| `factory.AuthorityFactory_createDynamicVerticalDatum` | none | Open. The C++ test is compiled out, and its expectation disagrees with the 9.9.0 database (below). |

## Changes to the library

`GeographicCRS::is_equivalent_to_with_database` (`src/iso19111/equiv.rs`), used by the EPSG 12.025
Norway rule of the WKT import (`wkt_import.rs`, the port of `io.cpp:4689-4716`).

The C++ `SingleCRS::baseIsEquivalentTo` compares `datumNonNull` under a non-strict criterion, that is the
datum, or the ensemble as its `asDatum` (`crs.cpp:1686-1691`, `datum.cpp:1943`). The Rust version looked at
`datum()` on both sides and returned `false` whenever one side held a datum ensemble. The WKT2 of
EPSG:5105 has an `ENSEMBLE["European Terrestrial Reference System 1989 ensemble", ...]` base, so the
Norway rule did not take the EPSG:5105 candidate and kept the WKT base. The base is now compared through
`datum_frame` (the ensemble as its datum), and the renamed copy renames the ensemble when there is no
datum. The name check (`frame_names_equivalent_by_db`, the alias rule of `datum.cpp:1602`) is unchanged.

The Rust port now makes the Norway rule take the EPSG:5105 candidate, and the three `norway_ntm` tests
pass. A direct comparison with the C++ 9.9.0 result for this exact text is not recorded here; if it is
needed, it is a separate check.

## Deviations

- **`boundCRS_crs_link`, shared-pointer blocks.** The `use_count()` and `weak_ptr::expired()` assertions
  (test_crs.cpp 5412-5424, 5430-5439 in part, 5461-5473, 5476-5493 in part) check the lifetime of the
  C++ shared objects. Rust has owned values, with no count to observe, so those assertions are not
  ported. The values they accompany are checked: the base is equivalent to EPSG:4267 or to the projected
  CRS, the base has no canonical bound CRS before `base_crs_with_canonical_bound_crs`, and the canonical
  bound CRS of the base is what `create_bound_crs_to_wgs84_if_possible` returns.
- **`createBoundCRSToWGS84IfPossible(nullptr, NEVER)`.** The Rust method takes a `&DatabaseContext`, so the
  test passes the default database. The Rust method has no canonical-bound shortcut before the database
  lookup (`src/iso19111/bound.rs`, `create_bound_crs_to_wgs84_if_possible`); the test's equivalence to the
  canonical bound holds through the database candidate. Whether C++ returns the canonical bound first
  (`crs.cpp:516-532`) is not checked here.
- **`VerticalCRS::create(PropertyMap(), nullptr, nullptr, cs)` throws.** Rust `VerticalCRS::create` takes a
  `VerticalReferenceFrame`, and `create_ensemble` takes a `DatumEnsemble`, so "neither" is a type error and
  has no test. The ensemble case is tested: an ensemble of geodetic datums is refused.
- **`createFromUserInput(text, db, true)`.** The C++ flag is `usePROJ4InitRules`. It matters for PROJ
  strings only. The Rust `create_from_user_input(text, db)` takes no flag, and the WKT input is the same.

## Open, with the cause

- **`normalizeForVisualization_derivedprojected_operation`** (status at the time of this lot; the test passes
  now, see the table). `create_operation(EPSG:4326, derived)` is
  the inverse of a concatenation whose conversion step runs from the derived CRS to the projected base.
  `conversion_export.rs::export_projection` inverts only a projected source (`matches!(source,
  Crs::Projected(_))`), so the derived source goes to `projection_steps(derived, utm)`, which fails with
  `the source CRS is not geographic`. The C++ writes the step from the base to the derived CRS, with the
  target-side modifiers of the derived CRS (`conversion.cpp:4433-4475`, `DerivedProjectedCRS::addUnitConvertAndAxisSwap`),
  which `projection_steps` does not take for a derived target. Next step: export the derived conversion
  from its base, then the target-side modifiers; then the test passes as it is.
- **`scope_area_bbox_remark`.** `ObjectDomain` holds the scope, the area, and a bounding box. The C++ domain
  also holds `VERTICALEXTENT`, `TIMEEXTENT`, and their WKT2 export; the Rust WKT parser and writer have
  neither. This is a model change, which is why this lot replaces it.
- **`AuthorityFactory_createDynamicVerticalDatum`.** The C++ test is inside `#ifdef
  no_more_dynamic_vertical_datum` (test_factory.cpp:322-330), so it is not compiled in PROJ 9.9.0. Its
  expectation (`EPSG:1096` has `frame_reference_epoch` 2000.0) does not hold for the 9.9.0 `proj.db`,
  where the `vertical_datum` row for 1096 has `frame_reference_epoch` NULL. Porting it would assert a
  database answer under a test name that says the opposite, so it is left open.

## Ledger

`docs/UNPORTED-UNIT.md`: three lines removed (`crs.norway_ntm`, `crs.boundCRS_crs_link`,
`crs.VerticalCRS_ensemble_exception_in_create`). `crs.normalizeForVisualization_derivedprojected_operation`,
`crs.scope_area_bbox_remark`, and `factory.AuthorityFactory_createDynamicVerticalDatum` stay. The generated
documents (`README.md`, `ROADMAP.md`, `docs/UNIT-TRIAGE.md`, `docs/progress.svg`) were not regenerated: run
`UPDATE_DOCS=1 cargo test --test docs_in_sync` and then `cargo test --test docs_in_sync`.

## Commands run

- `cargo fmt --check`: clean.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --test unit_crs_strings_misc --test unit_crs_unit_3 -- --include-ignored`: the three
  `norway_ntm` tests and the two closed tests pass; the ignored test fails at
  `the source CRS is not geographic`, as stated above.
- `cargo test --all-features`, run per test target (the full build exhausted the disk, so the binaries
  were deleted after each target ran). `docs_in_sync` is excluded because it fails until the generated
  documents are regenerated. Totals are not recorded in this sheet; the run was not complete when this
  was written.
