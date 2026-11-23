//! Names with multi-byte characters never panic. Slicing a name at a byte index that falls inside
//! a character panics in Rust, where PROJ's byte comparisons (`ci_starts_with`) just compare
//! bytes: `starts_with_ci`, `has_d_prefix` and the `UTM zone` and `Geoid` tests of the WKT1
//! import used to slice that way. The expected values are PROJ 9.9.0's `projinfo` outputs.

use proj_rs::projinfo::{run, Status};
use proj_rs::{AuthorityFactory, DatabaseContext, WktParser, WktStrictness};

fn wkt1_geographic(datum: &str) -> String {
    format!(
        r#"GEOGCS["{datum}",DATUM["{datum}",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#
    )
}

fn parse(wkt: &str) -> proj_rs::iso19111::Crs {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser
        .create_from_wkt(wkt)
        .expect("WKT reads")
        .crs()
        .cloned()
        .expect("a CRS")
}

#[test]
fn datum_names_with_multibyte_characters_are_compared_by_bytes() {
    let db = DatabaseContext::create().expect("database");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let crs = parse(&wkt1_geographic("aÉ"));
    let _ = crs.identify_with(Some(&factory));
    let report = run(&["-o", "PROJ", &wkt1_geographic("Référence éé")]);
    assert_eq!(report.status, Status::Success, "{:?}", report.errors);
}

#[test]
fn utm_and_geoid_names_with_multibyte_characters() {
    let utm = r#"PROJCS["WGS 84 / UTM zoné 31N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",3],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1]]"#;
    let _ = parse(utm);
    for vertical in ["Géoïde 2003", "Geoïd03", "NAVD88 - Géoïde03 (Feet)"] {
        let compound = format!(
            r#"COMPD_CS["NAD83 + {vertical}",GEOGCS["NAD83",DATUM["North_American_Datum_1983",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],VERT_CS["{vertical}",VERT_DATUM["{vertical}",2005],UNIT["metre",1],AXIS["Up",UP]]]"#
        );
        let mut parser = WktParser::new();
        parser.set_strictness(WktStrictness::Lenient);
        assert!(parser.create_from_wkt(&compound).is_ok(), "{vertical}");
    }
}
