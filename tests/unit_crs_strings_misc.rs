//! PROJ 9.9.0 `test/unit/test_crs.cpp`, `norway_ntm`: `ETRS89-NOR [EUREF89] / NTM zone 5`
//! (EPSG:5105). The ESRI WKT1 export and the ESRI import with its identification are checked
//! here, and the WKT2 import before EPSG 12.025 with its equivalence and identification.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, Crs, DatabaseContext, Equivalence, WktConvention,
    WktFormatter,
};

const NORWAY_ESRI_WKT: &str = r#"PROJCS["ETRS_1989_NTM_Zone_5",GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",100000.0],PARAMETER["False_Northing",1000000.0],PARAMETER["Central_Meridian",5.5],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",58.0],UNIT["Meter",1.0]]"#;

fn epsg(crs: &Crs) -> String {
    crs.authority_code("EPSG")
        .map_or_else(String::new, |identifier| identifier.code().to_owned())
}

/// `TEST(crs, norway_ntm)`, the ESRI WKT1 export of EPSG:5105 (test_crs.cpp:8002-8006).
#[test]
fn crs_norway_ntm_esri_wkt1_export() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    // "ETRS89-NOR [EUREF89] / NTM zone 5"
    let crs = factory
        .create_coordinate_reference_system("5105")
        .expect("EPSG:5105");

    // May change in a future version of the ESRI db
    assert_eq!(
        crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db))
            .expect("ESRI WKT1"),
        NORWAY_ESRI_WKT
    );
}

/// `TEST(crs, norway_ntm)`, the ESRI WKT1 import and its identification (test_crs.cpp:8009-8022).
#[test]
fn crs_norway_ntm_esri_wkt1_import_identifies_5105() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let crs_from_esri = create_from_user_input(NORWAY_ESRI_WKT, &db).expect("ESRI WKT1");
    let Crs::Projected(projected) = &crs_from_esri else {
        panic!("not a projected CRS")
    };
    assert_eq!(projected.name(), "ETRS89-NOR [EUREF89] / NTM zone 5");
    assert_eq!(projected.base_crs().name(), "ETRS89-NOR [EUREF89]");
    assert_eq!(
        projected.base_crs().datum().map(|datum| datum.name()),
        Some("EUREF89")
    );
    let res = crs_from_esri.identify_with(Some(&factory));
    assert_eq!(res.len(), 1);
    assert_eq!(epsg(&res[0].0), "5105");
    assert_eq!(res[0].1, 100);
}

const NORWAY_WKT2_BEFORE_EPSG_12_025: &str = r#"PROJCRS["ETRS89 / NTM zone 5",
    BASEGEOGCRS["ETRS89",
        ENSEMBLE["European Terrestrial Reference System 1989 ensemble",
            MEMBER["European Terrestrial Reference Frame 1989"],
            MEMBER["European Terrestrial Reference Frame 1990"],
            MEMBER["European Terrestrial Reference Frame 1991"],
            MEMBER["European Terrestrial Reference Frame 1992"],
            MEMBER["European Terrestrial Reference Frame 1993"],
            MEMBER["European Terrestrial Reference Frame 1994"],
            MEMBER["European Terrestrial Reference Frame 1996"],
            MEMBER["European Terrestrial Reference Frame 1997"],
            MEMBER["European Terrestrial Reference Frame 2000"],
            MEMBER["European Terrestrial Reference Frame 2005"],
            MEMBER["European Terrestrial Reference Frame 2014"],
            MEMBER["European Terrestrial Reference Frame 2020"],
            ELLIPSOID["GRS 1980",6378137,298.257222101,
                LENGTHUNIT["metre",1]],
            ENSEMBLEACCURACY[0.1]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",4258]],
    CONVERSION["Norway TM zone 5",
        METHOD["Transverse Mercator",
            ID["EPSG",9807]],
        PARAMETER["Latitude of natural origin",58,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8801]],
        PARAMETER["Longitude of natural origin",5.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8802]],
        PARAMETER["Scale factor at natural origin",1,
            SCALEUNIT["unity",1],
            ID["EPSG",8805]],
        PARAMETER["False easting",100000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",1000000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["northing (N)",north,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["easting (E)",east,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Engineering survey."],
        AREA["Norway - onshore - west of 6°E."],
        BBOX[58.32,4.39,62.64,6.01]],
    ID["EPSG",5105]]"#;

/// `TEST(crs, norway_ntm)`, the WKT2 import before EPSG 12.025 (test_crs.cpp:8024-8076): the
/// imported CRS is equivalent to EPSG:5105 both ways, and identifies as EPSG:5105 with 100.
#[test]
fn crs_norway_ntm_wkt2_before_epsg_12_025_is_equivalent_to_5105() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let crs = factory
        .create_coordinate_reference_system("5105")
        .expect("EPSG:5105");

    let crs_from_wkt2 = create_from_user_input(NORWAY_WKT2_BEFORE_EPSG_12_025, &db).expect("WKT2");
    let Crs::Projected(_) = &crs_from_wkt2 else {
        panic!("not a projected CRS")
    };
    assert!(crs.is_equivalent_to_with(&crs_from_wkt2, Equivalence::Equivalent));
    assert!(crs_from_wkt2.is_equivalent_to_with(&crs, Equivalence::Equivalent));

    let res = crs_from_wkt2.identify_with(Some(&factory));
    assert_eq!(res.len(), 1);
    assert_eq!(epsg(&res[0].0), "5105");
    assert_eq!(res[0].1, 100);
}
