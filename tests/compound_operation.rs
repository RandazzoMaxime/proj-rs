//! Operations between a compound CRS (horizontal geographic plus vertical) and a 3D geographic
//! CRS, the vertical branch of PROJ's `createOperationsCompoundToGeog`.
//!
//! `EPSG:4326+3855` (WGS 84 plus EGM2008 height) to `EPSG:4979` has two registered inverse
//! geoid transformations and the ballpark change of vertical unit, which PROJ also lists.
//! The names below are the ones PROJ gives; the order is not asserted here.

use proj_rs::iso19111::{
    best_pipeline, create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext,
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
fn compound_to_geographic_3d_lists_geoid_and_ballpark_candidates() {
    let names: Vec<String> = candidates("EPSG:4326+3855", "EPSG:4979")
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names
        .iter()
        .any(|name| name == "Transformation from EGM2008 height to WGS 84 (ballpark vertical transformation, without ellipsoid height to vertical height correction)"));
    assert!(names
        .iter()
        .any(|name| name == "Inverse of WGS 84 to EGM2008 height (1)"));
    assert!(names
        .iter()
        .any(|name| name == "Inverse of WGS 84 to EGM2008 height (2)"));
}

#[test]
fn geographic_3d_to_compound_is_the_inverse_of_the_vertical_branch() {
    let names: Vec<String> = candidates("EPSG:4979", "EPSG:4326+5773")
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect();
    assert!(
        names
            .iter()
            .any(|name| name == "WGS 84 to EGM96 height (1)"),
        "{names:?}"
    );
}

#[test]
fn vertical_pair_of_different_datums_lists_the_ballpark_change_of_unit() {
    let names: Vec<String> = candidates("EPSG:5773", "EPSG:5782")
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect();
    assert_eq!(
        names,
        ["Transformation from EGM96 height to Alicante height (ballpark vertical transformation)"]
    );
}

#[test]
fn compound_pair_with_the_same_geographic_part_keeps_the_ballpark() {
    // PROJ also lists a grid-based candidate first, which is not ported.
    let names: Vec<String> = candidates("EPSG:4326+5773", "EPSG:4326+5782")
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect();
    assert!(
        names.iter().any(|name| name
            == "Transformation from EGM96 height to Alicante height (ballpark vertical transformation)"),
        "{names:?}"
    );
}

#[test]
fn compound_pair_builds_a_pipeline() {
    let (definition, _) = best_pipeline("EPSG:4326+3855", "EPSG:4979").expect("pipeline");
    assert!(definition.contains("+proj="), "{definition}");
}
