//! PROJ 9.9.0 `test/unit/test_c_api.cpp`, `proj_alter_name` and `proj_alter_id`: a geographic
//! CRS with a renamed name, a deprecated flag from the " (deprecated)" suffix, and a replaced
//! identifier.

use proj_rs::iso19111::{EllipsoidalCS, GeographicCRS, UnitOfMeasure};

fn wgs84() -> GeographicCRS {
    let base = GeographicCRS::epsg_4326();
    GeographicCRS::create(
        "WGS 84",
        base.datum().cloned(),
        None,
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    )
    .expect("WGS 84")
}

/// `TEST_F(CApi, proj_alter_name)`
#[test]
fn capi_alter_name() {
    let obj = proj_rs::iso19111::Crs::Geographic(wgs84());

    let altered = obj.alter_name("new name");
    assert_eq!(altered.name(), "new name");
    assert!(!altered.is_deprecated());

    let altered = obj.alter_name("new name (deprecated)");
    assert_eq!(altered.name(), "new name");
    assert!(altered.is_deprecated());
}

/// `TEST_F(CApi, proj_alter_id)`
#[test]
fn capi_alter_id() {
    let obj = proj_rs::iso19111::Crs::Geographic(wgs84());

    let altered = obj.alter_id("auth", "code");
    let identifier = altered.authority_code("auth").expect("auth:code");
    assert_eq!(identifier.code(), "code");

    let altered2 = altered.alter_id("auth2", "code2");
    let identifier = altered2.authority_code("auth2").expect("auth2:code2");
    assert_eq!(identifier.code(), "code2");
    assert!(altered2.authority_code("auth").is_none());
}
