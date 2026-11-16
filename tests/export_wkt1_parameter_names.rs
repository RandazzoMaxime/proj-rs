//! The WKT1 name of a conversion parameter. `OperationParameterValue::_exportToWKT` takes the
//! name from the method mapping (parammappings.cpp:1882, projection methods only); a method with
//! no mapping, or the ESRI dialect, which passes no mapping, writes the EPSG name of the parameter
//! (singleoperation.cpp:930-949).
//!
//! Expected values: PROJ 9.9.0 `projinfo EPSG:<code> -o <format> -q`.

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
fn lambert_conic_near_conformal_keeps_epsg_names() {
    assert_eq!(
        projinfo(&["EPSG:22700", "-o", "WKT1_GDAL"]),
        concat!(
            "PROJCS[\"Deir ez Zor / Levant Zone\",\n",
            "    GEOGCS[\"Deir ez Zor\",\n",
            "        DATUM[\"Deir_ez_Zor\",\n",
            "            SPHEROID[\"Clarke 1880 (IGN)\",6378249.2,293.466021293627,\n",
            "                AUTHORITY[\"EPSG\",\"7011\"]],\n",
            "            AUTHORITY[\"EPSG\",\"6227\"]],\n",
            "        PRIMEM[\"Greenwich\",0,\n",
            "            AUTHORITY[\"EPSG\",\"8901\"]],\n",
            "        UNIT[\"degree\",0.0174532925199433,\n",
            "            AUTHORITY[\"EPSG\",\"9122\"]],\n",
            "        AUTHORITY[\"EPSG\",\"4227\"]],\n",
            "    PROJECTION[\"Lambert_Conic_Near-Conformal\"],\n",
            "    PARAMETER[\"Latitude of natural origin\",34.65],\n",
            "    PARAMETER[\"Longitude of natural origin\",37.35],\n",
            "    PARAMETER[\"Scale factor at natural origin\",0.9996256],\n",
            "    PARAMETER[\"False easting\",300000],\n",
            "    PARAMETER[\"False northing\",300000],\n",
            "    UNIT[\"metre\",1,\n",
            "        AUTHORITY[\"EPSG\",\"9001\"]],\n",
            "    AXIS[\"Easting\",EAST],\n",
            "    AXIS[\"Northing\",NORTH],\n",
            "    AUTHORITY[\"EPSG\",\"22700\"]]\n",
            "\n"
        )
    );
}

#[test]
fn polar_stereographic_variant_c_keeps_epsg_names() {
    assert_eq!(
        projinfo(&["EPSG:2985", "-o", "WKT1_GDAL"]),
        concat!(
            "PROJCS[\"Petrels 1972 / Terre Adelie Polar Stereographic\",\n",
            "    GEOGCS[\"Petrels 1972\",\n",
            "        DATUM[\"Petrels_1972\",\n",
            "            SPHEROID[\"International 1924\",6378388,297,\n",
            "                AUTHORITY[\"EPSG\",\"7022\"]],\n",
            "            AUTHORITY[\"EPSG\",\"6636\"]],\n",
            "        PRIMEM[\"Greenwich\",0,\n",
            "            AUTHORITY[\"EPSG\",\"8901\"]],\n",
            "        UNIT[\"degree\",0.0174532925199433,\n",
            "            AUTHORITY[\"EPSG\",\"9122\"]],\n",
            "        AUTHORITY[\"EPSG\",\"4636\"]],\n",
            "    PROJECTION[\"Polar_Stereographic_(variant_C)\"],\n",
            "    PARAMETER[\"Latitude of standard parallel\",-67],\n",
            "    PARAMETER[\"Longitude of origin\",140],\n",
            "    PARAMETER[\"Easting at false origin\",300000],\n",
            "    PARAMETER[\"Northing at false origin\",200000],\n",
            "    UNIT[\"metre\",1,\n",
            "        AUTHORITY[\"EPSG\",\"9001\"]],\n",
            "    AUTHORITY[\"EPSG\",\"2985\"]]\n",
            "\n"
        )
    );
}

#[test]
fn esri_writes_epsg_names_after_its_own_mappings() {
    assert_eq!(
        projinfo(&["EPSG:9311", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"NAD27_US_National_Atlas_Equal_Area\",GEOGCS[\"GCS_North_American_1927\",DATUM[\"D_North_American_1927\",SPHEROID[\"Clarke_1866\",6378206.4,294.978698213898]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],PARAMETER[\"Latitude of natural origin\",45.0],PARAMETER[\"Longitude of natural origin\",-100.0],PARAMETER[\"False easting\",0.0],PARAMETER[\"False northing\",0.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}

#[test]
fn esri_south_orientated_transverse_mercator() {
    assert_eq!(
        projinfo(&["EPSG:2048", "-o", "WKT1_ESRI"]),
        concat!("PROJCS[\"Hartebeesthoek94_Lo19\",GEOGCS[\"GCS_Hartebeesthoek_1994\",DATUM[\"D_Hartebeesthoek_1994\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator_South_Orientated\"],PARAMETER[\"Latitude of natural origin\",0.0],PARAMETER[\"Longitude of natural origin\",19.0],PARAMETER[\"Scale factor at natural origin\",1.0],PARAMETER[\"False easting\",0.0],PARAMETER[\"False northing\",0.0],UNIT[\"Meter\",1.0]]\n",
        "\n")
    );
}
