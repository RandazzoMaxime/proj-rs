//! A CRS with a PROJ string extension sends `createOperations` to `createOperationsFromProj4Ext`
//! before anything else (coordinateoperationfactory.cpp:3621-3631), so the celestial body check of
//! `createOperationsGeodToGeod` does not run: a geostationary view on an ellipsoid given in
//! kilometres is reached from WGS 84. Expected values from `projinfo -s ... -t ...` and `cs2cs` of
//! PROJ 9.9.0.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
};

const WGS84: &str = "+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs +type=crs";
const GEOS_KM: &str =
    "+proj=geos +lon_0=0.000000 +lat_0=0 +h=35807.414063 +a=6378.169000 +b=6356.583984 +type=crs";

#[test]
fn wgs84_to_geostationary_in_kilometres() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(WGS84, &db).expect("source");
    let target = create_from_user_input(GEOS_KM, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert_eq!(operations.len(), 1);
    // The PROJ string of PROJ keeps the numbers of the extension as written (`+lon_0=0.000000`):
    // the coordinates are compared instead, from `cs2cs -f %.6f` of PROJ 9.9.0.
    let proj = proj_rs::Proj::new_known_crs(WGS84, GEOS_KM, None).expect("pair");
    let (x, y) = proj.convert((3.23406_f64, 51.04715_f64)).expect("forward");
    assert!((x - 212.623382).abs() < 1e-6, "{x}");
    assert!((y - 4604.975492).abs() < 1e-6, "{y}");
}

#[test]
fn bodies_apart_without_extension_are_still_refused() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(WGS84, &db).expect("source");
    let target = create_from_user_input("+proj=longlat +a=6378.169 +b=6356.583984 +type=crs", &db)
        .expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let error = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect_err("different bodies");
    assert!(error.to_string().contains("same celestial body"), "{error}");
}
