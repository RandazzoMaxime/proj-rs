//! An inverted conversion is compared through the method PROJ exposes for it,
//! `InverseConversion::method`: `Inverse of Transverse Mercator`. It is then equivalent to its
//! copy read back from PROJJSON, and not to the forward conversion. Expected values from
//! `proj_is_equivalent_to_with_ctx(..., PJ_COMP_EQUIVALENT)` of PROJ 9.9.0 on the steps of the
//! first operation of `EPSG:28356` to `EPSG:7856` (pyproj's test_crs::test_operations compares
//! a step with its PROJJSON copy that way).

use proj_rs::iso19111::{
    create_from_user_input, create_object_from_json, Conversion, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, Equivalence,
    JsonFormat, JsonObject,
};

fn map_grid_steps(db: &DatabaseContext) -> (Conversion, Conversion) {
    let source = create_from_user_input("EPSG:28356", db).expect("source");
    let target = create_from_user_input("EPSG:7856", db).expect("target");
    let context = CoordinateOperationContext::create(db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let CoordinateOperation::Concatenated(concatenated) = &operations[0] else {
        panic!("a concatenated operation");
    };
    let conversion = |index: usize| match concatenated.steps()[index].operation() {
        CoordinateOperation::Conversion(conversion) => conversion.clone(),
        other => panic!("a conversion: {}", other.name()),
    };
    (conversion(0), conversion(2))
}

#[test]
fn an_inverse_conversion_meets_its_projjson_copy() {
    let db = DatabaseContext::create().expect("database");
    let (inverse, _) = map_grid_steps(&db);
    assert_eq!(inverse.name(), "Inverse of Map Grid of Australia zone 56");
    let json = CoordinateOperation::Conversion(inverse.clone())
        .export_to_json_with(&db, &JsonFormat::default())
        .expect("PROJJSON");
    let JsonObject::Conversion(copy) = create_object_from_json(&json, Some(&db)).expect("read")
    else {
        panic!("a conversion");
    };
    assert!(inverse.is_equivalent_to_with(&copy, Equivalence::Equivalent));
    assert!(copy.is_equivalent_to_with(&inverse, Equivalence::Equivalent));
}

#[test]
fn an_inverse_conversion_is_not_the_forward_one() {
    let db = DatabaseContext::create().expect("database");
    let (inverse, forward) = map_grid_steps(&db);
    assert_eq!(forward.name(), "Map Grid of Australia zone 56");
    assert!(!inverse.is_equivalent_to_with(&forward, Equivalence::Equivalent));
    assert!(!forward.is_equivalent_to_with(&inverse, Equivalence::Equivalent));
}
