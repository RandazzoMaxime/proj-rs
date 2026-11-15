//! The candidate set of a pivot through an intermediate datum (PROJ 9.9.0,
//! `coordinateoperationfactory.cpp` `findsOpsInRegistryWithIntermediate`, `createOperationsWithDatumPivot`
//! and `factory.cpp` `createBetweenGeodeticCRSWithDatumBasedIntermediates`).
//!
//! The expected lists are the ones `projinfo --summary --spatial-test intersects` prints for PROJ
//! 9.9.0, on a database copy with no grid file. They are PROJ's values, not this port's.
//! `projinfo` gives no authority to its context, so the tests use the empty authority.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, CrsExtentUse, DatabaseContext, GridAvailabilityUse,
    SpatialCriterion,
};

/// The candidates of `projinfo -s source -t target --summary --spatial-test intersects`, with
/// the grid policy given (`DiscardIfMissing` is `--grid-check discard_missing`).
fn candidates(source: &str, target: &str, grids: GridAvailabilityUse) -> Vec<CoordinateOperation> {
    candidates_with_extent_use(source, target, grids, CrsExtentUse::Smallest)
}

/// [`candidates`] with the CRS extent use given (`projinfo --crs-extent-use`).
fn candidates_with_extent_use(
    source: &str,
    target: &str,
    grids: GridAvailabilityUse,
    extent_use: CrsExtentUse,
) -> Vec<CoordinateOperation> {
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
    context.set_crs_extent_use(extent_use);
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

/// `projinfo -s EPSG:11108 -t EPSG:11128 --summary --spatial-test intersects` (PROJ 9.9.0): three
/// candidates, the ETRS89 pair at 0.1 m among them. The port's `projinfo` used the `EPSG` context
/// and listed two of them.
#[test]
fn etrs89_ensemble_pair_has_the_three_candidates_of_projinfo() {
    let operations = candidates(
        "EPSG:11108",
        "EPSG:11128",
        GridAvailabilityUse::UseForSorting,
    );
    assert_eq!(
        names(&operations),
        [
            "Conversion from ETRS89-PRT [1995] (geog2D) to ETRS89-PRT [1995] (geocentric) + ETRS89-PRT [1995] to ETRF2000 (2) + Inverse of ETRS89-ESP [ERGNSS] to ETRF2000 (1) + Conversion from ETRS89-ESP [ERGNSS] (geocentric) to ETRS89-ESP [ERGNSS] (geog2D)",
            "Inverse of ETRS89 to ETRS89-PRT [1995] + ETRS89 to ETRS89-ESP [ERGNSS]",
            "Ballpark geographic offset from ETRS89-PRT [1995] to ETRS89-ESP [ERGNSS]",
        ]
    );
}

/// `projinfo -s EPSG:4746 -t EPSG:4978 --summary --spatial-test intersects --grid-check
/// discard_missing` (PROJ 9.9.0, no grid file): the one candidate is the pivot through ETRS89, whose
/// legs need no grid. The pivot through WGS 84 with a grid leg is discarded, as the registry
/// operations with a missing grid are.
#[test]
fn pivot_leg_with_a_missing_grid_is_discarded() {
    let operations = candidates(
        "EPSG:4746",
        "EPSG:4978",
        GridAvailabilityUse::DiscardIfMissing,
    );
    assert_eq!(
        names(&operations),
        ["PD/83 to ETRS89 (1) + ETRS89 to WGS 84 (1) + Conversion from WGS 84 (geog2D) to WGS 84 (geocentric)"]
    );
}

/// `projinfo -s EPSG:3701 -t EPSG:32545 --summary --spatial-test intersects` (PROJ 9.9.0): the
/// ballpark only. The pivot through WGS 84 has a leg that does not meet the extent of UTM zone 45S,
/// the target CRS: the intermediate search drops that leg, as `createFromCRSCodesWithIntermediates`
/// does with the CRS extents of the outermost call.
#[test]
fn pivot_with_a_leg_outside_the_crs_extent_is_dropped() {
    let operations = candidates(
        "EPSG:3701",
        "EPSG:32545",
        GridAvailabilityUse::UseForSorting,
    );
    assert_eq!(
        names(&operations),
        ["Inverse of Wisconsin Transverse Mercator 83 + Ballpark geographic offset from NAD83(NSRS2007) to WGS 72BE + UTM zone 45S"]
    );
}

/// `projinfo -s EPSG:3701 -t EPSG:32545 --summary --spatial-test intersects --crs-extent-use none`
/// (PROJ 9.9.0): the same pivot is kept, with the ballpark after it. The extent of the CRS is what
/// drops it in the test above.
#[test]
fn pivot_with_a_leg_outside_the_crs_extent_is_kept_without_extent_use() {
    let operations = candidates_with_extent_use(
        "EPSG:3701",
        "EPSG:32545",
        GridAvailabilityUse::UseForSorting,
        CrsExtentUse::Ignore,
    );
    assert_eq!(
        names(&operations),
        [
            "Inverse of Wisconsin Transverse Mercator 83 + NAD83(NSRS2007) to WGS 84 (1) + Inverse of WGS 72BE to WGS 84 (1) + UTM zone 45S",
            "Inverse of Wisconsin Transverse Mercator 83 + Ballpark geographic offset from NAD83(NSRS2007) to WGS 72BE + UTM zone 45S",
        ]
    );
}

/// `projinfo -s EPSG:26898 -t EPSG:26957 --summary --spatial-test intersects` (PROJ 9.9.0): two
/// pivots through WGS 84, one with the NAD83(CSRS) (2) leg and one with the (1) leg, and no
/// ballpark. The pivot is searched by the inner candidate pass of `createOperationsWithDatumPivot`
/// and the intermediate block that follows it (factory.rs `datum_pivot_operations`, `create_candidates_at`).
#[test]
fn mtm1_to_delaware_keeps_both_wgs84_pivots() {
    let operations = candidates(
        "EPSG:26898",
        "EPSG:26957",
        GridAvailabilityUse::UseForSorting,
    );
    assert_eq!(
        names(&operations),
        [
            "Inverse of MTM zone 1 + NAD83(CSRS) to WGS 84 (2) + Inverse of NAD83 to WGS 84 (1) + SPCS83 Delaware zone (meter)",
            "Inverse of MTM zone 1 + NAD83(CSRS) to WGS 84 (1) + Inverse of NAD83 to WGS 84 (1) + SPCS83 Delaware zone (meter)",
        ]
    );
}

/// `projinfo -s EPSG:26898 -t EPSG:26957 --summary --spatial-test intersects --crs-extent-use none`
/// (PROJ 9.9.0): the 2 m direct operation, then the ballpark. Already the port's output.
#[test]
fn mtm1_to_delaware_without_extent_use_keeps_the_direct_operation() {
    let operations = candidates_with_extent_use(
        "EPSG:26898",
        "EPSG:26957",
        GridAvailabilityUse::UseForSorting,
        CrsExtentUse::Ignore,
    );
    assert_eq!(
        names(&operations),
        [
            "Inverse of MTM zone 1 + Inverse of NAD83 to NAD83(CSRS) (4) + SPCS83 Delaware zone (meter)",
            "Inverse of MTM zone 1 + Ballpark geographic offset from NAD83(CSRS) to NAD83 + SPCS83 Delaware zone (meter)",
        ]
    );
}

/// `projinfo -s EPSG:7815 -t EPSG:4232 --summary --spatial-test intersects` (PROJ 9.9.0):
/// `Fahud to WGS 84 (1)` is superseded by `(3)`, both legs of the datum-based pivot, so it is
/// dropped (`createBetweenGeodeticCRSWithDatumBasedIntermediates`, factory.cpp:8576-8658).
#[test]
fn datum_based_pivot_drops_a_superseded_leg() {
    let operations = candidates("EPSG:7815", "EPSG:4232", GridAvailabilityUse::UseForSorting);
    assert_eq!(
        names(&operations),
        [
            "Conversion from WGS 84 (Transit) (geocentric) to WGS 84 (Transit) (geog2D) + Inverse of WGS 84 to WGS 84 (Transit) + Inverse of Fahud to WGS 84 (3)",
            "Conversion from WGS 84 (Transit) (geocentric) to WGS 84 (Transit) (geog2D) + Inverse of WGS 84 to WGS 84 (Transit) + Inverse of Fahud to WGS 84 (2)",
        ]
    );
}

/// `projinfo -s EPSG:7912 -t EPSG:4959 --summary --spatial-test intersects` (PROJ 9.9.0): of the
/// chain `ITRF96 to NZGD2000 (1)` to `(7)`, each replacing the one before, only `(7)` is left.
#[test]
fn datum_based_pivot_keeps_the_last_replacement_of_a_chain() {
    let operations = candidates("EPSG:7912", "EPSG:4959", GridAvailabilityUse::UseForSorting);
    assert_eq!(
        names(&operations),
        [
            "Conversion from ITRF2014 (geog3D) to ITRF2014 (geocentric) + ITRF2014 to ITRF96 (2) + Conversion from ITRF96 (geocentric) to ITRF96 (geog3D) + ITRF96 to NZGD2000 (7)",
            "Conversion from ITRF2014 (geog3D) to ITRF2014 (geocentric) + Inverse of ITRF96 to ITRF2014 (1) + Conversion from ITRF96 (geocentric) to ITRF96 (geog3D) + ITRF96 to NZGD2000 (7)",
            "Ballpark geographic offset from ITRF2014 to NZGD2000",
        ]
    );
}

/// `projinfo -s EPSG:20037 -t EPSG:6548 --summary --spatial-test intersects` (PROJ 9.9.0): the
/// three replacements share one velocity grid. Their first step, a `Geographic3D offsets` whose
/// source is the compound CRS, has no PROJ string, so `removeDuplicateOps` keeps all three.
#[test]
fn unexportable_candidates_are_not_merged_as_duplicates() {
    let operations = candidates(
        "EPSG:20037",
        "EPSG:6548",
        GridAvailabilityUse::UseForSorting,
    );
    let tail = "Conversion from NAD83(2011) (geocentric) to NAD83(2011) (geog2D) + SPCS83 Ohio North zone (meter)";
    let leg = |version: &str, rank: &str| {
        format!(
            "Null geographic offset from NAD83(CSRS)v4 (geog2D) to NAD83(CSRS)v4 (geog3D) + NAD83(CSRS)v4 to NAD83(CSRS){version} ({rank}) + Conversion from NAD83(CSRS){version} (geog3D) to NAD83(CSRS){version} (geocentric) + NAD83(CSRS){version} to NAD83(2011) (1) + {tail}"
        )
    };
    assert_eq!(
        names(&operations),
        [
            "Ballpark geographic offset from NAD83(CSRS)v4 to NAD83(2011) + SPCS83 Ohio North zone (meter)".to_owned(),
            leg("v8", "3"),
            leg("v7", "3"),
            leg("v6", "4"),
        ]
    );
}

/// `projinfo -s EPSG:10874 -t EPSG:4908 --summary --spatial-test intersects` (PROJ 9.9.0): GR96
/// is defined by the datum ensemble EPSG:1421. The datum pivot reads the CRS of that ensemble
/// (`DatumEnsemble::asDatum` keeps its identifier), so it finds the WGS 84 legs through GR96.
#[test]
fn datum_pivot_reads_the_crs_of_a_datum_ensemble() {
    let operations = candidates(
        "EPSG:10874",
        "EPSG:4908",
        GridAvailabilityUse::UseForSorting,
    );
    assert_eq!(
        names(&operations),
        [
            "ETRS89-NOR [EUREF89] to WGS 84 (1) + Inverse of GR96 to WGS 84 (1) + Conversion from GR96 (geog3D) to GR96 (geocentric)",
            "Ballpark geographic offset from ETRS89-NOR [EUREF89] to GR96 (geographic) + Conversion from GR96 (geographic) to GR96",
        ]
    );
}
