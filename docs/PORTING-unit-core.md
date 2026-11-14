# Port — unit-core (ISO 19111 base objects)

Sources, PROJ 9.9.0, read-only: `src/src/test/unit/test_datum.cpp` (22 `TEST()`),
`test_common.cpp` (13), `test_metadata.cpp` (12); behaviour in `src/src/iso19111/datum.cpp`,
`common.cpp`, `metadata.cpp`. The audit row for each test is in `docs/AUDIT-unit-core.md`.

## Map

| PROJ | PROJ-RS |
|---|---|
| `Ellipsoid::createSphere`, `createFlattenedSphere`, `createTwoAxis` | `Ellipsoid::create(name, Measure, EllipsoidShape, body)`; the `f64` constructors delegate to it |
| `Ellipsoid::_isEquivalentTo` (datum.cpp:1102) | `ellipsoid_equivalent_criterion` (`equiv.rs`) |
| `PrimeMeridian::_exportToPROJString`, `_isEquivalentTo` (datum.cpp:534, 553) | `PrimeMeridian::export_to_proj_string`, `meridian_equivalent_criterion` |
| `PrimeMeridian` PROJJSON | `PrimeMeridian::export_to_json(db, format)`, through `UserObject` |
| `Datum::_isEquivalentTo`, the name rule of `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (datum.cpp:1602, 1657) | `datum_name_equivalent`, `datum_alias_equivalent` |
| `DynamicGeodeticReferenceFrame::_isEquivalentTo` (datum.cpp:1781) and the frame epoch rule | `frame_equivalent_by`, `epoch_equivalent` |
| `VerticalReferenceFrame::_isEquivalentTo` (datum.cpp:2400) | `vertical_equivalent` |
| `DatumEnsemble::create` checks (datum.cpp:2126) | `DatumEnsemble::create` |
| `TemporalDatum::_exportToWKT`, `ParametricDatum`, `EngineeringDatum` anchors | `TemporalDatum::export_to_wkt`, `ParametricDatum::export_to_wkt`, `EngineeringDatum::with_anchor` |
| `GeographicBoundingBox::create` (metadata.cpp:232) | `BoundingBox::create` |
| `Extent::contains`, `intersects`, `intersection`, `_isEquivalentTo`, `createFromBBOX` | `metadata::Extent` (`src/iso19111/metadata.rs`) |
| `metadata::Citation`, `common::DataEpoch` | `Citation`, `DataEpoch` (`metadata.rs`) |
| `Identifier::create`, its getters, `Identifier::isEquivalentName` | `Identifier::create` and `with_*`, `authority`, `version`, `description`, `uri`; `equivalent_name_bigger_differences` |
| `ID[...]` on its own (`WKTParser().createFromWKT`) | `WktObject::Identifier`, `build_standalone_id` (`wkt_import.rs`) |
| `Identifier` WKT export with `CITATION` and `URI` | `Identifier::export_to_wkt` (`wkt.rs`) |
| `Measure::operator==`, `UnitOfMeasure::operator==` (common.cpp:305, 423) | `Measure` derives `PartialEq`; `UnitOfMeasure` compares names |
| `Measure::convertToUnit` | `Measure::convert_to_unit` |

## What was changed outside the new tests

These changes reach the CRS-level equivalence, because `equiv.rs` shares one implementation
for the datum, the CRS and the identification code. They follow PROJ's C++, and the full suite
run is recorded in the commit message.

- `frame_equivalent_by` follows `GeodeticReferenceFrame::_isEquivalentTo` for the names
  (`unknown`, the `D_` prefix, `Identifier::isEquivalentName` with bigger differences), the frame
  epoch (compared only when both frames have one, except under the strict criterion) and the
  prime-meridian longitude (`1e-8` for the other criteria, `1e-12` under the strict one, see the
  divergences). The strict ellipsoid check of the CRS layer is the exact-SI one it had before; the
  C++ strict rule (presence of each parameter) is in `Ellipsoid::is_equivalent_to_with`.
- `vertical_equivalent` compares the names, the realization method and the epoch. The CRS
  vertical-datum check (`vertical_anchor`) keeps name and epoch only.
- `DatumEnsemble::create` refuses one member, and members that do not share an ellipsoid or a
  prime meridian, as PROJ does.
- `BoundingBox::intersects` and `intersection` are unchanged. The degenerate widening lives in
  `BoundingBox::create`, the factory. The DB and WKT readers build boxes without it, as before.
  C++ builds those boxes through the same factory, so that is a remaining difference, not a
  reviewed one. None of the 47 tests reads such a box.
- A `DatumEnsemble` written before WKT2:2019 is written as its datum, as the CRS writers do.
- `Ellipsoid::create` with an inverse flattening of zero is a sphere. The old
  `create_flattened_sphere(…, 0.0)` kept `Some(0.0)`, and now it has no inverse flattening.

## Deliberate divergences

- The strict prime-meridian longitude is compared to `1e-12` in SI, not as `Measure::operator==`
  (value and unit name). The CRS layer compares the same meridian written as `Degree` and as
  `degree`, and the GDA2020 identification test (`test_crs.rs`) depends on it. This applies to
  `PrimeMeridian::is_equivalent_to_with(Strict)` too, so the strict criterion is not exact for
  the meridian (an open point, see `docs/AUDIT-unit-core.md`).
- `ID[...]` is the only standalone identifier this reader accepts. `AUTHORITY[...]` alone stays
  an unsupported object, as `test_c_api.rs` expects.
- The strict criterion does not compare the anchor or the publication date (C++ does, in
  `Datum::_isEquivalentTo`). Comparing them on the CRS path broke the match of a geoid-grid
  compound CRS (`tests/unit_capi_custom_compound_geoid.rs`, 35 m against 35.0000037 m).
- Two frames that share an EPSG code are one datum whatever their names. This was the crate's
  rule before the criteria were ported, and it is kept (`shares_epsg_code`). PROJ does not do it
  on this path.
- `DatumEnsemble::positional_accuracy` is `f64`. PROJ keeps a string, and the test compares the
  number (`100.0`). The WKT still writes the text the database gave, when there is one.
- `Measure(double)`, which PROJ gives the unit `UnitOfMeasure::NONE`, is
  `Measure::new(value, UnitOfMeasure::none())`.
- A `DateTime` is text (`TemporalDatum::origin`). `is_iso_8601` is `DateTime::isISO_8601`.
- Aliases are kept on `Identified` and read by the `OperationParameter` builder only. The
  PROJJSON reader does not read them, as in PROJ (`io.cpp`).
- `Extent` compares the geographic elements only (note 7 of the audit).
- `semiMedianAxis` has no field (note 1 of the audit).

## Commands

```bash
cargo test --test unit_core_datum --test unit_core_common --test unit_core_metadata
cargo fmt --check
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```

The reference oracle for the degenerate boxes is the reference library:

```bash
g++ -w -std=c++17 -I target/proj-reference/src/include -I target/proj-reference/src/src \
    -I target/proj-reference/src -o probe probe.cpp \
    -L target/proj-reference/build/lib -lproj -Wl,-rpath,target/proj-reference/build/lib
PROJ_DATA=target/proj-reference/build/data ./probe
```
