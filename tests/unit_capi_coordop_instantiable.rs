//! PROJ 9.9.0 `test/unit/test_c_api_990.cpp`, `proj_coordoperation_is_instantiable`: the
//! registered EPSG:1671 operation is a PROJ pipeline whose grids are available or known.

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, DatabaseContext, GridAvailabilityUse,
};

/// `TEST_F(CApi, proj_coordoperation_is_instantiable)`
#[test]
fn capi_proj_coordoperation_is_instantiable() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let op = epsg
        .create_coordinate_operation("1671", AlternativeGridNames::Official)
        .expect("EPSG:1671");
    assert!(op.is_proj_instantiable(&db, GridAvailabilityUse::KnownAvailable));
}
