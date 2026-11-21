//! `createFromUserInput` reads a WKT without the strict grammar check (`setStrict(false)`,
//! io.cpp:7789-7792): a node out of the WKT1 grammar is a warning, not an error. Expected
//! WKT2_2019 comes from C PROJ 9.9.0 (`proj_create`, `proj_as_wkt`, `MULTILINE=NO`).

use proj_rs::iso19111::create_from_user_input;
use proj_rs::unstable::Crs;
use proj_rs::{DatabaseContext, WktConvention, WktFormatter, WktLines};

#[test]
fn user_input_wkt_out_of_grammar_is_read() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str)] = &[
        (
            "AUTHORITY before AXIS, short PROJCS",
            r#"PROJCS["x",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["central_meridian",3],UNIT["metre",1],AUTHORITY["EPSG","32631"],AXIS["E",EAST],AXIS["N",NORTH]]"#,
            r#"PROJCRS["x",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],CONVERSION["unknown",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["Scale factor at natural origin",1,SCALEUNIT["unity",1],ID["EPSG",8805]]],CS[Cartesian,2],AXIS["e",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["n",north,ORDER[2],LENGTHUNIT["metre",1]],ID["EPSG",32631]]"#,
        ),
        (
            "unknown node FOO in a GEOGCS",
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433],FOO["bar"]]"#,
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]]]"#,
        ),
        (
            "unknown node in a WKT2 GEOGCRS",
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],CS[ellipsoidal,2],AXIS["lat",north],AXIS["lon",east],ANGLEUNIT["degree",0.0174532925199433],FOO[1]]"#,
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1,ID["EPSG",9001]]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8901]],CS[ellipsoidal,2],AXIS["lat",north,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["lon",east,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]]]"#,
        ),
    ];
    for (label, wkt, expected) in cases {
        let crs =
            create_from_user_input(wkt, &db).unwrap_or_else(|error| panic!("{label}: {error:?}"));
        let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
        formatter.set_lines(WktLines::Single);
        assert_eq!(
            crs.export_to_wkt(&mut formatter).expect("WKT2_2019"),
            *expected,
            "{label}"
        );
    }
}

/// GDAL's `test_osr_basic_11` WKT: AUTHORITY before AXIS in the PROJCS.
#[test]
fn user_input_gdal_authority_before_axis_is_read() {
    let db = DatabaseContext::create().expect("proj.db");
    let Crs::Bound(bound) = create_from_user_input(GDAL_BASIC_11, &db).expect("PROJCS") else {
        panic!("bound CRS expected (TOWGS84)");
    };
    let Crs::Projected(projected) = bound.source_crs() else {
        panic!("projected source expected");
    };
    // PROJ drops the " (deprecated)" suffix of a WKT1 name, as its WKT2 export shows.
    assert_eq!(projected.name(), "NAD83(CSRS98) / UTM zone 20N");
    assert_eq!(projected.identifiers()[0].code(), "2038");
}

const GDAL_BASIC_11: &str = r#"PROJCS["NAD83(CSRS98) / UTM zone 20N (deprecated)", GEOGCS["NAD83(CSRS98)", DATUM["NAD83_Canadian_Spatial_Reference_System", SPHEROID["GRS 1980",6378137,298.257222101, AUTHORITY["EPSG","7019"]], TOWGS84[0,0,0,0,0,0,0], AUTHORITY["EPSG","6140"]], PRIMEM["Greenwich",0, AUTHORITY["EPSG","8901"]], UNIT["degree",0.0174532925199433, AUTHORITY["EPSG","9108"]], AUTHORITY["EPSG","4140"]], PROJECTION["Transverse_Mercator"], PARAMETER["central_meridian",-63], PARAMETER["scale_factor",0.9996], PARAMETER["false_easting",500000], PARAMETER["false_northing",0], UNIT["metre",1, AUTHORITY["EPSG","9001"]], AUTHORITY["EPSG","2038"], AXIS["Easting",EAST], AXIS["Northing",NORTH]]"#;

/// An unbalanced WKT is not a grammar warning: PROJ fails to build it at all.
#[test]
fn user_input_unbalanced_wkt_still_fails() {
    let db = DatabaseContext::create().expect("proj.db");
    assert!(create_from_user_input(r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]"#, &db).is_err());
}
