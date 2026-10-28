//! EPSG:4326 from `proj.db`, checked against one WKT2:2019 fragment in
//! PROJ `test/unit/test_crs.cpp` `EPSG_4326_as_WKT2_2019`.

use proj_rs::{AuthorityFactory, DatabaseContext, WktConvention, WktFormatter};

#[test]
fn epsg_4326_from_database_matches_wkt2_2019_fragment() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let wkt = crs
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2:2019");
    let fragment = "    PRIMEM[\"Greenwich\",0,\n        ANGLEUNIT[\"degree\",0.0174532925199433]],\n    CS[ellipsoidal,2],";
    assert!(
        wkt.contains(fragment),
        "EPSG:4326 WKT2:2019 did not contain the PROJ fragment:\n{wkt}"
    );
}
