//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a compound CRS read from WKT gives the
//! operations of the registered compound CRS it is equivalent to.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, Equivalence, GridAvailabilityUse,
    SpatialCriterion, WktConvention, WktFormatter, WktObject, WktParser,
};

/// `TEST(operation, compoundCRS_from_wkt_without_id_to_geogCRS)`
#[test]
fn operation_compound_from_wkt_without_id_to_geog() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let wkt = "COMPOUNDCRS[\"NAD83(2011) + NAVD88 height\",\n\
        \x20   GEOGCRS[\"NAD83(2011)\",\n\
        \x20       DATUM[\"NAD83 (National Spatial Reference System 2011)\",\n\
        \x20           ELLIPSOID[\"GRS 1980\",6378137,298.257222101,\n\
        \x20               LENGTHUNIT[\"metre\",1]]],\n\
        \x20       PRIMEM[\"Greenwich\",0,\n\
        \x20           ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        \x20       CS[ellipsoidal,2],\n\
        \x20           AXIS[\"geodetic latitude (Lat)\",north,\n\
        \x20               ORDER[1],\n\
        \x20               ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
        \x20           AXIS[\"geodetic longitude (Lon)\",east,\n\
        \x20               ORDER[2],\n\
        \x20               ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
        \x20   VERTCRS[\"NAVD88 height\",\n\
        \x20       VDATUM[\"North American Vertical Datum 1988\"],\n\
        \x20       CS[vertical,1],\n\
        \x20           AXIS[\"gravity-related height (H)\",up,\n\
        \x20               LENGTHUNIT[\"metre\",1]]]]";
    let source = create_from_user_input(wkt, &db).expect("source");
    let target = factory
        .create_coordinate_reference_system("6319")
        .expect("6319");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let reference = create_from_user_input("EPSG:6318+5703", &db).expect("reference");
    assert!(source.is_equivalent_to_with(&reference, Equivalence::Equivalent));
    let list_ref = CoordinateOperationFactory::create()
        .create_operations(&reference, &target, &context)
        .expect("reference operations");
    assert_eq!(list.len(), list_ref.len());
}

/// `TEST(operation, compoundCRS_from_WKT2_to_geogCRS_3D_context)`
#[test]
fn operation_compound_from_wkt2_to_geog_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let context = CoordinateOperationContext::create(&db, "EPSG");
    let source = factory
        .create_coordinate_reference_system("7415")
        .expect("7415");
    let target = factory
        .create_coordinate_reference_system("4937")
        .expect("4937");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(!list.is_empty());
    let wkt2 = source
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT2");
    let from_wkt2 = match WktParser::new().create_from_wkt(&wkt2).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    };
    let list2 = CoordinateOperationFactory::create()
        .create_operations(&from_wkt2, &target, &context)
        .expect("operations from WKT2");
    assert!(list.len() >= list2.len());
    for (op, op2) in list.iter().zip(&list2) {
        assert!(
            CoordinateOperation::is_equivalent_to(op, op2),
            "{} / {}",
            op.name(),
            op2.name()
        );
    }
    let _: Option<&Crs> = None;
}
