//! `proj_trans` over the alternatives of `proj_create_crs_to_crs` (PROJ 9.9.0, `trans.cpp:233-346`).
//! The expected values are the ones `cs2cs` prints in `tests/cli_yaml/test_cs2cs_various.yaml`
//! (PROJ 9.9.0, the case named in each test).

mod common;

use proj_rs::iso19111::{
    select_alternatives, AlternativeError, BestPolicy, CandidateFilter, Selection,
};
use proj_rs::Xyzt;

fn point(x: f64, y: f64, z: f64) -> Xyzt {
    Xyzt { x, y, z, t: 0.0 }
}

/// `test_cs2cs_various.yaml`, "Test cs2cs geographic -> geocentric outside BETA2007.gsb grid":
/// the first candidate's grid does not cover the point, so the ballpark candidate gives it.
#[test]
fn point_outside_the_grid_takes_the_next_candidate() {
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
        .forward(point(49.0, 2.0, 0.0))
        .expect("the ballpark candidate serves the point");
    assert!((coord.x - 4_189_881.02).abs() < 0.005, "x {}", coord.x);
    assert!((coord.y - 146_313.87).abs() < 0.005, "y {}", coord.y);
    assert!((coord.z - 4_790_558.75).abs() < 0.005, "z {}", coord.z);
}

/// `test_cs2cs_various.yaml#110`, "Test cs2cs --only-best (grid missing) for NTF to RGF93": under
/// `ONLY_BEST=YES` the chosen candidate's missing grid fails the point, and no other candidate is
/// tried (`trans.cpp:289-295`).
#[test]
fn only_best_fails_the_point_without_retry() {
    common::use_fixture_search_path();
    let selection = select_alternatives("NTF", "RGF93", BestPolicy::Error, CandidateFilter::NONE)
        .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("NTF to RGF93 has several candidates");
    };
    // Latitude first, as NTF is.
    match alternatives.forward(point(49.0, 2.0, 0.0)) {
        Err(AlternativeError::MissingGrid(message)) => {
            assert!(
                message.contains("Grid fr_ign_gr3df97a.tif is not available"),
                "message {message}"
            );
        }
        other => panic!("expected the missing grid, got {other:?}"),
    }
}
