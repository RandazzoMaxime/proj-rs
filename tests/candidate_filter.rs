//! PROJ 9.9.0 `coordinateoperationfactory.cpp`: the accuracy test runs only when the requested
//! `ACCURACY` is not 0. A requested 0 keeps every candidate, so the choice is the unfiltered one.

use proj_rs::iso19111::{best_pipeline_filtered, best_pipeline_with, CandidateFilter, OnlyBest};

#[test]
fn accuracy_zero_keeps_the_unfiltered_choice() {
    let unfiltered = best_pipeline_with("EPSG:4326", "EPSG:5070", OnlyBest::Off).expect("best");
    let zero = CandidateFilter {
        accuracy: Some(0.0),
        ballpark: proj_rs::iso19111::Ballpark::Allowed,
    };
    let filtered =
        best_pipeline_filtered("EPSG:4326", "EPSG:5070", OnlyBest::Off, zero).expect("best");
    assert_eq!(filtered, unfiltered);
}
