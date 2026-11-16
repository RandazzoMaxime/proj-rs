//! PROJ string of a registered projection conversion with no CRS pair
//! (`Conversion::_exportToPROJString` with no source CRS): the projection step alone, with no
//! ellipsoid and no `+inv`.
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:<code> -o PROJ -q`.

use std::process::Command;

fn projinfo(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_projinfo"))
        .args(args)
        .arg("-q")
        .output()
        .expect("run projinfo");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

#[test]
fn webmerc_conversion() {
    assert_eq!(
        projinfo(&["-k", "operation", "EPSG:3856", "-o", "PROJ"]),
        "+proj=webmerc +lat_0=0 +lon_0=0 +x_0=0 +y_0=0\n"
    );
}

#[test]
fn utm_conversion() {
    assert_eq!(
        projinfo(&["-k", "operation", "EPSG:16031", "-o", "PROJ"]),
        "+proj=utm +zone=31\n"
    );
}

#[test]
fn laea_spherical_keeps_r_a() {
    assert_eq!(
        projinfo(&["-k", "operation", "EPSG:3897", "-o", "PROJ"]),
        "+proj=laea +R_A +lat_0=90 +lon_0=0 +x_0=0 +y_0=0\n"
    );
}

#[test]
fn krovak_conversion_wraps_at_80_columns() {
    assert_eq!(
        projinfo(&["-k", "operation", "EPSG:5510", "-o", "PROJ"]),
        concat!(
            "+proj=krovak +lat_0=49.5 +lon_0=24.8333333333333 +alpha=30.2881397527778\n",
            "      +k=0.9999 +x_0=0 +y_0=0\n"
        )
    );
}

#[test]
fn krovak_west_south_axes() {
    assert_eq!(
        projinfo(&["-k", "operation", "EPSG:19952", "-o", "PROJ"]),
        concat!(
            "+proj=krovak +axis=swu +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397527778 +k=0.9999\n",
            "      +x_0=0 +y_0=0\n"
        )
    );
}
