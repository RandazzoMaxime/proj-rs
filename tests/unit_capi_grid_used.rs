//! PROJ 9.9.0 `test/unit/test_c_api_990.cpp`, `proj_coordoperation_get_grid_used`.

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, DatabaseContext, GridAvailabilityUse,
};

/// `TEST_F(CApi, proj_coordoperation_get_grid_used)`
#[test]
fn capi_proj_coordoperation_get_grid_used() {
    let db = DatabaseContext::create().expect("proj.db");
    let op = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_operation("1312", AlternativeGridNames::Proj)
        .expect("1312");
    let grids = op
        .grids_needed(&db, GridAvailabilityUse::IgnoreGridAvailability)
        .expect("grids");
    assert_eq!(grids.len(), 1);
    assert_eq!(grids[0].short_name, "ca_nrc_ntv1_can.tif");
    assert_eq!(grids[0].info.package_name, "");
    assert!(grids[0].info.direct_download);
    assert!(grids[0].info.open_license);
    assert!(
        grids[0].info.url.ends_with("/ca_nrc_ntv1_can.tif"),
        "{}",
        grids[0].info.url
    );
}
