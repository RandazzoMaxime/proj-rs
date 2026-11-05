//! PROJ 9.9.0 `test/unit/test_crs_990.cpp`, `crs_createBoundCRSToWGS84IfPossible`.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CartesianCS, CompoundCRS, Conversion, Crs,
    DatabaseContext, GeodeticCRS, GeographicCRS, OpProps, ProjectedCRS, UnitOfMeasure, VerticalCRS,
    VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter, WktObject, WktParser,
};

fn unchanged(db: &DatabaseContext, crs: &Crs) {
    let bound = crs.create_bound_crs_to_wgs84_if_possible(db);
    assert!(bound.is_equivalent_to(crs), "{}", crs.name());
}

/// `TEST(crs, crs_createBoundCRSToWGS84IfPossible)`
#[test]
fn crs_create_bound_crs_to_wgs84_if_possible() {
    let db = DatabaseContext::create().expect("proj.db");
    let epsg = AuthorityFactory::create(&db, "EPSG");
    for code in [
        "4326", "32631", "5670", "3844", "2171", "6316", "5340", "4269",
    ] {
        unchanged(
            &db,
            &epsg.create_coordinate_reference_system(code).expect(code),
        );
    }

    let crs_4807 = epsg
        .create_coordinate_reference_system("4807")
        .expect("4807");
    let bound = crs_4807.create_bound_crs_to_wgs84_if_possible(&db);
    assert!(!bound.is_equivalent_to(&crs_4807));
    assert!(bound
        .create_bound_crs_to_wgs84_if_possible(&db)
        .is_equivalent_to(&bound));
    assert_eq!(
        bound
            .export_to_proj_string_with_database(&db)
            .expect("PROJ"),
        "+proj=longlat +ellps=clrk80ign +pm=paris \
         +towgs84=-168,-60,320,0,0,0,0 +no_defs +type=crs"
    );

    let compound = create_from_user_input("EPSG:4326+3855", &db).expect("4326+3855");
    let res = compound.create_bound_crs_to_wgs84_if_possible(&db);
    assert!(!res.is_equivalent_to(&compound));
    assert!(res
        .create_bound_crs_to_wgs84_if_possible(&db)
        .is_equivalent_to(&res));
    assert_eq!(
        res.export_to_proj_string_with_database(&db).expect("PROJ"),
        "+proj=longlat +datum=WGS84 +geoidgrids=us_nga_egm08_25.tif \
         +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
    );

    let vertical = Crs::Vertical(
        VerticalCRS::create(
            "ODN height",
            VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101"),
            VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
        )
        .with_identifier("EPSG", "5701"),
    );
    unchanged(&db, &vertical);

    let projected = Crs::Projected(
        ProjectedCRS::create(
            "WGS 84 / UTM zone 31N",
            GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
            Conversion::create_utm_north(&OpProps::new(), 31).expect("UTM"),
            CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
        )
        .with_identifier("EPSG", "32631"),
    );
    let compound_crs = Crs::Compound(
        CompoundCRS::create("horizontal + vertical", vec![projected, vertical]).expect("compound"),
    );
    unchanged(&db, &compound_crs);

    let ignf = AuthorityFactory::create(&db, "IGNF");
    let tera = ignf
        .create_coordinate_reference_system("TERA50STEREO")
        .expect("TERA50STEREO");
    let tera_bound = tera.create_bound_crs_to_wgs84_if_possible(&db);
    assert!(!tera_bound.is_equivalent_to(&tera));
    assert_eq!(
        tera_bound
            .export_to_proj_string_with_database(&db)
            .expect("PROJ"),
        "+proj=stere +lat_0=-90 +lon_0=140 +k=0.960272946 \
         +x_0=300000 +y_0=-2299363.482 +ellps=intl \
         +towgs84=324.8,153.6,172.1,0,0,0,0 +units=m +no_defs +type=crs"
    );

    let pgp = ignf
        .create_coordinate_reference_system("PGP50")
        .expect("PGP50");
    let pgp_bound = pgp.create_bound_crs_to_wgs84_if_possible(&db);
    assert!(!pgp_bound.is_equivalent_to(&pgp));
    assert_eq!(
        pgp_bound
            .export_to_proj_string_with_database(&db)
            .expect("PROJ"),
        "+proj=geocent +ellps=intl +towgs84=324.8,153.6,172.1,0,0,0,0 \
         +units=m +no_defs +type=crs"
    );

    let crs_2936 = epsg
        .create_coordinate_reference_system("2936")
        .expect("2936");
    unchanged(&db, &crs_2936);
    let wkt = crs_2936
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db.clone()))
        .expect("WKT1");
    let mut parser = WktParser::new();
    parser.attach_database(db.clone());
    let from_wkt = match parser.create_from_wkt(&wkt).expect("parse") {
        WktObject::Crs(crs) => crs,
        other => panic!("expected a CRS: {other:?}"),
    };
    unchanged(&db, &from_wkt);
}
