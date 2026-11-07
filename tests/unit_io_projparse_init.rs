//! PROJ 9.9.0 `test/unit/test_io.cpp`, `projparse_init`: the `init=` PROJ string keyword, which
//! is refused outside the compatibility mode and read as `init=epsg:` in it.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, create_from_user_input_with_init_rules,
    Crs, DatabaseContext, InitRules, WktConvention, WktFormatter,
};

/// The axis directions of a coordinate system, as `isEquivalentTo` compares them for the
/// two-axis cases of this test.
fn directions(crs: &Crs) -> Vec<String> {
    match crs {
        Crs::Geographic(geographic) => geographic
            .coordinate_system()
            .axes()
            .iter()
            .map(|axis| axis.direction().to_owned())
            .collect(),
        Crs::Projected(projected) => projected
            .coordinate_system()
            .axes()
            .iter()
            .map(|axis| axis.direction().to_owned())
            .collect(),
        _ => Vec::new(),
    }
}

/// `TEST(io, projparse_init)`
#[test]
fn io_projparse_init() {
    let db = DatabaseContext::create().expect("proj.db");

    // Not allowed in non-compatibillity mode
    assert!(create_crs_from_proj_string("init=epsg:4326 +type=crs").is_err());
    assert!(create_from_user_input("init=epsg:4326", &db).is_err());

    {
        // EPSG:4326 is normally latitude-longitude order with degree,
        // but in compatibillity mode it will be long-lat
        let crs = create_from_user_input_with_init_rules("init=epsg:4326", &db, InitRules::Proj4)
            .expect("init=epsg:4326");
        assert!(matches!(crs, Crs::Geographic(_)));
        assert_eq!(directions(&crs), ["east", "north"]);
    }

    {
        // Test that +no_defs +type=crs have no effect
        let crs = create_from_user_input_with_init_rules(
            "+init=epsg:4326 +no_defs +type=crs +wktext",
            &db,
            InitRules::Proj4,
        )
        .expect("+init=epsg:4326");
        assert!(matches!(crs, Crs::Geographic(_)));
        let wkt = crs
            .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2015))
            .expect("WKT2");
        assert!(wkt.starts_with("GEODCRS[\"WGS 84\""), "{wkt}");
    }

    {
        // EPSG:3040 is normally northing-easting order, but in compatibillity
        // mode it will be easting-northing
        let crs = create_from_user_input_with_init_rules(
            "init=epsg:3040 +type=crs",
            &db,
            InitRules::Proj4,
        )
        .expect("init=epsg:3040");
        assert!(matches!(crs, Crs::Projected(_)));
        assert_eq!(directions(&crs), ["east", "north"]);
    }

    {
        let crs = create_from_user_input_with_init_rules(
            "+title=mytitle +init=epsg:4326 +over",
            &db,
            InitRules::Proj4,
        )
        .expect("+title=mytitle");
        assert_eq!(crs.name(), "mytitle");
        assert_eq!(
            crs.export_to_proj_string().expect("PROJ string"),
            "+proj=longlat +datum=WGS84 +over +no_defs +type=crs"
        );
    }
}
