//! A geodetic CRS with an authority code keeps the definition of its WKT. PROJ's
//! `buildGeodeticCRS` only takes the coordinate system of the code, and only for a BASEGEODCRS
//! without CS (io.cpp:3290-3312). Expected WKT2_2019 comes from C PROJ 9.9.0.

use proj_rs::{DatabaseContext, WktConvention, WktFormatter, WktLines, WktParser, WktStrictness};

#[test]
fn geodetic_crs_with_code_keeps_its_wkt() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "GEOCCS with AUTHORITY EPSG:4328 (GDAL test_osr_basic_16)",
            r#"GEOCCS["WGS 84 (geocentric)",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["metre",1],AXIS["Geocentric X",OTHER],AXIS["Geocentric Y",OTHER],AXIS["Geocentric Z",OTHER],AUTHORITY["EPSG","4328"]]"#,
            r#"GEODCRS["WGS 84 (geocentric)",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[Cartesian,3],AXIS["(X)",geocentricX,ORDER[1],LENGTHUNIT["metre",1]],AXIS["(Y)",geocentricY,ORDER[2],LENGTHUNIT["metre",1]],AXIS["(Z)",geocentricZ,ORDER[3],LENGTHUNIT["metre",1]],ID["EPSG",4328]]"#,
        ),
        (
            "GEOCCS with AUTHORITY EPSG:4978, no AXIS",
            r#"GEOCCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["metre",1],AUTHORITY["EPSG","4978"]]"#,
            r#"GEODCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[Cartesian,3],AXIS["(X)",geocentricX,ORDER[1],LENGTHUNIT["metre",1]],AXIS["(Y)",geocentricY,ORDER[2],LENGTHUNIT["metre",1]],AXIS["(Z)",geocentricZ,ORDER[3],LENGTHUNIT["metre",1]],ID["EPSG",4978]]"#,
        ),
        (
            "GEOCCS in kilometres with AUTHORITY EPSG:4978",
            r#"GEOCCS["x",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["kilometre",1000],AUTHORITY["EPSG","4978"]]"#,
            r#"GEODCRS["x",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[Cartesian,3],AXIS["(X)",geocentricX,ORDER[1],LENGTHUNIT["kilometre",1000]],AXIS["(Y)",geocentricY,ORDER[2],LENGTHUNIT["kilometre",1000]],AXIS["(Z)",geocentricZ,ORDER[3],LENGTHUNIT["kilometre",1000]],ID["EPSG",4978]]"#,
        ),
        (
            "BASEGEODCRS with ID EPSG:4978 and no CS",
            r#"PROJCRS["p",BASEGEODCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],ID["EPSG",4978]],CONVERSION["c",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433]]],CS[Cartesian,2],AXIS["e",east,LENGTHUNIT["metre",1]],AXIS["n",north,LENGTHUNIT["metre",1]]]"#,
            r#"PROJCRS["p",BASEGEODCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",4978]],CONVERSION["c",METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Longitude of natural origin",3,ANGLEUNIT["degree",0.0174532925199433]]],CS[Cartesian,2],AXIS["e",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["n",north,ORDER[2],LENGTHUNIT["metre",1]]]"#,
        ),
    ];
    for (label, wkt, expected) in cases {
        let mut parser = WktParser::new();
        parser.set_strictness(WktStrictness::Lenient);
        parser.attach_database(DatabaseContext::create().expect("proj.db"));
        let object = parser.create_from_wkt(wkt).expect(label);
        let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
        formatter.set_lines(WktLines::Single);
        let crs = object.crs().expect("CRS");
        assert_eq!(
            crs.export_to_wkt(&mut formatter).expect("WKT2_2019"),
            *expected,
            "{label}"
        );
    }
}
