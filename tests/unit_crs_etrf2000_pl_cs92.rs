//! PROJ 9.9.0 `test/unit/test_crs.cpp`, the WKT2 of EPSG:2180 from before the 12.041 rename
//! is read as the database object "ETRS89 / PL-1992".

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, Crs, DatabaseContext, Equivalence,
};

/// `TEST(crs, ETRF2000_PL_CS92)`
#[test]
fn crs_etrf2000_pl_cs92() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let crs = factory.create_projected_crs("2180").expect("2180");
    let wkt = r#"PROJCRS["ETRF2000-PL / CS92",
    BASEGEOGCRS["ETRF2000-PL",
        DATUM["ETRF2000 Poland",
            ELLIPSOID["GRS 1980",6378137,298.257222101,
                LENGTHUNIT["metre",1]]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433]],
        ID["EPSG",9702]],
    CONVERSION["Poland CS92",
        METHOD["Transverse Mercator",
            ID["EPSG",9807]],
        PARAMETER["Latitude of natural origin",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8801]],
        PARAMETER["Longitude of natural origin",19,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8802]],
        PARAMETER["Scale factor at natural origin",0.9993,
            SCALEUNIT["unity",1],
            ID["EPSG",8805]],
        PARAMETER["False easting",500000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8806]],
        PARAMETER["False northing",-5300000,
            LENGTHUNIT["metre",1],
            ID["EPSG",8807]]],
    CS[Cartesian,2],
        AXIS["northing (x)",north,
            ORDER[1],
            LENGTHUNIT["metre",1]],
        AXIS["easting (y)",east,
            ORDER[2],
            LENGTHUNIT["metre",1]],
    USAGE[
        SCOPE["Topographic mapping (medium and small scale)."],
        AREA["Poland - onshore and offshore."],
        BBOX[49,14.14,55.93,24.15]],
    ID["EPSG",2180]]"#;
    let from_wkt2 = match create_from_user_input(wkt, &db).expect("wkt") {
        Crs::Projected(crs) => crs,
        other => panic!("expected a projected CRS: {other:?}"),
    };
    assert_eq!(from_wkt2.name(), "ETRS89 / PL-1992");
    assert_eq!(from_wkt2.base_crs().name(), "ETRS89");
    assert!(crs.is_equivalent_to_with(&from_wkt2, Equivalence::Equivalent));
    assert!(from_wkt2.is_equivalent_to_with(&crs, Equivalence::Equivalent));
    let found = Crs::Projected(from_wkt2).identify_with(Some(&factory));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, 100);
}
