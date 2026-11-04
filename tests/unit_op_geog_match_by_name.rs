//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, geographic CRS to geographic CRS. A NAD27
//! geographic CRS without identifier gives the same number of operations in both directions,
//! and as many as the registered EPSG:4267 to EPSG:4326.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, Crs, DatabaseContext,
    EllipsoidalCS, GeodeticReferenceFrame, GeographicCRS, IntermediateCrsUse, SpatialCriterion,
    UnitOfMeasure,
};

/// `TEST(operation, geogCRS_to_geogCRS_context_match_by_name)`
#[test]
fn operation_geog_crs_to_geog_crs_context_match_by_name() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut ctxt = CoordinateOperationContext::create(&db, "EPSG");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt.set_intermediate_crs_use(IntermediateCrsUse::Never);
    let nad27 = Crs::Geographic(
        GeographicCRS::create(
            "NAD27",
            Some(GeodeticReferenceFrame::epsg_6267()),
            None,
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
        )
        .expect("NAD27 without identifier"),
    );
    let wgs84 = epsg
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326");
    let reference = epsg
        .create_coordinate_reference_system("4267")
        .expect("EPSG:4267");
    let factory = CoordinateOperationFactory::create();
    let list = factory
        .create_operations(&nad27, &wgs84, &ctxt)
        .expect("operations");
    let list_inv = factory
        .create_operations(&wgs84, &nad27, &ctxt)
        .expect("inverse operations");
    let list_ref = factory
        .create_operations(&reference, &wgs84, &ctxt)
        .expect("reference operations");
    assert_eq!(list.len(), list_ref.len());
    assert_eq!(list_inv.len(), list_ref.len());
    assert!(list_ref.len() >= 2);
}
