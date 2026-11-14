//! PROJ `test/unit/test_projinfo_lib.cpp`, `TEST(projinfo_lib, simple)`, `TEST(projinfo_lib,
//! error)` and `TEST(projinfo_lib, warning)`, through `proj_rs::projinfo::run`. The expected
//! values are PROJ 9.9.0's: the status that `projinfo()` returns, and the text of the INFO,
//! WARN or ERR channel. `simple` reads `EPSG:25832 -o WKT1_GDAL`, whose INFO holds
//! `PROJCS["ETRS89 / UTM zone 32N"` (`projinfo_lib.cpp`, `outputObject`, WKT1:GDAL block).
//! `error` reads `doesnotwork`, whose ERR line is
//! `input string: parsing of 'doesnotwork' failed: unrecognized format / unknown name`
//! (`projinfo_lib.cpp`, `buildObject`, the `strm.cerr` of the `catch`). `warning` reads
//! `EPSG:4937 -o WKT1_GDAL`: a 3D geographic CRS cannot be written in WKT1, so the WARN line
//! `Error when exporting to WKT1:GDAL: ...` is written and the status stays 0
//! (`projinfo_lib.cpp`, the `strm.cwarn` of the WKT1:GDAL block).

use proj_rs::projinfo::{run, Status};

const SIMPLE_ARGS: [&str; 3] = ["EPSG:25832", "-o", "WKT1_GDAL"];
const ERROR_ARGS: [&str; 1] = ["doesnotwork"];
const WARNING_ARGS: [&str; 3] = ["EPSG:4937", "-o", "WKT1_GDAL"];

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

/// `EXPECT_EQ(res, 1)`: `projinfo()` returns 1 when `doesnotwork` is neither a known format
/// nor a known name.
#[test]
fn error_returns_one() {
    let report = run(&ERROR_ARGS);
    assert_eq!(report.status, Status::Failure);
}

/// `EXPECT_EQ(found, true)`: an ERR line holds `unrecognized format / unknown name`. The text
/// is written on the ERR channel, so it is in `errors` and not in the output.
#[test]
fn error_found_unrecognized_format() {
    let report = run(&ERROR_ARGS);
    assert!(
        report
            .errors
            .iter()
            .any(|line| line.contains("unrecognized format / unknown name")),
        "{:?}",
        report.errors
    );
}

/// `EXPECT_EQ(res, 0)`: `projinfo()` returns 0 when the WKT1:GDAL export fails on a warning
/// only, here a geographic 3D CRS (EPSG:4937).
#[test]
fn warning_returns_zero() {
    let report = run(&WARNING_ARGS);
    assert_eq!(report.status, Status::Success);
}

/// `EXPECT_EQ(found, true)`: a WARN line holds `Error when exporting to WKT1:GDAL`. The text
/// is written on the WARN channel, so it is in `warnings` and not in the output or the errors.
#[test]
fn warning_found_error_when_exporting_wkt1_gdal() {
    let report = run(&WARNING_ARGS);
    assert!(
        report
            .warnings
            .iter()
            .any(|line| line.contains("Error when exporting to WKT1:GDAL")),
        "{:?}",
        report.warnings
    );
}
