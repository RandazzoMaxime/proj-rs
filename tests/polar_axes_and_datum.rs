//! Polar axes and datums of a CRS as PROJ 9.9.0 reads and writes them. Expected values come from
//! `projinfo` 9.9.0.

use proj_rs::iso19111::{create_user_object, DatabaseContext, InitRules, UserObject};
use proj_rs::unstable::Crs;
use proj_rs::{create_crs_from_proj_string, WktConvention, WktFormatter, WktLines};

fn wkt2(crs: &Crs) -> String {
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT2")
}

fn read(text: &str) -> Crs {
    let db = DatabaseContext::create().expect("proj.db");
    match create_user_object(text, Some(&db), InitRules::Iso19111).expect("CRS") {
        UserObject::Crs(crs) => crs,
        _ => panic!("not a CRS"),
    }
}

// io.cpp:12101-12140 and 11448-11495: a polar `stere` or `laea` has axes along meridians.
#[test]
fn polar_stere_and_laea_have_polar_axes() {
    let north = wkt2(
        &create_crs_from_proj_string("+proj=stere +lat_0=90 +lat_ts=70 +ellps=WGS84 +type=crs")
            .expect("stere"),
    );
    assert!(north.contains("AXIS[\"(E)\",south,MERIDIAN[90,"), "{north}");
    assert!(
        north.contains("AXIS[\"(N)\",south,MERIDIAN[180,"),
        "{north}"
    );
    let south = wkt2(
        &create_crs_from_proj_string("+proj=laea +lat_0=-90 +ellps=WGS84 +type=crs").expect("laea"),
    );
    assert!(south.contains("AXIS[\"(E)\",north,MERIDIAN[90,"), "{south}");
    assert!(south.contains("AXIS[\"(N)\",north,MERIDIAN[0,"), "{south}");
    let oblique = wkt2(
        &create_crs_from_proj_string("+proj=stere +lat_0=45 +ellps=WGS84 +type=crs")
            .expect("stere"),
    );
    assert!(!oblique.contains("MERIDIAN["), "{oblique}");
}

// `DatumEnsemble::asDatum` (crs.cpp:2544): the WGS 84 ensemble of a WKT2 is +datum=WGS84.
#[test]
fn wgs84_ensemble_of_a_wkt2_is_datum_wgs84() {
    let crs = read("GEOGCRS[\"WGS 84\",ENSEMBLE[\"World Geodetic System 1984 ensemble\",MEMBER[\"World Geodetic System 1984 (Transit)\"],MEMBER[\"World Geodetic System 1984 (G730)\"],MEMBER[\"World Geodetic System 1984 (G873)\"],MEMBER[\"World Geodetic System 1984 (G1150)\"],MEMBER[\"World Geodetic System 1984 (G1674)\"],MEMBER[\"World Geodetic System 1984 (G1762)\"],MEMBER[\"World Geodetic System 1984 (G2139)\"],MEMBER[\"World Geodetic System 1984 (G2296)\"],ELLIPSOID[\"WGS 84\",6378137,298.257223563,LENGTHUNIT[\"metre\",1]],ENSEMBLEACCURACY[2.0]],PRIMEM[\"Greenwich\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],CS[ellipsoidal,2],AXIS[\"geodetic latitude (Lat)\",north,ORDER[1],ANGLEUNIT[\"degree\",0.0174532925199433]],AXIS[\"geodetic longitude (Lon)\",east,ORDER[2],ANGLEUNIT[\"degree\",0.0174532925199433]],USAGE[SCOPE[\"Horizontal component of 3D system.\"],AREA[\"World.\"],BBOX[-90,-180,90,180]],ID[\"EPSG\",4326]]");
    assert_eq!(
        crs.export_to_proj_string().expect("PROJ string"),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

// `getMappingFromWKT1` finds Krovak (North Orientated) first (io.cpp:4380-4390): a WKT1 Krovak
// without AXIS is not south-west oriented.
#[test]
fn wkt1_krovak_without_axes_is_north_orientated() {
    let crs = read("PROJCS[\"S-JTSK / Krovak\",GEOGCS[\"S-JTSK\",DATUM[\"System_of_the_Unified_Trigonometrical_Cadastral_Network\",SPHEROID[\"Bessel 1841\",6377397.155,299.1528128,AUTHORITY[\"EPSG\",\"7004\"]],AUTHORITY[\"EPSG\",\"6156\"]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",\"8901\"]],UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\",\"9122\"]],AUTHORITY[\"EPSG\",\"4156\"]],PROJECTION[\"Krovak\"],PARAMETER[\"latitude_of_center\",49.5],PARAMETER[\"longitude_of_center\",24.8333333333333],PARAMETER[\"azimuth\",30.2881397527778],PARAMETER[\"pseudo_standard_parallel_1\",78.5],PARAMETER[\"scale_factor\",0.9999],PARAMETER[\"false_easting\",0],PARAMETER[\"false_northing\",0],UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],AUTHORITY[\"EPSG\",\"5513\"]]");
    let Crs::Projected(projected) = &crs else {
        panic!("not a projected CRS");
    };
    assert_eq!(
        projected.deriving_conversion().method().name(),
        "Krovak (North Orientated)"
    );
    assert!(!crs
        .export_to_proj_string()
        .expect("PROJ string")
        .contains("+axis=swu"));
}

// `ObjectContext` (io.cpp:13102-13113): the unit of a parameter that has an id writes none.
#[test]
fn parameter_unit_of_a_proj_string_crs_has_no_id() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_crs_from_proj_string("+proj=laea +lat_0=45 +units=km +ellps=WGS84 +type=crs")
        .expect("laea");
    let json: String = crs
        .export_to_json(&db)
        .expect("PROJJSON")
        .split_whitespace()
        .collect();
    assert!(
        json.contains(
            "\"name\":\"Falseeasting\",\"value\":0,\"unit\":{\"type\":\"LinearUnit\",\"name\":\"kilometre\",\"conversion_factor\":1000},\"id\""
        ),
        "{json}"
    );
}
