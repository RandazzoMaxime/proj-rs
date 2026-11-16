//! The errors of an operation exported to a WKT dialect that cannot hold it
//! (`SingleOperation::exportTransformationToWKT`, singleoperation.cpp:3045, and
//! `PointMotionOperation::_exportToWKT`, singleoperation.cpp:5712).
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:<code> -o <format> -q`, stderr.

use std::process::Command;

fn stderr_of(code: &str, format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_projinfo"))
        .args(["-k", "operation", code, "-o", format, "-q"])
        .output()
        .expect("run projinfo");
    String::from_utf8(output.stderr).expect("utf-8")
}

#[test]
fn a_transformation_has_no_wkt1() {
    assert_eq!(
        stderr_of("EPSG:1196", "WKT1_GDAL"),
        "Error when exporting to WKT1:GDAL: Transformation can only be exported to WKT2\n"
    );
}

#[test]
fn a_point_motion_operation_needs_wkt2_2019() {
    assert_eq!(
        stderr_of("EPSG:9375", "WKT2:2015"),
        "Error when exporting to WKT2:2015: Transformation can only be exported to WKT2:2019\n"
    );
}

#[test]
fn a_point_motion_operation_has_no_wkt1() {
    assert_eq!(
        stderr_of("EPSG:9375", "WKT1_GDAL"),
        "Error when exporting to WKT1:GDAL: Transformation can only be exported to WKT2:2019\n"
    );
}
