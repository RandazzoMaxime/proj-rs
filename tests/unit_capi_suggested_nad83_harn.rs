//! PROJ 9.9.0 `test/unit/test_c_api_990.cpp`, `proj_get_suggested_operation` for NAD83 (EPSG:4269)
//! to NAD83(HARN) (EPSG:4152), partial intersection, grids ignored: the point (lat 40, lon -100)
//! is suggested the CONUS transformation.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, SpatialCriterion,
};

/// `TEST_F(CApi, proj_get_suggested_operation_for_NAD83_to_NAD83_HARN)`
#[test]
fn capi_proj_get_suggested_operation_for_nad83_to_nad83_harn() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let source = epsg
        .create_coordinate_reference_system("4269")
        .expect("4269");
    let target = epsg
        .create_coordinate_reference_system("4152")
        .expect("4152");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let factory = CoordinateOperationFactory::create();
    let list = factory
        .create_operations(&source, &target, &ctxt)
        .expect("operations");
    let suggested = factory
        .suggested_operation(&list, &source, (40.0, -100.0))
        .expect("suggestion")
        .expect("an operation covers (40, -100)");
    assert_eq!(list[suggested].name(), "NAD83 to NAD83(HARN) (47)");
}
