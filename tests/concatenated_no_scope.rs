//! A concatenated operation built by the factory has the extent of its steps and no scope:
//! `ConcatenatedOperation::createComputeMetadata` sets a domain from `getExtent` only, while each
//! step keeps its own scope. Expected values from `proj_create_operations` of PROJ 9.9.0
//! (`EPSG:28356` to `EPSG:7856`, default context).

mod common;

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext,
};

#[test]
fn gda94_to_gda2020_map_grid_has_no_scope() {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:28356", &db).expect("source");
    let target = create_from_user_input("EPSG:7856", &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let first = operations.first().expect("an operation");
    assert_eq!(
        first.name(),
        "Inverse of Map Grid of Australia zone 56 + GDA94 to GDA2020 (1) + \
         Map Grid of Australia zone 56"
    );
    let domain = first.domains().first().expect("a domain");
    assert_eq!(domain.scope(), None);
    assert_eq!(
        domain.area(),
        Some(
            "Australia including Lord Howe Island, Macquarie Island, Ashmore and Cartier \
             Islands, Christmas Island, Cocos (Keeling) Islands, Norfolk Island. All onshore \
             and offshore."
        )
    );
    let CoordinateOperation::Concatenated(concatenated) = first else {
        panic!("a concatenated operation");
    };
    let scopes: Vec<Option<&str>> = concatenated
        .steps()
        .iter()
        .map(|step| {
            step.operation()
                .domains()
                .first()
                .and_then(|domain| domain.scope())
        })
        .collect();
    assert_eq!(
        scopes,
        [
            Some("Engineering survey, topographic mapping."),
            Some("Transformation of GDA94 coordinates that have been derived through GNSS CORS."),
            Some("Engineering survey, topographic mapping."),
        ]
    );
}
