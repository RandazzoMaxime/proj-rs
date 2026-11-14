//! PROJ 9.9.0 `test/unit/test_common.cpp`, the 13 `TEST(common, ...)` cases. Expected values are the
//! ones the C++ asserts. The alias invalid-type cases are not written: see
//! `docs/AUDIT-unit-core.md`, note 4.

use proj_rs::iso19111::create_object_from_json;
use proj_rs::{
    DataEpoch, FactoryError, Identifier, Measure, OperationParameter, UnitKind, UnitOfMeasure,
};

fn length(value: f64, unit: UnitOfMeasure) -> Measure {
    Measure::new(value, unit)
}

/// The PROJJSON reader is where untyped input meets the object model. A wrong-typed member is refused.
fn refused(text: &str) -> bool {
    matches!(
        create_object_from_json(text, None),
        Err(FactoryError::Invalid(_))
    )
}

/// `TEST(common, unit_of_measure)`
#[test]
fn unit_of_measure() {
    assert_eq!(UnitOfMeasure::metre().name(), "metre");
    assert_eq!(UnitOfMeasure::metre().conversion_to_si(), 1.0);
    assert_eq!(UnitOfMeasure::metre().kind(), UnitKind::Linear);

    assert_eq!(UnitOfMeasure::degree().name(), "degree");
    assert_eq!(
        UnitOfMeasure::degree().conversion_to_si(),
        0.017_453_292_519_943_295
    );
    assert_eq!(UnitOfMeasure::degree().kind(), UnitKind::Angular);

    assert_eq!(UnitOfMeasure::radian().name(), "radian");
    assert_eq!(UnitOfMeasure::radian().conversion_to_si(), 1.0);
    assert_eq!(UnitOfMeasure::radian().kind(), UnitKind::Angular);

    let km = UnitOfMeasure::new("km", 1000.0, UnitKind::Linear);
    assert_eq!(
        length(2.0, km).convert_to_unit(&UnitOfMeasure::metre()),
        2000.0
    );

    assert_eq!(
        length(2.0, UnitOfMeasure::degree()).convert_to_unit(&UnitOfMeasure::radian()),
        2.0 * 0.017_453_292_519_943_295
    );

    assert_eq!(
        length(2.596_921_3, UnitOfMeasure::grad()).convert_to_unit(&UnitOfMeasure::degree()),
        2.596_921_3 / 100.0 * 90.0
    );

    // Units compare by name. A copy is equal, and so is a unit moved into another binding.
    let my_uom = UnitOfMeasure::new("myUOM", 123.0, UnitKind::Linear);
    let my_uom_copy = my_uom.clone();
    assert!(my_uom == my_uom_copy);
    let other_uom = my_uom;
    assert!(other_uom == my_uom_copy);
}

/// `TEST(common, measure)`
#[test]
fn measure() {
    let m = |value: f64| Measure::new(value, UnitOfMeasure::none());

    assert!(m(0.0) == m(0.0));
    assert!(m(1.0) == m(1.0));
    assert!(!(m(1.0) == m(2.0)));
    assert!(!(m(1.0) == m(0.0)));
    assert!(!(m(0.0) == m(1.0)));
    assert!(m(f64::INFINITY) == m(f64::INFINITY));
    assert!(m(f64::NEG_INFINITY) == m(f64::NEG_INFINITY));
    assert!(!(m(f64::INFINITY) == m(f64::NEG_INFINITY)));
    assert!(!(m(f64::INFINITY) == m(1.0)));
    assert!(!(m(1.0) == m(f64::INFINITY)));
}

/// `TEST(common, identifiedobject_empty)`
#[test]
fn identifiedobject_empty() {
    let obj = OperationParameter::create("");
    assert!(obj.name().is_empty());
    assert!(obj.identifiers().is_empty());
    assert!(obj.aliases().is_empty());
    assert!(obj.remarks().is_empty());
    assert!(!obj.is_deprecated());
    assert!(obj.alias().is_empty());
}

/// `TEST(common, identifiedobject)`
#[test]
fn identifiedobject() {
    let obj = OperationParameter::create("name")
        .with_identifier_object(Identifier::create("identifier_code"))
        .with_alias("alias")
        .with_remarks("remarks")
        .with_deprecated();
    assert_eq!(obj.name(), "name");
    assert_eq!(obj.identifiers().len(), 1);
    assert_eq!(obj.identifiers()[0].code(), "identifier_code");
    assert_eq!(obj.aliases().len(), 1);
    assert_eq!(obj.aliases()[0], "alias");
    assert_eq!(obj.remarks(), "remarks");
    assert!(obj.is_deprecated());
}

/// `TEST(common, identifiedobject_name_invalid_type_integer)`: a name that is a number is refused.
#[test]
fn identifiedobject_name_invalid_type_integer() {
    assert!(refused(
        r#"{"type":"PrimeMeridian","name":123,"longitude":0}"#
    ));
}

/// `TEST(common, identifiedobject_name_invalid_type_citation)`: a name that is an object is refused.
#[test]
fn identifiedobject_name_invalid_type_citation() {
    assert!(refused(
        r#"{"type":"PrimeMeridian","name":{"description":"invalid"},"longitude":0}"#
    ));
}

/// `TEST(common, identifiedobject_identifier_invalid_type)`: identifiers given as text are refused.
#[test]
fn identifiedobject_identifier_invalid_type() {
    assert!(refused(
        r#"{"type":"PrimeMeridian","name":"x","longitude":0,"id":"string not allowed"}"#
    ));
}

/// `TEST(common, identifiedobject_identifier_array_of_identifier)`
#[test]
fn identifiedobject_identifier_array_of_identifier() {
    let obj = OperationParameter::create("")
        .with_identifier_object(Identifier::create("identifier_code1"))
        .with_identifier_object(Identifier::create("identifier_code2"));
    assert_eq!(obj.identifiers().len(), 2);
    assert_eq!(obj.identifiers()[0].code(), "identifier_code1");
    assert_eq!(obj.identifiers()[1].code(), "identifier_code2");
}

/// `TEST(common, identifiedobject_identifier_array_of_invalid_type)`: an identifier list with an entry
/// that is not an identifier is refused.
#[test]
fn identifiedobject_identifier_array_of_invalid_type() {
    assert!(refused(
        r#"{"type":"PrimeMeridian","name":"x","longitude":0,"id":["unexpected type"]}"#
    ));
}

/// `TEST(common, identifiedobject_alias_array_of_string)`
#[test]
fn identifiedobject_alias_array_of_string() {
    let obj = OperationParameter::create("")
        .with_alias("alias1")
        .with_alias("alias2");
    assert_eq!(obj.aliases().len(), 2);
    assert_eq!(obj.aliases()[0], "alias1");
    assert_eq!(obj.aliases()[1], "alias2");
    assert_eq!(obj.alias(), "alias1");
}

/// `TEST(common, DataEpoch)`
#[test]
fn data_epoch() {
    let year = UnitOfMeasure::year();
    let src = DataEpoch::new(Measure::new(2010.5, year.clone()));
    let epoch = src.clone();
    assert_eq!(epoch, src);
    assert_eq!(epoch.coordinate_epoch().value(), 2010.5);
    assert!(epoch.coordinate_epoch().unit() == &year);
}
