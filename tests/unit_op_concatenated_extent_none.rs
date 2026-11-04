//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, ED50 UTM 31N (EPSG:23031) to WGS 84: the
//! ED50 to WGS 84 (15) concatenated operation (EPSG:8047) appears only when the CRS extents are
//! not used.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, CrsExtentUse,
    DatabaseContext, SupersededOperations,
};

fn has_ed87_pathway(extent_use: Option<CrsExtentUse>) -> bool {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_superseded_operations(SupersededOperations::Keep);
    if let Some(extent_use) = extent_use {
        ctxt.set_crs_extent_use(extent_use);
    }
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &epsg
                .create_coordinate_reference_system("23031")
                .expect("23031"),
            &epsg
                .create_coordinate_reference_system("4326")
                .expect("4326"),
            &ctxt,
        )
        .expect("operations");
    list.iter().any(|op| op.name().contains("ED87"))
}

/// `TEST(operation, projCRS_to_geogCRS_concatenated_operation_extent_none)`
#[test]
fn operation_proj_crs_to_geog_crs_concatenated_operation_extent_none() {
    assert!(!has_ed87_pathway(None));
    assert!(has_ed87_pathway(Some(CrsExtentUse::Ignore)));
}
