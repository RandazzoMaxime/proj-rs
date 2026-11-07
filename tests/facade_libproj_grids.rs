//! The georust facade with the test grids on the search path, against libproj.
//!
//! `EPSG:4267` to `EPSG:4269` has NADCON grids (`conus`, `alaska`) and a ballpark offset. No
//! single candidate covers the three points below: each point takes the grid that covers it, as
//! `proj_trans` chooses per point (`trans.cpp:233-346`). The values are PROJ 9.9.0's, read from the
//! georust path with the same grids on `PROJ_DATA`.

mod common;

use proj_rs::Proj;

#[test]
fn nad27_to_nad83_takes_the_grid_that_covers_each_point() {
    common::use_fixture_search_path();
    let proj = Proj::new_known_crs("EPSG:4267", "EPSG:4269", None).expect("pair");
    let cases: [((f64, f64), (f64, f64)); 3] = [
        ((-74.0, 40.0), (-73.999_578_264_714, 40.000_117_770_281)),
        ((-100.0, 40.0), (-100.000_405_836_670, 40.000_005_894_722)),
        ((-122.3, 47.6), (-122.301_236_449_023, 47.599_820_816_087)),
    ];
    for (input, want) in cases {
        let (lon, lat) = proj.convert(input).expect("forward");
        assert!((lon - want.0).abs() < 1e-9, "{input:?}: {lon}");
        assert!((lat - want.1).abs() < 1e-9, "{input:?}: {lat}");
    }
}
