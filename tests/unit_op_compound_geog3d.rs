//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, NAD27 + NGVD29 height (ftUS) to WGS 84
//! (3D), first section of `compoundCRS_to_geogCRS_3D_context`: the operation's name. The
//! pipeline shape is not checked yet: the vertical unit change is not merged into the
//! horizontal conversion step as PROJ does.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse,
};

#[test]
fn operation_compound_to_geog_crs_3d_context_name() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let src = f.create_coordinate_reference_system("7406").expect("7406");
    let dst = f.create_coordinate_reference_system("4979").expect("4979");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert!(list[0].has_ballpark_transformation());
    assert_eq!(
        list[0].name(),
        "NAD27 to WGS 84 (79) + Transformation from NGVD29 height (ftUS) to WGS 84 (ballpark vertical transformation, without ellipsoid height to vertical height correction)"
    );
}
