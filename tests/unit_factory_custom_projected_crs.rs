//! PROJ 9.9.0 `test/unit/test_factory.cpp`, `FactoryWithTmpDatabase.custom_projected_crs`.
//!
//! The C++ fixture builds a temporary `proj.db` with `createStructure` and
//! `populateWithFakeEPSG`, then adds `TEST_NS` rows whose only definition is a
//! `text_definition` (PROJ string or WKT1_GDAL). The expected values are the
//! C++ ones.

#[path = "support/fake_epsg.rs"]
mod fake_epsg;

use proj_rs::iso19111::{create_crs_from_proj_string, AuthorityFactory, Crs};

const WKT1_GDAL: &str = r#"PROJCS["unknown",
    GEOGCS["unknown",
        DATUM["Unknown_based_on_WGS84_ellipsoid",
            SPHEROID["WGS 84",6378137,298.257223563,
                AUTHORITY["EPSG","7030"]]],
        PRIMEM["Greenwich",0,
            AUTHORITY["EPSG","8901"]],
        UNIT["degree",0.0174532925199433,
            AUTHORITY["EPSG","9122"]]],
    PROJECTION["Mercator_1SP"],
    PARAMETER["central_meridian",0],
    PARAMETER["scale_factor",1],
    PARAMETER["false_easting",0],
    PARAMETER["false_northing",0],
    UNIT["metre",1,
        AUTHORITY["EPSG","9001"]],
    AXIS["Easting",EAST],
    AXIS["Northing",NORTH]]"#;

/// `TEST_F(FactoryWithTmpDatabase, custom_projected_crs)`
#[test]
fn factory_with_tmp_database_custom_projected_crs() {
    let db = fake_epsg::database();
    db.execute_sql(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST','custom_projected_crs',NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,'+proj=mbt_s +unused_flag',0);",
    )
    .expect("TEST");
    db.execute_sql(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST_BOUND','custom_projected_crs2',NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,'+proj=mbt_s +unused_flag +towgs84=1,2,3',0);",
    )
    .expect("TEST_BOUND");
    db.execute_sql(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST_WRONG','custom_projected_crs3',NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,'+proj=longlat',0);",
    )
    .expect("TEST_WRONG");
    // Unknown ellipsoid
    db.execute_sql(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST_MERC','merc',NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,'+proj=merc +x_0=0 +R=1',0);",
    )
    .expect("TEST_MERC");
    // Well-known ellipsoid
    db.execute_sql(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST_MERC2','merc2',NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,'+proj=merc +x_0=0 +ellps=GRS80',0);",
    )
    .expect("TEST_MERC2");
    // WKT1_GDAL
    db.execute_sql(&format!(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','TEST_WKT1_GDAL','WKT1_GDAL',NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,'{WKT1_GDAL}',0);"
    ))
    .expect("TEST_WKT1_GDAL");

    let factory = AuthorityFactory::create(&db, "TEST_NS");
    {
        let crs = factory.create_projected_crs("TEST").expect("TEST");
        assert_eq!(crs.name(), "custom_projected_crs");
        assert_eq!(crs.identifiers().len(), 1);
        let crs = Crs::Projected(crs);
        assert_eq!(
            crs.export_to_proj_string().expect("proj string"),
            "+proj=mbt_s +datum=WGS84 +units=m +no_defs +type=crs"
        );
        assert!(crs.canonical_bound_crs().is_none());
    }
    {
        let crs = factory
            .create_projected_crs("TEST_BOUND")
            .expect("TEST_BOUND");
        assert_eq!(crs.name(), "custom_projected_crs2");
        assert_eq!(crs.identifiers().len(), 1);
        let crs = Crs::Projected(crs);
        assert_eq!(
            crs.export_to_proj_string().expect("proj string"),
            "+proj=mbt_s +datum=WGS84 +units=m +no_defs +type=crs"
        );
        assert!(crs.canonical_bound_crs().is_some());
    }

    assert!(factory.create_projected_crs("TEST_WRONG").is_err());

    {
        let crs = create_crs_from_proj_string("+proj=merc +a=1 +b=1 +type=crs").expect("merc");
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.name(), "merc");
    }
    {
        let crs = create_crs_from_proj_string("+proj=merc +ellps=GRS80 +type=crs").expect("merc");
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.name(), "merc2");
    }
    {
        let crs = create_crs_from_proj_string("+proj=merc +a=6378137 +rf=298.257222101 +type=crs")
            .expect("merc");
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.name(), "merc2");
    }
    {
        let crs = create_crs_from_proj_string("+proj=merc +ellps=WGS84 +type=crs").expect("merc");
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.name(), "WKT1_GDAL");
    }
    {
        let list = factory.get_crs_info_list().expect("list");
        assert!(list
            .iter()
            .any(|info| info.authority == "TEST_NS" && info.code == "TEST_BOUND"));
    }
}
