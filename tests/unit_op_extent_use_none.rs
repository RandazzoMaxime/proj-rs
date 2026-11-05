//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `compoundCRS_to_compoundCRS_context_extent_use_none`.

use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse,
    DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// `TEST(operation, compoundCRS_to_compoundCRS_context_extent_use_none)`
#[test]
fn operation_compound_to_compound_context_extent_use_none() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let source = f.create_coordinate_reference_system("7406").expect("7406");
    let target = f.create_coordinate_reference_system("5500").expect("5500");
    let count = |none: bool| {
        let mut c = CoordinateOperationContext::create(&db, "EPSG");
        c.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
            .set_spatial_criterion(SpatialCriterion::PartialIntersection);
        if none {
            c.set_crs_extent_use(CrsExtentUse::Ignore);
        }
        CoordinateOperationFactory::create()
            .create_operations(&source, &target, &c)
            .expect("ops")
            .len()
    };
    let default = count(false);
    assert!(default >= 1);
    assert!(count(true) > default);
}
