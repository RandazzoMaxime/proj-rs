//! The grid policy of `proj_create_crs_to_crs` reaches the operation factory (PROJ 9.9.0,
//! `crs_to_crs.cpp:566-573`): the grid availability of its context is KNOWN_AVAILABLE when
//! `ONLY_BEST` warns or errs, and DISCARD_OPERATION_IF_MISSING_GRID otherwise. The factory applies
//! it, intermediate CRS included. The expected values are what `cs2cs` 9.9.0 prints on a database
//! with no grid file (`PROJ_DATA` is the database directory only).

use proj_rs::iso19111::{
    select_alternatives, AlternativeError, BestPolicy, CandidateFilter, Selection,
};
use proj_rs::Xyzt;

fn grid_free_search_path() {
    std::env::set_var("PROJ_DATA", concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
}

fn point(x: f64, y: f64, z: f64) -> Xyzt {
    Xyzt { x, y, z, t: 0.0 }
}

/// `cs2cs -f %.8f EPSG:4267 EPSG:4269` at `40 -100`, with the default `only_best_default = off` of
/// the reference `proj.ini`, or `--only-best=no`: `39.99999688 -100.00041559`. With DISCARD the
/// factory lists the pivots through WGS 84 (`NAD27 to WGS 84 (6) + Inverse of NAD83 to WGS 84 (1)`,
/// 25 candidates); a filter on the KNOWN_AVAILABLE list kept only the ballpark offset, which does
/// not move the point.
#[test]
fn discard_policy_lists_the_pivots_through_wgs84() {
    grid_free_search_path();
    let selection = select_alternatives(
        "EPSG:4267",
        "EPSG:4269",
        BestPolicy::Discard,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4267 to EPSG:4269 has several candidates under DISCARD");
    };
    let coord = alternatives
        .forward(point(40.0, -100.0, 0.0))
        .expect("a pivot candidate serves the point");
    assert!((coord.x - 39.999_996_88).abs() < 1e-8, "lat {}", coord.x);
    assert!((coord.y - -100.000_415_59).abs() < 1e-8, "lon {}", coord.y);
}

/// `cs2cs --only-best -f %.8f EPSG:4258+5778 EPSG:4979`: a compound CRS on one side. The Austrian
/// geoid grid is missing, so a point inside Austria fails with the grid message (`47.5 14 0`); a
/// point outside it (`48.75 9 100`) takes the ballpark candidate and is the identity, because the
/// candidate is chosen per point.
#[test]
fn only_best_chooses_per_point_for_a_compound_on_one_side() {
    grid_free_search_path();
    let selection = select_alternatives(
        "EPSG:4258+5778",
        "EPSG:4979",
        BestPolicy::Error,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4258+5778 to EPSG:4979 has three candidates");
    };
    let outside = alternatives
        .forward(point(48.75, 9.0, 100.0))
        .expect("the ballpark candidate serves a point outside Austria");
    assert!((outside.x - 48.75).abs() < 1e-8, "lat {}", outside.x);
    assert!((outside.y - 9.0).abs() < 1e-8, "lon {}", outside.y);
    assert!((outside.z - 100.0).abs() < 1e-8, "height {}", outside.z);
    match alternatives.forward(point(47.5, 14.0, 0.0)) {
        Err(AlternativeError::MissingGrid(message)) => {
            assert!(
                message.contains("Grid at_bev_GEOID_GRS80_Oesterreich.tif is not available"),
                "message {message}"
            );
        }
        other => panic!("expected the missing grid inside Austria, got {other:?}"),
    }
}

/// `cs2cs --only-best EPSG:4326+3855 EPSG:4979`: the KNOWN_AVAILABLE factory drops `Inverse of
/// WGS 84 to EGM2008 height (2)` (its grid is unknown to the database), so the first candidate
/// tried is `(1)`, whose grid is `us_nga_egm08_25.tif`. Without the policy in the factory the list
/// kept `(2)` and tried it first.
#[test]
fn known_available_drops_the_candidate_with_an_unknown_grid() {
    grid_free_search_path();
    let selection = select_alternatives(
        "EPSG:4326+3855",
        "EPSG:4979",
        BestPolicy::Error,
        CandidateFilter::NONE,
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("EPSG:4326+3855 to EPSG:4979 has two candidates under KNOWN_AVAILABLE");
    };
    match alternatives.forward(point(49.0, 2.0, 0.0)) {
        Err(AlternativeError::MissingGrid(message)) => {
            assert!(
                message.contains("Inverse of WGS 84 to EGM2008 height (1)")
                    && message.contains("Grid us_nga_egm08_25.tif is not available"),
                "message {message}"
            );
        }
        other => panic!("expected the missing grid of (1), got {other:?}"),
    }
}
