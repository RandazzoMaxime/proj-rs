//! PROJ 9.9.0 `test/unit/test_common.cpp`, the alias cases of `TEST(common, identifiedobject_alias_*)`,
//! run through the untyped `PropertyMap` of this crate.
//!
//! The expected values are the ones PROJ 9.9.0 asserts. The refusals and the accepted forms of the
//! other keys were measured against PROJ 9.9.0 `libproj` with `OperationParameter::create(properties)`,
//! not taken from this crate.

use proj_rs::iso19111::{
    Citation, Identifier, InvalidValueTypeError, OperationParameter, PropertyMap, PropertyValue,
    ALIAS_KEY, DEPRECATED_KEY, IDENTIFIERS_KEY, NAME_KEY, REMARKS_KEY,
};

/// `OperationParameter::create_from_properties` must refuse this map, and the error names the key.
fn refused(properties: &PropertyMap) -> InvalidValueTypeError {
    OperationParameter::create_from_properties(properties)
        .expect_err("PROJ refuses this value type")
}

fn text(value: &str) -> PropertyValue {
    PropertyValue::Text(value.to_owned())
}

/// `TEST(common, identifiedobject_alias_invalid_type)`: a `Citation` is not an alias.
#[test]
fn identifiedobject_alias_invalid_type() {
    let mut properties = PropertyMap::new();
    properties.set(
        ALIAS_KEY,
        PropertyValue::Citation(Citation::new("unexpected type")),
    );
    assert_eq!(refused(&properties).key(), ALIAS_KEY);
}

/// `TEST(common, identifiedobject_alias_array_of_invalid_type)`: an array holding a `Citation` is refused.
#[test]
fn identifiedobject_alias_array_of_invalid_type() {
    let mut properties = PropertyMap::new();
    properties.set(
        ALIAS_KEY,
        PropertyValue::Array(vec![PropertyValue::Citation(Citation::new(
            "unexpected type",
        ))]),
    );
    assert_eq!(refused(&properties).key(), ALIAS_KEY);
}

/// `TEST(common, identifiedobject_alias_array_of_string)`, with the aliases given as a `PropertyMap`
/// array value. PROJ 9.9.0 asserts two aliases, `alias1` then `alias2`.
#[test]
fn identifiedobject_alias_array_of_string_property_map() {
    let mut properties = PropertyMap::new();
    properties.set(
        ALIAS_KEY,
        PropertyValue::Array(vec![text("alias1"), text("alias2")]),
    );
    let obj = OperationParameter::create_from_properties(&properties)
        .expect("PROJ accepts an array of strings");
    assert_eq!(obj.aliases().len(), 2);
    assert_eq!(obj.aliases()[0], "alias1");
    assert_eq!(obj.aliases()[1], "alias2");
}

/// A single string is one alias. PROJ 9.9.0 `IdentifiedObject::Private::setAliases` takes the
/// `BoxedValue` string path, measured as one alias `alias1`.
#[test]
fn identifiedobject_alias_string_property_map() {
    let mut properties = PropertyMap::new();
    properties.set(ALIAS_KEY, text("alias1"));
    let obj = OperationParameter::create_from_properties(&properties)
        .expect("PROJ accepts a string alias");
    assert_eq!(obj.aliases().len(), 1);
    assert_eq!(obj.aliases()[0], "alias1");
    assert_eq!(obj.alias(), "alias1");
}

/// An empty array gives no alias, measured with PROJ 9.9.0.
#[test]
fn identifiedobject_alias_empty_array_property_map() {
    let mut properties = PropertyMap::new();
    properties.set(ALIAS_KEY, PropertyValue::Array(Vec::new()));
    let obj = OperationParameter::create_from_properties(&properties)
        .expect("PROJ accepts an empty array");
    assert!(obj.aliases().is_empty());
    assert_eq!(obj.alias(), "");
}

/// Setting a key twice keeps the last value, as `PropertyMap::set` replaces it in place.
#[test]
fn identifiedobject_alias_set_twice_keeps_last_property_map() {
    let mut properties = PropertyMap::new();
    properties.set(ALIAS_KEY, text("first"));
    properties.set(ALIAS_KEY, text("second"));
    let obj = OperationParameter::create_from_properties(&properties).expect("PROJ accepts it");
    assert_eq!(obj.aliases().len(), 1);
    assert_eq!(obj.aliases()[0], "second");
}

/// Alias values PROJ 9.9.0 refuses besides `Citation`: a boolean, an integer, and an array
/// that mixes a string with a boolean.
#[test]
fn identifiedobject_alias_refuses_other_scalars_property_map() {
    let mut boolean = PropertyMap::new();
    boolean.set(ALIAS_KEY, PropertyValue::Bool(true));
    assert_eq!(refused(&boolean).key(), ALIAS_KEY);

    let mut integer = PropertyMap::new();
    integer.set(ALIAS_KEY, PropertyValue::Integer(3));
    assert_eq!(refused(&integer).key(), ALIAS_KEY);

    let mut mixed = PropertyMap::new();
    mixed.set(
        ALIAS_KEY,
        PropertyValue::Array(vec![text("ok"), PropertyValue::Bool(true)]),
    );
    assert_eq!(refused(&mixed).key(), ALIAS_KEY);
}

/// The other keys of `IdentifiedObject::setProperties` with a valid value, measured with PROJ 9.9.0:
/// name `the name`, remarks `rem`, deprecated true.
#[test]
fn identifiedobject_name_remarks_deprecated_property_map() {
    let mut properties = PropertyMap::new();
    properties.set(NAME_KEY, text("the name"));
    properties.set(REMARKS_KEY, text("rem"));
    properties.set(DEPRECATED_KEY, PropertyValue::Bool(true));
    let obj = OperationParameter::create_from_properties(&properties).expect("PROJ accepts them");
    assert_eq!(obj.name(), "the name");
    assert_eq!(obj.remarks(), "rem");
    assert!(obj.is_deprecated());
}

/// Wrong value types for the other keys, measured with PROJ 9.9.0: each is refused, and the error
/// names the key.
#[test]
fn identifiedobject_other_keys_refuse_wrong_type_property_map() {
    let mut name = PropertyMap::new();
    name.set(NAME_KEY, PropertyValue::Citation(Citation::new("invalid")));
    assert_eq!(refused(&name).key(), NAME_KEY);

    let mut remarks = PropertyMap::new();
    remarks.set(REMARKS_KEY, PropertyValue::Bool(true));
    assert_eq!(refused(&remarks).key(), REMARKS_KEY);

    let mut deprecated = PropertyMap::new();
    deprecated.set(DEPRECATED_KEY, text("yes"));
    assert_eq!(refused(&deprecated).key(), DEPRECATED_KEY);

    let mut identifier = PropertyMap::new();
    identifier.set(IDENTIFIERS_KEY, text("string not allowed"));
    assert_eq!(refused(&identifier).key(), IDENTIFIERS_KEY);

    let mut identifier_array = PropertyMap::new();
    identifier_array.set(
        IDENTIFIERS_KEY,
        PropertyValue::Array(vec![PropertyValue::Citation(Citation::new("x"))]),
    );
    assert_eq!(refused(&identifier_array).key(), IDENTIFIERS_KEY);
}

/// `IDENTIFIERS_KEY` with one identifier, then with two in an array, measured with PROJ 9.9.0
/// (`TEST(common, identifiedobject_identifier_array_of_identifier)` asserts the two codes).
#[test]
fn identifiedobject_identifier_property_map() {
    let mut single = PropertyMap::new();
    single.set(
        IDENTIFIERS_KEY,
        PropertyValue::Identifier(Identifier::create("single_code")),
    );
    let obj = OperationParameter::create_from_properties(&single).expect("PROJ accepts it");
    assert_eq!(obj.identifiers().len(), 1);
    assert_eq!(obj.identifiers()[0].code(), "single_code");

    let mut array = PropertyMap::new();
    array.set(
        IDENTIFIERS_KEY,
        PropertyValue::Array(vec![
            PropertyValue::Identifier(Identifier::create("identifier_code1")),
            PropertyValue::Identifier(Identifier::create("identifier_code2")),
        ]),
    );
    let obj = OperationParameter::create_from_properties(&array).expect("PROJ accepts it");
    assert_eq!(obj.identifiers().len(), 2);
    assert_eq!(obj.identifiers()[0].code(), "identifier_code1");
    assert_eq!(obj.identifiers()[1].code(), "identifier_code2");
}
