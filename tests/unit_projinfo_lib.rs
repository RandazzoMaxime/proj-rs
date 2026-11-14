//! PROJ `test/unit/test_projinfo_lib.cpp`, `TEST(projinfo_lib, simple)`, through
//! `proj_rs::projinfo::run`. The expected values are PROJ 9.9.0's: the status 0 that
//! `projinfo()` returns, and the INFO text of `EPSG:25832 -o WKT1_GDAL`, which holds
//! `PROJCS["ETRS89 / UTM zone 32N"` (`projinfo_lib.cpp`, `outputObject`, WKT1:GDAL block).

use proj_rs::projinfo::{run, Status};

const SIMPLE_ARGS: [&str; 3] = ["EPSG:25832", "-o", "WKT1_GDAL"];

/// `EXPECT_EQ(res, 0)`: `projinfo()` returns 0 for a CRS written in WKT1:GDAL.
#[test]
fn simple_returns_zero() {
    let report = run(&SIMPLE_ARGS);
    assert_eq!(report.status, Status::Success);
}

/// `EXPECT_EQ(found, true)`: an INFO line holds the WKT1:GDAL name of EPSG:25832. The text
/// is written on the INFO channel, so it is in `output` and not in the warnings or errors.
#[test]
fn simple_found_etrs89_utm_zone_32n() {
    let report = run(&SIMPLE_ARGS);
    assert!(
        report.output.contains("PROJCS[\"ETRS89 / UTM zone 32N\""),
        "{}",
        report.output
    );
}
