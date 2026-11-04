//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, IG05/12 Intermediate CRS to ITRF2014: the
//! datum-based intermediate operation, in both directions.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
};

/// `TEST(operation, createBetweenGeodeticCRSWithDatumBasedIntermediates)`
#[test]
fn operation_create_between_geodetic_crs_with_datum_based_intermediates() {
    let db = DatabaseContext::create().expect("proj.db");
    let f = AuthorityFactory::create(&db, "EPSG");
    let ctxt = CoordinateOperationContext::create(&db, "EPSG");
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &f.create_coordinate_reference_system("6990").expect("6990"),
            &f.create_coordinate_reference_system("9000").expect("9000"),
            &ctxt,
        )
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Inverse of ITRF2008 to IG05/12 Intermediate CRS + Conversion from ITRF2008 (geog2D) to ITRF2008 (geocentric) + ITRF2008 to ITRF2014 (1) + Conversion from ITRF2014 (geocentric) to ITRF2014 (geog2D)"
    );

    let list_inv = CoordinateOperationFactory::create()
        .create_operations(
            &f.create_coordinate_reference_system("9000").expect("9000"),
            &f.create_coordinate_reference_system("6990").expect("6990"),
            &ctxt,
        )
        .expect("operations");
    assert_eq!(list_inv.len(), 1);
    assert_eq!(
        list_inv[0].name(),
        "Conversion from ITRF2014 (geog2D) to ITRF2014 (geocentric) + Inverse of ITRF2008 to ITRF2014 (1) + Conversion from ITRF2008 (geocentric) to ITRF2008 (geog2D) + ITRF2008 to IG05/12 Intermediate CRS"
    );
}
