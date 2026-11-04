//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`, datum pivot cases.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected names and PROJ
//! strings are copied from that file.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, GridAvailabilityUse, ProjLines, SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn plain(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.export_to_proj_string(),
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    }
    .expect("PROJ string")
}

/// TEST(operation, createOperation_test_createOperationsWithDatumPivot_iter_1)
#[test]
fn operation_create_operation_test_create_operations_with_datum_pivot_iter_1() {
    let db = database();
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let source = epsg
        .create_coordinate_reference_system("8237")
        .expect("8237");
    let target = epsg
        .create_coordinate_reference_system("8240")
        .expect("8240");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0].name(),
        "Conversion from NAD83(CSRS)v2 (geog2D) to \
         NAD83(CSRS)v2 (geocentric) + \
         NAD83(CSRS)v2 to NAD83(CSRS)v3 (3) + \
         Conversion from NAD83(CSRS)v3 (geocentric) to \
         NAD83(CSRS)v3 (geog2D)"
    );
    assert_eq!(plain(&list[0]), "+proj=noop");
}

/// TEST(operation, createOperation_ETRF2000_to_Amersfoort)
#[test]
fn operation_create_operation_etrf2000_to_amersfoort() {
    let db = database();
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let factory = CoordinateOperationFactory::create();
    let source = epsg
        .create_coordinate_reference_system("9067")
        .expect("9067");
    let target = epsg
        .create_coordinate_reference_system("4289")
        .expect("4289");
    let list = factory
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Conversion from ETRF2000 (geog2D) to ETRF2000 (geocentric) + \
         Inverse of ETRS89-NLD [AGRS2010] to ETRF2000 (1) + \
         Conversion from ETRS89-NLD [AGRS2010] (geocentric) to \
         ETRS89-NLD [AGRS2010] (geog2D) + \
         Inverse of Amersfoort to ETRS89-NLD [AGRS2010] (9)"
    );
    let list = factory
        .create_operations(&target, &source, &context)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Amersfoort to ETRS89-NLD [AGRS2010] (9) + \
         Conversion from ETRS89-NLD [AGRS2010] (geog2D) to \
         ETRS89-NLD [AGRS2010] (geocentric) + \
         ETRS89-NLD [AGRS2010] to ETRF2000 (1) + \
         Conversion from ETRF2000 (geocentric) to ETRF2000 (geog2D)"
    );
}

/// TEST(operation, createOperation_ETRS89_XXX_to_ETRS89_YYY_using_ETRF2000)
#[test]
fn operation_create_operation_etrs89_xxx_to_etrs89_yyy_using_etrf2000() {
    let db = database();
    let epsg = AuthorityFactory::create(&db, "EPSG");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    // ETRS89-PRT [1995] to ETRS89-ESP [ERGNSS]
    let source = epsg
        .create_coordinate_reference_system("11108")
        .expect("11108");
    let target = epsg
        .create_coordinate_reference_system("11128")
        .expect("11128");
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(list.len() >= 2);

    assert_eq!(
        list[0].name(),
        "Conversion from ETRS89-PRT [1995] (geog2D) to \
         ETRS89-PRT [1995] (geocentric) + \
         ETRS89-PRT [1995] to ETRF2000 (2) + \
         Inverse of ETRS89-ESP [ERGNSS] to ETRF2000 (1) + \
         Conversion from ETRS89-ESP [ERGNSS] (geocentric) to \
         ETRS89-ESP [ERGNSS] (geog2D)"
    );
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        plain(&list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0.0063 +y=0.00294 +z=0.01726 \
         +rx=-0.0007616 +ry=-6.4e-05 +rz=-0.0008768 +s=-0.001534 \
         +dx=0 +dy=0.0006 +dz=0.0014 +drx=-0.000119 +dry=-1e-05 \
         +drz=-0.000162 +ds=-1e-05 +t_epoch=1995.4 \
         +convention=position_vector \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(list[0].accuracy(), Some(0.1));

    assert_eq!(
        list[1].name(),
        "Inverse of ETRS89 to ETRS89-PRT [1995] + ETRS89 to ETRS89-ESP [ERGNSS]"
    );
    assert!(!list[1].has_ballpark_transformation());
    assert_eq!(plain(&list[1]), "+proj=noop");
    assert_eq!(list[1].accuracy(), Some(0.1));
}
