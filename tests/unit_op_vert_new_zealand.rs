//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, NZVD2016 height to Auckland 1946 height.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    ProjLines,
};

/// `TEST(operation, vertCRS_to_vertCRS_New_Zealand_context)`
#[test]
fn operation_vert_crs_to_vert_crs_new_zealand_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &f.create_coordinate_reference_system("7839").expect("7839"),
            &f.create_coordinate_reference_system("5759").expect("5759"),
            &ctxt,
        )
        .expect("operations");
    assert_eq!(list.len(), 2);
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=vgridshift +grids=nz_linz_auckht1946-nzvd2016.tif +multiplier=1"
    );
}
