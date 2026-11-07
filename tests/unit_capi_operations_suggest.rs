//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: the operation list of `proj_create_operations` and the
//! suggestion of `proj_get_suggested_operation` for a point, with the grid of the suggested
//! operation counted by `proj_coordoperation_get_grid_used_count`.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    GridAvailabilityUse, SpatialCriterion,
};

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

/// `proj_create_operation_factory_context` with partial intersection and grids ignored, the
/// two `proj_operation_factory_context_set_*` calls of the C tests.
fn context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut context = CoordinateOperationContext::create(db, "");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    context
}

/// `TEST_F(CApi, proj_create_operations_prime_meridian_non_greenwich)`
///
/// `NTF (Paris) / Lambert Centre France` (EPSG:27562) to `ETRS89` (EPSG:4258), in the Paris
/// prime meridian. The point is in the area of the NTv2 grid transformation, which is the
/// suggestion, and that operation uses one grid.
#[test]
fn capi_proj_create_operations_prime_meridian_non_greenwich() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = epsg_crs(&db, "27562");
    let target = epsg_crs(&db, "4258");
    let context = context(&db);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(!operations.is_empty());

    // lat,long=49,-4 if using grid
    let index = CoordinateOperationFactory::create()
        .suggested_operation(
            &operations,
            &source,
            (136_555.582_889_92, 463_344.518_942_96),
        )
        .expect("suggestion")
        .expect("an operation covers the point");
    let grids = operations[index]
        .grids_needed(&db, GridAvailabilityUse::IgnoreGridAvailability)
        .expect("grids");
    // Transformation using grid
    assert_eq!(grids.len(), 1);
}

/// `TEST_F(CApi, proj_get_suggested_operation_with_operations_without_area_of_use)`
///
/// NAD83(2011) geocentric (EPSG:6317) to NAD83(2011) 2D (EPSG:6318), partial intersection, grids
/// ignored. The point is geocentric: PROJ converts it to longitude and latitude before it tests
/// the boxes of the candidates (`pjSrcGeocentricToLonLat`), and PROJ expects a suggestion.
#[test]
fn capi_proj_get_suggested_operation_with_operations_without_area_of_use() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = epsg_crs(&db, "6317");
    let target = epsg_crs(&db, "6318");
    let context = context(&db);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let suggested = CoordinateOperationFactory::create()
        .suggested_operation_xyz(
            &operations,
            &source,
            (-463_930.0, -4_414_006.0, 4_562_247.0),
        )
        .expect("suggestion");
    assert!(suggested.is_some());
}
