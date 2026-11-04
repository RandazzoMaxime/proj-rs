//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: CRS read from WKT without an identifier give
//! the operations of the registered CRS they are equivalent to.

use proj_rs::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, SpatialCriterion, WktObject, WktParser,
};

fn parse(wkt: &str) -> Crs {
    match WktParser::new().create_from_wkt(wkt).expect("WKT") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    }
}

fn operations(
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    criterion: SpatialCriterion,
) -> Vec<CoordinateOperation> {
    let mut context = CoordinateOperationContext::create(db, "EPSG");
    context.set_spatial_criterion(criterion);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .expect("operations")
}

/// `TEST(operation, projCRS_no_id_to_geogCRS_context)`
#[test]
fn operation_proj_crs_no_id_to_geog_crs_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory
        .create_coordinate_reference_system("28992")
        .expect("28992");
    let target = factory
        .create_coordinate_reference_system("4258")
        .expect("4258");
    let list = operations(&db, &source, &target, SpatialCriterion::StrictContainment);
    assert!(!list.is_empty());
    let from_wkt = parse(
        "PROJCRS[\"unknown\",\n\
         \x20   BASEGEOGCRS[\"Amersfoort\",\n\
         \x20      DATUM[\"Amersfoort\",\n\
         \x20           ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128]]],\n\
         \x20   CONVERSION[\"unknown\",\n\
         \x20       METHOD[\"Oblique Stereographic\"],\n\
         \x20       PARAMETER[\"Latitude of natural origin\",52.1561605555556],\n\
         \x20       PARAMETER[\"Longitude of natural origin\",5.38763888888889],\n\
         \x20       PARAMETER[\"Scale factor at natural origin\",0.9999079],\n\
         \x20       PARAMETER[\"False easting\",155000],\n\
         \x20       PARAMETER[\"False northing\",463000]],\n\
         \x20   CS[Cartesian,2],\n\
         \x20       AXIS[\"(E)\",east],\n\
         \x20       AXIS[\"(N)\",north],\n\
         \x20   LENGTHUNIT[\"metre\",1],\n\
         \x20   ID[\"EPSG\",28992]]",
    );
    let list2 = operations(&db, &from_wkt, &target, SpatialCriterion::StrictContainment);
    assert!(list.len() + 1 >= list2.len());
    for (op, op2) in list.iter().zip(&list2) {
        assert!(op.is_equivalent_to(op2), "{} / {}", op.name(), op2.name());
    }
}

/// `TEST(operation, geogCRS_without_id_to_geogCRS_3D_context)`
#[test]
fn operation_geog_crs_without_id_to_geog_crs_3d_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let source = factory
        .create_coordinate_reference_system("4289")
        .expect("4289");
    let target = factory
        .create_coordinate_reference_system("4937")
        .expect("4937");
    let list = operations(&db, &source, &target, SpatialCriterion::PartialIntersection);
    assert!(!list.is_empty());
    let from_wkt = parse(
        "GEOGCRS[\"unnamed\",\n\
         \x20   DATUM[\"Amersfoort\",\n\
         \x20       ELLIPSOID[\"Bessel 1841\",6377397.155,299.1528128,\n\
         \x20           LENGTHUNIT[\"metre\",1]]],\n\
         \x20   PRIMEM[\"Greenwich\",0,\n\
         \x20       ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20   CS[ellipsoidal,2],\n\
         \x20       AXIS[\"geodetic latitude (Lat)\",north,\n\
         \x20           ORDER[1],\n\
         \x20           ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20       AXIS[\"geodetic longitude (Lon)\",east,\n\
         \x20           ORDER[2],\n\
         \x20           ANGLEUNIT[\"degree\",0.0174532925199433]],\
         \x20   USAGE[\n\
         \x20       SCOPE[\"unknown\"],\n\
         \x20       AREA[\"Netherlands - onshore\"],\n\
         \x20       BBOX[50.75,3.2,53.7,7.22]]]\n",
    );
    let list2 = operations(
        &db,
        &from_wkt,
        &target,
        SpatialCriterion::PartialIntersection,
    );
    assert!(list.len() >= list2.len());
    for (op, op2) in list.iter().zip(&list2) {
        assert!(op.is_equivalent_to(op2), "{} {}", op.name(), op2.name());
    }
}
