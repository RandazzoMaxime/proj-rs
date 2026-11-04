//! PROJ 9.9.0 `test/unit/test_c_api_990.cpp`, `proj_create_operations` with and without
//! `proj_operation_factory_context_set_discard_superseded`: AGD84 (EPSG:4203) to WGS 84 (EPSG:4326).

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, SpatialCriterion, SupersededOperations,
};

fn count_operations(superseded: SupersededOperations) -> usize {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let source = epsg
        .create_coordinate_reference_system("4203")
        .expect("4203");
    let target = epsg
        .create_coordinate_reference_system("4326")
        .expect("4326");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_superseded_operations(superseded);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &ctxt)
        .expect("operations")
        .len()
}

/// `TEST_F(CApi, proj_create_operations_discard_superseded)`
#[test]
fn capi_proj_create_operations_discard_superseded() {
    assert_eq!(count_operations(SupersededOperations::Discard), 4);
}

/// `TEST_F(CApi, proj_create_operations_dont_discard_superseded)`
#[test]
fn capi_proj_create_operations_dont_discard_superseded() {
    assert_eq!(count_operations(SupersededOperations::Keep), 5);
}
