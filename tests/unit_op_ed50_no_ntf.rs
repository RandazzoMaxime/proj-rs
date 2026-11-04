//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, ED50 to WGS 72: the NTF (Paris) datum is
//! not used as an intermediate.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    SpatialCriterion,
};

/// `TEST(operation, geogCRS_to_geogCRS_context_ED50_to_WGS72_no_NTF_intermediate)`
#[test]
fn operation_geog_crs_to_geog_crs_context_ed50_to_wgs72_no_ntf_intermediate() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &f.create_coordinate_reference_system("4230").expect("4230"),
            &f.create_coordinate_reference_system("4322").expect("4322"),
            &ctxt,
        )
        .expect("operations");
    assert!(list.len() >= 2);
    for op in &list {
        assert!(!op.name().contains("NTF"), "{}", op.name());
    }
}
