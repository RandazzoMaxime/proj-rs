//! The area of interest of `proj_create_crs_to_crs_from_pj` (PROJ 9.9.0, `crs_to_crs.cpp:557-566`),
//! which `cs2cs --area` and `--bbox` set. The expected values are the ones `cs2cs` 9.9.0 prints in
//! `tests/cli_yaml/test_cs2cs_datumfile.yaml` (the case named in each test).

mod common;

use proj_rs::iso19111::{
    area_of_interest_from_name, select_alternatives_in_area, BestPolicy, CandidateFilter, Selection,
};
use proj_rs::{BoundingBox, ObjectDomain, Xyzt};

fn point(x: f64, y: f64) -> Xyzt {
    Xyzt {
        x,
        y,
        z: 0.0,
        t: 0.0,
    }
}

/// `--area "USA - CONUS including EEZ" NAD27 NAD83`: `proj_operation_factory_context_set_area_of_interest_name`
/// keeps the description of the extent, and the filter keeps only the operations whose extent has
/// it (`coordinateoperationfactory.cpp:1313-1346`). One operation is left, as `projinfo
/// --area "USA - CONUS including EEZ" -s NAD27 -t NAD83 --spatial-test intersects` lists it.
#[test]
fn a_named_area_keeps_the_operations_of_that_extent() {
    common::use_fixture_search_path();
    let area = area_of_interest_from_name("USA - CONUS including EEZ").expect("one extent");
    assert_eq!(
        area.description(),
        Some(
            "United States (USA) - CONUS including EEZ - onshore and offshore - Alabama; Arizona; Arkansas; California; Colorado; Connecticut; Delaware; Florida; Georgia; Idaho; Illinois; Indiana; Iowa; Kansas; Kentucky; Louisiana; Maine; Maryland; Massachusetts; Michigan; Minnesota; Mississippi; Missouri; Montana; Nebraska; Nevada; New Hampshire; New Jersey; New Mexico; New York; North Carolina; North Dakota; Ohio; Oklahoma; Oregon; Pennsylvania; Rhode Island; South Carolina; South Dakota; Tennessee; Texas; Utah; Vermont; Virginia; Washington; West Virginia; Wisconsin; Wyoming. US Gulf of Mexico (GoM) OCS."
        )
    );
    let selection = select_alternatives_in_area(
        "NAD27",
        "NAD83",
        BestPolicy::Warn,
        CandidateFilter::NONE,
        "",
        Some(area),
    )
    .expect("selection");
    assert!(
        matches!(selection, Selection::Single(_)),
        "a single operation is left"
    );
}

/// `test_cs2cs_datumfile.yaml#3`, "Test --bbox -141.01,40.04,-47.74,83.17 NAD27 NAD83 (using
/// NTv2)": a box has no description, so the operations that intersect it stay (ten for
/// `projinfo --bbox ... --spatial-test intersects`) and the point takes the Canadian grid.
#[test]
fn a_box_keeps_every_operation_that_intersects_it() {
    common::use_fixture_search_path();
    let bbox = BoundingBox::create(-141.01, 40.04, -47.74, 83.17).expect("box");
    let selection = select_alternatives_in_area(
        "NAD27",
        "NAD83",
        BestPolicy::Warn,
        CandidateFilter::NONE,
        "",
        Some(ObjectDomain::from_bounding_box(bbox)),
    )
    .expect("selection");
    let Selection::Several(alternatives) = selection else {
        panic!("a box leaves several operations");
    };
    // Latitude first, as NAD27 is: 40d30'0.368"N 59d59'56.617"W.
    let coord = alternatives
        .forward(point(40.5, -60.0))
        .expect("the NTv2 candidate serves the point");
    let (lat, lon) = (coord.x, coord.y);
    let want_lat = 40.0 + 30.0 / 60.0 + 0.368 / 3600.0;
    let want_lon = -(59.0 + 59.0 / 60.0 + 56.617 / 3600.0);
    // `cs2cs` prints seconds to 0.001": half of that step is the tolerance.
    let half_step = 0.0005 / 3600.0;
    assert!((lat - want_lat).abs() < half_step, "lat {lat}");
    assert!((lon - want_lon).abs() < half_step, "lon {lon}");
}
