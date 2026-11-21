//! `create_crs_to_crs_from_pj` towards a CRS given as WKT2 that the PROJ-string translation of
//! `Proj::new` does not cover. PROJ 9.9.0's `proj_create` reads any CRS its WKT reader reads, and
//! GDAL hands its CRS to PROJ as such WKT. The three texts are GDAL 3.13.3's
//! `ExportToWkt(["FORMAT=WKT2_2019"])`; the expected values are the same calls on PROJ 9.9.0's C
//! library, from `OGC:CRS84` in WKT2 (longitude first), point (-95, 49).

use proj_rs::Proj;

const CRS84: &str = r#"GEOGCRS["WGS 84 (CRS84)",ENSEMBLE["World Geodetic System 1984 ensemble",MEMBER["World Geodetic System 1984 (Transit)"],MEMBER["World Geodetic System 1984 (G730)"],MEMBER["World Geodetic System 1984 (G873)"],MEMBER["World Geodetic System 1984 (G1150)"],MEMBER["World Geodetic System 1984 (G1674)"],MEMBER["World Geodetic System 1984 (G1762)"],MEMBER["World Geodetic System 1984 (G2139)"],MEMBER["World Geodetic System 1984 (G2296)"],ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ENSEMBLEACCURACY[2.0]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["geodetic longitude (Lon)",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["geodetic latitude (Lat)",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]],USAGE[SCOPE["Not known."],AREA["World."],BBOX[-90,-180,90,180]],ID["OGC","CRS84"]]"#;

const SPHERICAL_LAEA: &str = r#"PROJCRS["unknown",BASEGEOGCRS["unknown",DATUM["Unknown based on Normal Sphere (r=6370997) ellipsoid",ELLIPSOID["Normal Sphere (r=6370997)",6370997,0,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]]],CONVERSION["unknown",METHOD["Lambert Azimuthal Equal Area (Spherical)",ID["EPSG",1027]],PARAMETER["Latitude of natural origin",45,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",-100,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["False easting",0,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1,ID["EPSG",9001]]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1,ID["EPSG",9001]]]]"#;

const BONNE: &str = r#"PROJCRS["unknown",BASEGEOGCRS["unknown",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ID["EPSG",6326]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]]],CONVERSION["unknown",METHOD["Bonne",ID["EPSG",9827]],PARAMETER["Latitude of natural origin",10,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["False easting",0,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1,ID["EPSG",9001]]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1,ID["EPSG",9001]]]]"#;

const ESRI_53037: &str = r#"PROJCRS["Sphere_Equal_Earth_Asia_Pacific",BASEGEOGCRS["GCS_Sphere_GRS_1980_Mean_Radius",DATUM["D_Sphere_GRS_1980_Mean_Radius",ELLIPSOID["Sphere_GRS_1980_Mean_Radius",6371008.7714,0,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],ID["ESRI",104047]],CONVERSION["Sphere_Equal_Earth_Asia_Pacific",METHOD["Equal Earth",ID["EPSG",1078]],PARAMETER["Longitude of natural origin",150,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["False easting",0,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["(N)",north,ORDER[2],LENGTHUNIT["metre",1]],USAGE[SCOPE["Not known."],AREA["World."],BBOX[-90,-180,90,180]],ID["ESRI",53037]]"#;

fn convert(target: &str) -> (f64, f64) {
    let from = Proj::new(CRS84).expect("source");
    let to = Proj::new(target).unwrap_or_else(|error| panic!("{error}"));
    let transform = from
        .create_crs_to_crs_from_pj(&to, None, None)
        .unwrap_or_else(|error| panic!("{error}"));
    transform.convert((-95.0, 49.0)).expect("point")
}

fn assert_near(got: (f64, f64), expected: (f64, f64)) {
    assert!(
        (got.0 - expected.0).abs() <= 1e-6 && (got.1 - expected.1).abs() <= 1e-6,
        "{got:?} vs {expected:?}"
    );
}

#[test]
fn a_spherical_lambert_azimuthal_crs() {
    assert_near(convert(SPHERICAL_LAEA), (364_672.445_757, 456_144.312_765));
}

#[test]
fn a_bonne_crs() {
    assert_near(convert(BONNE), (-6_896_272.502_418, 5_079_286.387_806));
}

#[test]
fn an_esri_crs() {
    assert_near(convert(ESRI_53037), (9_160_937.889_227, 5_897_356.323_731));
}
