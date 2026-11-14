//! PROJ 9.9.0 `test/unit/test_coordinates.cpp`, `TEST(coordinateMetadata, static_crs)`.
//!
//! A static CRS takes no coordinate epoch. WGS 84 is the one static CRS that accepts an epoch,
//! as PROJ tolerates it under its dynamic reading. The WKT and PROJJSON round trips keep the CRS
//! and the absence of an epoch.

use proj_rs::iso19111::{
    create_object_from_user_input, CoordinateMetadata, Crs, Equivalence, GeodeticCRS, JsonFormat,
    UnitOfMeasure, UserObject, VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention,
    WktFormatter, WktObject, WktParser,
};
use proj_rs::{AuthorityFactory, DatabaseContext};

fn epsg_4326(db: &DatabaseContext) -> Crs {
    Crs::Geodetic(GeodeticCRS::Geographic(
        AuthorityFactory::create(db, "EPSG")
            .create_geographic_crs("4326")
            .expect("EPSG:4326"),
    ))
}

/// The C++ `createVerticalCRS()`: the Newlyn datum with the ODN height system.
fn vertical_crs() -> Crs {
    let datum = VerticalReferenceFrame::named("Ordnance Datum Newlyn");
    Crs::Vertical(VerticalCRS::create(
        "ODN height",
        datum,
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    ))
}

#[test]
fn static_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = epsg_4326(&db);
    let metadata = CoordinateMetadata::create_without_epoch(&crs).expect("no epoch");
    assert!(metadata
        .crs()
        .is_equivalent_to_with(&crs, Equivalence::Strict));
    assert_eq!(metadata.coordinate_epoch(), None);

    // We tolerate coordinate epochs for EPSG:4326.
    let tolerated = CoordinateMetadata::create_without_database(&crs, 2025.0);
    assert!(tolerated.is_ok());

    // A coordinate epoch should NOT be provided for a vertical CRS.
    assert!(CoordinateMetadata::create_without_database(&vertical_crs(), 2025.0).is_err());

    // WKT2:2019 round trip, through the parser.
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
    assert_eq!(parsed.coordinate_epoch(), None);

    // PROJJSON round trip, through the user-input reader.
    let json = UserObject::CoordinateMetadata(metadata)
        .export_to_json(&db, &JsonFormat::default())
        .expect("PROJJSON export");
    let from_json = match create_object_from_user_input(&json, &db).expect("read") {
        UserObject::CoordinateMetadata(object) => object,
        _ => panic!("PROJJSON did not read as a coordinate metadata"),
    };
    assert!(from_json
        .crs()
        .is_equivalent_to_with(&crs, Equivalence::Strict));
    assert_eq!(from_json.coordinate_epoch(), None);
}
