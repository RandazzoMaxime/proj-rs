//! An EPSG projected CRS written as WKT1_ESRI is the ESRI definition of the CRS that bears its
//! ESRI name, when the two are equivalent (`ProjectedCRS::_exportToWKT`, crs.cpp:4397-4457): the
//! text of `text_definition`, with its own digits and datum names, not a reformatted CRS.
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
fn krovak_east_north_is_the_esri_text() {
    assert_eq!(
        projinfo(&["EPSG:5514", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"S-JTSK_Krovak_East_North\",GEOGCS[\"GCS_S_JTSK\",DATUM[\"D_S_JTSK\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.28813975277778],PARAMETER[\"Longitude_Of_Center\",24.83333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",-1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",90.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}

#[test]
fn krovak_east_north_ferro_is_the_esri_text() {
    assert_eq!(
        projinfo(&["EPSG:5221", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"S-JTSK_Ferro_Krovak_East_North\",GEOGCS[\"GCS_S_JTSK_Ferro\",DATUM[\"D_S_JTSK\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Ferro\",-17.66666666666667],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.28813975277778],PARAMETER[\"Longitude_Of_Center\",42.5],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",-1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",90.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}

#[test]
fn krovak_east_north_gk_is_the_esri_text() {
    assert_eq!(
        projinfo(&["EPSG:5513", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"S-JTSK_Krovak\",GEOGCS[\"GCS_S_JTSK\",DATUM[\"D_S_JTSK\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528128]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Krovak\"],PARAMETER[\"False_Easting\",0.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Pseudo_Standard_Parallel_1\",78.5],PARAMETER[\"Scale_Factor\",0.9999],PARAMETER[\"Azimuth\",30.28813975277778],PARAMETER[\"Longitude_Of_Center\",24.83333333333333],PARAMETER[\"Latitude_Of_Center\",49.5],PARAMETER[\"X_Scale\",1.0],PARAMETER[\"Y_Scale\",1.0],PARAMETER[\"XY_Plane_Rotation\",0.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}
