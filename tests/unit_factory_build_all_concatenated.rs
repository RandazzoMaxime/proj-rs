//! PROJ 9.9.0 `test/unit/test_factory.cpp`, every concatenated operation of the EPSG authority
//! can be built, except the ones PROJ rejects.

use std::collections::BTreeSet;

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, DatabaseContext, DeprecatedCodes, ObjectType,
};

/// `TEST(factory, AuthorityFactory_build_all_concatenated)`
#[test]
fn factory_authority_factory_build_all_concatenated() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let all = factory
        .get_authority_codes(ObjectType::ConcatenatedOperation, DeprecatedCodes::Included)
        .expect("codes");
    let no_deprecated = factory
        .get_authority_codes(ObjectType::ConcatenatedOperation, DeprecatedCodes::Excluded)
        .expect("codes");
    assert!(no_deprecated.len() < all.len());
    let rejected: BTreeSet<&str> = [
        "8422", "8481", "8482", "8565", "8566", "8572", "11005", "11398", "11206",
    ]
    .into_iter()
    .collect();
    let mut failures = Vec::new();
    for code in &all {
        let built = factory.create_coordinate_operation(code, AlternativeGridNames::Official);
        if rejected.contains(code.as_str()) {
            if built.is_ok() {
                failures.push(format!("{code} should fail"));
            }
        } else if let Err(error) = built {
            failures.push(format!("{code}: {error:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures: {:?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
}
