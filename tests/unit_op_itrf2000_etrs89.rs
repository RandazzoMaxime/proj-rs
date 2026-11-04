//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, ITRF2000 to ETRS89 finds both grid-based
//! and non-grid-based operations.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, SpatialCriterion,
};

/// `TEST(operation, createOperation_ITRF2000_to_ETRS89)`
#[test]
fn operation_itrf2000_to_etrs89() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &epsg
                .create_coordinate_reference_system("9989")
                .expect("9989"),
            &epsg
                .create_coordinate_reference_system("4937")
                .expect("4937"),
            &ctxt,
        )
        .expect("operations");
    let mut found_non_grid = false;
    let mut found_grid = false;
    for op in &list {
        if !op.has_ballpark_transformation() {
            let grids = op
                .grids_needed(&db, GridAvailabilityUse::UseForSorting)
                .expect("grids");
            if grids.is_empty() {
                found_non_grid = true;
            } else {
                found_grid = true;
            }
        }
    }
    assert!(found_non_grid);
    assert!(found_grid);
}
