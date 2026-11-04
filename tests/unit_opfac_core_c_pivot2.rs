//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`, datum pivot cases.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected names and PROJ
//! strings are copied from that file.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, GridAvailabilityUse, ProjLines,
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
