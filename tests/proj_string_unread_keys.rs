//! `createFromPROJString` builds the PROJ operation of a one-step string first, then drops the
//! keys it does not read and keeps the first value of a repeated key (io.cpp:12811-12852). An
//! unknown key is therefore not a reason for a PROJ-based CRS or for an `EXTENSION_PROJ4`.
//! Expected values come from C PROJ 9.9.0 (`proj_create`, then `proj_as_wkt` or
//! `proj_as_proj_string`).

use proj_rs::iso19111::create_from_user_input;
use proj_rs::{DatabaseContext, WktConvention, WktFormatter, WktLines};

#[test]
fn unread_keys_are_dropped_before_the_crs_is_built() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str)] = &[
        (
            "etmerc with the +nodefs typo (GDAL test_osr_proj4_14)",
            r#"+proj=etmerc +lat_0=1 +lon_0=2 +k=0.9996 +x_0=3 +y_0=4 +datum=WGS84 +nodefs +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",1],PARAMETER["central_meridian",2],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",3],PARAMETER["false_northing",4],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            "same with +units=m",
            r#"+proj=etmerc +lat_0=1 +lon_0=2 +k=0.9996 +x_0=3 +y_0=4 +datum=WGS84 +units=m +nodefs +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",1],PARAMETER["central_meridian",2],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",3],PARAMETER["false_northing",4],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            "unknown key with a value",
            r#"+proj=tmerc +datum=WGS84 +foo=bar +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",0],PARAMETER["scale_factor",1],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            "a key tmerc does not read",
            r#"+proj=tmerc +datum=WGS84 +lat_ts=5 +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",0],PARAMETER["scale_factor",1],PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            "unknown key on a geographic CRS",
            r#"+proj=longlat +datum=WGS84 +nodefs +type=crs"#,
            r#"GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]]"#,
        ),
    ];
    for (label, text, expected) in cases {
        let crs = create_from_user_input(text, &db).expect(label);
        let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
        formatter.set_lines(WktLines::Single);
        assert_eq!(
            crs.export_to_wkt(&mut formatter).expect("WKT1_GDAL"),
            *expected,
            "{label}"
        );
    }
}

#[test]
fn read_keys_stay_in_the_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str)] = &[
        (
            "geocent keeps +axis, which every PROJ object reads",
            r#"+proj=geocent +axis=neu +datum=WGS84 +type=crs"#,
            r#"+proj=geocent +axis=neu +datum=WGS84 +type=crs"#,
        ),
        (
            "a repeated key keeps its first value",
            r#"+proj=utm +zone=31 +ellps=GRS80 +zone=32 +type=crs"#,
            r#"+proj=utm +zone=31 +ellps=GRS80 +units=m +no_defs +type=crs"#,
        ),
        (
            "ob_tran keeps the keys its rotated projection reads",
            r#"+proj=ob_tran +o_proj=rhealpix +o_lat_p=90 +o_lon_p=-180 +lon_0=180 +north_square=1 +south_square=0 +ellps=WGS84 +type=crs"#,
            r#"+proj=ob_tran +o_proj=rhealpix +o_lat_p=90 +o_lon_p=-180 +lon_0=180 +north_square=1 +south_square=0 +ellps=WGS84 +type=crs"#,
        ),
    ];
    for (label, text, expected) in cases {
        let crs = create_from_user_input(text, &db).expect(label);
        assert_eq!(
            crs.export_to_proj_string().expect("PROJ string"),
            *expected,
            "{label}"
        );
    }
}
