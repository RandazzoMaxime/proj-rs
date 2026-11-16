//! An engineering CRS in WKT1 (`EngineeringDatum::_exportToWKT`, datum.cpp:2739): `LOCAL_DATUM`
//! carries the name and 32767, never an authority.
//!
//! Expected values: PROJ 9.9.0 `projinfo EPSG:<code> -o WKT1_GDAL -q`.

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
fn local_datum_has_no_authority() {
    assert_eq!(
        projinfo(&["EPSG:5800", "-o", "WKT1_GDAL"]),
        concat!(
            "LOCAL_CS[\"Astra Minas Grid\",\n",
            "    LOCAL_DATUM[\"Astra Minas\",32767],\n",
            "    UNIT[\"metre\",1,\n",
            "        AUTHORITY[\"EPSG\",\"9001\"]],\n",
            "    AXIS[\"First local axis\",NORTH],\n",
            "    AXIS[\"Second local axis\",WEST],\n",
            "    AUTHORITY[\"EPSG\",\"5800\"]]\n",
            "\n"
        )
    );
}
