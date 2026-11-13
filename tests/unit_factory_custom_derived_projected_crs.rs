//! PROJ 9.9.0 `test/unit/test_factory.cpp`, `FactoryWithTmpDatabase.custom_derived_projected_crs`.
//!
//! The C++ fixture builds a temporary `proj.db` (`createStructure` and
//! `populateWithFakeEPSG`), then adds `TEST_NS` rows for a deriving conversion
//! and for derived projected CRSs, stored relationally and as `text_definition`.
//! Triggers of the structure reject the bad rows. The expected values are the
//! C++ ones.

#[path = "support/fake_epsg.rs"]
mod fake_epsg;

use proj_rs::iso19111::{AuthorityFactory, Crs, DeprecatedCodes, ObjectType};

const DERIVED2_WKT: &str = r#"DERIVEDPROJCRS["my derived projected crs text_def",
  BASEPROJCRS["WGS 84 / UTM zone 31N",
    BASEGEOGCRS["WGS 84",
      DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,
          LENGTHUNIT["metre",1]]],
      PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],
      CS[ellipsoidal,2],
        AXIS["latitude",north],AXIS["longitude",east],
        ANGLEUNIT["degree",0.0174532925199433]],
    CONVERSION["UTM zone 31N",METHOD["Transverse Mercator"],
      PARAMETER["Latitude of natural origin",0,
        ANGLEUNIT["degree",0.0174532925199433]],
      PARAMETER["Longitude of natural origin",3,
        ANGLEUNIT["degree",0.0174532925199433]],
      PARAMETER["Scale factor at natural origin",0.9996,
        SCALEUNIT["unity",1]],
      PARAMETER["False easting",500000,LENGTHUNIT["metre",1]],
      PARAMETER["False northing",0,LENGTHUNIT["metre",1]]]],
  DERIVINGCONVERSION["Deriving conversion",
    METHOD["Transverse Mercator"]],
  CS[Cartesian,2],
    AXIS["easting (E)",east,LENGTHUNIT["metre",1]],
    AXIS["northing (N)",north,LENGTHUNIT["metre",1]]]"#;

const PROJ_WITH_DERIVED_WKT: &str = r#"DERIVEDPROJCRS["test",
  BASEPROJCRS["WGS 84 / UTM zone 31N",
    BASEGEOGCRS["WGS 84",
      DATUM["World Geodetic System 1984",
        ELLIPSOID["WGS 84",6378137,298.257223563,
          LENGTHUNIT["metre",1]]],
      PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],
      CS[ellipsoidal,2],
        AXIS["latitude",north],AXIS["longitude",east],
        ANGLEUNIT["degree",0.0174532925199433]],
    CONVERSION["UTM zone 31N",METHOD["Transverse Mercator"],
      PARAMETER["Latitude of natural origin",0,
        ANGLEUNIT["degree",0.0174532925199433]],
      PARAMETER["Longitude of natural origin",3,
        ANGLEUNIT["degree",0.0174532925199433]],
      PARAMETER["Scale factor at natural origin",0.9996,
        SCALEUNIT["unity",1]],
      PARAMETER["False easting",500000,LENGTHUNIT["metre",1]],
      PARAMETER["False northing",0,LENGTHUNIT["metre",1]]]],
  DERIVINGCONVERSION["test",METHOD["Transverse Mercator"]],
  CS[Cartesian,2],
    AXIS["easting (E)",east,LENGTHUNIT["metre",1]],
    AXIS["northing (N)",north,LENGTHUNIT["metre",1]]]"#;

/// `TEST_F(FactoryWithTmpDatabase, custom_derived_projected_crs)`
#[test]
fn factory_with_tmp_database_custom_derived_projected_crs() {
    let db = fake_epsg::database();
    // Deriving conversion, affine identity without parameters.
    db.execute_sql(
        "INSERT INTO conversion_table VALUES('TEST_NS','DERIVING_CONV',\
         'Deriving conversion',NULL,'EPSG','9807',\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         NULL,NULL,NULL,NULL,NULL,\
         0);",
    )
    .expect("DERIVING_CONV");
    db.execute_sql(
        "INSERT INTO usage VALUES('TEST_NS','deriving_conv_usage','conversion',\
         'TEST_NS','DERIVING_CONV','EPSG','1262','EPSG','1024');",
    )
    .expect("deriving conversion usage");

    // Relational registration.
    db.execute_sql(
        "INSERT INTO derived_projected_crs \
         VALUES('TEST_NS','DERIVED1','my derived projected crs',NULL,\
         'EPSG','4400','EPSG','32631','TEST_NS','DERIVING_CONV',NULL,0);",
    )
    .expect("DERIVED1");
    db.execute_sql(
        "INSERT INTO usage VALUES('TEST_NS','derived1_usage','derived_projected_crs',\
         'TEST_NS','DERIVED1','EPSG','1262','EPSG','1024');",
    )
    .expect("derived1 usage");

    // text_definition registration.
    db.execute_sql(&format!(
        "INSERT INTO derived_projected_crs \
         VALUES('TEST_NS','DERIVED2','my derived projected crs text_def',NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,'{DERIVED2_WKT}',0);"
    ))
    .expect("DERIVED2");

    // Not a DERIVEDPROJCRS: a PROJ string in the derived table.
    db.execute_sql(
        "INSERT INTO derived_projected_crs \
         VALUES('TEST_NS','DERIVED_WRONG','wrong',NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,'+proj=utm +zone=31',0);",
    )
    .expect("DERIVED_WRONG");

    // A DERIVEDPROJCRS in the projected table.
    db.execute_sql(&format!(
        "INSERT INTO projected_crs \
         VALUES('TEST_NS','PROJ_WITH_DERIVED_WKT','proj with derived wkt',NULL,\
         NULL,NULL,NULL,NULL,NULL,NULL,'{PROJ_WITH_DERIVED_WKT}',0);"
    ))
    .expect("PROJ_WITH_DERIVED_WKT");

    // Triggers: duplicate code, deprecated base, missing conversion, wrong CS types.
    assert!(db
        .execute_sql(
            "INSERT INTO derived_projected_crs \
             VALUES('EPSG','32631','duplicate',NULL,'EPSG','4400','EPSG','32631',\
             'TEST_NS','DERIVING_CONV',NULL,0);"
        )
        .is_err());
    db.execute_sql(
        "UPDATE projected_crs SET deprecated=1 WHERE auth_name='EPSG' AND code='32631';",
    )
    .expect("deprecate base");
    assert!(db
        .execute_sql(
            "INSERT INTO derived_projected_crs \
             VALUES('TEST_NS','DERIVED_DEP_BASE','dep base',NULL,\
             'EPSG','4400','EPSG','32631','TEST_NS','DERIVING_CONV',NULL,0);"
        )
        .is_err());
    db.execute_sql(
        "UPDATE projected_crs SET deprecated=0 WHERE auth_name='EPSG' AND code='32631';",
    )
    .expect("restore base");
    assert!(db
        .execute_sql(
            "INSERT INTO derived_projected_crs \
             VALUES('TEST_NS','DERIVED_NO_CONV','no conv',NULL,\
             'EPSG','4400','EPSG','32631',NULL,NULL,NULL,0);"
        )
        .is_err());
    assert!(db
        .execute_sql(
            "INSERT INTO derived_projected_crs \
             VALUES('TEST_NS','DERIVED_ELLIPS_CS','ellips cs',NULL,\
             'EPSG','6422','EPSG','32631','TEST_NS','DERIVING_CONV',NULL,0);"
        )
        .is_err());
    assert!(db
        .execute_sql(
            "INSERT INTO derived_projected_crs \
             VALUES('TEST_NS','DERIVED_VERT_CS','vert cs',NULL,\
             'EPSG','6499','EPSG','32631','TEST_NS','DERIVING_CONV',NULL,0);"
        )
        .is_err());

    let factory = AuthorityFactory::create(&db, "TEST_NS");

    // Relational lookup.
    {
        let crs = factory
            .create_derived_projected_crs("DERIVED1")
            .expect("DERIVED1");
        assert_eq!(crs.name(), "my derived projected crs");
        assert_eq!(Crs::DerivedProjected(crs.clone()).identifiers().len(), 1);
        assert_eq!(crs.base_crs().name(), "WGS 84 / UTM zone 31N");
    }

    // createCoordinateReferenceSystem dispatches correctly.
    {
        let crs = factory
            .create_coordinate_reference_system("DERIVED1")
            .expect("DERIVED1 as CRS");
        assert!(matches!(crs, Crs::DerivedProjected(_)));
    }

    // text_definition lookup.
    {
        let crs = factory
            .create_derived_projected_crs("DERIVED2")
            .expect("DERIVED2");
        assert_eq!(crs.name(), "my derived projected crs text_def");
        assert_eq!(crs.base_crs().name(), "WGS 84 / UTM zone 31N");
    }

    // A text_definition that is not a DERIVEDPROJCRS is an error.
    assert!(factory
        .create_derived_projected_crs("DERIVED_WRONG")
        .is_err());

    // A DERIVEDPROJCRS text in the projected table is an error.
    assert!(factory
        .create_projected_crs("PROJ_WITH_DERIVED_WKT")
        .is_err());

    // Authority codes.
    {
        let codes = factory
            .get_authority_codes(ObjectType::DerivedProjectedCrs, DeprecatedCodes::Included)
            .expect("codes");
        assert_eq!(codes.len(), 3);
        assert!(codes.contains("DERIVED1"));
    }

    // Description text.
    assert_eq!(
        factory
            .get_description_text("DERIVED1")
            .expect("description"),
        "my derived projected crs"
    );

    // CRS info list.
    {
        let list = factory.get_crs_info_list().expect("list");
        let info = list
            .iter()
            .find(|info| info.authority == "TEST_NS" && info.code == "DERIVED1")
            .expect("DERIVED1 in getCRSInfoList");
        assert_eq!(info.name, "my derived projected crs");
        assert_eq!(info.type_name, "derived projected");
    }
}
