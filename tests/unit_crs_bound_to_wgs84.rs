//! PROJ 9.9.0 `test/unit/test_crs.cpp`, `crs_createBoundCRSToWGS84IfPossible`: the CRS that
//! `createBoundCRSToWGS84IfPossible` gives back, bound to WGS 84 when the database has one
//! transformation, and itself otherwise.
//!
//! The C++ test compares pointers: a CRS that is returned unchanged is the same object. The
//! Rust side compares the structure with [`Equivalence::Strict`], which is the closest check.
//! The C++ call passes `IntermediateCRSUse::NEVER`; this crate's call takes no such argument and
//! uses its one policy. The timing check of the C++ test (GDA2020 geocentric, under
//! `IF_NO_DIRECT_TRANSFORMATION`) is not ported: it measures a wall clock, not a result.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CartesianCS, CompoundCRS, Conversion, Crs,
    DatabaseContext, Equivalence, GeodeticCRS, GeographicCRS, OpProps, ProjectedCRS, UnitOfMeasure,
    VerticalCRS, VerticalCS, VerticalReferenceFrame, WktConvention, WktFormatter, WktParser,
};

/// The object `createBoundCRSToWGS84IfPossible` returns is the input object (`EXPECT_EQ` on the
/// pointers): the two CRS are the same structure.
fn same(a: &Crs, b: &Crs) -> bool {
    a.is_equivalent_to_with(b, Equivalence::Strict)
}

/// The helper `createProjected()` of the C++ test: `WGS 84 / UTM zone 31N` (EPSG:32631).
fn create_projected() -> ProjectedCRS {
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

/// The helper `createVerticalCRS()` of the C++ test: `ODN height` on `Ordnance Datum Newlyn`.
fn create_vertical_crs() -> VerticalCRS {
    VerticalCRS::create(
        "ODN height",
        VerticalReferenceFrame::named("Ordnance Datum Newlyn").with_identifier("EPSG", "5101"),
        VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "5701")
}

/// The helper `createCompoundCRS()` of the C++ test: the projected CRS and the vertical CRS.
fn create_compound_crs() -> CompoundCRS {
    CompoundCRS::create(
        "horizontal + vertical",
        vec![
            Crs::Projected(create_projected()),
            Crs::Vertical(create_vertical_crs()),
        ],
    )
    .expect("compound")
    .with_identifier("codespace", "code")
}

/// `TEST(crs, crs_createBoundCRSToWGS84IfPossible)`
#[test]
fn crs_create_bound_crs_to_wgs84_if_possible() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "EPSG");
    let ignf = AuthorityFactory::create(&db, "IGNF");
    let code = |code: &str| {
        factory
            .create_coordinate_reference_system(code)
            .expect(code)
    };

    // Each of these has no transformation to WGS 84 in the database that is not ballpark, so
    // the CRS comes back as it is.
    for unchanged in ["4326", "32631", "5670", "3844", "2171"] {
        let crs = code(unchanged);
        let res = crs.create_bound_crs_to_wgs84_if_possible(&db);
        assert!(same(&res, &crs), "EPSG:{unchanged}: {res:?}");
    }

    {
        // NTF (Paris)
        let crs_4807 = code("4807");
        let bound = crs_4807.create_bound_crs_to_wgs84_if_possible(&db);
        assert!(!same(&bound, &crs_4807));
        assert!(same(
            &bound.create_bound_crs_to_wgs84_if_possible(&db),
            &bound
        ));
        assert!(matches!(bound, Crs::Bound(_)));
        assert_eq!(
            bound.export_to_proj_string().expect("PROJ string"),
            "+proj=longlat +ellps=clrk80ign +pm=paris \
             +towgs84=-168,-60,320,0,0,0,0 +no_defs +type=crs"
        );
    }

    {
        // WGS 84 + EGM2008 height
        let crs = create_from_user_input("EPSG:4326+3855", &db).expect("EPSG:4326+3855");
        let res = crs.create_bound_crs_to_wgs84_if_possible(&db);
        assert!(!same(&res, &crs));
        assert!(same(&res.create_bound_crs_to_wgs84_if_possible(&db), &res));
        assert!(matches!(res, Crs::Compound(_)));
        assert_eq!(
            res.export_to_proj_string().expect("PROJ string"),
            "+proj=longlat +datum=WGS84 +geoidgrids=us_nga_egm08_25.tif \
             +geoid_crs=WGS84 +vunits=m +no_defs +type=crs"
        );
    }

    {
        let crs = Crs::Vertical(create_vertical_crs());
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
    }

    {
        let crs = Crs::Compound(create_compound_crs());
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
    }

    {
        // TERA50STEREO (IGNF)
        let crs = ignf
            .create_coordinate_reference_system("TERA50STEREO")
            .expect("TERA50STEREO");
        let bound = crs.create_bound_crs_to_wgs84_if_possible(&db);
        assert!(!same(&bound, &crs));
        assert!(matches!(bound, Crs::Bound(_)));
        assert_eq!(
            bound.export_to_proj_string().expect("PROJ string"),
            "+proj=stere +lat_0=-90 +lon_0=140 +k=0.960272946 \
             +x_0=300000 +y_0=-2299363.482 +ellps=intl \
             +towgs84=324.8,153.6,172.1,0,0,0,0 +units=m +no_defs +type=crs"
        );
    }

    {
        // PGP50 (IGNF)
        let crs = ignf
            .create_coordinate_reference_system("PGP50")
            .expect("PGP50");
        let bound = crs.create_bound_crs_to_wgs84_if_possible(&db);
        assert!(!same(&bound, &crs));
        assert!(matches!(bound, Crs::Bound(_)));
        assert_eq!(
            bound.export_to_proj_string().expect("PROJ string"),
            "+proj=geocent +ellps=intl \
             +towgs84=324.8,153.6,172.1,0,0,0,0 +units=m +no_defs +type=crs"
        );
    }

    {
        // NAD83
        let crs = code("4269");
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
    }

    {
        // POSGAR 2007: it has 2 helmert shifts to WGS84 (#2356). Don't take an arbitrary one
        let crs = code("5340");
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
    }

    {
        // "MGI 1901 / Balkans zone 7": two areas of validity, one per country group
        let crs = code("6316");
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
    }

    {
        // Pulkovo 1942 / CS63 zone A2. Two candidate transformations found, so not picking any.
        let crs = code("2936");
        assert!(same(&crs.create_bound_crs_to_wgs84_if_possible(&db), &crs));
        let wkt = crs
            .export_to_wkt(
                &mut WktFormatter::create(WktConvention::Wkt1Gdal).with_database(db.clone()),
            )
            .expect("WKT1");
        let mut parser = WktParser::new();
        parser.attach_database(db.clone());
        let crs_from_wkt = parser
            .create_from_wkt(&wkt)
            .expect("WKT1 import")
            .crs()
            .cloned()
            .expect("a CRS");
        assert!(same(
            &crs_from_wkt.create_bound_crs_to_wgs84_if_possible(&db),
            &crs_from_wkt
        ));
    }
}
