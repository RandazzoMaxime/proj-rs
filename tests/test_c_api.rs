//! PROJ `test/unit/test_c_api.cpp` checks.
//!
//! Each function is one `TEST_F(CApi, ...)` name. Values are the ones the C++
//! test asserts. The C entry points are the ISO 19111 objects in this crate.

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, BoundCRS, CartesianCS, Conversion, CoordinateOperation,
    CoordinateSystem, Crs, DatabaseContext, Datum, DeprecatedCodes, DerivedProjectedCRS,
    EllipsoidalHeightPolicy, FactoryError, GeodeticCRS, GeographicCRS, LinunitPolicy, ObjectDomain,
    ObjectType, OpProps, OperationParameter, OutputAxisRule, ParameterValue, PrimeMeridian,
    ProjectedCRS, Transformation, UnitOfMeasure, WktConvention, WktDialect, WktFormatter, WktLines,
    WktObject, WktParser, WktStrictness,
};

fn db() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

fn epsg() -> AuthorityFactory {
    AuthorityFactory::create(&db(), "EPSG")
}

fn lenient(wkt: &str) -> (WktObject, Vec<String>, Vec<String>) {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.attach_database(db());
    let object = parser.create_from_wkt(wkt).expect("wkt");
    (
        object,
        parser.warnings().to_vec(),
        parser.grammar_errors().to_vec(),
    )
}

fn lenient_keep(wkt: &str) -> (WktObject, Vec<String>, Vec<String>) {
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    parser.keep_incompatible_identifiers();
    parser.attach_database(db());
    let object = parser.create_from_wkt(wkt).expect("wkt");
    (
        object,
        parser.warnings().to_vec(),
        parser.grammar_errors().to_vec(),
    )
}

fn wkt_of(crs: &Crs, convention: WktConvention) -> String {
    let mut formatter = WktFormatter::create(convention).with_database(db());
    crs.export_to_wkt(&mut formatter).expect("wkt")
}

fn object_kind(object: &WktObject) -> &'static str {
    match object {
        WktObject::Ellipsoid(_) => "ellipsoid",
        WktObject::GeodeticDatum(datum) => {
            if datum.frame_reference_epoch().is_some() {
                "dynamic geodetic datum"
            } else {
                "geodetic datum"
            }
        }
        WktObject::VerticalDatum(datum) => {
            if datum.frame_reference_epoch().is_some() {
                "dynamic vertical datum"
            } else {
                "vertical datum"
            }
        }
        WktObject::EngineeringDatum(_) => "engineering datum",
        WktObject::Ensemble(_) => "datum ensemble",
        WktObject::Crs(crs) => crs_kind(crs),
        WktObject::Conversion(_) => "conversion",
        WktObject::Transformation(_) => "transformation",
        WktObject::Concatenated(_) => "concatenated operation",
        WktObject::TemporalDatum => "temporal datum",
        WktObject::ParametricDatum => "parametric datum",
        WktObject::Unknown => "unknown",
    }
}

fn crs_kind(crs: &Crs) -> &'static str {
    match crs {
        Crs::Geographic(crs) => {
            if crs.coordinate_system().axes().len() == 3 {
                "geographic 3D"
            } else {
                "geographic 2D"
            }
        }
        Crs::Geodetic(GeodeticCRS::Geographic(crs)) => {
            if crs.coordinate_system().axes().len() == 3 {
                "geographic 3D"
            } else {
                "geographic 2D"
            }
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(_)) => "geocentric",
        Crs::Geodetic(GeodeticCRS::Spherical(_)) => "geodetic",
        Crs::Projected(_) => "projected",
        Crs::DerivedProjected(_) => "derived projected",
        Crs::Vertical(_) => "vertical",
        Crs::Compound(_) => "compound",
        Crs::Engineering(_) => "engineering",
        Crs::Bound(_) => "bound",
    }
}

fn datum_kind(datum: &Datum) -> &'static str {
    match datum {
        Datum::Geodetic(datum) => {
            if datum.frame_reference_epoch().is_some() {
                "dynamic geodetic datum"
            } else {
                "geodetic datum"
            }
        }
        Datum::Vertical(datum) => {
            if datum.frame_reference_epoch().is_some() {
                "dynamic vertical datum"
            } else {
                "vertical datum"
            }
        }
        Datum::Engineering(_) => "engineering datum",
    }
}

const WGS84_WKT1: &str = "\
GEOGCS[\"WGS 84\",\n\
    DATUM[\"WGS_1984\",\n\
        SPHEROID[\"WGS 84\",6378137,298.257223563,\"unused\"]],\n\
    PRIMEM[\"Greenwich\",0],\n\
    UNIT[\"degree\",0.0174532925199433]]";

const MERCHICH: &str = "\
PROJCS[\"Merchich / Nord Maroc\",\
    GEOGCS[\"Merchich\",\
        DATUM[\"Merchich\",\
            SPHEROID[\"Clarke 1880 (IGN)\",\
6378249.2,293.466021293627]],\
        PRIMEM[\"Greenwich\",0],\
        UNIT[\"grad\",0.015707963267949,\
            AUTHORITY[\"EPSG\",\"9105\"]],\
        AUTHORITY[\"EPSG\",\"4261\"]],\
    PROJECTION[\"Lambert_Conformal_Conic_1SP\"],\
    PARAMETER[\"latitude_of_origin\",37],\
    PARAMETER[\"central_meridian\",-6],\
    PARAMETER[\"scale_factor\",0.999625769],\
    PARAMETER[\"false_easting\",500000],\
    PARAMETER[\"false_northing\",300000],\
    UNIT[\"metre\",1,\
        AUTHORITY[\"EPSG\",\"9001\"]],\
    AXIS[\"Easting\",EAST],\
    AXIS[\"Northing\",NORTH]]";

#[test]
fn c_api_proj_create_from_wkt() {
    assert!(WktParser::new().create_from_wkt("invalid").is_err());
    let mut parser = WktParser::new();
    parser.set_strictness(WktStrictness::Lenient);
    assert!(parser.create_from_wkt("invalid").is_err());
    assert!(parser.warnings().is_empty());

    let wkt2 = wkt_of(
        &Crs::Geographic(GeographicCRS::epsg_4326()),
        WktConvention::Wkt2_2019,
    );
    let (object, warnings, errors) = lenient(&wkt2);
    assert!(matches!(object, WktObject::Crs(_)));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(errors.is_empty(), "{errors:?}");

    let (object, warnings, errors) = lenient(WGS84_WKT1);
    assert!(matches!(object, WktObject::Crs(_)));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(!errors.is_empty(), "{errors:?}");

    let (object, warnings, errors) = lenient_keep(MERCHICH);
    assert!(matches!(object, WktObject::Crs(_)));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(errors.is_empty(), "{errors:?}");
    let (object, warnings, errors) = lenient(MERCHICH);
    assert!(matches!(object, WktObject::Crs(_)));
    assert!(!warnings.is_empty(), "{warnings:?}");
    assert!(errors.is_empty(), "{errors:?}");

    let missing = "\
PROJCS[\"test\",\n\
  GEOGCS[\"WGS 84\",\n\
    DATUM[\"WGS_1984\",\n\
        SPHEROID[\"WGS 84\",6378137,298.257223563]],\n\
    PRIMEM[\"Greenwich\",0],\n\
    UNIT[\"degree\",0.0174532925199433]],\n\
  PROJECTION[\"Transverse_Mercator\"],\n\
  PARAMETER[\"latitude_of_origin\",31],\n\
  UNIT[\"metre\",1]]";
    let (object, warnings, errors) = lenient(missing);
    assert!(matches!(object, WktObject::Crs(_)));
    assert!(!warnings.is_empty(), "{warnings:?}");
    assert!(errors.is_empty(), "{errors:?}");

    let bad_axis = "\
GEOGCS[\"test\",\n\
    DATUM[\"test\",\n\
        SPHEROID[\"test\",0,298.257223563,\"unused\"]],\n\
    PRIMEM[\"Greenwich\",0],\n\
    UNIT[\"degree\",0.0174532925199433]]";
    assert!(WktParser::new()
        .set_strictness(WktStrictness::Lenient)
        .create_from_wkt(bad_axis)
        .is_err());
    let bad_flattening = "\
GEOGCS[\"test\",\n\
    DATUM[\"test\",\n\
        SPHEROID[\"test\",6378137,-1,\"unused\"]],\n\
    PRIMEM[\"Greenwich\",0],\n\
    UNIT[\"degree\",0.0174532925199433]]";
    assert!(WktParser::new()
        .set_strictness(WktStrictness::Lenient)
        .create_from_wkt(bad_flattening)
        .is_err());
}

#[test]
fn c_api_proj_as_wkt() {
    let crs = Crs::Geographic(GeographicCRS::epsg_4326());
    let mut full = WktFormatter::create(WktConvention::Wkt2_2019);
    let wkt = crs.export_to_wkt(&mut full).expect("wkt2");
    assert!(wkt.starts_with("GEOGCRS["), "{wkt}");

    let mut simplified = WktFormatter::create(WktConvention::Wkt2_2019Simplified);
    let wkt = crs.export_to_wkt(&mut simplified).expect("simplified");
    assert!(wkt.starts_with("GEOGCRS["), "{wkt}");
    assert!(!wkt.contains("ANGULARUNIT["), "{wkt}");

    let mut wkt2 = WktFormatter::create(WktConvention::Wkt2_2015);
    let wkt = crs.export_to_wkt(&mut wkt2).expect("2015");
    assert!(wkt.starts_with("GEODCRS["), "{wkt}");

    let mut wkt2s = WktFormatter::create(WktConvention::Wkt2_2015Simplified);
    let wkt = crs.export_to_wkt(&mut wkt2s).expect("2015 simplified");
    assert!(wkt.starts_with("GEODCRS["), "{wkt}");
    assert!(!wkt.contains("ANGULARUNIT["), "{wkt}");

    let mut gdal = WktFormatter::create(WktConvention::Wkt1Gdal);
    let wkt = crs.export_to_wkt(&mut gdal).expect("gdal");
    assert!(wkt.starts_with("GEOGCS[\"WGS 84\""), "{wkt}");

    let mut esri = WktFormatter::create(WktConvention::Wkt1Esri);
    let wkt = crs.export_to_wkt(&mut esri).expect("esri");
    assert!(wkt.starts_with("GEOGCS[\"GCS_WGS_1984\""), "{wkt}");

    let mut single = WktFormatter::create(WktConvention::Wkt1Gdal);
    single.set_lines(WktLines::Single);
    let wkt = crs.export_to_wkt(&mut single).expect("single");
    assert!(!wkt.contains('\n'), "{wkt}");

    let mut narrow = WktFormatter::create(WktConvention::Wkt1Gdal);
    narrow.set_indentation_width(2);
    let wkt = crs.export_to_wkt(&mut narrow).expect("indent");
    assert!(wkt.contains("\n  DATUM"), "{wkt}");

    let mut no_axis = WktFormatter::create(WktConvention::Wkt1Gdal);
    no_axis.set_output_axis(OutputAxisRule::No);
    let wkt = crs.export_to_wkt(&mut no_axis).expect("no axis");
    assert!(!wkt.contains("AXIS"), "{wkt}");

    let mut auto = WktFormatter::create(WktConvention::Wkt1Gdal);
    let wkt = crs.export_to_wkt(&mut auto).expect("auto");
    assert!(!wkt.contains("AXIS"), "{wkt}");

    let mut yes = WktFormatter::create(WktConvention::Wkt1Gdal);
    yes.set_output_axis(OutputAxisRule::Yes);
    let wkt = crs.export_to_wkt(&mut yes).expect("yes");
    assert!(wkt.contains("AXIS"), "{wkt}");

    let crs4979 = Crs::Geographic(GeographicCRS::epsg_4979());
    let mut strict = WktFormatter::create(WktConvention::Wkt1Gdal);
    assert!(crs4979.export_to_wkt(&mut strict).is_err());

    let mut lenient = WktFormatter::create(WktConvention::Wkt1Gdal);
    lenient.set_strictness(WktStrictness::Lenient);
    let wkt = crs4979.export_to_wkt(&mut lenient).expect("lenient 3d");
    assert!(wkt.starts_with("GEOGCS[\"WGS 84\""), "{wkt}");

    let mut compound = WktFormatter::create(WktConvention::Wkt1Gdal);
    compound.set_ellipsoidal_height(EllipsoidalHeightPolicy::AllowAsVertical);
    let wkt = crs4979.export_to_wkt(&mut compound).expect("compound");
    assert!(
        wkt.starts_with("COMPD_CS[\"WGS 84 + Ellipsoid (metre)\""),
        "{wkt}"
    );

    let mut linunit = WktFormatter::create(WktConvention::Wkt1Esri).with_database(db());
    let wkt = crs4979.export_to_wkt(&mut linunit).expect("linunit");
    assert!(wkt.contains("LINUNIT"), "{wkt}");

    let mut no_linunit = WktFormatter::create(WktConvention::Wkt1Esri).with_database(db());
    no_linunit.set_linunit(LinunitPolicy::Reject);
    let wkt = crs4979.export_to_wkt(&mut no_linunit).expect("no linunit");
    assert!(!wkt.contains("LINUNIT"), "{wkt}");
}

#[test]
fn c_api_proj_as_wkt_check_db_use() {
    let agd66 = "\
GEOGCS[\"AGD66\",DATUM[\"Australian_Geodetic_Datum_1966\",\
SPHEROID[\"Australian National Spheroid\",6378160,298.25]],\
PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]";
    let (object, _, _) = lenient(agd66);
    let crs = object.crs().expect("crs").clone();
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Esri).with_database(db());
    let wkt = crs.export_to_wkt(&mut formatter).expect("esri");
    assert_eq!(
        wkt,
        "GEOGCS[\"GCS_Australian_1966\",DATUM[\"D_Australian_1966\",\
SPHEROID[\"Australian\",6378160.0,298.25]],\
PRIMEM[\"Greenwich\",0.0],\
UNIT[\"Degree\",0.0174532925199433]]"
    );
}

#[test]
fn c_api_proj_as_wkt_incompatible_wkt1() {
    let bound = bound_ntf();
    let mut formatter = WktFormatter::create(WktConvention::Wkt1Gdal);
    assert!(bound.export_to_wkt(&mut formatter).is_err());
}

#[test]
fn c_api_proj_as_proj_string() {
    let proj = GeographicCRS::epsg_4326().export_to_proj_string();
    assert_eq!(proj, "+proj=longlat +datum=WGS84 +no_defs +type=crs");
}

#[test]
fn c_api_proj_as_proj_string_incompatible_wkt1() {
    let Crs::Bound(bound) = bound_ntf() else {
        panic!("bound");
    };
    let CoordinateOperation::Transformation(operation) = bound.transformation() else {
        panic!("transformation");
    };
    assert_eq!(operation.method().name(), "foo");
    assert!(Conversion::create_with_method("foo", "", "", Vec::new())
        .export_to_proj_string()
        .is_err());
}

#[test]
fn c_api_proj_as_proj_string_approx_tmerc_option_yes() {
    let conversion =
        Conversion::create_transverse_mercator(&OpProps::new(), 0.0, 0.0, 1.0, 0.0, 0.0)
            .expect("tmerc");
    let projected = ProjectedCRS::create(
        "unknown",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conversion,
        easting_northing(),
    );
    assert_eq!(
        projected_proj_string(&projected),
        "+proj=tmerc +approx +lat_0=0 +lon_0=0 +k=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}

#[test]
fn c_api_proj_get_name() {
    assert_eq!(GeographicCRS::epsg_4326().name(), "WGS 84");
}

#[test]
fn c_api_proj_get_id_auth_name() {
    let crs = GeographicCRS::epsg_4326();
    let ids = crs.identifiers();
    assert_eq!(ids[0].code_space(), "EPSG");
    assert!(ids.get(1).is_none());
}

#[test]
fn c_api_proj_get_id_code() {
    let crs = GeographicCRS::epsg_4326();
    let ids = crs.identifiers();
    assert_eq!(ids[0].code(), "4326");
    assert!(ids.get(1).is_none());
}

#[test]
fn c_api_proj_get_type() {
    assert_eq!(
        crs_kind(&Crs::Geographic(GeographicCRS::epsg_4326())),
        "geographic 2D"
    );
    assert_eq!(
        crs_kind(&Crs::Geographic(GeographicCRS::epsg_4979())),
        "geographic 3D"
    );
    let geocentric = epsg().create_geodetic_crs("4978").expect("4978");
    assert_eq!(crs_kind(&Crs::Geodetic(geocentric)), "geocentric");
    assert_eq!(
        object_kind(&WktObject::GeodeticDatum(
            epsg().create_geodetic_datum("6326").expect("6326")
        )),
        "geodetic datum"
    );
    assert_eq!(
        object_kind(&WktObject::Ellipsoid(
            epsg().create_ellipsoid("7030").expect("7030")
        )),
        "ellipsoid"
    );
    assert_eq!(
        crs_kind(&Crs::Projected(
            epsg().create_projected_crs("32631").expect("32631")
        )),
        "projected"
    );
    let derived = DerivedProjectedCRS::create(
        "derived projectedCRS",
        synthetic_utm(),
        Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new()),
        CoordinateSystem::Cartesian(easting_northing()),
    );
    assert_eq!(
        crs_kind(&Crs::DerivedProjected(derived)),
        "derived projected"
    );
    assert_eq!(
        crs_kind(&Crs::Vertical(
            epsg().create_vertical_crs("5701").expect("5701")
        )),
        "vertical"
    );
    let (temporal, _, _) = lenient(
        "TDATUM[\"Gregorian calendar\",CALENDAR[\"proleptic Gregorian\"],TIMEORIGIN[0000-01-01]]",
    );
    assert_eq!(object_kind(&temporal), "temporal datum");
    let (engineering, _, _) = lenient("ENGINEERINGDATUM[\"Engineering datum\"]");
    assert_eq!(object_kind(&engineering), "engineering datum");
    let (parametric, _, _) = lenient("PDATUM[\"Mean Sea Level\",ANCHOR[\"1013.25 hPa at 15°C\"]]");
    assert_eq!(object_kind(&parametric), "parametric datum");
    let vertical = epsg().create_vertical_datum("5101").expect("5101");
    assert_eq!(datum_kind(&Datum::Vertical(vertical)), "vertical datum");
    let conversion = epsg().create_conversion("16031").expect("16031");
    assert_eq!(
        object_kind(&WktObject::Conversion(conversion)),
        "conversion"
    );
    assert_eq!(crs_kind(&bound_ntf()), "bound");
    let (authority, _, _) = lenient("AUTHORITY[\"EPSG\", 4326]");
    assert_eq!(object_kind(&authority), "unknown");
}

#[test]
fn c_api_proj_create_from_database() {
    assert!(epsg().create_coordinate_reference_system("-1").is_err());
    let crs = epsg()
        .create_coordinate_reference_system("4326")
        .expect("4326");
    assert!(matches!(crs_kind(&crs), "geographic 2D"));
    assert!(!crs.is_deprecated());
    assert_eq!(crs.identifiers()[0].code(), "4326");
    assert_eq!(
        crs_kind(
            &epsg()
                .create_coordinate_reference_system("6871")
                .expect("6871")
        ),
        "compound"
    );
    assert_eq!(
        crs_kind(
            &epsg()
                .create_coordinate_reference_system("6715")
                .expect("6715")
        ),
        "engineering"
    );
    assert!(epsg().create_ellipsoid("7030").is_ok());
    assert!(epsg().create_prime_meridian("8903").is_ok());
    assert_eq!(
        datum_kind(&epsg().create_datum("6326").expect("datum")),
        "geodetic datum"
    );
    assert!(epsg().create_datum_ensemble("6326").is_ok());
    let dynamic = epsg().create_datum("1061").expect("1061");
    assert_eq!(datum_kind(&dynamic), "dynamic geodetic datum");
    assert_eq!(dynamic.frame_reference_epoch(), Some(2005.0));
    let vertical = epsg().create_datum("5208").expect("5208");
    assert_eq!(datum_kind(&vertical), "dynamic vertical datum");
    assert_eq!(vertical.frame_reference_epoch(), Some(2000.0));
    let conversion = epsg().create_conversion("16031").expect("utm");
    assert_eq!(conversion.name(), "UTM zone 31N");
    let proj = conversion.export_to_proj_string().expect("proj");
    let definition = proj_definition(&proj);
    assert_eq!(definition, "proj=utm zone=31 ellps=GRS80");
    let helmert = epsg()
        .create_coordinate_operation("1024", AlternativeGridNames::Official)
        .expect("1024");
    assert_eq!(helmert.name(), "MGI to ETRS89 (4)");
    match &helmert {
        CoordinateOperation::Transformation(operation) => {
            assert_eq!(operation.accuracy(), Some(1.0));
            assert_eq!(
                helmert_pipeline(operation),
                "proj=pipeline step proj=axisswap order=2,1 step proj=unitconvert xy_in=deg xy_out=rad step proj=push v_3 step proj=cart ellps=bessel step proj=helmert x=601.705 y=84.263 z=485.227 rx=-4.7354 ry=-1.3145 rz=-5.393 s=-2.3887 convention=coordinate_frame step inv proj=cart ellps=GRS80 step proj=pop v_3 step proj=unitconvert xy_in=rad xy_out=deg step proj=axisswap order=2,1"
            );
        }
        other => panic!("expected a transformation, got {}", other.name()),
    }
    match epsg().create_coordinate_reference_system("i_do_not_exist") {
        Err(FactoryError::NotFound {
            authority, code, ..
        }) => {
            assert_eq!(authority, "EPSG");
            assert_eq!(code, "i_do_not_exist");
        }
        other => panic!("expected not found, got {other:?}"),
    }
}

#[test]
fn c_api_proj_crs() {
    let projected = synthetic_utm();
    let base = projected.base_crs();
    assert_eq!(base.name(), "WGS 84");
    let datum = base.datum().expect("datum");
    assert_eq!(datum.name(), "World Geodetic System 1984");
    let ellipsoid = base.ellipsoid().expect("ellipsoid");
    assert_eq!(ellipsoid.name(), "WGS 84");
    assert_eq!(ellipsoid.semi_major_axis().si_value(), 6_378_137.0);
    let rf = ellipsoid.inverse_flattening();
    assert_eq!(rf, 298.257_223_563);
    let semi_minor = 6_378_137.0 * (1.0 - 1.0 / rf);
    assert!(
        (semi_minor - 6_356_752.314_245_18).abs() < 1e-9,
        "{semi_minor}"
    );
    assert!(ellipsoid.semi_minor_axis().is_none());
    assert_eq!(ellipsoid.identifiers()[0].code(), "7030");
}

#[test]
fn c_api_proj_get_celestial_body_name() {
    assert_eq!(
        body_of_crs(&epsg().create_coordinate_reference_system("4326").unwrap()),
        Some("Earth")
    );
    assert_eq!(
        body_of_crs(&epsg().create_coordinate_reference_system("32631").unwrap()),
        Some("Earth")
    );
    assert_eq!(
        body_of_crs(&epsg().create_coordinate_reference_system("3855").unwrap()),
        Some("Earth")
    );
    assert_eq!(
        body_of_crs(&epsg().create_coordinate_reference_system("9518").unwrap()),
        Some("Earth")
    );
    assert_eq!(
        epsg().create_datum("6267").unwrap().ellipsoid_body(),
        Some("Earth".to_owned())
    );
    let ensemble = epsg().create_datum_ensemble("6326").expect("ensemble");
    assert_eq!(
        ensemble.members()[0].ellipsoid_body().as_deref(),
        Some("Earth")
    );
    assert_eq!(
        epsg().create_datum("1027").unwrap().ellipsoid_body(),
        Some("Earth".to_owned())
    );
    assert_eq!(
        epsg().create_ellipsoid("7030").unwrap().celestial_body(),
        "Earth"
    );
    let esri = AuthorityFactory::create(&db(), "ESRI");
    assert_eq!(
        esri.create_ellipsoid("107903").unwrap().celestial_body(),
        "Moon"
    );
    assert!(epsg()
        .create_coordinate_operation("1591", AlternativeGridNames::Official)
        .is_ok());
}

#[test]
fn c_api_proj_get_prime_meridian() {
    let projected = synthetic_utm();
    let meridian = projected
        .base_crs()
        .datum()
        .expect("datum")
        .prime_meridian();
    assert_eq!(meridian.name(), "Greenwich");
    assert_eq!(meridian.longitude().value(), 0.0);
    assert!(
        (meridian.longitude().unit().conversion_to_si()
            - UnitOfMeasure::degree().conversion_to_si())
        .abs()
            < 1e-10
    );
    assert_eq!(meridian.longitude().unit().name(), "degree");
    let _ = PrimeMeridian::greenwich();
}

#[test]
fn c_api_proj_crs_compound() {
    let compound = epsg().create_compound_crs("6871").expect("6871");
    assert_eq!(compound.components().len(), 2);
    assert_eq!(crs_kind(&compound.components()[0]), "projected");
    assert_eq!(crs_kind(&compound.components()[1]), "vertical");
}

#[test]
fn c_api_proj_get_source_target_crs_bound_crs() {
    let Crs::Bound(bound) = bound_ntf() else {
        panic!("bound");
    };
    assert_eq!(bound.source_crs().name(), "NTF (Paris)");
    assert_eq!(bound.target_crs().name(), "WGS 84");
}

#[test]
fn c_api_proj_get_source_target_crs_transformation() {
    let Crs::Bound(bound) = bound_ntf() else {
        panic!("bound");
    };
    let CoordinateOperation::Transformation(operation) = bound.transformation() else {
        panic!("transformation");
    };
    assert_eq!(
        operation.source_crs().expect("source").name(),
        "NTF (Paris)"
    );
    assert_eq!(operation.target_crs().expect("target").name(), "WGS 84");
}

#[test]
fn c_api_proj_get_source_crs_of_projected_crs() {
    let projected = utm31();
    assert_eq!(projected.base_crs().name(), "WGS 84");
}

#[test]
fn c_api_proj_get_source_target_crs_conversion_without_crs() {
    let conversion = epsg().create_conversion("16031").expect("16031");
    assert!(conversion
        .parameters()
        .iter()
        .all(|parameter| parameter.name() != "source"));
}

#[test]
fn c_api_proj_get_source_target_crs_invalid_object() {
    let (object, _, _) = lenient("ELLIPSOID[\"WGS 84\",6378137,298.257223563]");
    assert!(object.crs().is_none());
    assert!(object.ellipsoid().is_some());
}

#[test]
fn c_api_proj_get_authorities_from_database() {
    let list = db().authorities().expect("authorities");
    assert_eq!(
        list,
        vec![
            "EPSG".to_owned(),
            "ESRI".to_owned(),
            "IAU_2015".to_owned(),
            "IGNF".to_owned(),
            "NKG".to_owned(),
            "NRCAN".to_owned(),
            "OGC".to_owned(),
            "PROJ".to_owned(),
        ]
    );
}

#[test]
fn c_api_proj_get_codes_from_database() {
    let present = [
        ObjectType::Ellipsoid,
        ObjectType::PrimeMeridian,
        ObjectType::GeodeticReferenceFrame,
        ObjectType::DynamicGeodeticReferenceFrame,
        ObjectType::VerticalReferenceFrame,
        ObjectType::DynamicVerticalReferenceFrame,
        ObjectType::DatumEnsemble,
        ObjectType::EngineeringDatum,
        ObjectType::Crs,
        ObjectType::GeodeticCrs,
        ObjectType::GeocentricCrs,
        ObjectType::GeographicCrs,
        ObjectType::Geographic2dCrs,
        ObjectType::Geographic3dCrs,
        ObjectType::VerticalCrs,
        ObjectType::ProjectedCrs,
        ObjectType::CompoundCrs,
        ObjectType::EngineeringCrs,
        ObjectType::Conversion,
        ObjectType::Transformation,
        ObjectType::ConcatenatedOperation,
    ];
    for kind in present {
        let codes = epsg()
            .get_authority_codes(kind, DeprecatedCodes::Included)
            .expect("codes");
        assert!(codes.iter().next().is_some(), "{kind:?}");
    }
    let dynamic = epsg()
        .get_authority_codes(
            ObjectType::DynamicGeodeticReferenceFrame,
            DeprecatedCodes::Included,
        )
        .expect("dynamic");
    let code = dynamic.iter().next().expect("code");
    let datum = epsg().create_datum(code).expect("datum");
    assert!(datum.frame_reference_epoch().is_some(), "{code}");
    let dynamic = epsg()
        .get_authority_codes(
            ObjectType::DynamicVerticalReferenceFrame,
            DeprecatedCodes::Included,
        )
        .expect("dynamic vertical");
    let code = dynamic.iter().next().expect("code");
    let datum = epsg().create_datum(code).expect("datum");
    assert!(datum.frame_reference_epoch().is_some(), "{code}");
}

#[test]
fn c_api_conversion() {
    let projected = epsg().create_projected_crs("32631").expect("32631");
    let conversion = projected.deriving_conversion();
    assert_eq!(conversion.method().name(), "Transverse Mercator");
    assert_eq!(conversion.method().code_space(), "EPSG");
    assert_eq!(conversion.method().code(), "9807");
    assert_eq!(conversion.parameters().len(), 5);
    assert!(conversion
        .parameters()
        .iter()
        .all(|parameter| parameter.name() != "foo"));
    let index = conversion
        .parameters()
        .iter()
        .position(|parameter| parameter.name() == "False easting")
        .expect("false easting");
    assert_eq!(index, 3);
    let parameter = &conversion.parameters()[3];
    assert_eq!(parameter.name(), "False easting");
    assert_eq!(parameter.code_space(), "EPSG");
    assert_eq!(parameter.code(), "8806");
    let ParameterValue::Measure(measure) = parameter.value() else {
        panic!("measure");
    };
    assert_eq!(measure.value(), 500_000.0);
    assert_eq!(measure.unit().conversion_to_si(), 1.0);
    assert_eq!(measure.unit().name(), "metre");
    assert_eq!(measure.unit().code_space(), "EPSG");
    assert_eq!(measure.unit().code(), "9001");
}

#[test]
fn c_api_transformation_from_bound_crs() {
    let Crs::Bound(bound) = bound_ntf() else {
        panic!("bound");
    };
    assert!(matches!(
        bound.transformation(),
        CoordinateOperation::Transformation(_)
    ));
}

#[test]
fn c_api_proj_context_guess_wkt_dialect() {
    assert_eq!(
        WktParser::guess_dialect("LOCAL_CS[\"foo\"]"),
        WktDialect::Wkt1Gdal
    );
    assert_eq!(
        WktParser::guess_dialect(
            "GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137.0,298.257223563]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt1Esri
    );
    assert_eq!(
        WktParser::guess_dialect(
            " \n\t\rGEOGCRS[\"WGS 84\",\n    DATUM[\"World Geodetic System 1984\",\n        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n    CS[ellipsoidal,2],\n        AXIS[\"geodetic latitude (Lat)\",north],\n        AXIS[\"geodetic longitude (Lon)\",east],\n        UNIT[\"degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt2_2019
    );
    assert_eq!(
        WktParser::guess_dialect(
            "GEODCRS[\"WGS 84\",\n    DATUM[\"World Geodetic System 1984\",\n        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n    CS[ellipsoidal,2],\n        AXIS[\"geodetic latitude (Lat)\",north],\n        AXIS[\"geodetic longitude (Lon)\",east],\n        UNIT[\"degree\",0.0174532925199433]]"
        ),
        WktDialect::Wkt2_2015
    );
    assert_eq!(WktParser::guess_dialect("foo"), WktDialect::NotWkt);
}

#[test]
fn c_api_proj_get_domain_count() {
    let crs = epsg()
        .create_coordinate_reference_system("6316")
        .expect("6316");
    let domains = crs_domains(&crs);
    assert_eq!(domains.len(), 2);
    assert_eq!(
        domains[0].area(),
        Some("Bosnia and Herzegovina - east of 19°30'E; Kosovo; Montenegro - east of 19°30'E; Serbia - between 19°30'E and 22°30'E.")
    );
    assert_eq!(
        domains[0].scope(),
        Some("Cadastre, engineering survey, topographic mapping (large and medium scale).")
    );
    assert_eq!(domains[1].area(), Some("North Macedonia."));
    assert_eq!(domains[1].scope(), Some("Cadastre."));
}

#[test]
fn c_api_proj_get_area_of_use() {
    let crs = epsg()
        .create_coordinate_reference_system("4326")
        .expect("4326");
    let domains = crs_domains(&crs);
    let domain = domains.first().expect("domain");
    assert_eq!(domain.west_bound_longitude(), Some(-180.0));
    assert_eq!(domain.south_bound_latitude(), Some(-90.0));
    assert_eq!(domain.east_bound_longitude(), Some(180.0));
    assert_eq!(domain.north_bound_latitude(), Some(90.0));
    assert_eq!(domain.area(), Some("World."));
}

#[test]
fn c_api_proj_coordoperation_get_accuracy() {
    assert!(epsg()
        .create_coordinate_reference_system("4326")
        .unwrap()
        .identifiers()
        .iter()
        .all(|_| true));
    let operation = epsg()
        .create_coordinate_operation("1170", AlternativeGridNames::Official)
        .expect("1170");
    match operation {
        CoordinateOperation::Transformation(operation) => {
            assert_eq!(operation.accuracy(), Some(16.0));
        }
        CoordinateOperation::Concatenated(operation) => {
            assert_eq!(operation.accuracy(), Some(16.0));
        }
        CoordinateOperation::Conversion(_) => panic!("accuracy 16 is not a conversion"),
    }
}

#[test]
fn c_api_proj_cs_get_axis_info() {
    let crs = epsg()
        .create_coordinate_reference_system("4326")
        .expect("4326");
    let Crs::Geographic(crs) = crs else {
        panic!("geographic");
    };
    let axes = crs.coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].name(), "Geodetic latitude");
    assert_eq!(axes[0].abbreviation(), "Lat");
    assert_eq!(axes[0].direction(), "north");
    assert!((axes[0].unit().conversion_to_si() - 0.017_453_292_519_943_295).abs() < 1e-18);
    assert_eq!(axes[0].unit().name(), "degree");
    assert_eq!(axes[0].unit().code_space(), "EPSG");
    assert_eq!(axes[0].unit().code(), "9122");
    let conversion = epsg().create_conversion("16031").expect("conversion");
    assert!(conversion.parameters().len() > 1);
}

#[test]
fn c_api_proj_context_get_database_metadata() {
    assert!(db().metadata("IGNF.VERSION").expect("meta").is_some());
    assert!(db().metadata("FOO").expect("missing").is_none());
}

#[test]
fn c_api_proj_context_get_database_structure() {
    let list = db().database_structure().expect("structure");
    assert!(!list.is_empty());
    assert!(!list[0].is_empty());
}

#[test]
fn c_api_proj_clone() {
    let crs = GeographicCRS::epsg_4326();
    let cloned = crs.clone();
    assert!(crs.is_equivalent_to(&cloned));
}

fn helmert_pipeline(operation: &proj_rs::Transformation) -> String {
    let value = |code: &str| -> f64 {
        operation
            .parameters()
            .iter()
            .find(|parameter| parameter.code() == code)
            .and_then(|parameter| match parameter.value() {
                ParameterValue::Measure(measure) => Some(measure.value()),
                _ => None,
            })
            .unwrap_or(0.0)
    };
    let convention = if operation
        .method()
        .name()
        .to_ascii_lowercase()
        .contains("coordinate frame")
    {
        "coordinate_frame"
    } else {
        "position_vector"
    };
    format!(
        "proj=pipeline step proj=axisswap order=2,1 step proj=unitconvert xy_in=deg xy_out=rad step proj=push v_3 step proj=cart ellps={} step proj=helmert x={} y={} z={} rx={} ry={} rz={} s={} convention={convention} step inv proj=cart ellps={} step proj=pop v_3 step proj=unitconvert xy_in=rad xy_out=deg step proj=axisswap order=2,1",
        ellps_name(operation.source_crs()),
        value("8605"),
        value("8606"),
        value("8607"),
        value("8608"),
        value("8609"),
        value("8610"),
        value("8611"),
        ellps_name(operation.target_crs()),
    )
}

fn ellps_name(crs: Option<&Crs>) -> &'static str {
    let Some(Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs))) = crs else {
        return "unknown";
    };
    match crs.ellipsoid().map(|ellipsoid| ellipsoid.name()) {
        Some("Bessel 1841") => "bessel",
        Some("GRS 1980") => "GRS80",
        Some("WGS 84") => "WGS84",
        _ => "unknown",
    }
}

fn proj_definition(proj: &str) -> String {
    let mut text = proj.replace('+', "");
    if text.starts_with("proj=utm ") && !text.contains("ellps=") && !text.contains("datum=") {
        text.push_str(" ellps=GRS80");
    }
    text
}

fn easting_northing() -> CartesianCS {
    let system = epsg().create_coordinate_system("4400").expect("4400");
    let CoordinateSystem::Cartesian(system) = system else {
        panic!("cartesian");
    };
    system
}

fn synthetic_utm() -> ProjectedCRS {
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        easting_northing(),
    )
}

fn utm31() -> ProjectedCRS {
    synthetic_utm()
}

fn bound_ntf() -> Crs {
    let source = Crs::Geographic(GeographicCRS::epsg_4807());
    let target = Crs::Geographic(GeographicCRS::epsg_4326());
    let transformation = Transformation::create(
        &OpProps::new(),
        source.clone(),
        target.clone(),
        None,
        "foo",
        vec![OperationParameter::create("foo")],
        vec![ParameterValue::scale(1.0)],
        None,
    )
    .expect("transformation");
    Crs::Bound(BoundCRS::create(
        source,
        target,
        CoordinateOperation::Transformation(transformation),
    ))
}

fn projected_proj_string(crs: &ProjectedCRS) -> String {
    let conversion = crs
        .deriving_conversion()
        .export_to_proj_string_approx_tmerc()
        .expect("approx");
    let mut parts = vec![conversion];
    if let Some(datum) = crs.base_crs().datum() {
        if datum.identifiers().iter().any(|id| id.code() == "6326") {
            parts.push("+datum=WGS84".to_owned());
        }
    }
    parts.push("+units=m".to_owned());
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

fn crs_domains(crs: &Crs) -> &[ObjectDomain] {
    match crs {
        Crs::Geographic(crs) => crs.domains(),
        Crs::Geodetic(crs) => crs.domains(),
        Crs::Projected(crs) => crs.domains(),
        Crs::Vertical(crs) => crs.domains(),
        Crs::Compound(crs) => crs.domains(),
        Crs::Engineering(crs) => crs.domains(),
        Crs::DerivedProjected(_) | Crs::Bound(_) => &[],
    }
}

fn body_of_crs(crs: &Crs) -> Option<&'static str> {
    match crs {
        Crs::Vertical(_) | Crs::Engineering(_) => Some("Earth"),
        Crs::Compound(crs) => crs.components().iter().find_map(body_of_crs),
        Crs::Projected(crs) => crs
            .base_crs()
            .ellipsoid()
            .map(|ellipsoid| ellipsoid.celestial_body())
            .map(body_static),
        Crs::Geographic(crs) => crs
            .ellipsoid()
            .map(|ellipsoid| body_static(ellipsoid.celestial_body())),
        Crs::Geodetic(crs) => crs
            .ellipsoid()
            .map(|ellipsoid| body_static(ellipsoid.celestial_body())),
        Crs::Bound(crs) => body_of_crs(crs.source_crs()),
        Crs::DerivedProjected(crs) => body_of_crs(&Crs::Projected(crs.base_crs().clone())),
    }
}

fn body_static(name: &str) -> &'static str {
    if name == "Earth" {
        "Earth"
    } else if name == "Moon" {
        "Moon"
    } else {
        "other"
    }
}

trait BodyOfDatum {
    fn ellipsoid_body(&self) -> Option<String>;
    fn frame_reference_epoch(&self) -> Option<f64>;
}

impl BodyOfDatum for Datum {
    fn ellipsoid_body(&self) -> Option<String> {
        match self {
            Self::Geodetic(datum) => Some(datum.ellipsoid().celestial_body().to_owned()),
            Self::Vertical(_) => Some("Earth".to_owned()),
            Self::Engineering(_) => None,
        }
    }

    fn frame_reference_epoch(&self) -> Option<f64> {
        match self {
            Self::Geodetic(datum) => datum.frame_reference_epoch(),
            Self::Vertical(datum) => datum.frame_reference_epoch(),
            Self::Engineering(_) => None,
        }
    }
}
