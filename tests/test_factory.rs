//! PROJ `test/unit/test_factory.cpp` property checks.
//!
//! Each function is one `TEST()` name. Values are the ones the C++ test asserts.

use proj_rs::{
    AuthorityFactory, AuthorityObject, CoordinateOperation, Crs, DatabaseContext, Equivalence,
    FactoryError, GeographicCRS, NameMatch, NameSpelling, ObjectType, UnitKind,
};

fn epsg() -> AuthorityFactory {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, "EPSG")
}

fn esri() -> AuthorityFactory {
    let db = DatabaseContext::create().expect("proj.db");
    AuthorityFactory::create(&db, "ESRI")
}

fn db() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

fn is_missing(error: &FactoryError) -> bool {
    matches!(error, FactoryError::NotFound { .. })
}

#[test]
fn factory_databasecontext_create() {
    DatabaseContext::create().expect("proj.db");
}

#[test]
fn factory_authority_factory_create_object() {
    let factory = epsg();
    let missing = factory.create_object("-1").expect_err("missing");
    assert!(is_missing(&missing), "{missing}");
    let ambiguous = factory.create_object("4326").expect_err("area and crs");
    assert!(
        matches!(ambiguous, FactoryError::Factory { .. }),
        "{ambiguous}"
    );
}

#[test]
fn factory_authority_factory_create_unit_of_measure_linear() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_unit_of_measure("-1").expect_err("missing")
    ));
    let uom = factory.create_unit_of_measure("9001").expect("metre");
    assert_eq!(uom.name(), "metre");
    assert_eq!(uom.kind(), UnitKind::Linear);
    assert_eq!(uom.conversion_to_si(), 1.0);
    assert_eq!(uom.code_space(), "EPSG");
    assert_eq!(uom.code(), "9001");
}

#[test]
fn factory_authority_factory_create_unit_of_measure_linear_us_survey_foot() {
    let uom = epsg().create_unit_of_measure("9003").expect("us-ft");
    assert_eq!(uom.conversion_to_si(), 12.0 / 39.37);
}

#[test]
fn factory_authority_factory_create_unit_of_measure_angular() {
    let factory = epsg();
    let uom = factory.create_unit_of_measure("9102").expect("degree");
    assert_eq!(uom.name(), "degree");
    assert_eq!(uom.kind(), UnitKind::Angular);
    assert_eq!(
        uom.conversion_to_si(),
        proj_rs::UnitOfMeasure::degree().conversion_to_si()
    );
    assert_eq!(uom.code_space(), "EPSG");
    assert_eq!(uom.code(), "9102");
}

#[test]
fn factory_authority_factory_create_unit_of_measure_angular_9107() {
    let uom = epsg().create_unit_of_measure("9107").expect("dms");
    assert_eq!(uom.name(), "degree minute second");
    assert_eq!(uom.kind(), UnitKind::Angular);
    assert_eq!(
        uom.conversion_to_si(),
        proj_rs::UnitOfMeasure::degree().conversion_to_si()
    );
    assert_eq!(uom.code_space(), "EPSG");
    assert_eq!(uom.code(), "9107");
}

#[test]
fn factory_authority_factory_create_unit_of_measure_scale() {
    let uom = epsg().create_unit_of_measure("1028").expect("ppb");
    assert_eq!(uom.name(), "parts per billion");
    assert_eq!(uom.kind(), UnitKind::Scale);
    assert_eq!(uom.conversion_to_si(), 1e-9);
    assert_eq!(uom.code_space(), "EPSG");
    assert_eq!(uom.code(), "1028");
}

#[test]
fn factory_authority_factory_create_unit_of_measure_time() {
    let uom = epsg().create_unit_of_measure("1029").expect("year");
    assert_eq!(uom.name(), "year");
    assert_eq!(uom.kind(), UnitKind::Time);
    assert_eq!(uom.conversion_to_si(), 31_556_925.445);
    assert_eq!(uom.code_space(), "EPSG");
    assert_eq!(uom.code(), "1029");
}

#[test]
fn factory_authority_factory_create_prime_meridian() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_prime_meridian("-1").expect_err("missing")
    ));
    assert!(matches!(
        esri().create_object("108900").expect("ESRI prime meridian"),
        AuthorityObject::PrimeMeridian(_)
    ));
    let pm = factory.create_prime_meridian("8903").expect("Paris");
    assert_eq!(pm.identifiers().len(), 1);
    assert_eq!(pm.identifiers()[0].code(), "8903");
    assert_eq!(pm.identifiers()[0].code_space(), "EPSG");
    assert_eq!(pm.name(), "Paris");
    assert_eq!(pm.longitude().value(), 2.596_921_3);
    assert_eq!(pm.longitude().unit().name(), "grad");
}

#[test]
fn factory_authority_factory_identify_body_from_semi_major_axis() {
    let factory = epsg();
    assert_eq!(
        factory
            .identify_body_from_semi_major_axis(6_378_137.0, 1e-5)
            .expect("Earth"),
        "Earth"
    );
    assert!(factory
        .identify_body_from_semi_major_axis(1.0, 1e-5)
        .is_err());
}

#[test]
fn factory_authority_factory_create_ellipsoid() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_ellipsoid("-1").expect_err("missing")
    ));
    let ellipsoid = factory.create_ellipsoid("7030").expect("WGS 84");
    assert_eq!(ellipsoid.identifiers().len(), 1);
    assert_eq!(ellipsoid.identifiers()[0].code(), "7030");
    assert_eq!(ellipsoid.identifiers()[0].code_space(), "EPSG");
    assert_eq!(ellipsoid.name(), "WGS 84");
    assert_eq!(
        ellipsoid.declared_inverse_flattening(),
        Some(298.257_223_563)
    );
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_137.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
    assert_eq!(ellipsoid.celestial_body(), "Earth");
}

#[test]
fn factory_authority_factory_create_ellipsoid_sphere() {
    let ellipsoid = epsg().create_ellipsoid("7035").expect("sphere");
    assert!(ellipsoid.is_sphere());
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_371_000.0);
    assert_eq!(ellipsoid.semi_major_axis().unit().name(), "metre");
}

#[test]
fn factory_authority_factory_create_ellipsoid_with_semi_minor_axis() {
    let ellipsoid = epsg().create_ellipsoid("7011").expect("clarke ign");
    let minor = ellipsoid.semi_minor_axis().expect("b");
    assert_eq!(ellipsoid.semi_major_axis().value(), 6_378_249.2);
    assert_eq!(minor.value(), 6_356_515.0);
    assert_eq!(minor.unit().name(), "metre");
}

#[test]
fn factory_authority_factory_create_extent() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_extent("-1").expect_err("missing")
    ));
    let extent = factory.create_extent("1262").expect("world");
    assert_eq!(extent.description(), Some("World."));
    assert_eq!(extent.west_bound_longitude(), Some(-180.0));
    assert_eq!(extent.east_bound_longitude(), Some(180.0));
    assert_eq!(extent.north_bound_latitude(), Some(90.0));
    assert_eq!(extent.south_bound_latitude(), Some(-90.0));
}

#[test]
fn factory_authority_factory_create_extent_no_bbox() {
    let extent = epsg().create_extent("1361").expect("sudan");
    assert_eq!(extent.description(), Some("Sudan - south."));
    assert_eq!(extent.west_bound_longitude(), None);
    assert_eq!(extent.south_bound_latitude(), None);
}

#[test]
fn factory_authority_factory_create_geodetic_datum() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_geodetic_datum("-1").expect_err("missing")
    ));
    let datum = factory.create_geodetic_datum("6326").expect("wgs84");
    assert_eq!(datum.frame_reference_epoch(), None);
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "6326");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    assert!(datum
        .ellipsoid()
        .is_equivalent_to(&factory.create_ellipsoid("7030").expect("7030")));
    assert!(datum
        .prime_meridian()
        .is_equivalent_to(&factory.create_prime_meridian("8901").expect("greenwich")));
    assert!(!datum.domains().is_empty());
    assert!(datum.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
    assert_eq!(datum.publication_date(), None);
}

#[test]
fn factory_authority_factory_create_geodetic_datum_with_publication_date() {
    let datum = epsg().create_geodetic_datum("6269").expect("nad83");
    assert_eq!(datum.frame_reference_epoch(), None);
    assert_eq!(datum.publication_date(), Some("1986-01-01"));
}

#[test]
fn factory_authority_factory_create_dynamic_geodetic_datum() {
    let datum = epsg().create_geodetic_datum("1165").expect("itrf2014");
    assert_eq!(datum.frame_reference_epoch(), Some(2010.0));
}

#[test]
fn factory_authority_factory_create_vertical_datum() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_vertical_datum("-1").expect_err("missing")
    ));
    let datum = factory.create_vertical_datum("1027").expect("egm2008");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "1027");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "EGM2008 geoid");
    assert!(datum.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
    assert_eq!(datum.publication_date(), Some("2008-01-01"));
    assert_eq!(datum.anchor_epoch(), None);
}

#[test]
fn factory_authority_factory_create_vertical_datum_with_anchor_epoch() {
    let datum = epsg().create_vertical_datum("1256").expect("cgvd");
    let epoch = datum.anchor_epoch().expect("epoch");
    assert!((epoch - 2010.0).abs() < 1e-6, "{epoch}");
}

#[test]
fn factory_authority_factory_create_engineering_datum() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_engineering_datum("-1").expect_err("missing")
    ));
    let datum = factory.create_engineering_datum("1134").expect("christmas");
    assert_eq!(datum.identifiers().len(), 1);
    assert_eq!(datum.identifiers()[0].code(), "1134");
    assert_eq!(datum.identifiers()[0].code_space(), "EPSG");
    assert_eq!(datum.name(), "Christmas Island Datum 1985");
    assert!(datum.domains()[0].is_equivalent_to(&factory.create_extent("4169").expect("4169")));
    assert_eq!(datum.publication_date(), Some("1985-01-01"));
    assert_eq!(datum.anchor_epoch(), None);
}

#[test]
fn factory_authority_factory_create_datum() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_datum("-1").expect_err("missing")
    ));
    assert!(factory
        .create_datum("6326")
        .expect("6326")
        .is_equivalent_to(&proj_rs::Datum::Geodetic(
            factory.create_geodetic_datum("6326").expect("frame")
        )));
    assert!(factory
        .create_datum("1027")
        .expect("1027")
        .is_equivalent_to(&proj_rs::Datum::Vertical(
            factory.create_vertical_datum("1027").expect("vertical")
        )));
}

#[test]
fn factory_authority_factory_create_datum_ensemble_geodetic() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_datum_ensemble("-1").expect_err("missing")
    ));
    assert!(is_missing(
        &factory
            .create_datum_ensemble_in("6326", "vertical_datum")
            .expect_err("wrong table")
    ));
    let ensemble = factory.create_datum_ensemble("6326").expect("ensemble");
    assert_eq!(ensemble.name(), "World Geodetic System 1984 ensemble");
    assert_eq!(ensemble.identifiers().len(), 1);
    assert_eq!(ensemble.identifiers()[0].code(), "6326");
    assert_eq!(ensemble.identifiers()[0].code_space(), "EPSG");
    assert_eq!(ensemble.members().len(), 8);
    assert_eq!(ensemble.positional_accuracy(), 2.0);
    assert!(ensemble.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
    let datum = ensemble.as_datum().expect("datum");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    assert!(datum.is_equivalent_to(&factory.create_datum("6326").expect("6326")));
}

#[test]
fn factory_authority_factory_create_datum_ensemble_vertical() {
    let factory = epsg();
    assert!(is_missing(
        &factory
            .create_datum_ensemble_in("1288", "geodetic_datum")
            .expect_err("wrong table")
    ));
    let ensemble = factory.create_datum_ensemble("1288").expect("ensemble");
    assert_eq!(ensemble.name(), "British Isles height ensemble");
    assert_eq!(ensemble.identifiers().len(), 1);
    assert_eq!(ensemble.identifiers()[0].code(), "1288");
    assert_eq!(ensemble.identifiers()[0].code_space(), "EPSG");
    assert_eq!(ensemble.members().len(), 9);
    assert_eq!(ensemble.positional_accuracy(), 0.4);
    assert!(ensemble.domains()[0].is_equivalent_to(&factory.create_extent("4606").expect("4606")));
    let datum = ensemble.as_datum().expect("datum");
    assert!(datum.is_equivalent_to(&factory.create_datum("1288").expect("1288")));
}

#[test]
fn factory_authority_factory_create_coordinate_system_ellipsoidal_2_axis() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_coordinate_system("-1").expect_err("missing")
    ));
    let cs = factory.create_coordinate_system("6422").expect("6422");
    let proj_rs::CoordinateSystem::Ellipsoidal(cs) = cs else {
        panic!("not ellipsoidal");
    };
    assert_eq!(cs.identifiers().len(), 1);
    assert_eq!(cs.identifiers()[0].code(), "6422");
    assert_eq!(cs.identifiers()[0].code_space(), "EPSG");
    let axes = cs.axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].name(), "Geodetic latitude");
    assert_eq!(axes[0].abbreviation(), "Lat");
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(axes[0].unit().name(), "degree");
    assert_eq!(axes[1].name(), "Geodetic longitude");
    assert_eq!(axes[1].abbreviation(), "Lon");
    assert_eq!(axes[1].direction(), "east");
    assert_eq!(axes[1].unit().name(), "degree");
}

#[test]
fn factory_authority_factory_create_coordinate_system_ellipsoidal_3_axis() {
    let cs = epsg().create_coordinate_system("6423").expect("6423");
    let proj_rs::CoordinateSystem::Ellipsoidal(cs) = cs else {
        panic!("not ellipsoidal");
    };
    assert_eq!(cs.identifiers()[0].code(), "6423");
    let axes = cs.axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[2].name(), "Ellipsoidal height");
    assert_eq!(axes[2].abbreviation(), "h");
    assert_eq!(axes[2].direction(), "up");
    assert_eq!(axes[2].unit().name(), "metre");
}

#[test]
fn factory_authority_factory_create_coordinate_system_geocentric() {
    let cs = epsg().create_coordinate_system("6500").expect("6500");
    let proj_rs::CoordinateSystem::Cartesian(cs) = cs else {
        panic!("not cartesian");
    };
    assert_eq!(cs.identifiers()[0].code(), "6500");
    assert_eq!(cs.identifiers()[0].code_space(), "EPSG");
    let axes = cs.axes();
    assert_eq!(axes.len(), 3);
    assert_eq!(axes[0].name(), "Geocentric X");
    assert_eq!(axes[0].abbreviation(), "X");
    assert_eq!(axes[0].direction(), "geocentricX");
    assert_eq!(axes[0].unit().name(), "metre");
    assert_eq!(axes[1].direction(), "geocentricY");
    assert_eq!(axes[2].direction(), "geocentricZ");
}

#[test]
fn factory_authority_factory_create_coordinate_system_vertical() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_coordinate_system("-1").expect_err("missing")
    ));
    let cs = factory.create_coordinate_system("6499").expect("6499");
    let proj_rs::CoordinateSystem::Vertical(cs) = cs else {
        panic!("not vertical");
    };
    assert_eq!(cs.identifiers()[0].code(), "6499");
    let axis = cs.axis();
    assert_eq!(axis.name(), "Gravity-related height");
    assert_eq!(axis.abbreviation(), "H");
    assert_eq!(axis.direction(), "up");
    assert_eq!(axis.unit().name(), "metre");
}

#[test]
fn factory_authority_factory_create_geodetic_crs_geographic2d() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_geodetic_crs("-1").expect_err("missing")
    ));
    let crs = factory.create_geodetic_crs("4326").expect("4326");
    let proj_rs::GeodeticCRS::Geographic(geographic) = &crs else {
        panic!("not geographic");
    };
    assert_eq!(geographic.identifiers()[0].code(), "4326");
    assert_eq!(geographic.identifiers()[0].code_space(), "EPSG");
    assert_eq!(geographic.name(), "WGS 84");
    assert!(geographic.datum().is_none());
    assert!(geographic
        .datum_ensemble()
        .expect("ensemble")
        .is_equivalent_to(&factory.create_datum_ensemble("6326").expect("6326")));
    assert!(crs
        .coordinate_system()
        .is_equivalent_to(&factory.create_coordinate_system("6422").expect("6422")));
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
    assert_eq!(
        crs.export_to_proj_string().expect("proj"),
        "+proj=longlat +datum=WGS84 +no_defs +type=crs"
    );
}

#[test]
fn factory_authority_factory_create_geodetic_crs_geographic2d_area_no_bbox() {
    let factory = epsg();
    let crs = factory.create_geodetic_crs("4296").expect("4296");
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("1361").expect("1361")));
}

#[test]
fn factory_authority_factory_create_geodetic_crs_geographic3d() {
    let factory = epsg();
    let crs = factory.create_geodetic_crs("4979").expect("4979");
    let proj_rs::GeodeticCRS::Geographic(geographic) = &crs else {
        panic!("not geographic");
    };
    assert_eq!(geographic.identifiers()[0].code(), "4979");
    assert_eq!(geographic.name(), "WGS 84");
    assert!(geographic.datum().is_none());
    assert!(geographic
        .datum_ensemble()
        .expect("ensemble")
        .is_equivalent_to(&factory.create_datum_ensemble("6326").expect("6326")));
    assert!(crs
        .coordinate_system()
        .is_equivalent_to(&factory.create_coordinate_system("6423").expect("6423")));
}

#[test]
fn factory_authority_factory_create_geodetic_crs_geocentric() {
    let factory = epsg();
    let crs = factory.create_geodetic_crs("4978").expect("4978");
    assert!(!matches!(crs, proj_rs::GeodeticCRS::Geographic(_)));
    assert_eq!(crs.identifiers()[0].code(), "4978");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "WGS 84");
    assert!(crs.datum().is_none());
    assert!(crs
        .datum_ensemble()
        .expect("ensemble")
        .is_equivalent_to(&factory.create_datum_ensemble("6326").expect("6326")));
    assert!(crs
        .coordinate_system()
        .is_equivalent_to(&factory.create_coordinate_system("6500").expect("6500")));
}

#[test]
fn factory_authority_factory_create_geographic_crs() {
    let factory = epsg();
    let crs = factory.create_geographic_crs("4979").expect("4979");
    assert_eq!(crs.identifiers()[0].code(), "4979");
    assert!(factory.create_geographic_crs("4978").is_err());
}

#[test]
fn factory_authority_factory_create_vertical_crs() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_vertical_crs("-1").expect_err("missing")
    ));
    let crs = factory.create_vertical_crs("3855").expect("3855");
    assert_eq!(crs.identifiers()[0].code(), "3855");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "EGM2008 height");
    assert!(crs
        .datum()
        .expect("datum")
        .is_equivalent_to_datum(&factory.create_datum("1027").expect("1027")));
    assert!(
        proj_rs::CoordinateSystem::Vertical(crs.coordinate_system().clone())
            .is_equivalent_to(&factory.create_coordinate_system("6499").expect("6499"))
    );
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
}

#[test]
fn factory_authority_factory_create_vertical_crs_with_datum_ensemble() {
    let factory = epsg();
    let crs = factory.create_vertical_crs("9451").expect("9451");
    assert!(crs.datum().is_none());
    assert!(crs
        .datum_ensemble()
        .expect("ensemble")
        .is_equivalent_to(&factory.create_datum_ensemble("1288").expect("1288")));
}

#[test]
fn factory_authority_factory_create_engineering_crs() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_engineering_crs("-1").expect_err("missing")
    ));
    let crs = factory.create_engineering_crs("6715").expect("6715");
    assert_eq!(crs.identifiers()[0].code(), "6715");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "Christmas Island Grid 1985");
    assert!(crs
        .datum()
        .is_equivalent_to_datum(&factory.create_datum("1134").expect("1134")));
    assert!(crs
        .coordinate_system()
        .is_equivalent_to(&factory.create_coordinate_system("4400").expect("4400")));
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("4169").expect("4169")));
}

#[test]
fn factory_authority_factory_create_conversion() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_conversion("-1").expect_err("missing")
    ));
    let conversion = factory.create_conversion("16031").expect("utm 31");
    assert_eq!(conversion.identifiers()[0].code(), "16031");
    assert_eq!(conversion.identifiers()[0].code_space(), "EPSG");
    assert_eq!(conversion.name(), "UTM zone 31N");
    let method = conversion.method();
    let ids = method.identifiers();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].code(), "9807");
    assert_eq!(ids[0].code_space(), "EPSG");
    assert_eq!(method.name(), "Transverse Mercator");
    let values = conversion.parameters();
    assert_eq!(values.len(), 5);
    assert_eq!(values[0].code(), "8801");
    assert_eq!(values[0].name(), "Latitude of natural origin");
    let proj_rs::ParameterValue::Measure(measure) = values[0].value() else {
        panic!("measure");
    };
    assert_eq!(measure.unit().name(), "degree");
    assert_eq!(measure.value(), 0.0);
    assert_eq!(values[1].code(), "8802");
    assert_eq!(values[1].name(), "Longitude of natural origin");
    let proj_rs::ParameterValue::Measure(measure) = values[1].value() else {
        panic!("measure");
    };
    assert_eq!(measure.unit().name(), "degree");
    assert_eq!(measure.value(), 3.0);
}

#[test]
fn factory_authority_factory_create_conversion_from_other_transformation() {
    let operation = epsg()
        .create_coordinate_operation("7984", proj_rs::AlternativeGridNames::Official)
        .expect("7984");
    assert!(matches!(operation, CoordinateOperation::Conversion(_)));
}

#[test]
fn factory_authority_factory_create_projected_crs() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_projected_crs("-1").expect_err("missing")
    ));
    let crs = factory.create_projected_crs("32631").expect("32631");
    assert_eq!(crs.identifiers()[0].code(), "32631");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(crs.name(), "WGS 84 / UTM zone 31N");
    assert!(crs
        .base_crs()
        .is_equivalent_to(&factory.create_geodetic_crs("4326").expect("4326")));
    assert!(projected_cs(&crs)
        .is_equivalent_to(&factory.create_coordinate_system("4400").expect("4400")));
    assert!(crs
        .deriving_conversion()
        .is_equivalent_to(&factory.create_conversion("16031").expect("16031")));
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("2060").expect("2060")));
}

fn projected_cs(crs: &proj_rs::ProjectedCRS) -> proj_rs::CoordinateSystem {
    proj_rs::CoordinateSystem::Cartesian(crs.coordinate_system().clone())
}

#[test]
fn factory_authority_factory_create_projected_crs_south_pole() {
    let crs = epsg().create_projected_crs("32761").expect("32761");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert!(axes[0].meridian().is_some());
    assert_eq!(axes[0].direction(), "north");
    assert_eq!(
        axes[0].meridian().expect("meridian").longitude().value(),
        0.0
    );
    assert_eq!(axes[1].direction(), "north");
    assert_eq!(
        axes[1].meridian().expect("meridian").longitude().value(),
        90.0
    );
}

#[test]
fn factory_authority_factory_create_projected_crs_north_pole() {
    let crs = epsg().create_projected_crs("32661").expect("32661");
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].direction(), "south");
    assert_eq!(
        axes[0].meridian().expect("meridian").longitude().value(),
        180.0
    );
    assert_eq!(axes[1].direction(), "south");
    assert_eq!(
        axes[1].meridian().expect("meridian").longitude().value(),
        90.0
    );
}

#[test]
fn factory_authority_factory_create_compound_crs() {
    let factory = epsg();
    assert!(is_missing(
        &factory.create_compound_crs("-1").expect_err("missing")
    ));
    let crs = factory.create_compound_crs("6871").expect("6871");
    assert_eq!(crs.identifiers()[0].code(), "6871");
    assert_eq!(crs.identifiers()[0].code_space(), "EPSG");
    assert_eq!(
        crs.name(),
        "WGS 84 / Pseudo-Mercator +  EGM2008 geoid height"
    );
    let components = crs.components();
    assert_eq!(components.len(), 2);
    assert!(components[0].is_equivalent_to(&proj_rs::Crs::Projected(
        factory.create_projected_crs("3857").expect("3857")
    )));
    assert!(components[1].is_equivalent_to(&proj_rs::Crs::Vertical(
        factory.create_vertical_crs("3855").expect("3855")
    )));
    assert!(crs.domains()[0].is_equivalent_to(&factory.create_extent("1262").expect("1262")));
}

#[test]
fn factory_authority_factory_create_coordinate_reference_system() {
    let factory = epsg();
    assert!(is_missing(
        &factory
            .create_coordinate_reference_system("-1")
            .expect_err("missing")
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("4326")
            .expect("4326"),
        proj_rs::Crs::Geographic(_)
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("4979")
            .expect("4979"),
        proj_rs::Crs::Geographic(_)
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("4978")
            .expect("4978"),
        proj_rs::Crs::Geodetic(_)
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("32631")
            .expect("32631"),
        proj_rs::Crs::Projected(_)
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("3855")
            .expect("3855"),
        proj_rs::Crs::Vertical(_)
    ));
    assert!(matches!(
        factory
            .create_coordinate_reference_system("6871")
            .expect("6871"),
        proj_rs::Crs::Compound(_)
    ));
}

#[test]
fn factory_create_objects_from_name_finds_the_ellipsoid_by_name() {
    let factory = epsg();
    let found = factory
        .create_objects_from_name(
            "WGS 84",
            &[proj_rs::ObjectType::Ellipsoid],
            proj_rs::NameMatch::Exact,
            10,
        )
        .expect("search");
    let wgs84 = found
        .iter()
        .find(|object| object.name() == "WGS 84")
        .expect("WGS 84 ellipsoid");
    assert!(!wgs84.is_deprecated());
}

#[test]
fn database_structure_lists_tables_then_layout_version() {
    let db = DatabaseContext::create().expect("proj.db");
    let statements = db.database_structure().expect("structure");
    assert!(statements[0].starts_with("CREATE TABLE metadata("));
    assert!(statements[0].ends_with(';'));
    let tail = &statements[statements.len() - 2..];
    assert!(tail[0].starts_with("INSERT INTO metadata VALUES('DATABASE.LAYOUT.VERSION.MAJOR',"));
    assert!(tail[1].starts_with("INSERT INTO metadata VALUES('DATABASE.LAYOUT.VERSION.MINOR',"));
}

#[test]
fn ellipsoid_proj_string_is_radius_for_a_sphere() {
    let ellipsoid = epsg().create_ellipsoid("7035").expect("7035");
    assert_eq!(ellipsoid.export_to_proj_string(), "+R=6371000");
    let wgs84 = epsg().create_ellipsoid("7030").expect("7030");
    assert_eq!(wgs84.export_to_proj_string(), "+ellps=WGS84");
}

/// PROJ's `ComparePROJString`: same token count, same keys, numbers within 1e-14 relative.
fn assert_proj_string_matches(got: &str, expected: &str) {
    let got_tokens: Vec<&str> = got.split(' ').collect();
    let expected_tokens: Vec<&str> = expected.split(' ').collect();
    assert_eq!(
        got_tokens.len(),
        expected_tokens.len(),
        "{got} vs {expected}"
    );
    for (g, e) in got_tokens.iter().zip(&expected_tokens) {
        match (g.split_once('='), e.split_once('=')) {
            (Some((gk, gv)), Some((ek, ev))) => {
                assert_eq!(gk, ek, "{got} vs {expected}");
                match (gv.parse::<f64>(), ev.parse::<f64>()) {
                    (Ok(gn), Ok(en)) => {
                        assert!((gn - en).abs() <= 1e-14 * gn.abs(), "{got} vs {expected}")
                    }
                    _ => assert_eq!(gv, ev, "{got} vs {expected}"),
                }
            }
            _ => assert_eq!(g, e, "{got} vs {expected}"),
        }
    }
}

#[test]
fn factory_get_metadata() {
    let db = DatabaseContext::create().expect("proj.db");
    assert_eq!(db.metadata("i_do_not_exist").expect("query"), None);
    assert_eq!(
        db.metadata("IGNF.VERSION").expect("query").as_deref(),
        Some("3.1.0")
    );
}

#[test]
fn factory_ogc_crs() {
    let factory = AuthorityFactory::create(&db(), "OGC");
    factory
        .create_coordinate_reference_system("CRS84")
        .expect("CRS84");
    factory
        .create_coordinate_reference_system("84")
        .expect("84");
    factory
        .create_coordinate_reference_system("CRS27")
        .expect("CRS27");
    factory
        .create_coordinate_reference_system("CRS83")
        .expect("CRS83");
}

#[test]
fn factory_authority_factory_get_description_text() {
    let factory = epsg();
    assert!(is_missing(
        &factory.get_description_text("-1").expect_err("missing")
    ));
    assert_eq!(
        factory.get_description_text("10000").expect("10000"),
        "ETRS89-FRA [RGF93 v1] to NGF-IGN69 height (1)"
    );
    // Several objects share code 4326, including an area of use; the CRS wins.
    assert_eq!(
        factory.get_description_text("4326").expect("4326"),
        "WGS 84"
    );
}

#[test]
fn factory_authority_factory_epsg_4326_approximate_equivalent_to_builtin() {
    let crs = match epsg()
        .create_coordinate_reference_system("4326")
        .expect("4326")
    {
        Crs::Geographic(crs) => crs,
        other => panic!("expected a geographic CRS, got {other:?}"),
    };
    assert!(crs.is_equivalent_to_with(&GeographicCRS::epsg_4326(), Equivalence::Equivalent));
}

#[test]
fn factory_authority_factory_test_uom_9110() {
    let crs = epsg().create_projected_crs("2172").expect("2172");
    assert_proj_string_matches(
        &crs.export_to_proj_string().expect("PROJ string"),
        "+proj=sterea +lat_0=53.0019444444444 +lon_0=21.5027777777778 \
         +k=0.9998 +x_0=4603000 +y_0=5806000 +ellps=krass +units=m \
         +no_defs +type=crs",
    );
}

#[test]
fn factory_list_area_of_use_from_name() {
    let db = DatabaseContext::create().expect("proj.db");
    let factory = AuthorityFactory::create(&db, "");
    let exact = factory
        .list_area_of_use_from_name("Denmark - onshore", NameMatch::Exact)
        .expect("exact");
    assert_eq!(exact, vec![("EPSG".to_owned(), "3237".to_owned())]);
    let approximate = factory
        .list_area_of_use_from_name("Denmark", NameMatch::Approximate)
        .expect("approximate");
    assert!(approximate.len() > 1);
    let none = factory
        .list_area_of_use_from_name("no where land", NameMatch::Exact)
        .expect("none");
    assert!(none.is_empty());
}

#[test]
fn factory_get_crs_info_list() {
    let db = db();
    let all = AuthorityFactory::create(&db, "")
        .get_crs_info_list()
        .expect("all");
    assert!(all.len() > 1);
    assert!(all.iter().any(|info| info.authority == "EPSG"));
    assert!(all.iter().any(|info| info.authority == "IGNF"));
    let iau = all
        .iter()
        .find(|info| info.authority == "IAU_2015" && info.code == "19902")
        .expect("IAU_2015:19902");
    assert_eq!(iau.object_type(), ObjectType::GeodeticCrs);

    let epsg_list = AuthorityFactory::create(&db, "EPSG")
        .get_crs_info_list()
        .expect("EPSG");
    assert!(epsg_list.iter().all(|info| info.authority == "EPSG"));

    let find = |code: &str| {
        epsg_list
            .iter()
            .find(|info| info.code == code)
            .unwrap_or_else(|| panic!("EPSG:{code}"))
    };
    let wgs84 = find("4326");
    assert_eq!(wgs84.name, "WGS 84");
    assert_eq!(wgs84.object_type(), ObjectType::Geographic2dCrs);
    assert!(!wgs84.deprecated);
    assert_eq!(wgs84.west_lon, Some(-180.0));
    assert_eq!(wgs84.south_lat, Some(-90.0));
    assert_eq!(wgs84.east_lon, Some(180.0));
    assert_eq!(wgs84.north_lat, Some(90.0));
    assert!(wgs84.area_name.starts_with("World"), "{}", wgs84.area_name);
    assert!(wgs84.projection_method_name.is_empty());

    let soudan = find("4296");
    assert_eq!(soudan.west_lon, None);

    let geocentric = find("4978");
    assert_eq!(geocentric.name, "WGS 84");
    assert_eq!(geocentric.object_type(), ObjectType::GeocentricCrs);

    let geographic_3d = find("4979");
    assert_eq!(geographic_3d.name, "WGS 84");
    assert_eq!(geographic_3d.object_type(), ObjectType::Geographic3dCrs);

    let utm = find("32631");
    assert_eq!(utm.name, "WGS 84 / UTM zone 31N");
    assert_eq!(utm.object_type(), ObjectType::ProjectedCrs);
    assert!(!utm.deprecated);
    assert_eq!(utm.west_lon, Some(0.0));
    assert_eq!(utm.south_lat, Some(0.0));
    assert_eq!(utm.east_lon, Some(6.0));
    assert_eq!(utm.north_lat, Some(84.0));
    assert!(utm
        .area_name
        .starts_with("Between 0\u{b0}E and 6\u{b0}E, northern hemisphere"));
    assert_eq!(utm.projection_method_name, "Transverse Mercator");

    let geoid = find("3855");
    assert_eq!(geoid.name, "EGM2008 height");
    assert_eq!(geoid.object_type(), ObjectType::VerticalCrs);

    let compound = find("6871");
    assert_eq!(
        compound.name,
        "WGS 84 / Pseudo-Mercator +  EGM2008 geoid height"
    );
    assert_eq!(compound.object_type(), ObjectType::CompoundCrs);
}

#[test]
fn factory_get_unit_list() {
    let db = db();
    let all = AuthorityFactory::create(&db, "")
        .get_unit_list()
        .expect("all");
    assert!(all.iter().any(|info| info.authority == "EPSG"));
    assert!(all.iter().any(|info| info.authority == "PROJ"));
    let find = |code: &str| {
        all.iter()
            .find(|info| info.authority == "EPSG" && info.code == code)
            .unwrap_or_else(|| panic!("EPSG:{code}"))
    };
    let mm_year = find("1027");
    assert_eq!(mm_year.name, "millimetres per year");
    assert_eq!(mm_year.category, "linear_per_time");
    let ppb = find("1028");
    assert_eq!(ppb.name, "parts per billion");
    assert_eq!(ppb.category, "scale");
    let mas_year = find("1032");
    assert_eq!(mas_year.name, "milliarc-seconds per year");
    assert_eq!(mas_year.category, "angular_per_time");
    let unity_second = find("1036");
    assert_eq!(unity_second.name, "unity per second");
    assert_eq!(unity_second.category, "scale_per_time");
    let metre = find("9001");
    assert_eq!(metre.name, "metre");
    assert_eq!(metre.category, "linear");
    assert_eq!(metre.conv_factor, 1.0);
    assert_eq!(metre.proj_short_name, "m");
    assert!(!metre.deprecated);
    let radian = find("9101");
    assert_eq!(radian.name, "radian");
    assert_eq!(radian.category, "angular");
    assert!(!radian.deprecated);

    let epsg_only = AuthorityFactory::create(&db, "EPSG")
        .get_unit_list()
        .expect("EPSG");
    assert!(epsg_only.len() > 1);
    assert!(epsg_only.iter().all(|info| info.authority == "EPSG"));
}

#[test]
fn factory_get_celestial_body_list() {
    let db = db();
    let all = AuthorityFactory::create(&db, "")
        .get_celestial_body_list()
        .expect("all");
    assert!(all.len() > 1);
    assert!(all.iter().any(|info| info.authority == "ESRI"));
    assert!(all
        .iter()
        .any(|info| info.authority == "PROJ" && info.name == "Earth"));

    let esri = AuthorityFactory::create(&db, "ESRI")
        .get_celestial_body_list()
        .expect("ESRI");
    assert!(esri.len() > 1);
    assert!(esri.iter().all(|info| info.authority == "ESRI"));
}

#[test]
fn factory_to_wgs84_autocorrect_wrong_values() {
    let db = db();
    let mut unchanged = [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0];
    assert!(!db
        .to_wgs84_autocorrect_wrong_values(&mut unchanged)
        .expect("query"));
    assert_eq!(unchanged, [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0]);

    // Incorrect parameters for EPSG:15929: WGS84 -> Belgian Lambert 72, PROJ issue 4170.
    let mut wrong = [
        -106.8686, 52.2978, -103.7239, -0.3366, 0.457, -1.8422, -1.2747,
    ];
    assert!(db
        .to_wgs84_autocorrect_wrong_values(&mut wrong)
        .expect("query"));
    assert_eq!(
        wrong,
        [-106.8686, 52.2978, -103.7239, 0.3366, -0.457, 1.8422, -1.2747]
    );

    // Almost incorrect: tx differs from the database row by more than 1e-8 relative.
    let mut almost = [-106.0, 52.2978, -103.7239, -0.3366, 0.457, -1.8422, -1.2747];
    assert!(!db
        .to_wgs84_autocorrect_wrong_values(&mut almost)
        .expect("query"));
    assert_eq!(
        almost,
        [-106.0, 52.2978, -103.7239, -0.3366, 0.457, -1.8422, -1.2747]
    );

    // Correct position vector transformation, EPSG:15869 (DHDN to WGS 84 (3)).
    let mut correct_pv = [612.4, 77.0, 440.2, -0.054, 0.057, -2.797, 2.55];
    assert!(!db
        .to_wgs84_autocorrect_wrong_values(&mut correct_pv)
        .expect("query"));
    assert_eq!(
        correct_pv,
        [612.4, 77.0, 440.2, -0.054, 0.057, -2.797, 2.55]
    );

    // Correct coordinate frame rotation for EPSG:15929.
    let mut correct_cf = [
        -106.8686, 52.2978, -103.7239, 0.3366, -0.457, 1.8422, -1.2747,
    ];
    assert!(!db
        .to_wgs84_autocorrect_wrong_values(&mut correct_cf)
        .expect("query"));
    assert_eq!(
        correct_cf,
        [-106.8686, 52.2978, -103.7239, 0.3366, -0.457, 1.8422, -1.2747]
    );
}

#[test]
fn factory_authority_factory_get_available_geoidmodels() {
    let factory = epsg();
    assert!(factory.get_geoid_models("4326").expect("4326").is_empty());

    let navd88 = |code: &str| {
        let models = factory.get_geoid_models(code).expect(code);
        assert!(models.iter().any(|model| model == "GEOID12B"), "{code}");
        assert!(models.iter().any(|model| model == "GEOID18"), "{code}");
        assert!(!models.iter().any(|model| model == "OSGM15"), "{code}");
    };
    for code in ["5703", "6360", "8228", "6357", "6358"] {
        navd88(code);
    }

    for code in ["5701", "5732"] {
        let models = factory.get_geoid_models(code).expect(code);
        assert!(!models.iter().any(|model| model == "GEOID12B"), "{code}");
        assert!(!models.iter().any(|model| model == "GEOID18"), "{code}");
        assert!(models.iter().any(|model| model == "OSGM15"), "{code}");
    }
}

#[test]
fn factory_get_official_name_from_alias() {
    let factory = AuthorityFactory::create(&db(), "");
    let found = factory
        .official_name_from_alias("GCS_WGS_1984", None, None, NameSpelling::Exact)
        .expect("query")
        .expect("WGS 84");
    assert_eq!(found.name, "WGS 84");
    assert_eq!(found.table_name, "geodetic_crs");
    assert_eq!(found.auth_name, "EPSG");
    assert_eq!(found.code, "4326");

    let found = factory
        .official_name_from_alias(
            "GCS_WGS_1984",
            Some("geodetic_crs"),
            Some("ESRI"),
            NameSpelling::Exact,
        )
        .expect("query")
        .expect("WGS 84");
    assert_eq!(found.name, "WGS 84");
    assert_eq!(found.auth_name, "EPSG");
    assert_eq!(found.code, "4326");

    assert!(factory
        .official_name_from_alias("no match", None, None, NameSpelling::Exact)
        .expect("query")
        .is_none());

    let found = factory
        .official_name_from_alias(
            "System_Jednotne_Trigonometricke_Site_Katastralni_Ferro",
            Some("geodetic_datum"),
            None,
            NameSpelling::Equivalent,
        )
        .expect("query")
        .expect("Ferro");
    assert_eq!(
        found.name,
        "System of the Unified Trigonometrical Cadastral Network (Ferro)"
    );
}
