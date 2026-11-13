# Audit — unit-core (`test_datum.cpp`, `test_common.cpp`, `test_metadata.cpp`)

Sources, PROJ 9.9.0, read-only: `src/src/test/unit/test_datum.cpp` (22 `TEST()`),
`test_common.cpp` (13), `test_metadata.cpp` (12). Total 47. Behaviour checked in
`src/src/iso19111/datum.cpp`, `common.cpp` and `metadata.cpp`, and against the reference
binaries (`target/proj-reference/build/lib/libproj.so`, through a small C++ probe linked to it).

Starting point: `PORT-NOTES-tests.md` lists none of these 47 names, and `docs/UNPORTED-UNIT.md`
has no section for these three files. Before this lot no Rust test asserted these bodies.
`tests/unit_metadata_extent.rs` covered only the intersection boxes of `TEST(metadata, extent)`.

Columns: **before** is the state at the start of the lot. **Rust test** is the test that closes
the row, in `tests/unit_core_datum.rs`, `tests/unit_core_common.rs` or `tests/unit_core_metadata.rs`.

## test_datum.cpp (22)

| C++ `TEST()` | Before | Rust test | Notes |
|---|---|---|---|
| `datum::ellipsoid_from_sphere` | gap: no sphere constructor with a body | `ellipsoid_from_sphere` | note 1 (`semiMedianAxis`), note 2 (unrelated object) |
| `datum::ellipsoid_non_earth` | gap: no body argument | `ellipsoid_non_earth` | |
| `datum::ellipsoid_from_inverse_flattening` | gap: no criterion equivalence on `Ellipsoid`, no public PROJ export | `ellipsoid_from_inverse_flattening` | |
| `datum::ellipsoid_from_null_inverse_flattening` | gap: `Scale(0)` kept an inverse flattening of 0 | `ellipsoid_from_null_inverse_flattening` | fixed in `Ellipsoid::create` |
| `datum::ellipsoid_from_semi_minor_axis` | gap: as above | `ellipsoid_from_semi_minor_axis` | note 1 |
| `datum::prime_meridian_to_PROJString` | gap: the PROJ-string helper was private | `prime_meridian_to_proj_string` | `+proj=noop` for an empty export, as PROJ |
| `datum::prime_meridian_to_JSON` | gap: no `PrimeMeridian` PROJJSON export | `prime_meridian_to_json` | takes a `DatabaseContext`, as the Ellipsoid export does |
| `datum::datum_with_ANCHOR` | not ported | `datum_with_anchor` | |
| `datum::datum_with_ANCHOREPOCH` | not ported | `datum_with_anchor_epoch` | the epoch is in years, as `with_anchor_epoch` already takes it |
| `datum::unknown_datum` | gap: no criterion equivalence; the `unknown` rule is in the datum path only | `unknown_datum` | |
| `datum::compare_with_D_prefixing` | gap: as above, and the `D_` rule | `compare_with_d_prefixing` | |
| `datum::dynamic_geodetic_reference_frame` | gap: no deformation-model setter | `dynamic_geodetic_reference_frame` | note 3 (no `DynamicGeodeticReferenceFrame` type) |
| `datum::ellipsoid_to_PROJString` | gap: a `km` unit could not be given | `ellipsoid_to_proj_string` | `Ellipsoid::create` takes a `Measure` |
| `datum::temporal_datum_WKT2` | gap: no WKT export for `TemporalDatum` | `temporal_datum_wkt2` | note 4 (`DateTime` is text) |
| `datum::temporal_datum_time_origin_non_ISO8601` | gap: as above | `temporal_datum_time_origin_non_iso8601` | |
| `datum::temporal_datum_WKT2_2019` | gap: as above | `temporal_datum_wkt2_2019` | |
| `datum::dynamic_vertical_reference_frame` | gap: no deformation-model setter, no criterion equivalence | `dynamic_vertical_reference_frame` | note 3 |
| `datum::datum_ensemble` | gap: positional accuracy kept as text in C++ | `datum_ensemble` | note 5 |
| `datum::datum_ensemble_vertical` | not ported | `datum_ensemble_vertical` | |
| `datum::datum_ensemble_exceptions` | gap: a single member, or members with another ellipsoid or prime meridian, were accepted | `datum_ensemble_exceptions` | fixed in `DatumEnsemble::create` |
| `datum::edatum` | gap: no anchor setter | `edatum` | |
| `datum::pdatum` | gap: no anchor setter, no WKT export | `pdatum` | |

## test_common.cpp (13)

| C++ `TEST()` | Before | Rust test | Notes |
|---|---|---|---|
| `common::unit_of_measure` | gap: no public `convertToUnit`, no unit equality, no `radian`/`year` | `unit_of_measure` | units compare by name, as C++ |
| `common::measure` | gap: no equality | `measure` | `Measure::new(value, UnitOfMeasure::none())` for `Measure(double)` |
| `common::identifiedobject_empty` | gap: no remarks, aliases, deprecation or identifier accessors on `OperationParameter` | `identifiedobject_empty` | |
| `common::identifiedobject` | gap: as above | `identifiedobject` | typed builder in place of `PropertyMap` (note 6) |
| `common::identifiedobject_name_invalid_type_integer` | route: PROJJSON reader | `identifiedobject_name_invalid_type_integer` | note 6 |
| `common::identifiedobject_name_invalid_type_citation` | route: PROJJSON reader | `identifiedobject_name_invalid_type_citation` | note 6 |
| `common::identifiedobject_identifier_invalid_type` | route: PROJJSON reader | `identifiedobject_identifier_invalid_type` | note 6 |
| `common::identifiedobject_identifier_array_of_identifier` | gap: no identifier list on `OperationParameter` | `identifiedobject_identifier_array_of_identifier` | typed builder |
| `common::identifiedobject_identifier_array_of_invalid_type` | route: PROJJSON reader | `identifiedobject_identifier_array_of_invalid_type` | note 6 |
| `common::identifiedobject_alias_array_of_string` | gap: no aliases on `Identified` | `identifiedobject_alias_array_of_string` | typed builder |
| `common::identifiedobject_alias_invalid_type` | **not written** | none | note 4 |
| `common::identifiedobject_alias_array_of_invalid_type` | **not written** | none | note 4 |
| `common::DataEpoch` | gap: no `DataEpoch` type | `data_epoch` | |

## test_metadata.cpp (12)

| C++ `TEST()` | Before | Rust test | Notes |
|---|---|---|---|
| `metadata::citation` | gap: no `Citation` type | `citation` | |
| `metadata::extent` | partly ported: only the intersection boxes (`unit_metadata_extent.rs`) | `extent_cases` | note 7 (vertical and temporal elements not compared) |
| `metadata::extent_edge_cases` | gap: no validating constructor, no degenerate-box widening | `extent_edge_cases` | note 8 |
| `metadata::identifier_empty` | gap: no argument-less identifier, no description | `identifier_empty` | `Identifier::create("")` |
| `metadata::identifier_properties` | gap: no description, authority, version builders | `identifier_properties` | |
| `metadata::identifier_code_integer` | route: PROJJSON `id.code` as a number | `identifier_code_integer` | note 6 |
| `metadata::identifier_code_string` | route: PROJJSON `id.code` as text | `identifier_code_string` | note 6 |
| `metadata::identifier_code_invalid_type` | route: PROJJSON `id.code` as a boolean | `identifier_code_invalid_type` | note 6 |
| `metadata::identifier_authority_citation` | gap: no `Citation` | `identifier_authority_citation` | |
| `metadata::identifier_authority_invalid_type` | route: PROJJSON `id.authority` as a boolean | `identifier_authority_invalid_type` | note 6 |
| `metadata::id` | gap: no standalone `ID[...]` parse, no `CITATION`/`URI` export | `id` | |
| `metadata::Identifier_isEquivalentName` | ported with two assertions left out (note 9) | `identifier_is_equivalent_name` | `equivalent_name_bigger_differences`, the C++ two-argument default |

## Notes

1. `semiMedianAxis` (triaxial ellipsoid) is read by PROJ 9.9.0 and written nowhere in its source
   (`grep` of `src/src/iso19111` finds only the accessor and the equivalence code). Every
   ellipsoid PROJ builds has no semi-median axis, so the assertions that it is absent cannot fail.
   The Rust `Ellipsoid` has no field for it; the other assertions of those tests are written.
2. `createUnrelatedObject()` (`test_datum.cpp`) tests a runtime dispatch on the C++ `IComparable`
   base. A Rust `is_equivalent_to` takes its own type, so a different-kind argument does not
   compile. Those lines are not written.
3. There is no `DynamicGeodeticReferenceFrame` or `DynamicVerticalReferenceFrame` type. As in the
   WKT and PROJJSON readers, a dynamic frame is a frame with a frame reference epoch and a
   deformation model, set with `with_frame_epoch` and `with_deformation_model`.
4. Not written: `identifiedobject_alias_invalid_type` and `identifiedobject_alias_array_of_invalid_type`.
   Both check that a `PropertyMap` alias has a text type. The PROJJSON reader does not read
   `alias` (`src/src/iso19111/io.cpp` has no `"alias"` key), so there is no runtime input to
   refuse. In Rust the alias is `&str`, and the wrong type is a compile error.
5. Identifier names (`metadata::Identifier_isEquivalentName`): the C++ cases `"\xc3"` (a lone UTF-8
   lead byte) cannot be a Rust `&str`, so those two assertions are not written.
6. `PropertyMap` values are untyped in C++, and the invalid-type checks run on them. In Rust the
   typed builders take the right type, so the wrong type cannot be given to them. The same checks
   are run on untyped input, the PROJJSON reader, on a `PrimeMeridian` whose base members (`name`,
   `id`) carry the wrong type. The identifier-code cases use the same reader, because a number
   code is text in Rust.
7. `Extent::contains`, `intersects` and `intersection` compare the geographic elements, when each
   side has exactly one, as PROJ does. PROJ also compares the vertical and temporal elements in the
   same cases. The Rust `Extent` keeps them, but does not compare them yet. No assertion in the 47
   tests uses them. This is a remaining gap, not a justified divergence.
8. Degenerate boxes. The factory `GeographicBoundingBox::create` (`metadata.cpp:244`) widens a box
   of zero width or zero height by one ULP on each side (`std::nextafter`), unless it is on the
   world limit. `createFromBBOX` is that factory. Without the widening the strict comparisons of
   `intersects`, `contains` and `intersection` give the wrong answer for a point or a line (the
   reference library answers `true` for a point inside a box, and `false` for a box that only
   touches an edge). `BoundingBox::create` reproduces the factory, and `Extent::from_bbox` uses it.
9. `Identifier_isEquivalentName`: the 2-argument `isEquivalentName` defaults to
   `biggerDifferencesAllowed = true` (`metadata.cpp:1518`). The test calls
   `equivalent_name_bigger_differences`, which is that rule. `equivalent_name` is the exact one.
10. Open point, prime meridian under the strict criterion. `PrimeMeridian::_isEquivalentTo(STRICT)`
    compares the longitude with `Measure::operator==` (value and unit name), so `Degree` and
    `degree` differ. The Rust strict check keeps a `1e-12` SI tolerance: the CRS layer
    (`same_geographic_base`, the database identification, GDA2020 in `test_crs.rs`) relies on it.
    Making the strict check exact needs those callers to compare under `EQUIVALENT`, which is what
    PROJ does in `isSameGeodeticDatum`. That change is not made in this lot.
