//! The candidates of a projected pair whose registry operations are followed by the operations
//! through the base geographic CRS (PROJ 9.9.0, `coordinateoperationfactory.cpp`
//! `createOperations`, the `derivedSrc` branch: 3694-3701, and `createOperationsDerivedTo`,
//! 5523-5575).
//!
//! The expected lists are the ones `projinfo -s S -t T --summary --spatial-test intersects` prints
//! for PROJ 9.9.0 on a database copy with no grid file. They are PROJ's values, not this port's.
//! `projinfo` gives no authority to its context, so the tests use the empty authority.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// The candidates of `projinfo -s source -t target --summary --spatial-test intersects` (the
/// projinfo default grid policy, `UseForSorting`).
fn candidates(source: &str, target: &str) -> Vec<CoordinateOperation> {
    // The database only: the oracle runs on a copy with no grid file, and `data/tests` holds the
    // NADCON grid (`conus`) that would make the NAD27 legs available.
    std::env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::UseForSorting);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

/// The candidate names in the order PROJ lists them.
fn names(operations: &[CoordinateOperation]) -> Vec<&str> {
    operations.iter().map(CoordinateOperation::name).collect()
}

/// `projinfo -s EPSG:3912 -t EPSG:3794 --summary --spatial-test intersects` (PROJ 9.9.0): 36
/// candidates. Twelve go through the base geographic CRS, `Inverse of Slovene National Grid`
/// (the inverse projection) then `MGI 1901 to ETRS89-SVN [D96] (n)` then `Slovene National Grid`;
/// the 24 others are the registry operations of the pair. The port listed the 24 only.
#[test]
fn d48_gk_to_slovene_grid_has_the_thirty_six_candidates_of_projinfo() {
    let operations = candidates("EPSG:3912", "EPSG:3794");
    assert_eq!(
        names(&operations),
        [
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (1) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (12) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (3) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (2) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (7) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (4) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (10) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (9) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (6) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (11) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (5) + Slovene National Grid",
        "Inverse of Slovene National Grid + MGI 1901 to ETRS89-SVN [D96] (8) + Slovene National Grid",
        "D48/GK to D96/TM (10)",
        "D48/GK to D96/TM (2)",
        "D48/GK to D96/TM (24)",
        "D48/GK to D96/TM (22)",
        "D48/GK to D96/TM (20)",
        "D48/GK to D96/TM (4)",
        "D48/GK to D96/TM (8)",
        "D48/GK to D96/TM (5)",
        "D48/GK to D96/TM (7)",
        "D48/GK to D96/TM (21)",
        "D48/GK to D96/TM (23)",
        "D48/GK to D96/TM (6)",
        "D48/GK to D96/TM (15)",
        "D48/GK to D96/TM (1)",
        "D48/GK to D96/TM (12)",
        "D48/GK to D96/TM (9)",
        "D48/GK to D96/TM (3)",
        "D48/GK to D96/TM (13)",
        "D48/GK to D96/TM (11)",
        "D48/GK to D96/TM (19)",
        "D48/GK to D96/TM (18)",
        "D48/GK to D96/TM (16)",
        "D48/GK to D96/TM (17)",
        "D48/GK to D96/TM (14)",
        ]
    );
}

/// The reverse pair (PROJ 9.9.0): 36 candidates, the twelve datum legs inverted.
#[test]
fn slovene_grid_to_d48_gk_has_the_thirty_six_candidates_of_projinfo() {
    let operations = candidates("EPSG:3794", "EPSG:3912");
    assert_eq!(operations.len(), 36);
}

/// `projinfo -s EPSG:23031 -t EPSG:25831 --summary --spatial-test intersects` (PROJ 9.9.0): 11
/// candidates, the registry operation `EPSG:5166` and ten `Inverse of UTM zone 31N + ED50 to
/// ETRS89 (n) + UTM zone 31N` legs through the geographic CRS. The port listed `EPSG:5166` only.
#[test]
fn ed50_utm31_to_etrs89_utm31_has_the_eleven_candidates_of_projinfo() {
    let operations = candidates("EPSG:23031", "EPSG:25831");
    assert_eq!(
        names(&operations),
        [
        "Inverse of UTM zone 31N + ED50 to ETRS89 (10) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (1) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (7) + UTM zone 31N",
        "ED50 / UTM zone 31N to ETRS89 / UTM zone 31N (1)",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (6) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (15) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (12) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (14) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (16) + UTM zone 31N",
        "Inverse of UTM zone 31N + ED50 to ETRS89 (17) + UTM zone 31N",
        "Inverse of UTM zone 31N + Ballpark geographic offset from ED50 to ETRS89 + UTM zone 31N",
        ]
    );
}
