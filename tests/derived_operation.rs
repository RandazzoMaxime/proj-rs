//! Operations from a projected (derived) CRS, `createOperationsDerivedTo` in PROJ's
//! `coordinateoperationfactory.cpp`: the inverse of the deriving conversion, chained with
//! the operations from the base CRS. The names are the ones `projinfo` 9.9.0 prints.

use proj_rs::iso19111::{
    create_from_user_input, BoundingBox, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, CrsExtentUse, DatabaseContext, ObjectDomain,
};

fn candidates(source: &str, target: &str) -> Vec<CoordinateOperation> {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "EPSG");
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

#[test]
fn projected_to_its_base_is_the_inverse_deriving_conversion() {
    let operations = candidates("EPSG:32631", "EPSG:4326");
    assert_eq!(operations.len(), 1);
    let CoordinateOperation::Conversion(_) = &operations[0] else {
        panic!("expected a conversion, got {}", operations[0].name());
    };
    assert_eq!(operations[0].name(), "Inverse of UTM zone 31N");
    let identifier = &operations[0].identifiers()[0];
    assert_eq!(identifier.code_space(), "INVERSE(EPSG)");
    assert_eq!(identifier.code(), "16031");
}

#[test]
fn inverse_conversion_has_no_forward_proj_string() {
    let operations = candidates("EPSG:32631", "EPSG:4326");
    let CoordinateOperation::Conversion(conversion) = &operations[0] else {
        panic!("expected a conversion");
    };
    assert!(conversion.export_to_proj_string().is_err());
}

#[test]
fn projected_to_compound_chains_the_inverse_with_the_base_null_offset() {
    let operations = candidates("EPSG:32631", "EPSG:4326+3855");
    let names: Vec<&str> = operations.iter().map(CoordinateOperation::name).collect();
    assert_eq!(
        names,
        ["Inverse of UTM zone 31N + Inverse of Null geographic offset from WGS 84 to WGS 84"]
    );
    assert_eq!(operations[0].accuracy(), Some(0.0));
    let CoordinateOperation::Concatenated(concatenated) = &operations[0] else {
        panic!("expected a concatenated operation");
    };
    assert_eq!(concatenated.steps().len(), 2);
    assert!(operations[0].identifiers().is_empty());
}

#[test]
fn projected_to_geographic_inside_a_bbox_names_the_area_of_the_chained_operation() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:23031", &db).expect("source");
    let target = create_from_user_input("EPSG:4326", &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context
        .set_area_of_interest(Some(ObjectDomain::from_bounding_box(BoundingBox::new(
            -13.87, 34.91, -7.24, 41.88,
        ))))
        .set_crs_extent_use(CrsExtentUse::Ignore);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let names: Vec<&str> = operations.iter().map(CoordinateOperation::name).collect();
    assert_eq!(names, ["Inverse of UTM zone 31N + ED50 to WGS 84 (42)"]);
    assert_eq!(
        operations[0].domains()[0].area(),
        Some("Portugal - mainland - offshore.")
    );
    assert_eq!(operations[0].accuracy(), Some(5.0));
}
