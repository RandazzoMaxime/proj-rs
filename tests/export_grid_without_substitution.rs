//! A registered grid transformation whose method `substitutePROJAlternativeGridNames` does not
//! cover keeps the grid file of the registry (singleoperation.cpp, the final `return self`).
//! EPSG:12219 uses `Geographic3D to Depth (gtx)`, which is not in `isGeographic3DToGravityRelatedHeight`.
//!
//! Expected values: PROJ 9.9.0 `projinfo -k operation EPSG:12219 -o <format> -q`.

use std::process::Command;

fn projinfo(format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_projinfo"))
        .args(["-k", "operation", "EPSG:12219", "-o", format, "-q"])
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
fn the_wkt_names_the_registry_file() {
    assert!(projinfo("WKT2:2019").contains(
        "    PARAMETERFILE[\"Geoid (height correction) model file\",\"nlgeo2018.gtx\"],\n"
    ));
}

#[test]
fn the_json_names_the_registry_file() {
    assert!(projinfo("PROJJSON").contains("      \"value\": \"nlgeo2018.gtx\",\n"));
}
