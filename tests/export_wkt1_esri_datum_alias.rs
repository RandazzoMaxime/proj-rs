//! The ESRI name of a geodetic datum comes from its official name only
//! (`GeodeticReferenceFrame::_exportToWKT`, datum.cpp:1373): a CRS on a datum ensemble writes the
//! alias of the datum `asDatum` builds (`Greenland 1996`), not the alias of the ensemble.
//!
//! Expected values: PROJ 9.9.0 `projinfo EPSG:<code> -o WKT1_ESRI -q`.

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
fn projected_on_an_ensemble() {
    assert_eq!(
        projinfo(&["EPSG:6053", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"GR96_EPSG_Arctic_zone_3-29\",GEOGCS[\"GCS_Greenland_1996\",DATUM[\"D_Greenland_1996\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Conformal_Conic\"],PARAMETER[\"False_Easting\",29500000.0],PARAMETER[\"False_Northing\",3500000.0],PARAMETER[\"Central_Meridian\",-69.0],PARAMETER[\"Standard_Parallel_1\",80.3333333333333],PARAMETER[\"Standard_Parallel_2\",77.0],PARAMETER[\"Latitude_Of_Origin\",78.7073375277778],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}

#[test]
fn geographic_3d_on_an_ensemble() {
    assert_eq!(
        projinfo(&["EPSG:11452", "-o", "WKT1_ESRI"]),
        concat!("GEOGCS[\"GCS_Greenland_1996\",DATUM[\"D_Greenland_1996\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],VERTCS[\"GLLAT_2025_depth\",VDATUM[\"Greenland_Lowest_Astronomic_Tide_2025\"],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",-1.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}
