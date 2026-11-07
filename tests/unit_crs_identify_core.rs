//! PROJ 9.9.0 `test/unit/test_crs.cpp`: identification of compound CRS against the database
//! (`compoundCRS_identify_db`).

use proj_rs::iso19111::{
    create_crs_from_proj_string_with_database, create_from_user_input,
    equivalent_name_bigger_differences, AuthorityFactory, Crs, DatabaseContext, WktConvention,
    WktFormatter, WktParser,
};

/// `exportToWKT` with the WKT1 ESRI convention, as the C++ test writes the compound.
fn wkt1_esri(crs: &Crs, db: &DatabaseContext) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Esri).with_database(db.clone()))
        .expect("ESRI WKT1")
}

/// The EPSG code of a CRS, `""` when it has none, as `getEPSGCode()` reads it.
fn epsg(crs: &Crs) -> String {
    crs.authority_code("EPSG")
        .map_or_else(String::new, |identifier| identifier.code().to_owned())
}

fn parse_wkt(db: &DatabaseContext, wkt: &str) -> Crs {
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    parser
        .create_from_wkt(wkt)
        .expect("WKT")
        .crs()
        .cloned()
        .expect("a CRS")
}

/// `TEST(crs, compoundCRS_identify_db)`
#[test]
fn crs_compound_crs_identify_db() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    {
        // Identify by existing code
        let res = Crs::Compound(factory.create_compound_crs("8769").expect("8769"))
            .identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8769");
        assert_eq!(res[0].1, 100);

        // Test with null
        let crs = Crs::Compound(factory.create_compound_crs("8769").expect("8769"));
        assert!(crs.identify_with(None).is_empty());
    }
    {
        // Non-existing code
        let source = factory.create_compound_crs("8769").expect("8769");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create(source.name(), source.components().to_vec())
                .expect("compound")
                .with_identifier("EPSG", "1"),
        );
        assert!(crs.identify_with(Some(&factory)).is_empty());
    }
    {
        // Existing code, but not matching content
        let source = factory.create_compound_crs("8769").expect("8769");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create(source.name(), source.components().to_vec())
                .expect("compound")
                .with_identifier("EPSG", "8770"),
        );
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8770");
        assert_eq!(res[0].1, 25);
    }
    {
        // Identify by exact name
        let source = factory.create_compound_crs("8769").expect("8769");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create(source.name(), source.components().to_vec())
                .expect("compound"),
        );
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8769");
        assert_eq!(res[0].1, 100);
    }
    {
        assert!(equivalent_name_bigger_differences(
            "NAD83_Ohio_North_ftUS_NAVD88_height_ftUS",
            "NAD83 / Ohio North (ftUS) + NAVD88 height (ftUS)"
        ));

        // Identify by equivalent name
        let source = factory.create_compound_crs("8769").expect("8769");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create(
                "NAD83_Ohio_North_ftUS_NAVD88_height_ftUS",
                source.components().to_vec(),
            )
            .expect("compound"),
        );
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8769");
        assert_eq!(res[0].1, 90);
    }
    {
        // Identify by name, but objects aren't equivalent
        let source = factory.create_compound_crs("8770").expect("8770");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create(
                "NAD83_Ohio_North_ftUS_NAVD88_height_ftUS",
                source.components().to_vec(),
            )
            .expect("compound"),
        );
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8769");
        assert_eq!(res[0].1, 25);
    }
    {
        // Identify by name, not matching at all
        let source = factory.create_compound_crs("8769").expect("8769");
        let crs = Crs::Compound(
            proj_rs::iso19111::CompoundCRS::create("unrelated name", source.components().to_vec())
                .expect("compound"),
        );
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "8769");
        assert_eq!(res[0].1, 70);
    }
    {
        // A PROJ string that is not a compound: only checks that identification does not fail.
        let crs = create_crs_from_proj_string_with_database(
            "+proj=tmerc +lat_0=0 +lon_0=72.05 +k=1 +x_0=3500000 \
             +y_0=-5811057.63 +ellps=krass \
             +towgs84=23.57,-140.95,-79.8,0,-0.35,-0.79,-0.22 \
             +geoidgrids=egm08_25.gtx +units=m +no_defs +type=crs",
            &db,
        )
        .expect("PROJ string");
        assert!(matches!(crs, Crs::Compound(_)), "not compound");
        let _ = crs.identify_with(Some(&factory));
    }
    {
        // Identify from ESRI WKT
        let source = factory.create_compound_crs("7405").expect("7405");
        let wkt = wkt1_esri(&Crs::Compound(source), &db);
        let crs = parse_wkt(&db, &wkt);
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "7405");
        assert_eq!(res[0].1, 100);
    }
    {
        // Identify a CompoundCRS whose horizontal and vertical parts are known
        // but not the composition.
        let source = create_from_user_input("EPSG:4326+5703", &db).expect("EPSG:4326+5703");
        let wkt = wkt1_esri(&source, &db);
        let crs = parse_wkt(&db, &wkt);
        let res = crs.identify_with(Some(&factory));
        assert_eq!(res.len(), 1);
        assert_eq!(epsg(&res[0].0), "");
        assert_eq!(res[0].1, 100);
        let components = match &res[0].0 {
            Crs::Compound(compound) => compound.components(),
            other => panic!("not compound: {other:?}"),
        };
        assert_eq!(epsg(&components[0]), "4326");
        assert_eq!(epsg(&components[1]), "5703");
    }
}
