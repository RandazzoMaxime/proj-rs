//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, NGVD29 height (m) to NAVD88 height (1):
//! the registry operations, then the ballpark vertical transformation.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    ProjLines, SpatialCriterion,
};

/// `TEST(operation, vertCRS_to_vertCRS_context)`
#[test]
fn operation_vert_crs_to_vert_crs_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let src = f.create_coordinate_reference_system("7968").expect("7968");
    let dst = f.create_coordinate_reference_system("5703").expect("5703");
    let list = CoordinateOperationFactory::create()
        .create_operations(&src, &dst, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 4);
    assert_eq!(list[0].name(), "NGVD29 height (m) to NAVD88 height (3)");
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=vgridshift +grids=us_noaa_vertcone.tif +multiplier=1"
    );
}
