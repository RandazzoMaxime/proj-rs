//! The geographic/geocentric step that `fixSteps` appends to a concatenated operation is a
//! `Conversion` (`createGeographicGeocentric`), written without CRS inside the concatenation.
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:9094 -o WKT2:2019 -q` and
//! `-o PROJJSON -q`.

use std::process::Command;

fn projinfo(format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_projinfo"))
        .args(["-k", "operation", "EPSG:9094", "-o", format, "-q"])
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
fn bridge_step_is_a_wkt2_conversion() {
    let wkt = projinfo("WKT2:2019");
    let needle = concat!(
        "    STEP[\n",
        "        CONVERSION[\"Conversion from ITRF96 (geocentric) to ITRF96 (geog3D)\",\n",
        "            METHOD[\"Geographic/geocentric conversions\",\n",
        "                ID[\"EPSG\",9602]]]],\n",
    );
    assert!(wkt.contains(needle), "{wkt}");
}

#[test]
fn bridge_step_is_a_projjson_conversion() {
    let json = projinfo("PROJJSON");
    let needle = concat!(
        "      \"type\": \"Conversion\",\n",
        "      \"name\": \"Conversion from ITRF96 (geocentric) to ITRF96 (geog3D)\",\n",
        "      \"method\": {\n",
        "        \"name\": \"Geographic/geocentric conversions\",\n",
        "        \"id\": {\n",
        "          \"authority\": \"EPSG\",\n",
        "          \"code\": 9602\n",
        "        }\n",
        "      }\n",
    );
    assert!(json.contains(needle), "{json}");
}
