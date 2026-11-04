//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, operations from and to spherical
//! ocentric (`+geoc`) geographic CRS read from PROJ strings.

use proj_rs::iso19111::{
    create_crs_from_proj_string, CoordinateOperationFactory, Crs, DatabaseContext, GeographicCRS,
    ProjLines,
};

fn crs(definition: &str) -> Crs {
    create_crs_from_proj_string(definition).expect("proj string")
}

/// `TEST(operation, createOperation_spherical_ocentric_to_geographic)`
#[test]
fn operation_spherical_ocentric_to_geographic() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = crs("+proj=longlat +geoc +datum=WGS84 +type=crs");
    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &Crs::Geographic(GeographicCRS::epsg_4326()))
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=geoc +ellps=WGS84 +step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, createOperation_geographic_to_spherical_ocentric)`
#[test]
fn operation_geographic_to_spherical_ocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let dest = crs("+proj=longlat +geoc +datum=WGS84 +type=crs");
    let op = CoordinateOperationFactory::create()
        .create_operation(&Crs::Geographic(GeographicCRS::epsg_4326()), &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=geoc +ellps=WGS84 +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    );
}

/// `TEST(operation, createOperation_spherical_ocentric_to_geocentric)`
#[test]
fn operation_spherical_ocentric_to_geocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = crs("+proj=longlat +geoc +datum=WGS84 +type=crs");
    let dest = crs("+proj=geocent +datum=WGS84 +type=crs");
    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=geoc +ellps=WGS84 +step +proj=cart +ellps=WGS84"
    );
}

/// `TEST(operation, createOperation_spherical_ocentric_to_projected)`
#[test]
fn operation_spherical_ocentric_to_projected() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = crs("+proj=longlat +geoc +datum=WGS84 +type=crs");
    let dest = crs("+proj=utm +zone=11 +datum=WGS84 +type=crs");
    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=geoc +ellps=WGS84 +step +proj=utm +zone=11 +ellps=WGS84"
    );
}

/// `TEST(operation, createOperation_spherical_ocentric_to_projected_of_spherical_ocentric)`
#[test]
fn operation_spherical_ocentric_to_projected_of_spherical_ocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let src = crs("+proj=longlat +geoc +datum=WGS84 +type=crs");
    let dest = crs("+proj=utm +geoc +zone=11 +datum=WGS84 +type=crs");
    let op = CoordinateOperationFactory::create()
        .create_operation(&src, &dest)
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=geoc +ellps=WGS84 +step +proj=utm +zone=11 +ellps=WGS84"
    );
}
