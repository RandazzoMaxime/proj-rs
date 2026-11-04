//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, compound CRS to geographic 3D contexts.

use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse,
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
            &factory
                .create_coordinate_reference_system("9537")
                .expect("9537"),
            &factory
                .create_coordinate_reference_system("4557")
                .expect("4557"),
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
