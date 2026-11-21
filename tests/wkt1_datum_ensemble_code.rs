//! A WKT1 DATUM named after a datum ensemble with underscores, without AUTHORITY, takes the
//! ensemble seen as a datum: its name and its code. PROJ's `identifyFromName` finds it by
//! `createObjectsFromName` (io.cpp). Expected WKT2_2019 comes from C PROJ 9.9.0.

use proj_rs::iso19111::create_from_user_input;
use proj_rs::{DatabaseContext, WktConvention, WktFormatter, WktLines};

#[test]
fn ensemble_datum_name_takes_the_ensemble_code() {
    let db = DatabaseContext::create().expect("proj.db");
    let cases: &[(&str, &str, &str)] = &[
        (
            "ETRS89 ensemble name with underscores (GDAL test_osr_esri_17)",
            r#"GEOGCS["x",DATUM["European_Terrestrial_Reference_System_1989",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
            r#"GEOGCRS["x",DATUM["European Terrestrial Reference System 1989",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]],ID["EPSG",6258]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]]]"#,
        ),
        (
            "WGS 84 ensemble name with underscores",
            r#"GEOGCS["x",DATUM["World_Geodetic_System_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
            r#"GEOGCRS["x",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ID["EPSG",6326]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]]]"#,
        ),
        (
            "a datum that is not an ensemble",
            r#"GEOGCS["x",DATUM["North_American_Datum_1983",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
            r#"GEOGCRS["x",DATUM["North American Datum 1983",ELLIPSOID["GRS 1980",6378137,298.257222101,LENGTHUNIT["metre",1]],ID["EPSG",6269]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],CS[ellipsoidal,2],AXIS["longitude",east,ORDER[1],ANGLEUNIT["degree",0.0174532925199433]],AXIS["latitude",north,ORDER[2],ANGLEUNIT["degree",0.0174532925199433]]]"#,
        ),
    ];
    for (label, wkt, expected) in cases {
        let crs = create_from_user_input(wkt, &db).expect(label);
        let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
        formatter.set_lines(WktLines::Single);
        assert_eq!(
            crs.export_to_wkt(&mut formatter).expect("WKT2_2019"),
            *expected,
            "{label}"
        );
    }
}
