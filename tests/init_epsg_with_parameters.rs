//! `+init=AUTH:CODE` with other parameters, under the PROJ4 init rules
//! (`proj_context_use_proj4_init_rules`). PROJ expands the init into the PROJ
//! string of the CRS, then adds the other words (`io.cpp`). Expected WKT1_GDAL
//! comes from C PROJ 9.9.0.

use proj_rs::iso19111::{create_from_user_input_with_init_rules, InitRules};
use proj_rs::{DatabaseContext, WktConvention, WktFormatter, WktLines};

fn wkt1_gdal(text: &str) -> String {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_from_user_input_with_init_rules(text, &db, InitRules::Proj4)
        .unwrap_or_else(|error| panic!("{text}: {error:?}"));
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    formatter.set_lines(WktLines::Single);
    crs.export_to_wkt(&mut formatter).expect("WKT1_GDAL")
}

#[test]
fn init_with_parameters_matches_proj() {
    let cases: &[(&str, &str)] = &[
        (
            r#"+init=epsg:32631 +units=cm +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",3],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",50000000],PARAMETER["false_northing",0],UNIT["centimetre",0.01,AUTHORITY["EPSG","1033"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            r#"+init=epsg:3004 +towgs84=-122.74,-34.27,-22.83,-1.884,-3.400,-3.030,-15.62 +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["Unknown based on International 1924 (Hayford 1909, 1910) ellipsoid using towgs84=-122.74,-34.27,-22.83,-1.884,-3.400,-3.030,-15.62",SPHEROID["International 1924 (Hayford 1909, 1910)",6378388,297],TOWGS84[-122.74,-34.27,-22.83,-1.884,-3.4,-3.03,-15.62]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",15],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",2520000],PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            r#"+init=epsg:4326 +over +type=crs"#,
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]]"#,
        ),
        (
            r#"+init=epsg:2154 +x_0=0 +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["Unknown based on GRS 1980 ellipsoid",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Lambert_Conformal_Conic_2SP"],PARAMETER["latitude_of_origin",46.5],PARAMETER["central_meridian",3],PARAMETER["standard_parallel_1",49],PARAMETER["standard_parallel_2",44],PARAMETER["false_easting",0],PARAMETER["false_northing",6600000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            r#"+init=epsg:4326 +lon_0=10 +type=crs"#,
            r#"GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],EXTENSION["PROJ4","+proj=longlat +lon_0=10 +datum=WGS84 +no_defs"]]"#,
        ),
        (
            r#"+init=epsg:4326 +type=crs"#,
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]]"#,
        ),
        (
            r#"+init=epsg:27700 +units=m +no_defs=True +type=crs"#,
            r#"PROJCS["unknown",GEOGCS["unknown",DATUM["Unknown based on Airy 1830 ellipsoid",SPHEROID["Airy 1830",6377563.396,299.3249646]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",49],PARAMETER["central_meridian",-2],PARAMETER["scale_factor",0.9996012717],PARAMETER["false_easting",400000],PARAMETER["false_northing",-100000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            r#"+title=Mon CRS +init=epsg:32631 +units=km +type=crs"#,
            r#"PROJCS["Mon CRS",GEOGCS["unknown",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",3],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500],PARAMETER["false_northing",0],UNIT["kilometre",1000,AUTHORITY["EPSG","9036"]],AXIS["Easting",EAST],AXIS["Northing",NORTH]]"#,
        ),
        (
            r#"+title="Quoted title" +init=epsg:4326 +lon_0=1 +type=crs"#,
            r#"GEOGCS["Quoted title",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],EXTENSION["PROJ4","+proj=longlat +lon_0=1 +datum=WGS84 +no_defs"]]"#,
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(wkt1_gdal(text), *expected, "{text}");
    }
}
