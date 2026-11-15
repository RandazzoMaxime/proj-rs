//! The candidate set of a compound CRS to or from a geographic or geocentric CRS (PROJ 9.9.0,
//! `coordinateoperationfactory.cpp` `createOperationsCompoundToGeog`, `createOperationsToGeod`,
//! `createOperationsGeodToGeod`).
//!
//! The expected lists are the ones `projinfo --summary --spatial-test intersects` prints for PROJ
//! 9.9.0, on a database copy with no grid file. They are PROJ's values, not this port's.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, SpatialCriterion,
};

/// The candidate names of `projinfo -s source -t target --summary --spatial-test intersects`.
fn names(source: &str, target: &str) -> Vec<String> {
    std::env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
        .iter()
        .map(|operation| CoordinateOperation::name(operation).to_owned())
        .collect()
}

/// `projinfo -s EPSG:11087 -t EPSG:7815 --summary --spatial-test intersects` (PROJ 9.9.0): a geographic CRS to a geocentric CRS of another datum: after the registry result, the
/// geographic CRS of the geocentric one and the full `createOperations` from it (the second
/// candidate, coordinateoperationfactory.cpp:5314-5331).
#[test]
fn geographic_to_geocentric_of_another_datum_reads_the_registry() {
    assert_eq!(
        names("EPSG:11087", "EPSG:7815"),
        [
            "ETRS89-FRO [2008] to WGS 84 (1) + WGS 84 to WGS 84 (Transit) + Conversion from WGS 84 (Transit) (geog2D) to WGS 84 (Transit) (geocentric)",
            "ETRS89-FRO [2008] to WGS 84 (1) + WGS 84 to WGS 84 (Transit) + axis order change (geographic3D horizontal) + Conversion from WGS 84 (Transit) (geographic) to WGS 84 (Transit)",
        ]
    );
}
