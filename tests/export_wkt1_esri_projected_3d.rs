//! A projected 3D CRS in WKT1_ESRI is its demoted 2D CRS followed by a `VERTCS`
//! (`exportAsESRIWktCompoundCRSWithEllipsoidalHeight`, crs.cpp:2044): the base CRS is the 2D
//! geographic CRS the database knows by that datum, not the 3D CRS without its height.
//!
//! Expected values: PROJ 9.9.0 `projinfo EPSG:9895 -o WKT1_ESRI -q`.

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
fn base_is_the_database_2d_crs() {
    assert_eq!(
        projinfo(&["EPSG:9895", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"LUREF_Luxembourg_TM_3D\",GEOGCS[\"GCS_LUREF\",DATUM[\"D_Luxembourg_Reference_Frame\",SPHEROID[\"International_1924\",6378388.0,297.0]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"Latitude of natural origin\",49.8333333333333],PARAMETER[\"Longitude of natural origin\",6.16666666666667],PARAMETER[\"Scale factor at natural origin\",1.0],PARAMETER[\"False easting\",80000.0],PARAMETER[\"False northing\",100000.0],UNIT[\"Meter\",1.0]],VERTCS[\"LUREF\",DATUM[\"D_Luxembourg_Reference_Frame\",SPHEROID[\"International_1924\",6378388.0,297.0]],PARAMETER[\"Vertical_Shift\",0.0],PARAMETER[\"Direction\",1.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}
