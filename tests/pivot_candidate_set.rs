//! The candidate set of a pivot through an intermediate datum (PROJ 9.9.0,
//! `coordinateoperationfactory.cpp` `findsOpsInRegistryWithIntermediate`, `createOperationsWithDatumPivot`
//! and `factory.cpp` `createBetweenGeodeticCRSWithDatumBasedIntermediates`).
//!
//! The expected lists are the ones `projinfo --summary --spatial-test intersects` prints for PROJ
//! 9.9.0, on a database copy with no grid file. They are PROJ's values, not this port's.
//! `projinfo` gives no authority to its context, so the tests use the empty authority.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// The candidates of `projinfo -s source -t target --summary --spatial-test intersects`, with
/// the grid policy given (`DiscardIfMissing` is `--grid-check discard_missing`).
fn candidates(source: &str, target: &str, grids: GridAvailabilityUse) -> Vec<CoordinateOperation> {
    // The database only: the oracle runs on a copy with no grid file, and `data/tests` holds the
    // NADCON grid (`conus`) that would make the NAD27 legs available.
    std::env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(grids);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

/// The candidate names in the order PROJ lists them.
fn names(operations: &[CoordinateOperation]) -> Vec<&str> {
    operations.iter().map(CoordinateOperation::name).collect()
}

/// `projinfo -s EPSG:4746 -t EPSG:4230 --summary` (PROJ 9.9.0): seven candidates, none through
/// an ESRI or IGNF intermediate. The port used to add the ESRI legs of the WGS 84 pivot.
#[test]
fn ed50_pivot_reads_only_the_authorities_of_the_pair() {
    let operations = candidates("EPSG:4746", "EPSG:4230", GridAvailabilityUse::UseForSorting);
    assert_eq!(
        names(&operations),
        [
            "PD/83 to ETRS89 (1) + Inverse of ED50 to ETRS89 (10)",
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (2)",
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (1)",
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (18)",
            "PD/83 to ETRS89 (2) + Inverse of ED50 to ETRS89 (10)",
            "PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (17)",
            "Ballpark geographic offset from PD/83 to ED50",
        ]
    );
}

/// `projinfo -s EPSG:26915 -t EPSG:26715 --summary --spatial-test intersects --grid-check
/// discard_missing` (PROJ 9.9.0, no grid file): 11 candidates, all through `NAD83 to WGS 84 (1)`.
/// The NAD27 legs that need a grid are discarded, and the pivot is searched through the CRS of
/// the registry (`createFromCRSCodesWithIntermediates`), not through every datum-based leg.
#[test]
fn utm15n_pair_without_grids_pivots_through_wgs84() {
    let operations = candidates(
        "EPSG:26915",
        "EPSG:26715",
        GridAvailabilityUse::DiscardIfMissing,
    );
    assert_eq!(
        names(&operations),
        [
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (3) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (4) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (5) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (6) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (13) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (11) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (18) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (84) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (81) + UTM zone 15N",
            "Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (82) + UTM zone 15N",
            "Inverse of UTM zone 15N + Ballpark geographic offset from NAD83 to NAD27 + UTM zone 15N",
        ]
    );
}
