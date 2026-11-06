//! The area of use (domain of validity) of a concatenated operation built by the factory: the
//! intersection of the extents of its steps, a conversion counting as the world
//! (`ConcatenatedOperation::createComputeMetadata`, `oputils.cpp` `getExtent`, PROJ 9.9.0). The
//! expected names are the ones `projinfo --summary` 9.9.0 prints for the pair; they are PROJ's
//! values, not this port's.

mod common;

use proj_rs::iso19111::{
    create_from_user_input, select_alternatives, BestPolicy, CandidateFilter, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, Selection,
};
use proj_rs::Xyzt;

fn candidates(source: &str, target: &str) -> Vec<CoordinateOperation> {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "EPSG");
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

/// The area name of the first domain, or `None` when the operation has no domain.
fn area(operation: &CoordinateOperation) -> Option<&str> {
    operation.domains().first().and_then(|domain| domain.area())
}

/// `projinfo -s EPSG:4746 -t EPSG:4978 --summary` (PROJ 9.9.0): the registry candidate is limited
/// to Germany - Thuringen by its transformation, and the conversion to geocentric adds no limit.
/// The ballpark candidate has no area of its own, so it is the world.
#[test]
fn geographic_to_geocentric_takes_the_area_of_its_transformation() {
    let operations = candidates("EPSG:4746", "EPSG:4978");
    let names: Vec<&str> = operations.iter().map(CoordinateOperation::name).collect();
    assert_eq!(
        names,
        [
            "PD/83 to WGS 84 (1) + Conversion from WGS 84 (geog2D) to WGS 84 (geocentric)",
            "Ballpark geographic offset from PD/83 to WGS 84 (geographic) + Conversion from WGS 84 (geographic) to WGS 84",
        ]
    );
    assert_eq!(area(&operations[0]), Some("Germany - Thuringen."));
    assert_eq!(area(&operations[1]), Some("World"));
}

/// The reverse pair, `projinfo -s EPSG:4978 -t EPSG:4746 --summary` (PROJ 9.9.0): the same two
/// areas on the inverse steps.
#[test]
fn geocentric_to_geographic_takes_the_area_of_its_transformation() {
    let operations = candidates("EPSG:4978", "EPSG:4746");
    assert_eq!(operations.len(), 2);
    assert_eq!(area(&operations[0]), Some("Germany - Thuringen."));
    assert_eq!(area(&operations[1]), Some("World"));
}

/// `cs2cs EPSG:4746 EPSG:4978` on `54 9 0` (Germany, outside Thuringia), PROJ 9.9.0: the
/// transformation's area does not cover the point, so the ballpark candidate, whose area is the
/// world, gives it. Before the area was carried, the transformation's world box kept the point.
#[test]
fn point_outside_thuringia_takes_the_ballpark_candidate() {
    common::use_fixture_search_path();
    let selection = select_alternatives(
        "EPSG:4746",
        "EPSG:4978",
        BestPolicy::Warn,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4746 to EPSG:4978 has several candidates");
    };
    let coord = alternatives
        .forward(Xyzt {
            x: 54.0,
            y: 9.0,
            z: 0.0,
            t: 0.0,
        })
        .expect("the ballpark candidate serves the point");
    assert!((coord.x - 3_710_957.52).abs() < 0.005, "x {}", coord.x);
    assert!((coord.y - 587_757.93).abs() < 0.005, "y {}", coord.y);
    assert!((coord.z - 5_136_743.83).abs() < 0.005, "z {}", coord.z);
}
