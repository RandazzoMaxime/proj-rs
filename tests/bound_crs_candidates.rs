//! Candidate operations between a CRS on a named datum (`+datum=NAD83`, `+datum=nzgd49`) and a CRS
//! given by an ellipsoid and `+towgs84`, which PROJ reads as a bound CRS
//! (`createOperationsBoundToGeog`, `coordinateoperationfactory.cpp`, PROJ 9.9.0). The expected
//! names and areas are what `projinfo -s ... -t ... --summary` 9.9.0 prints for each pair.

mod common;

use proj_rs::iso19111::{
    create_from_user_input, select_alternatives, BestPolicy, CandidateFilter, CoordinateOperation,
    CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext, Selection,
};
use proj_rs::Xyzt;

const INTL_BOUND: &str = "+proj=longlat +ellps=intl +towgs84=-192.2272,207.1305,-46.9829,-2.0011,-2.5186,-2.2355,1.2420 +type=crs";
const NZGD49_BOUND: &str = "+proj=longlat +ellps=NWL9D +towgs84=94.8512,-118.1941,-205.9635,-2.6921,-1.4301,1.2408,-2.2088 +type=crs";

fn candidates(source: &str, target: &str) -> Vec<CoordinateOperation> {
    common::use_fixture_search_path();
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input(source, &db).expect("source");
    let target = create_from_user_input(target, &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations")
}

fn area(operation: &CoordinateOperation) -> Option<&str> {
    operation.domains().first().and_then(|domain| domain.area())
}

/// A bound source and a NAD83 target: the hub (WGS 84) to NAD83 operations come from the
/// registry, and the concatenation takes the area of the one step that has one.
#[test]
fn bound_source_to_nad83_keeps_the_area_of_the_registry_step() {
    let operations = candidates(INTL_BOUND, "+proj=longlat +datum=NAD83 +type=crs");
    let head: Vec<(&str, Option<&str>)> = operations
        .iter()
        .take(4)
        .map(|operation| (operation.name(), area(operation)))
        .collect();
    assert_eq!(
        head,
        [
            (
                "Transformation from unknown to WGS84 + Inverse of NAD83 to WGS 84 (24) + axis order change (2D)",
                Some("United States (USA) - Delaware, District of Columbia (DC) and Maryland."),
            ),
            (
                "Transformation from unknown to WGS84 + Inverse of NAD83 to WGS 84 (1) + axis order change (2D)",
                Some("North America - onshore and offshore: Canada; United States (USA) - Alaska (mainland) and conterminous (contiguous) lower 48 states (CONUS)."),
            ),
            (
                "Transformation from unknown to WGS84 + Inverse of NAD83 to WGS 84 (3) + axis order change (2D)",
                Some("United States (USA) - Hawaii - main islands onshore and offshore."),
            ),
            (
                "Transformation from unknown to WGS84 + Inverse of NAD83 to WGS 84 (2) + axis order change (2D)",
                Some("United States (USA) - Alaska - Aleutian Islands onshore."),
            ),
        ]
    );
}

/// The reverse pair, a NZGD49 source and a bound target: the registry steps come first, the
/// inverse of the binding last.
#[test]
fn nzgd49_to_bound_target_keeps_the_area_of_the_registry_step() {
    let operations = candidates("+proj=longlat +datum=nzgd49 +type=crs", NZGD49_BOUND);
    let areas: Vec<Option<&str>> = operations.iter().map(area).collect();
    let nz =
        Some("New Zealand - North Island, South Island, Stewart Island - onshore and nearshore.");
    assert_eq!(
        areas,
        [
            nz,
            nz,
            nz,
            Some("New Zealand - North Island, South Island, Stewart Island - onshore."),
            Some("World"),
        ]
    );
}

fn forward(source: &str, target: &str, x: f64, y: f64) -> Xyzt {
    common::use_fixture_search_path();
    let selection = select_alternatives(source, target, BestPolicy::Discard, CandidateFilter::NONE)
        .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("{source} to {target} has several candidates");
    };
    alternatives
        .forward(Xyzt {
            x,
            y,
            z: 0.0,
            t: 0.0,
        })
        .expect("a candidate serves the point")
}

/// `cs2cs +proj=longlat +ellps=aust_SA +towgs84=-220.4747,182.1311,-62.8347 +to +proj=utm
/// +zone=50 +datum=ire65` (PROJ 9.9.0, `PROJ_ONLY_BEST_DEFAULT=NO`) lists two candidates, TM65 to
/// WGS 84 (2) limited to Ireland and the ballpark one. A point in Ireland takes the first, a point
/// in Latvia the ballpark. The bound source has no geodetic base of its own, which the selection
/// refused, and the facade then took one candidate for every point.
#[test]
fn bound_source_to_projected_target_picks_the_candidate_by_area() {
    let source = "+proj=longlat +ellps=aust_SA +towgs84=-220.4747,182.1311,-62.8347";
    let target = "+proj=utm +zone=50 +datum=ire65";
    let ireland = forward(source, target, -8.0, 53.0);
    assert!(
        (ireland.x - -2_949_544.926_487_443).abs() < 5e-4,
        "x {}",
        ireland.x
    );
    assert!(
        (ireland.y - 12_602_040.871_912_831).abs() < 5e-4,
        "y {}",
        ireland.y
    );
    let latvia = forward(source, target, 23.7, 56.9);
    assert!(
        (latvia.x - -3_406_700.849_216_791).abs() < 5e-4,
        "x {}",
        latvia.x
    );
    assert!(
        (latvia.y - 10_236_032.000_499_101).abs() < 5e-4,
        "y {}",
        latvia.y
    );
}

/// `cs2cs -d 10 +proj=longlat +ellps=intl +towgs84=-192.2272,... +to +proj=longlat +datum=NAD83` on
/// `-44.19 -17.46` (Brazil), PROJ 9.9.0: outside the Maryland grid and every other area, the
/// ballpark candidate gives the point. The port printed `* * inf`.
#[test]
fn bound_source_to_nad83_outside_every_area_takes_the_ballpark_candidate() {
    let source = "+proj=longlat +ellps=intl +towgs84=-192.2272,207.1305,-46.9829,-2.0011,-2.5186,-2.2355,1.2420";
    let coord = forward(source, "+proj=longlat +datum=NAD83", -44.19, -17.46);
    assert!((coord.x - -44.190_456_384_0).abs() < 5e-9, "x {}", coord.x);
    assert!((coord.y - -17.459_804_568_5).abs() < 5e-9, "y {}", coord.y);
}
