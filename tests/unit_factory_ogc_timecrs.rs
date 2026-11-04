//! PROJ 9.9.0 `test/unit/test_factory.cpp`, the OGC temporal CRS of the OGC authority.

use proj_rs::iso19111::{AuthorityFactory, DatabaseContext};

/// `TEST(factory, ogc_timecrs)`
#[test]
fn factory_ogc_timecrs() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "OGC");
    factory
        .create_coordinate_reference_system("AnsiDate")
        .expect("AnsiDate");
    factory
        .create_coordinate_reference_system("JulianDate")
        .expect("JulianDate");
    factory
        .create_coordinate_reference_system("UnixTime")
        .expect("UnixTime");
}
