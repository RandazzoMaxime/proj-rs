//! PROJ 9.9.0 `test/unit/test_coordinates.cpp`, `TEST(coordinateMetadata, dynamic_crs)`.
//!
//! A dynamic CRS, here a geographic CRS on a `DynamicGeodeticReferenceFrame` with its anchor and
//! deformation model, takes a coordinate epoch and refuses none. The epoch survives the WKT and
//! PROJJSON round trips.

use proj_rs::iso19111::{
    create_object_from_user_input, CoordinateMetadata, Crs, DynamicGeodeticReferenceFrame,
    Ellipsoid, EllipsoidalCS, Equivalence, GeographicCRS, JsonFormat, PrimeMeridian, UnitOfMeasure,
    UserObject, WktConvention, WktFormatter, WktObject, WktParser,
};

/// The C++ test's geographic CRS on a dynamic frame, with an empty name.
fn make_dynamic_crs() -> Crs {
    let frame = DynamicGeodeticReferenceFrame::create(
        "test",
        Ellipsoid::wgs84(),
        Some("My anchor".to_owned()),
        PrimeMeridian::greenwich(),
        2018.5,
        Some("My model".to_owned()),
    );
    let crs = GeographicCRS::create(
        "",
        Some(frame.into()),
        None,
        EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
    )
    .expect("dynamic geographic CRS");
    Crs::Geographic(crs)
}

#[test]
fn dynamic_crs() {
    let db = proj_rs::DatabaseContext::create().expect("proj.db");
    let crs = make_dynamic_crs();
    let metadata = CoordinateMetadata::create_without_database(&crs, 2023.5).expect("epoch");
    assert!(metadata
        .crs()
        .is_equivalent_to_with(&crs, Equivalence::Strict));
    let epoch = metadata.coordinate_epoch().expect("an epoch");
    assert!((epoch - 2023.5).abs() <= 1e-10, "{epoch}");

    // A dynamic CRS needs an epoch: creating it without one is an error.
    assert!(CoordinateMetadata::create_without_epoch(&crs).is_err());

    // WKT2:2019 round trip. The frame epoch and the model are in the WKT, and the epoch too.
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    let wkt = metadata
        .export_to_wkt(&mut formatter)
        .expect("COORDINATEMETADATA export");
    let parsed = match WktParser::new().create_from_wkt(&wkt).expect("parse") {
        WktObject::CoordinateMetadata(parsed) => parsed,
        other => panic!("not a coordinate metadata: {other:?}"),
    };
    assert!(parsed
        .crs()
        .is_equivalent_to_with(&crs, Equivalence::Strict));
    let parsed_epoch = parsed.coordinate_epoch().expect("an epoch");
    assert!((parsed_epoch - 2023.5).abs() <= 1e-10, "{parsed_epoch}");

    // PROJJSON round trip, compared with the Equivalent criterion as the C++ test does.
    let json = UserObject::CoordinateMetadata(metadata)
        .export_to_json(&db, &JsonFormat::default())
        .expect("PROJJSON export");
    let from_json = match create_object_from_user_input(&json, &db).expect("read") {
        UserObject::CoordinateMetadata(object) => object,
        _ => panic!("PROJJSON did not read as a coordinate metadata"),
    };
    assert!(from_json
        .crs()
        .is_equivalent_to_with(&crs, Equivalence::Equivalent));
    let json_epoch = from_json.coordinate_epoch().expect("an epoch");
    assert!((json_epoch - 2023.5).abs() <= 1e-10, "{json_epoch}");
}
