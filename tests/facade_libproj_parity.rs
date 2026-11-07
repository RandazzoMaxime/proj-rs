//! The georust facade against libproj: `proj_create_crs_to_crs`, then
//! `proj_normalize_for_visualization`, then `proj_trans`, as georust `proj` 0.31 calls them.
//!
//! The expected values are PROJ 9.9.0's. They were read from a C program that makes those three
//! calls against the PROJ 9.9.0 shared library (`scratchpad/facade-parity/oracle/georust_oracle.c`),
//! with the `proj.db` of that build and no grid. The facade points are longitude then latitude,
//! or easting then northing, in the CRS units, as georust gives them.

use proj_rs::Proj;

fn metres(got: f64, want: f64) -> bool {
    (got - want).abs() < 1e-6
}

#[test]
fn polar_stereographic_3413_keeps_easting_first() {
    // EPSG:3413 writes easting first on axes that point south. libproj's
    // `mustAxisOrderBeSwitchedForVisualization` switches a polar CRS only along the meridians it
    // names (`crs.cpp:869-947`), so the output keeps the pipeline's order: easting, then northing.
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:3413", None).expect("pair");
    let (east, north, _) = proj.convert_xyz(-45.0, 75.0, 0.0).expect("forward");
    assert!(metres(east, 0.0), "{east}");
    assert!(metres(north, -1_633_879.497_470_785), "{north}");
}

#[test]
fn ups_north_32661_is_switched_for_visualization() {
    // EPSG:32661 writes north first along the 180 and 90 meridians, so it is switched.
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32661", None).expect("pair");
    let (east, north, _) = proj.convert_xyz(-45.0, 75.0, 0.0).expect("forward");
    assert!(metres(east, 815_914.202_587_687), "{east}");
    assert!(metres(north, 815_914.202_587_687), "{north}");
}
