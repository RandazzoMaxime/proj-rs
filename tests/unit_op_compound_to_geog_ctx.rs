//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, compound CRS to geographic 3D contexts.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// `TEST(operation, compoundCRS_to_geogCRS_3D_KNOWN_AVAILABLE_context)`
#[test]
fn operation_compound_to_geog_3d_known_available_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &factory.create_coordinate_reference_system("9537").expect("9537"),
            &factory.create_coordinate_reference_system("4557").expect("4557"),
            &ctxt,
        )
        .expect("operations");
    assert!(list.len() >= 2, "{}", list.len());
    assert_eq!(
        list[0].name(),
        "Inverse of RGAF09 to Martinique 1987 height (2) + Inverse of RRAF 1991 to RGAF09 (1)"
    );
    assert_eq!(
        list[1].name(),
        "Inverse of RRAF 1991 to RGAF09 (1) + Inverse of RRAF 1991 to Martinique 1987 height (1)"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_2D_promote_to_3D_context)`
#[test]
fn operation_compound_to_geog_2d_promote_to_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let src = create_from_user_input("EPSG:4269+5703", &db).expect("NAD83 + NAVD88");
    let dst = factory
        .create_coordinate_reference_system("4269")
        .expect("4269")
        .promote_to_3d(Some(&db));
    let forward = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("forward");
    assert_eq!(forward.len(), 133);
    let backward = CoordinateOperationFactory::create()
        .create_operations(&dst, &src, &ctxt)
        .expect("backward");
    assert_eq!(backward.len(), forward.len());
}
