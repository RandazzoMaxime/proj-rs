//! PROJ strings of a CRS (`CRS::_exportToPROJString`): a linear unit with no PROJ name is
//! `+to_meter` (`ProjectedCRS::addUnitConvertAndAxisSwap`, crs.cpp:4758), a spherical base drops
//! the `+R_A` of its method (conversion.cpp:4316-4333), and a geocentric CRS on an ensemble is
//! written from the datum `datumNonNull` gives (crs.cpp:2544).
//!
//! Expected values: PROJ 9.9.0 `projinfo EPSG:<code> -o PROJ -q`.

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
fn unit_without_proj_name_is_to_meter() {
    assert_eq!(
        projinfo(&["EPSG:2314", "-o", "PROJ"]),
        "+proj=cass +lat_0=10.4416666666667 +lon_0=-61.3333333333333 +x_0=86501.46392052 +y_0=65379.0134283 +a=6378293.64520876 +b=6356617.98767984 +to_meter=0.3047972654 +no_defs +type=crs\n"
    );
}

#[test]
fn to_meter_follows_towgs84() {
    assert_eq!(
        projinfo(&["EPSG:29872", "-o", "PROJ"]),
        "+proj=omerc +lat_0=4 +lonc=115 +alpha=53.3158204722222 +gamma=53.1301023611111 +k=0.99984 +x_0=590476.872743198 +y_0=442857.654557398 +ellps=evrstSS +towgs84=-679,669,-48,0,0,0,0 +to_meter=0.304799471538676 +no_defs +type=crs\n"
    );
}
