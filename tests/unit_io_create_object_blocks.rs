//! The object blocks of PROJ 9.9.0 `test/unit/test_io.cpp`, `TEST(io, createFromUserInput)`:
//! the typed OGC URNs (datum, ensemble, meridian, ellipsoid, coordinate operation), the
//! concatenated `urn:ogc:def:coordinateOperation,` list, and the `AUTH:CODE@epoch` coordinate
//! metadata. Lines 13746-13783, 14096-14103 and 14219-14268 of the C++ file.
//!
//! The C++ calls return an object of any kind; `create_object_from_user_input` returns the same
//! objects as [`UserObject`]. Expected values are copied from the C++ test.

use proj_rs::iso19111::{create_object_from_user_input, DatabaseContext, FactoryError, UserObject};

fn object(text: &str, db: &DatabaseContext) -> UserObject {
    create_object_from_user_input(text, db).unwrap_or_else(|error| panic!("{text}: {error}"))
}

#[test]
fn io_create_from_user_input_object_urns() {
    let db = DatabaseContext::create().expect("proj.db");

    // Errors of the `coordinateOperation,` list (lines 13746-13762).
    assert!(create_object_from_user_input(
        "urn:ogc:def:coordinateOperation,coordinateOperation:EPSG::3895,unhandled_type:EPSG::1618",
        &db
    )
    .is_err());
    assert!(matches!(
        create_object_from_user_input(
            "urn:ogc:def:coordinateOperation,coordinateOperation:EPSG::3895,coordinateOperation:EPSG::unexisting_code",
            &db
        ),
        Err(FactoryError::NotFound { .. })
    ));
    assert!(create_object_from_user_input(
        "urn:ogc:def:coordinateOperation,coordinateOperation:EPSG::3895::extra_element,coordinateOperation:EPSG::1618",
        &db
    )
    .is_err());

    // One object of each typed URN (lines 13767-13783).
    assert!(matches!(
        object("urn:ogc:def:coordinateOperation:EPSG::1671", &db),
        UserObject::Operation(_)
    ));
    assert!(matches!(
        object("urn:ogc:def:datum:EPSG::6326", &db),
        UserObject::GeodeticDatum(_)
    ));
    assert!(matches!(
        object("urn:ogc:def:ensemble:EPSG::6326", &db),
        UserObject::Ensemble(_)
    ));
    assert!(matches!(
        object("urn:ogc:def:meridian:EPSG::8901", &db),
        UserObject::PrimeMeridian(_)
    ));
    assert!(matches!(
        object("urn:ogc:def:ellipsoid:EPSG::7030", &db),
        UserObject::Ellipsoid(_)
    ));
    // A CRS URN is still a CRS.
    assert!(matches!(
        object("urn:ogc:def:crs:EPSG::4326", &db),
        UserObject::Crs(_)
    ));
}

#[test]
fn io_create_from_user_input_concatenated_operation() {
    let db = DatabaseContext::create().expect("proj.db");
    // Lines 14096-14103.
    match object(
        "urn:ogc:def:coordinateOperation,coordinateOperation:EPSG::3895,coordinateOperation:EPSG::1618",
        &db,
    ) {
        UserObject::Operation(operation) => {
            assert_eq!(operation.name(), "MGI (Ferro) to MGI (1) + MGI to WGS 84 (3)");
        }
        other => panic!("not an operation: {other:?}"),
    }
}

#[test]
fn io_create_from_user_input_coordinate_metadata() {
    let db = DatabaseContext::create().expect("proj.db");
    // Invalid forms (lines 14219-14248).
    for text in [
        "@",
        "ITRF2014@",
        "ITRF2014@foo",
        "foo@2025",
        "@2025",
        "RGF93@2025",
    ] {
        assert!(create_object_from_user_input(text, &db).is_err(), "{text}");
    }
    // The epoch is a decimal year (lines 14249-14268).
    for text in ["ITRF2014@2025.1", "ITRF2014 @ 2025.1", "EPSG:9000 @ 2025.1"] {
        match object(text, &db) {
            UserObject::CoordinateMetadata(metadata) => {
                assert_eq!(metadata.coordinate_epoch(), Some(2025.1), "{text}");
            }
            other => panic!("{text}: not a coordinate metadata: {other:?}"),
        }
    }
}
