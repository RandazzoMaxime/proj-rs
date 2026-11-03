//! PROJ `test/unit/test_io.cpp` and `test_operationfactory.cpp` checks on the rare CRS kinds
//! (`niche` class of `docs/UNIT-TRIAGE.md`): temporal, parametric, engineering, derived CRS, and
//! other celestial bodies.
//!
//! Each function is one `TEST()`; the name is the C++ suite and test name in snake case. Expected
//! text and numbers are copied from PROJ 9.9.0 `test/unit`.

use proj_rs::iso19111::{
    create_crs_from_proj_string, create_from_user_input, create_object_from_user_input,
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    FactoryError, JsonFormat, ProjLines, SpatialCriterion, UserObject,
};
use proj_rs::{
    CoordinateSystem, Crs, DatabaseContext, TemporalCsKind, UnitKind, WktConvention, WktFormatter,
    WktObject, WktParser,
};

fn parse(wkt: &str) -> WktObject {
    WktParser::new().create_from_wkt(wkt).expect("wkt")
}

fn parse_db(wkt: &str) -> WktObject {
    let mut parser = WktParser::new();
    parser.attach_database(DatabaseContext::create().expect("proj.db"));
    parser.create_from_wkt(wkt).expect("wkt")
}

fn refused(wkt: &str) {
    assert!(
        WktParser::new().create_from_wkt(wkt).is_err(),
        "should be refused: {wkt}"
    );
}

fn accepted(wkt: &str) {
    if let Err(error) = WktParser::new().create_from_wkt(wkt) {
        panic!("should be accepted: {error:?}\n{wkt}");
    }
}

fn crs_of(object: &WktObject) -> &Crs {
    object.crs().expect("a CRS")
}

fn export(crs: &Crs, convention: WktConvention) -> String {
    crs.export_to_wkt(&mut WktFormatter::create(convention))
        .expect("export")
}

fn temporal(object: &WktObject) -> &proj_rs::TemporalCRS {
    match crs_of(object) {
        Crs::Temporal(crs) => crs,
        other => panic!("not a temporal CRS: {}", other.name()),
    }
}

fn parametric(object: &WktObject) -> &proj_rs::ParametricCRS {
    match crs_of(object) {
        Crs::Parametric(crs) => crs,
        other => panic!("not a parametric CRS: {}", other.name()),
    }
}

fn engineering(object: &WktObject) -> &proj_rs::EngineeringCRS {
    match crs_of(object) {
        Crs::Engineering(crs) => crs,
        other => panic!("not an engineering CRS: {}", other.name()),
    }
}

fn derived(object: &WktObject) -> &proj_rs::DerivedCRS {
    match crs_of(object) {
        Crs::Derived(crs) => crs,
        other => panic!("not a derived CRS: {}", other.name()),
    }
}

#[test]
fn wkt_parse_compoundcrs_spatio_parametric_2015() {
    let object = parse(
        "COMPOUNDCRS[\"ICAO layer 0\",\n\
         \x20   GEODETICCRS[\"WGS 84\",\n\
         \x20       DATUM[\"World Geodetic System 1984\",\n\
         \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
         \x20               LENGTHUNIT[\"metre\",1]]],\n\
         \x20       PRIMEM[\"Greenwich\",0,\n\
         \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         \x20           ID[\"EPSG\",8901]],\n\
         \x20       CS[ellipsoidal,2],\n\
         \x20           AXIS[\"latitude\",north,\n\
         \x20               ORDER[1],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20           AXIS[\"longitude\",east,\n\
         \x20               ORDER[2],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
         \x20   PARAMETRICCRS[\"WMO standard atmosphere\",\n\
         \x20       PARAMETRICDATUM[\"Mean Sea Level\",\n\
         \x20           ANCHOR[\"Mean Sea Level = 1013.25 hPa\"]],\n\
         \x20       CS[parametric,1],\n\
         \x20           AXIS[\"pressure (P)\",unspecified,\n\
         \x20               PARAMETRICUNIT[\"HectoPascal\",100]]]]",
    );
    assert!(matches!(crs_of(&object), Crs::Compound(_)));
}

#[test]
fn wkt_parse_compoundcrs_spatio_parametric_2019() {
    let object = parse(
        "COMPOUNDCRS[\"ICAO layer 0\",\n\
         \x20   GEOGRAPHICCRS[\"WGS 84\",\n\
         \x20   DYNAMIC[FRAMEEPOCH[2005]],\n\
         \x20       DATUM[\"World Geodetic System 1984\",\n\
         \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
         \x20               LENGTHUNIT[\"metre\",1]]],\n\
         \x20       PRIMEM[\"Greenwich\",0,\n\
         \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
         \x20           ID[\"EPSG\",8901]],\n\
         \x20       CS[ellipsoidal,2],\n\
         \x20           AXIS[\"latitude\",north,\n\
         \x20               ORDER[1],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20           AXIS[\"longitude\",east,\n\
         \x20               ORDER[2],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
         \x20   PARAMETRICCRS[\"WMO standard atmosphere\",\n\
         \x20       PARAMETRICDATUM[\"Mean Sea Level\",\n\
         \x20           ANCHOR[\"Mean Sea Level = 1013.25 hPa\"]],\n\
         \x20       CS[parametric,1],\n\
         \x20           AXIS[\"pressure (P)\",unspecified,\n\
         \x20               PARAMETRICUNIT[\"HectoPascal\",100]]]]",
    );
    assert!(matches!(crs_of(&object), Crs::Compound(_)));
}

#[test]
fn wkt_parse_compoundcrs_spatio_temporal_2015() {
    let object = parse(
        "COMPOUNDCRS[\"GPS position and time\",\n\
         \x20   GEODCRS[\"WGS 84 (G1762)\",\n\
         \x20       DATUM[\"World Geodetic System 1984 (G1762)\",\n\
         \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
         \x20               LENGTHUNIT[\"metre\",1,\n\
         \x20                   ID[\"EPSG\",9001]]]],\n\
         \x20       CS[ellipsoidal,2],\n\
         \x20           AXIS[\"latitude\",north,\n\
         \x20               ORDER[1],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20           AXIS[\"longitude\",east,\n\
         \x20               ORDER[2],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
         \x20   TIMECRS[\"GPS Time\",\n\
         \x20       TIMEDATUM[\"Time origin\",TIMEORIGIN[1980-01-01]],\n\
         \x20       CS[temporal,1],\n\
         \x20           AXIS[\"time (T)\",future]]]",
    );
    assert!(matches!(crs_of(&object), Crs::Compound(_)));
}

#[test]
fn wkt_parse_compoundcrs_spatio_temporal_2019() {
    let object = parse(
        "COMPOUNDCRS[\"2D GPS position with civil time in ISO 8601 format\",\n\
         \x20   GEOGCRS[\"WGS 84 (G1762)\",\n\
         \x20       DATUM[\"World Geodetic System 1984 (G1762)\",\n\
         \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
         \x20               LENGTHUNIT[\"metre\",1,\n\
         \x20                   ID[\"EPSG\",9001]]]],\n\
         \x20       CS[ellipsoidal,2],\n\
         \x20           AXIS[\"latitude\",north,\n\
         \x20               ORDER[1],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]],\n\
         \x20           AXIS[\"longitude\",east,\n\
         \x20               ORDER[2],\n\
         \x20               ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
         \x20   TIMECRS[\"DateTime\",\n\
         \x20       TDATUM[\"Gregorian Calendar\"],\n\
         \x20       CS[TemporalDateTime,1],\n\
         \x20           AXIS[\"time (T)\",future]]]",
    );
    assert!(matches!(crs_of(&object), Crs::Compound(_)));
}

#[test]
fn wkt_parse_temporaldatum() {
    let object = parse(
        "TDATUM[\"Gregorian calendar\",\n\
         \x20   CALENDAR[\"my calendar\"],\n\
         \x20   TIMEORIGIN[0000-01-01]]",
    );
    let WktObject::TemporalDatum(datum) = object else {
        panic!("not a temporal datum");
    };
    assert_eq!(datum.name(), "Gregorian calendar");
    assert_eq!(datum.temporal_origin(), "0000-01-01");
    assert_eq!(datum.calendar(), "my calendar");
}

#[test]
fn wkt_parse_temporaldatum_no_calendar() {
    let object = parse(
        "TDATUM[\"Gregorian calendar\",\n\
         \x20   TIMEORIGIN[0000-01-01]]",
    );
    let WktObject::TemporalDatum(datum) = object else {
        panic!("not a temporal datum");
    };
    assert_eq!(datum.name(), "Gregorian calendar");
    assert_eq!(datum.temporal_origin(), "0000-01-01");
    assert_eq!(datum.calendar(), "proleptic Gregorian");
}

fn check_temporal(
    wkt: &str,
    name: &str,
    datum_name: &str,
    origin: &str,
    calendar: &str,
    kind: TemporalCsKind,
) -> WktObject {
    let object = parse(wkt);
    let crs = temporal(&object);
    assert_eq!(crs.name(), name);
    assert_eq!(crs.datum().name(), datum_name);
    assert_eq!(crs.datum().temporal_origin(), origin);
    assert_eq!(crs.datum().calendar(), calendar);
    assert_eq!(crs.coordinate_system().kind(), kind);
    object
}

#[test]
fn wkt_parse_datetimetemporalcrs_wkt2_2015() {
    let object = check_temporal(
        "TIMECRS[\"Temporal CRS\",\n\
         \x20   TDATUM[\"Gregorian calendar\",\n\
         \x20       TIMEORIGIN[0000-01-01]],\n\
         \x20   CS[temporal,1],\n\
         \x20       AXIS[\"time (T)\",future]]",
        "Temporal CRS",
        "Gregorian calendar",
        "0000-01-01",
        "proleptic Gregorian",
        TemporalCsKind::DateTime,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.unit().kind(), UnitKind::None);
    assert_eq!(axis.unit().name(), "");
}

#[test]
fn wkt_parse_datetimetemporalcrs_wkt2_2019() {
    let object = check_temporal(
        "TIMECRS[\"Temporal CRS\",\n\
         \x20   TDATUM[\"Gregorian calendar\",\n\
         \x20       CALENDAR[\"proleptic Gregorian\"],\n\
         \x20       TIMEORIGIN[0000-01-01]],\n\
         \x20   CS[TemporalDateTime,1],\n\
         \x20       AXIS[\"time (T)\",future]]",
        "Temporal CRS",
        "Gregorian calendar",
        "0000-01-01",
        "proleptic Gregorian",
        TemporalCsKind::DateTime,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.unit().kind(), UnitKind::None);
    assert_eq!(axis.unit().name(), "");
}

#[test]
fn wkt_parse_temporalcountcrswithconvfactor_wkt2_2019() {
    let object = check_temporal(
        "TIMECRS[\"GPS milliseconds\",\n\
         \x20   TDATUM[\"GPS time origin\",\n\
         \x20       TIMEORIGIN[1980-01-01T00:00:00.0Z]],\n\
         \x20   CS[TemporalCount,1],\n\
         \x20       AXIS[\"(T)\",future,\n\
         \x20           TIMEUNIT[\"milliseconds (ms)\",0.001]]]",
        "GPS milliseconds",
        "GPS time origin",
        "1980-01-01T00:00:00.0Z",
        "proleptic Gregorian",
        TemporalCsKind::Count,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.unit().name(), "milliseconds (ms)");
    assert_eq!(axis.unit().conversion_to_si(), 0.001);
}

#[test]
fn wkt_parse_temporalcountcrswithoutconvfactor_wkt2_2019() {
    let object = check_temporal(
        "TIMECRS[\"Calendar hours from 1979-12-29\",\n\
         \x20   TDATUM[\"29 December 1979\",\n\
         \x20       CALENDAR[\"proleptic Gregorian\"],\n\
         \x20       TIMEORIGIN[1979-12-29T00Z]],\n\
         \x20   CS[TemporalCount,1],\n\
         \x20       AXIS[\"time\",future,\n\
         \x20           TIMEUNIT[\"hour\"]]]",
        "Calendar hours from 1979-12-29",
        "29 December 1979",
        "1979-12-29T00Z",
        "proleptic Gregorian",
        TemporalCsKind::Count,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.unit().name(), "hour");
    assert_eq!(axis.unit().conversion_to_si(), 0.0);
}

#[test]
fn wkt_parse_temporalmeasurecrs_wkt2_2015() {
    let object = check_temporal(
        "TIMECRS[\"GPS Time\",\n\
         \x20   TDATUM[\"Time origin\",\n\
         \x20       TIMEORIGIN[1980-01-01T00:00:00.0Z]],\n\
         \x20   CS[temporal,1],\n\
         \x20   AXIS[\"time\",future],\n\
         \x20   TIMEUNIT[\"day\",86400.0]]",
        "GPS Time",
        "Time origin",
        "1980-01-01T00:00:00.0Z",
        "proleptic Gregorian",
        TemporalCsKind::Measure,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.name(), "Time");
    assert_eq!(axis.unit().name(), "day");
    assert_eq!(axis.unit().conversion_to_si(), 86400.0);
}

#[test]
fn wkt_parse_temporalmeasurecrswithoutconvfactor_wkt2_2019() {
    let object = check_temporal(
        "TIMECRS[\"Decimal Years CE\",\n\
         \x20   TIMEDATUM[\"Common Era\",\n\
         \x20       TIMEORIGIN[0000]],\n\
         \x20   CS[TemporalMeasure,1],\n\
         \x20       AXIS[\"decimal years (a)\",future,\n\
         \x20           TIMEUNIT[\"year\"]]]",
        "Decimal Years CE",
        "Common Era",
        "0000",
        "proleptic Gregorian",
        TemporalCsKind::Measure,
    );
    let axis = temporal(&object).coordinate_system().axis();
    assert_eq!(axis.name(), "Decimal years");
    assert_eq!(axis.unit().name(), "year");
    assert_eq!(axis.unit().conversion_to_si(), 0.0);
}

#[test]
fn wkt_parse_engineeringdatum() {
    let object = parse("ENGINEERINGDATUM[\"Engineering datum\"]");
    let WktObject::EngineeringDatum(datum) = object else {
        panic!("not an engineering datum");
    };
    assert_eq!(datum.name(), "Engineering datum");
    assert!(datum.anchor_definition().is_none());
}

#[test]
fn wkt_parse_engineeringcrs() {
    let object = parse(
        "ENGINEERINGCRS[\"Engineering CRS\",\n\
         \x20   ENGINEERINGDATUM[\"Engineering datum\"],\n\
         \x20   CS[Cartesian,2],\n\
         \x20       AXIS[\"(E)\",east,\n\
         \x20           ORDER[1],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]],\n\
         \x20       AXIS[\"(N)\",north,\n\
         \x20           ORDER[2],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]]",
    );
    let crs = engineering(&object);
    assert_eq!(crs.name(), "Engineering CRS");
    assert_eq!(crs.datum().name(), "Engineering datum");
    assert_eq!(crs.coordinate_system().axes().len(), 2);
}

#[test]
fn wkt_parse_local_cs_short() {
    let object = parse("LOCAL_CS[\"Engineering CRS\"]");
    let crs = engineering(&object);
    assert_eq!(crs.name(), "Engineering CRS");
    assert_eq!(crs.datum().name(), "Unknown engineering datum");
    assert_eq!(crs.coordinate_system().axes().len(), 2);
    assert_eq!(
        export(crs_of(&object), WktConvention::Wkt1Gdal),
        "LOCAL_CS[\"Engineering CRS\",\n\
         \x20   UNIT[\"metre\",1,\n\
         \x20       AUTHORITY[\"EPSG\",\"9001\"]],\n\
         \x20   AXIS[\"Easting\",EAST],\n\
         \x20   AXIS[\"Northing\",NORTH]]"
    );
}

fn check_local_cs(wkt: &str, axes: usize) {
    let object = parse(wkt);
    let crs = engineering(&object);
    assert_eq!(crs.name(), "Engineering CRS");
    assert_eq!(crs.datum().name(), "Engineering datum");
    assert_eq!(crs.coordinate_system().axes().len(), axes);
}

#[test]
fn wkt_parse_local_cs_long_one_axis() {
    check_local_cs(
        "LOCAL_CS[\"Engineering CRS\",\n\
         \x20   LOCAL_DATUM[\"Engineering datum\",12345],\n\
         \x20   UNIT[\"meter\",1],\n\
         \x20   AXIS[\"height\",up]]",
        1,
    );
}

#[test]
fn wkt_parse_local_cs_long_two_axis() {
    check_local_cs(
        "LOCAL_CS[\"Engineering CRS\",\n\
         \x20   LOCAL_DATUM[\"Engineering datum\",12345],\n\
         \x20   UNIT[\"meter\",1],\n\
         \x20   AXIS[\"Easting\",EAST],\n\
         \x20   AXIS[\"Northing\",NORTH]]",
        2,
    );
}

#[test]
fn wkt_parse_local_cs_long_three_axis() {
    check_local_cs(
        "LOCAL_CS[\"Engineering CRS\",\n\
         \x20   LOCAL_DATUM[\"Engineering datum\",12345],\n\
         \x20   UNIT[\"meter\",1],\n\
         \x20   AXIS[\"Easting\",EAST],\n\
         \x20   AXIS[\"Northing\",NORTH],\n\
         \x20   AXIS[\"Elevation\",UP]]",
        3,
    );
}

#[test]
fn wkt_parse_parametricdatum() {
    let object = parse(
        "PARAMETRICDATUM[\"Parametric datum\",\n\
         \x20   ANCHOR[\"my anchor\"]]",
    );
    let WktObject::ParametricDatum(datum) = object else {
        panic!("not a parametric datum");
    };
    assert_eq!(datum.name(), "Parametric datum");
    assert_eq!(datum.anchor_definition(), Some("my anchor"));
}

#[test]
fn wkt_parse_parametriccrs() {
    let object = parse(
        "PARAMETRICCRS[\"WMO standard atmosphere layer 0\",\
         PDATUM[\"Mean Sea Level\",ANCHOR[\"1013.25 hPa at 15°C\"]],\
         CS[parametric,1],\
         AXIS[\"pressure (hPa)\",up],\
         PARAMETRICUNIT[\"HectoPascal\",100.0]\
         ]",
    );
    let crs = parametric(&object);
    assert_eq!(crs.name(), "WMO standard atmosphere layer 0");
    assert_eq!(crs.datum().name(), "Mean Sea Level");
    let axis = crs.coordinate_system().axis();
    assert_eq!(axis.name(), "Pressure");
    assert_eq!(axis.unit().name(), "HectoPascal");
    assert_eq!(axis.unit().kind(), UnitKind::Parametric);
    assert_eq!(axis.unit().conversion_to_si(), 100.0);
}

#[test]
fn wkt_parse_parametriccrs_parametricdatum() {
    let object = parse(
        "PARAMETRICCRS[\"WMO standard atmosphere layer 0\",\
         PARAMETRICDATUM[\"Mean Sea Level\"],\
         CS[parametric,1],\
         AXIS[\"pressure (hPa)\",up],\
         PARAMETRICUNIT[\"HectoPascal\",100.0]\
         ]",
    );
    assert!(matches!(crs_of(&object), Crs::Parametric(_)));
}

const DERIVED_GEOGRAPHIC_BODY: &str = "    BASEGEODCRS[\"WGS 84\",\n\
     \x20       DATUM[\"World Geodetic System 1984\",\n\
     \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
     \x20               LENGTHUNIT[\"metre\",1]]],\n\
     \x20       PRIMEM[\"Greenwich\",0,\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
     \x20   DERIVINGCONVERSION[\"Atlantic pole\",\n\
     \x20       METHOD[\"Pole rotation\"],\n\
     \x20       PARAMETER[\"Latitude of rotated pole\",52,\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433,\n\
     \x20               ID[\"EPSG\",9122]]],\n\
     \x20       PARAMETER[\"Longitude of rotated pole\",-30,\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433,\n\
     \x20               ID[\"EPSG\",9122]]],\n\
     \x20       PARAMETER[\"Axis rotation\",-25,\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433,\n\
     \x20               ID[\"EPSG\",9122]]]],\n\
     \x20   CS[ellipsoidal,2],\n\
     \x20       AXIS[\"latitude\",north,\n\
     \x20           ORDER[1],\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433,\n\
     \x20               ID[\"EPSG\",9122]]],\n\
     \x20       AXIS[\"longitude\",east,\n\
     \x20           ORDER[2],\n\
     \x20           ANGLEUNIT[\"degree\",0.0174532925199433,\n\
     \x20               ID[\"EPSG\",9122]]]]";

fn check_derived_geographic(keyword: &str) {
    let wkt = format!("{keyword}[\"WMO Atlantic Pole\",\n{DERIVED_GEOGRAPHIC_BODY}");
    let object = parse(&wkt);
    let crs = derived(&object);
    assert_eq!(crs.name(), "WMO Atlantic Pole");
    assert_eq!(crs.base_crs().name(), "WGS 84");
    assert!(matches!(crs.base_crs(), Crs::Geographic(_)));
    assert_eq!(crs.deriving_conversion().name(), "Atlantic pole");
    assert!(matches!(
        crs.coordinate_system(),
        CoordinateSystem::Ellipsoidal(_)
    ));
}

#[test]
fn wkt_parse_derivedgeographiccrs_wkt2() {
    check_derived_geographic("GEODCRS");
}

#[test]
fn wkt_parse_derivedgeographiccrs_wkt2_2019() {
    check_derived_geographic("GEOGCRS");
}

#[test]
fn wkt_parse_derivedgeodeticcrs() {
    let object = parse(
        "GEODCRS[\"Derived geodetic CRS\",\n\
         \x20   BASEGEODCRS[\"WGS 84\",\n\
         \x20       DATUM[\"World Geodetic System 1984\",\n\
         \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
         \x20               LENGTHUNIT[\"metre\",1]]],\n\
         \x20       PRIMEM[\"Greenwich\",0,\n\
         \x20           ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
         \x20   DERIVINGCONVERSION[\"Some conversion\",\n\
         \x20       METHOD[\"Some method\"],\n\
         \x20       PARAMETER[\"foo\",1.0,UNIT[\"metre\",1]]],\n\
         \x20   CS[Cartesian,3],\n\
         \x20       AXIS[\"(X)\",geocentricX,\n\
         \x20           ORDER[1],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]],\n\
         \x20       AXIS[\"(Y)\",geocentricY,\n\
         \x20           ORDER[2],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]],\n\
         \x20       AXIS[\"(Z)\",geocentricZ,\n\
         \x20           ORDER[3],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]]",
    );
    let crs = derived(&object);
    assert_eq!(crs.name(), "Derived geodetic CRS");
    assert_eq!(crs.base_crs().name(), "WGS 84");
    assert!(matches!(crs.base_crs(), Crs::Geographic(_)));
    assert_eq!(crs.deriving_conversion().name(), "Some conversion");
    assert!(matches!(
        crs.coordinate_system(),
        CoordinateSystem::Cartesian(_)
    ));
}

#[test]
fn wkt_parse_derivedgeodeticcrs_where_base_is_geocentric() {
    let wkt = "GEODCRS[\"Local CRS derived from WGS-84\",\n\
               \x20   BASEGEODCRS[\"WGS 84\",\n\
               \x20       ENSEMBLE[\"World Geodetic System 1984 ensemble\",\n\
               \x20           MEMBER[\"World Geodetic System 1984 (Transit)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G730)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G873)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G1150)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G1674)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G1762)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G2139)\"],\n\
               \x20           MEMBER[\"World Geodetic System 1984 (G2296)\"],\n\
               \x20           ELLIPSOID[\"WGS 84\",6378137,298.257223563,\n\
               \x20               LENGTHUNIT[\"metre\",1]],\n\
               \x20           ENSEMBLEACCURACY[2.0]],\n\
               \x20       PRIMEM[\"Greenwich\",0,\n\
               \x20           ANGLEUNIT[\"degree\",0.0174532925199433]]],\n\
               \x20   DERIVINGCONVERSION[\"Local origin shift\",\n\
               \x20       METHOD[\"Position Vector transformation (geocentric domain)\",\n\
               \x20           ID[\"EPSG\",1033]],\n\
               \x20       PARAMETER[\"X-axis translation\",10,\n\
               \x20           LENGTHUNIT[\"metre\",1],\n\
               \x20           ID[\"EPSG\",8605]],\n\
               \x20       PARAMETER[\"Y-axis translation\",20,\n\
               \x20           LENGTHUNIT[\"metre\",1],\n\
               \x20           ID[\"EPSG\",8606]],\n\
               \x20       PARAMETER[\"Z-axis translation\",1,\n\
               \x20           LENGTHUNIT[\"metre\",1],\n\
               \x20           ID[\"EPSG\",8607]],\n\
               \x20       PARAMETER[\"X-axis rotation\",0,\n\
               \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
               \x20           ID[\"EPSG\",8608]],\n\
               \x20       PARAMETER[\"Y-axis rotation\",0,\n\
               \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
               \x20           ID[\"EPSG\",8609]],\n\
               \x20       PARAMETER[\"Z-axis rotation\",0,\n\
               \x20           ANGLEUNIT[\"degree\",0.0174532925199433],\n\
               \x20           ID[\"EPSG\",8610]]],\n\
               \x20   CS[Cartesian,3],\n\
               \x20       AXIS[\"(X)\",geocentricX,\n\
               \x20           ORDER[1],\n\
               \x20           LENGTHUNIT[\"metre\",1,\n\
               \x20               ID[\"EPSG\",9001]]],\n\
               \x20       AXIS[\"(Y)\",geocentricY,\n\
               \x20           ORDER[2],\n\
               \x20           LENGTHUNIT[\"metre\",1,\n\
               \x20               ID[\"EPSG\",9001]]],\n\
               \x20       AXIS[\"(Z)\",geocentricZ,\n\
               \x20           ORDER[3],\n\
               \x20           LENGTHUNIT[\"metre\",1,\n\
               \x20               ID[\"EPSG\",9001]]]]";
    let object = parse(wkt);
    let crs = derived(&object);
    assert!(matches!(
        crs.base_crs(),
        Crs::Geodetic(proj_rs::GeodeticCRS::Geocentric(_))
    ));
    assert_eq!(export(crs_of(&object), WktConvention::Wkt2_2019), wkt);
}

#[test]
fn wkt_parse_derivedverticalcrs() {
    let object = parse(
        "VERTCRS[\"Derived vertCRS\",\n\
         \x20   BASEVERTCRS[\"ODN height\",\n\
         \x20       VDATUM[\"Ordnance Datum Newlyn\"]],\n\
         \x20   DERIVINGCONVERSION[\"unknown\",\n\
         \x20       METHOD[\"PROJ unimplemented\"],\n\
         \x20       PARAMETER[\"foo\",1,\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]],\n\
         \x20   CS[vertical,1],\n\
         \x20       AXIS[\"gravity-related height (H)\",up,\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]]",
    );
    assert!(matches!(derived(&object).base_crs(), Crs::Vertical(_)));
}

#[test]
fn wkt_parse_derivedengineeringcrs() {
    let object = parse(
        "ENGCRS[\"Derived EngineeringCRS\",\n\
         \x20   BASEENGCRS[\"Engineering CRS\",\n\
         \x20       EDATUM[\"Engineering datum\"]],\n\
         \x20   DERIVINGCONVERSION[\"unknown\",\n\
         \x20       METHOD[\"PROJ unimplemented\"],\n\
         \x20       PARAMETER[\"foo\",1,\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]],\n\
         \x20   CS[Cartesian,2],\n\
         \x20       AXIS[\"(E)\",east,\n\
         \x20           ORDER[1],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]],\n\
         \x20       AXIS[\"(N)\",north,\n\
         \x20           ORDER[2],\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]]",
    );
    assert!(matches!(derived(&object).base_crs(), Crs::Engineering(_)));
}

#[test]
fn wkt_parse_derivedparametriccrs() {
    let object = parse(
        "PARAMETRICCRS[\"Derived ParametricCRS\",\n\
         \x20   BASEPARAMCRS[\"Parametric CRS\",\n\
         \x20       PDATUM[\"Parametric datum\"]],\n\
         \x20   DERIVINGCONVERSION[\"unknown\",\n\
         \x20       METHOD[\"PROJ unimplemented\"],\n\
         \x20       PARAMETER[\"foo\",1,\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]],\n\
         \x20   CS[parametric,1],\n\
         \x20       AXIS[\"pressure (hPa)\",up,\n\
         \x20           PARAMETRICUNIT[\"HectoPascal\",100]]]",
    );
    assert!(matches!(derived(&object).base_crs(), Crs::Parametric(_)));
}

#[test]
fn wkt_parse_derivedtemporalcrs() {
    let object = parse(
        "TIMECRS[\"Derived TemporalCRS\",\n\
         \x20   BASETIMECRS[\"Temporal CRS\",\n\
         \x20       TDATUM[\"Gregorian calendar\",\n\
         \x20           CALENDAR[\"proleptic Gregorian\"],\n\
         \x20           TIMEORIGIN[0000-01-01]]],\n\
         \x20   DERIVINGCONVERSION[\"unknown\",\n\
         \x20       METHOD[\"PROJ unimplemented\"],\n\
         \x20       PARAMETER[\"foo\",1,\n\
         \x20           LENGTHUNIT[\"metre\",1,\n\
         \x20               ID[\"EPSG\",9001]]]],\n\
         \x20   CS[TemporalDateTime,1],\n\
         \x20       AXIS[\"time (T)\",future]]",
    );
    assert!(matches!(derived(&object).base_crs(), Crs::Temporal(_)));
}

#[test]
fn wkt_parse_derivedprojectedcrs() {
    let object = parse(
        r#"DERIVEDPROJCRS["derived projectedCRS",
    BASEPROJCRS["WGS 84 / UTM zone 31N",
        BASEGEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["UTM zone 31N",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",3,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.9996,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",500000,
                LENGTHUNIT["metre",1],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["metre",1],
                ID["EPSG",8807]]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,2],
        AXIS["(E)",east,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["(N)",north,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#,
    );
    let Crs::DerivedProjected(crs) = crs_of(&object) else {
        panic!("not a derived projected CRS");
    };
    assert_eq!(crs.name(), "derived projectedCRS");
    assert_eq!(crs.base_crs().name(), "WGS 84 / UTM zone 31N");
    assert_eq!(crs.deriving_conversion().name(), "unknown");
    assert!(matches!(
        crs.coordinate_system(),
        CoordinateSystem::Cartesian(_)
    ));
}

#[test]
fn wkt_parse_derivedprojectedcrs_ordinal() {
    let wkt = r#"DERIVEDPROJCRS["derived projectedCRS",
    BASEPROJCRS["BASEPROJCRS",
        BASEGEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8901]]],
        CONVERSION["unknown",
            METHOD["PROJ unimplemented"],
            PARAMETER["foo",1,
                LENGTHUNIT["metre",1,
                    ID["EPSG",9001]]]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1,
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]],
    CS[ordinal,2],
        AXIS["inline (I)",northNorthWest,
            ORDER[1]],
        AXIS["crossline (J)",westSouthWest,
            ORDER[2]]]"#;
    let object = parse(wkt);
    let Crs::DerivedProjected(crs) = crs_of(&object) else {
        panic!("not a derived projected CRS");
    };
    assert!(matches!(
        crs.coordinate_system(),
        CoordinateSystem::Ordinal(_)
    ));
    assert_eq!(export(crs_of(&object), WktConvention::Wkt2_2019), wkt);
}

fn check_us_foot_base(object: &WktObject) {
    let Crs::DerivedProjected(crs) = crs_of(object) else {
        panic!("not a derived projected CRS");
    };
    let axes = crs.base_crs().coordinate_system().axes();
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].unit().name(), "US survey foot");
    assert_eq!(axes[0].unit().conversion_to_si(), 12.0 / 39.37);
}

#[test]
fn wkt_parse_derivedprojectedcrs_base_crs_cs_non_metre_from_conversion() {
    let wkt = r#"DERIVEDPROJCRS["Ground for NAD83(2011) / Idaho West (ftUS)",
    BASEPROJCRS["foo",
        BASEGEOGCRS["NAD83(2011)",
            DATUM["NAD83 (National Spatial Reference System 2011)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["SPCS83 Idaho West zone (US Survey feet)",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",41.6666666666667,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-115.75,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.999933333,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",2624666.667,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8807]]]],
    DERIVINGCONVERSION["Grid to ground",
        METHOD["Similarity transformation",
            ID["EPSG",9621]],
        PARAMETER["Ordinate 1 of evaluation point in target CRS",1000,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8621]],
        PARAMETER["Ordinate 2 of evaluation point in target CRS",0,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8622]],
        PARAMETER["Scale factor for source CRS axes",1,
            SCALEUNIT["unity",1],
            ID["EPSG",1061]],
        PARAMETER["Rotation angle of source CRS axes",0,
            ANGLEUNIT["degree",0],
            ID["EPSG",8614]]],
    CS[Cartesian,2],
        AXIS["easting (X)",east,
            ORDER[1],
            LENGTHUNIT["US survey foot",0.304800609601219]],
        AXIS["northing (Y)",north,
            ORDER[2],
            LENGTHUNIT["US survey foot",0.304800609601219]]]"#;
    let object = parse(wkt);
    check_us_foot_base(&object);
    let expected = r#"DERIVEDPROJCRS["Ground for NAD83(2011) / Idaho West (ftUS)",
    BASEPROJCRS["foo",
        BASEGEOGCRS["NAD83(2011)",
            DATUM["NAD83 (National Spatial Reference System 2011)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["SPCS83 Idaho West zone (US Survey feet)",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]],
            PARAMETER["Latitude of natural origin",41.6666666666667,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8801]],
            PARAMETER["Longitude of natural origin",-115.75,
                ANGLEUNIT["degree",0.0174532925199433],
                ID["EPSG",8802]],
            PARAMETER["Scale factor at natural origin",0.999933333,
                SCALEUNIT["unity",1],
                ID["EPSG",8805]],
            PARAMETER["False easting",2624666.667,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8806]],
            PARAMETER["False northing",0,
                LENGTHUNIT["US survey foot",0.304800609601219],
                ID["EPSG",8807]]],
        CS[Cartesian,2],
            AXIS["(E)",east,
                ORDER[1],
                LENGTHUNIT["US survey foot",0.304800609601219]],
            AXIS["(N)",north,
                ORDER[2],
                LENGTHUNIT["US survey foot",0.304800609601219]]],
    DERIVINGCONVERSION["Grid to ground",
        METHOD["Similarity transformation",
            ID["EPSG",9621]],
        PARAMETER["Ordinate 1 of evaluation point in target CRS",1000,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8621]],
        PARAMETER["Ordinate 2 of evaluation point in target CRS",0,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8622]],
        PARAMETER["Scale factor for source CRS axes",1,
            SCALEUNIT["unity",1],
            ID["EPSG",1061]],
        PARAMETER["Rotation angle of source CRS axes",0,
            ANGLEUNIT["degree",0],
            ID["EPSG",8614]]],
    CS[Cartesian,2],
        AXIS["easting (X)",east,
            ORDER[1],
            LENGTHUNIT["US survey foot",0.304800609601219]],
        AXIS["northing (Y)",north,
            ORDER[2],
            LENGTHUNIT["US survey foot",0.304800609601219]]]"#;
    assert_eq!(export(crs_of(&object), WktConvention::Wkt2_2019), expected);
    let db = DatabaseContext::create().expect("proj.db");
    let with_db = crs_of(&object)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db))
        .expect("export");
    assert_eq!(with_db, expected);
}

#[test]
fn wkt_parse_derivedprojectedcrs_base_crs_cs_non_metre_from_conversion_context_from_baseprojcrs_name(
) {
    let wkt = r#"DERIVEDPROJCRS["Ground for NAD83(2011) / Idaho West (ftUS)",
    BASEPROJCRS["NAD83(2011) / Idaho West (ftUS)",
        BASEGEOGCRS["NAD83(2011)",
            DATUM["NAD83 (National Spatial Reference System 2011)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["incomplete conversion",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]]]],
    DERIVINGCONVERSION["Grid to ground",
        METHOD["Similarity transformation",
            ID["EPSG",9621]],
        PARAMETER["Ordinate 1 of evaluation point in target CRS",1000,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8621]],
        PARAMETER["Ordinate 2 of evaluation point in target CRS",0,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8622]],
        PARAMETER["Scale factor for source CRS axes",1,
            SCALEUNIT["unity",1],
            ID["EPSG",1061]],
        PARAMETER["Rotation angle of source CRS axes",0,
            ANGLEUNIT["degree",0],
            ID["EPSG",8614]]],
    CS[Cartesian,2],
        AXIS["easting (X)",east,
            ORDER[1],
            LENGTHUNIT["US survey foot",0.304800609601219]],
        AXIS["northing (Y)",north,
            ORDER[2],
            LENGTHUNIT["US survey foot",0.304800609601219]]]"#;
    let object = parse_db(wkt);
    check_us_foot_base(&object);
    let db = DatabaseContext::create().expect("proj.db");
    let exported = crs_of(&object)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db))
        .expect("export");
    assert_eq!(exported, wkt);
}

#[test]
fn wkt_parse_derivedprojectedcrs_base_crs_cs_non_metre_from_conversion_context_from_baseprojcrs_id()
{
    let wkt = r#"DERIVEDPROJCRS["Ground for NAD83(2011) / Idaho West (ftUS)",
    BASEPROJCRS["foo",
        BASEGEOGCRS["NAD83(2011)",
            DATUM["NAD83 (National Spatial Reference System 2011)",
                ELLIPSOID["GRS 1980",6378137,298.257222101,
                    LENGTHUNIT["metre",1]]],
            PRIMEM["Greenwich",0,
                ANGLEUNIT["degree",0.0174532925199433]]],
        CONVERSION["incomplete conversion",
            METHOD["Transverse Mercator",
                ID["EPSG",9807]]],
        ID["EPSG",6453]],
    DERIVINGCONVERSION["Grid to ground",
        METHOD["Similarity transformation",
            ID["EPSG",9621]],
        PARAMETER["Ordinate 1 of evaluation point in target CRS",1000,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8621]],
        PARAMETER["Ordinate 2 of evaluation point in target CRS",0,
            LENGTHUNIT["US survey foot",0.304800609601219],
            ID["EPSG",8622]],
        PARAMETER["Scale factor for source CRS axes",1,
            SCALEUNIT["unity",1],
            ID["EPSG",1061]],
        PARAMETER["Rotation angle of source CRS axes",0,
            ANGLEUNIT["degree",0],
            ID["EPSG",8614]]],
    CS[Cartesian,2],
        AXIS["easting (X)",east,
            ORDER[1],
            LENGTHUNIT["US survey foot",0.304800609601219]],
        AXIS["northing (Y)",north,
            ORDER[2],
            LENGTHUNIT["US survey foot",0.304800609601219]]]"#;
    let object = parse_db(wkt);
    check_us_foot_base(&object);
    let db = DatabaseContext::create().expect("proj.db");
    let exported = crs_of(&object)
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019).with_database(db))
        .expect("export");
    assert_eq!(exported, wkt);
}

#[test]
fn wkt_parse_invalid_derivedgeographiccrs() {
    accepted(
        r#"GEODCRS["WMO Atlantic Pole",
    BASEGEODCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    DERIVINGCONVERSION["foo",
        METHOD["bar"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[ellipsoidal,2],
        AXIS["latitude",north],
        AXIS["longitude",east],
        UNIT["degree",0.0174532925199433]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"GEODCRS["WMO Atlantic Pole",
    BASEGEODCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    CS[ellipsoidal,2],
        AXIS["latitude",north],
        AXIS["longitude",east],
        UNIT["degree",0.0174532925199433]]"#,
    );
    // Missing CS
    refused(
        r#"GEODCRS["WMO Atlantic Pole",
    BASEGEODCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    DERIVINGCONVERSION["foo",
        METHOD["bar"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
    // CS should be ellipsoidal given root node is GEOGCRS
    refused(
        r#"GEOGCRS["WMO Atlantic Pole",
    BASEGEOGCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    DERIVINGCONVERSION["foo",
        METHOD["bar"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,3],
        AXIS["(X)",geocentricX],
        AXIS["(Y)",geocentricY],
        AXIS["(Z)",geocentricZ],
        UNIT["metre",1]]"#,
    );
    // CS should have 3 axis
    refused(
        r#"GEODCRS["WMO Atlantic Pole",
    BASEGEODCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    DERIVINGCONVERSION["foo",
        METHOD["bar"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,2],
        AXIS["(X)",geocentricX],
        AXIS["(Y)",geocentricY],
        UNIT["metre",1]]"#,
    );
    // Invalid CS type
    refused(
        r#"GEODCRS["WMO Atlantic Pole",
    BASEGEODCRS["WGS 84",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563]]],
    DERIVINGCONVERSION["foo",
        METHOD["bar"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[vertical,1],
        AXIS["gravity-related height (H)",up],
        UNIT["metre",1]]"#,
    );
}

#[test]
fn wkt_parse_invalid_temporalcrs() {
    accepted(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[temporal,1],
        AXIS["time (T)",future]]"#,
    );
    // Missing TDATUM
    refused(
        r#"TIMECRS["Temporal CRS",
    CS[temporal,1],
        AXIS["time (T)",future]]"#,
    );
    // Missing CS
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]]]"#,
    );
    // CS should be temporal
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[Cartesian,2],
        AXIS["(X)",geocentricX],
        AXIS["(Y)",geocentricY],
        UNIT["metre",1]]"#,
    );
    // CS should have 1 axis
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[temporal,2],
        AXIS["time (T)",future],
        AXIS["time2 (T)",future]]"#,
    );
    // CS should have 1 axis
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[TemporalDateTime,2],
        AXIS["time (T)",future],
        AXIS["time2 (T)",future]]"#,
    );
    // CS should have 1 axis
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[TemporalCount,2],
        AXIS["time (T)",future],
        AXIS["time2 (T)",future]]"#,
    );
    // CS should have 1 axis
    refused(
        r#"TIMECRS["Temporal CRS",
    TDATUM["Gregorian calendar",
        TIMEORIGIN[0000-01-01]],
    CS[TemporalMeasure,2],
        AXIS["time (T)",future],
        AXIS["time2 (T)",future]]"#,
    );
}

#[test]
fn wkt_parse_invalid_parametriccrs() {
    accepted(
        r#"PARAMETRICCRS["name",
    PDATUM["name"],
    CS[parametric,1],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
    // Missing PDATUM
    refused(
        r#"PARAMETRICCRS["name",
    CS[parametric,1],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
    // Missing CS
    refused(
        r#"PARAMETRICCRS["name",
    PDATUM["name"]]"#,
    );
    // Invalid number of axis for CS
    refused(
        r#"PARAMETRICCRS["name",
    PDATUM["name"],
    CS[parametric,2],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
    // Invalid CS type
    refused(
        r#"PARAMETRICCRS["name",
    PDATUM["name"],
    CS[temporal,1],
        AXIS["time (T)",future]]"#,
    );
}

#[test]
fn wkt_parse_invalid_derivedprojcrs() {
    accepted(
        r#"DERIVEDPROJCRS["derived projectedCRS",
    BASEPROJCRS["BASEPROJCRS",
        BASEGEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]]],
        CONVERSION["unknown",
            METHOD["PROJ unimplemented"],
            PARAMETER["foo",1.0,UNIT["metre",1]]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
        UNIT["metre",1]]"#,
    );
    refused(
        r#"DERIVEDPROJCRS["derived projectedCRS",
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
        UNIT["metre",1]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"DERIVEDPROJCRS["derived projectedCRS",
    BASEPROJCRS["BASEPROJCRS",
        BASEGEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]]],
        CONVERSION["unknown",
            METHOD["PROJ unimplemented"],
            PARAMETER["foo",1.0,UNIT["metre",1]]]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
        UNIT["metre",1]]"#,
    );
    // Missing CS
    refused(
        r#"DERIVEDPROJCRS["derived projectedCRS",
    BASEPROJCRS["BASEPROJCRS",
        BASEGEOGCRS["WGS 84",
            DATUM["World Geodetic System 1984",
                ELLIPSOID["WGS 84",6378137,298.257223563,
                    LENGTHUNIT["metre",1]]]],
        CONVERSION["unknown",
            METHOD["PROJ unimplemented"],
            PARAMETER["foo",1.0,UNIT["metre",1]]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
}

#[test]
fn wkt_parse_invalid_derivedverticalcrs() {
    accepted(
        r#"VERTCRS["Derived vertCRS",
    BASEVERTCRS["ODN height",
        VDATUM["Ordnance Datum Newlyn"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[vertical,1],
        AXIS["gravity-related height (H)",up],
        UNIT["metre",1]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"VERTCRS["Derived vertCRS",
    BASEVERTCRS["ODN height",
        VDATUM["Ordnance Datum Newlyn"]],
    CS[vertical,1],
        AXIS["gravity-related height (H)",up],
        UNIT["metre",1]]"#,
    );
    // Missing CS
    refused(
        r#"VERTCRS["Derived vertCRS",
    BASEVERTCRS["ODN height",
        VDATUM["Ordnance Datum Newlyn"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
    // Wrong CS type
    refused(
        r#"VERTCRS["Derived vertCRS",
    BASEVERTCRS["ODN height",
        VDATUM["Ordnance Datum Newlyn"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[parametric,1],
        AXIS["gravity-related height (H)",up],
        UNIT["metre",1]]"#,
    );
}

#[test]
fn wkt_parse_invalid_derivedengineeringcrs() {
    accepted(
        r#"ENGCRS["Derived EngineeringCRS",
    BASEENGCRS["Engineering CRS",
        EDATUM["Engineering datum"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
        LENGTHUNIT["metre",1]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"ENGCRS["Derived EngineeringCRS",
    BASEENGCRS["Engineering CRS",
        EDATUM["Engineering datum"]],
    CS[Cartesian,2],
        AXIS["(E)",east],
        AXIS["(N)",north],
        LENGTHUNIT["metre",1]]"#,
    );
    // Missing CS
    refused(
        r#"ENGCRS["Derived EngineeringCRS",
    BASEENGCRS["Engineering CRS",
        EDATUM["Engineering datum"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
}

#[test]
fn wkt_parse_invalid_derivedparametriccrs() {
    accepted(
        r#"PARAMETRICCRS["Derived ParametricCRS",
    BASEPARAMCRS["Parametric CRS",
        PDATUM["Parametric datum"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[parametric,1],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"PARAMETRICCRS["Derived ParametricCRS",
    BASEPARAMCRS["Parametric CRS",
        PDATUM["Parametric datum"]],
    CS[parametric,1],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
    // Missing CS
    refused(
        r#"PARAMETRICCRS["Derived ParametricCRS",
    BASEPARAMCRS["Parametric CRS",
        PDATUM["Parametric datum"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
    // Wrong CS type
    refused(
        r#"PARAMETRICCRS["Derived ParametricCRS",
    BASEPARAMCRS["Parametric CRS",
        PDATUM["Parametric datum"]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[TemporalDateTime,1],
        AXIS["time (T)",future]]"#,
    );
}

#[test]
fn wkt_parse_invalid_derivedtemporalcrs() {
    accepted(
        r#"TIMECRS["Derived TemporalCRS",
    BASETIMECRS["Temporal CRS",
        TDATUM["Gregorian calendar",
            CALENDAR["proleptic Gregorian"],
            TIMEORIGIN[0000-01-01]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[TemporalDateTime,1],
        AXIS["time (T)",future]]"#,
    );
    // Missing DERIVINGCONVERSION
    refused(
        r#"TIMECRS["Derived TemporalCRS",
    BASETIMECRS["Temporal CRS",
        TDATUM["Gregorian calendar",
            CALENDAR["proleptic Gregorian"],
            TIMEORIGIN[0000-01-01]]],
    CS[TemporalDateTime,1],
        AXIS["time (T)",future]]"#,
    );
    // Missing CS
    refused(
        r#"TIMECRS["Derived TemporalCRS",
    BASETIMECRS["Temporal CRS",
        TDATUM["Gregorian calendar",
            CALENDAR["proleptic Gregorian"],
            TIMEORIGIN[0000-01-01]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]]]"#,
    );
    // Wrong CS type
    refused(
        r#"TIMECRS["Derived TemporalCRS",
    BASETIMECRS["Temporal CRS",
        TDATUM["Gregorian calendar",
            CALENDAR["proleptic Gregorian"],
            TIMEORIGIN[0000-01-01]]],
    DERIVINGCONVERSION["unknown",
        METHOD["PROJ unimplemented"],
        PARAMETER["foo",1.0,UNIT["metre",1]]],
    CS[parametric,1],
        AXIS["pressure (hPa)",up,
            PARAMETRICUNIT["HectoPascal",100]]]"#,
    );
}

#[test]
fn wkt_parse_derivedverticalcrs_epsg_code_for_horizontal_crs() {
    let wkt = r#"VERTCRS["Derived vertCRS",
    BASEVERTCRS["ODN height",
        VDATUM["Ordnance Datum Newlyn",
            ID["EPSG",5101]]],
    DERIVINGCONVERSION["Conv Vertical Offset and Slope",
        METHOD["Vertical Offset and Slope",
            ID["EPSG",1046]],
        PARAMETER["Ordinate 1 of evaluation point",40.5,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8617]],
        PARAMETER["EPSG code for Horizontal CRS",4277,
            ID["EPSG",1037]]],
    CS[vertical,1],
        AXIS["gravity-related height (H)",up,
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    let object = parse_db(wkt);
    let crs = derived(&object);
    // "EPSG code for Horizontal CRS" is removed and set as interpolation CRS
    assert_eq!(crs.deriving_conversion().parameters().len(), 1);
    assert!(crs.deriving_conversion().interpolation_crs().is_some());
    assert_eq!(export(crs_of(&object), WktConvention::Wkt2_2019), wkt);
}

// ---------------------------------------------------------------------------
// test_operationfactory.cpp

fn authority_crs(db: &DatabaseContext, authority: &str, code: &str) -> Crs {
    AuthorityFactory::create(db, authority)
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn operations_between(
    source: &Crs,
    target: &Crs,
    db: &DatabaseContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let context = CoordinateOperationContext::create(db, "");
    CoordinateOperationFactory::create().create_operations(source, target, &context)
}

fn proj_string_of(operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.export_to_proj_string(),
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    }
    .expect("PROJ string")
}

fn is_unsupported(result: Result<Vec<CoordinateOperation>, FactoryError>) -> bool {
    matches!(result, Err(FactoryError::Unsupported(_)))
}

#[test]
fn operation_geogcrs_to_geogcrs_context_incompatible_celestial_body() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = authority_crs(&db, "EPSG", "4326");
    let target = authority_crs(&db, "IAU_2015", "51200");
    assert!(is_unsupported(operations_between(&source, &target, &db)));
}

#[test]
fn operation_geogcrs_to_geogcrs_context_incompatible_celestial_body_but_same_radius() {
    let db = DatabaseContext::create().expect("proj.db");
    // GCS_Pan_2000 and Ananke
    let source = authority_crs(&db, "ESRI", "104936");
    let target = authority_crs(&db, "IAU_2015", "51200");
    assert!(is_unsupported(operations_between(&source, &target, &db)));
}

#[test]
fn operation_geogcrs_to_geogcrs_compatible_unknown_celestial_body() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input("+proj=longlat +R=10000 +type=crs", &db).expect("source");
    let target = create_from_user_input("+proj=longlat +R=10001 +type=crs", &db).expect("target");
    let list = operations_between(&source, &target, &db).expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(proj_string_of(&list[0]), "+proj=noop");
}

#[test]
fn operation_geogcrs_to_geogcrs_incompatible_unknown_celestial_body() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input("+proj=longlat +R=10000 +type=crs", &db).expect("source");
    let target = create_from_user_input("+proj=longlat +R=99999 +type=crs", &db).expect("target");
    assert!(is_unsupported(operations_between(&source, &target, &db)));
}

#[test]
fn operation_geogcrs_to_geogcrs_compatible_celestial_body_through_semi_major_axis() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = authority_crs(&db, "IAU_2015", "51200"); // Ananke
    let wkt = r#"GEOGCRS["unknown",
    DATUM["unknown",
        ELLIPSOID["unknown",9999,0,
            LENGTHUNIT["metre",1]]],
    PRIMEM["Reference_Meridian",0,
        ANGLEUNIT["degree",0.0174532925199433]],
    CS[ellipsoidal,2],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433]]]"#;
    let target = crs_of(&parse(wkt)).clone();
    let list = operations_between(&source, &target, &db).expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(proj_string_of(&list[0]), "+proj=noop");
}

#[test]
fn operation_geogcrs_to_derived_geogcrs_3d() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = authority_crs(&db, "EPSG", "4979");
    let wkt = r#"GEOGCRS["CH1903+ with 10m offset on ellipsoidal height",
    BASEGEOGCRS["CH1903+",
        DATUM["CH1903+",
            ELLIPSOID["Bessel 1841",6377397.155,299.1528128,
                LENGTHUNIT["metre",1]],
            ID["EPSG",6150]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8901]]],
    DERIVINGCONVERSION["Offset on ellipsoidal height",
        METHOD["Geographic3D offsets",
            ID["EPSG",9660]],
        PARAMETER["Latitude offset",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8601]],
        PARAMETER["Longitude offset",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8602]],
        PARAMETER["Vertical Offset",10,
            LENGTHUNIT["metre",1],
            ID["EPSG",8603]]],
    CS[ellipsoidal,3],
        AXIS["geodetic latitude (Lat)",north,
            ORDER[1],
            ANGLEUNIT["degree",0.0174532925199433,
                ID["EPSG",9122]]],
        AXIS["geodetic longitude (Lon)",east,
            ORDER[2],
            ANGLEUNIT["degree",0.0174532925199433,
                ID["EPSG",9122]]],
        AXIS["ellipsoidal height (h)",up,
            ORDER[3],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#;
    let target = crs_of(&parse(wkt)).clone();
    let mut context = CoordinateOperationContext::create(&db, "EPSG");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let list = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of CH1903+ to WGS 84 (1) + Offset on ellipsoidal height"
    );
    assert_eq!(
        proj_string_of(&list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=-674.374 +y=-15.056 +z=-405.346 \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=geogoffset +dlat=0 +dlon=0 +dh=10 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

#[test]
fn io_projparse_topocentric() {
    let crs = create_crs_from_proj_string(
        "+proj=topocentric +datum=WGS84 +X_0=-3982059.42 +Y_0=3331314.88 \
         +Z_0=3692463.58 +no_defs +type=crs",
    )
    .expect("topocentric");
    assert!(matches!(crs, Crs::Projected(_)));
    assert_eq!(
        export(&crs, WktConvention::Wkt2_2019),
        r#"PROJCRS["unknown",
    BASEGEODCRS["unknown",
        DATUM["World Geodetic System 1984",
            ELLIPSOID["WGS 84",6378137,298.257223563,
                LENGTHUNIT["metre",1]],
            ID["EPSG",6326]],
        PRIMEM["Greenwich",0,
            ANGLEUNIT["degree",0.0174532925199433],
            ID["EPSG",8901]]],
    CONVERSION["unknown",
        METHOD["Geocentric/topocentric conversions",
            ID["EPSG",9836]],
        PARAMETER["Geocentric X of topocentric origin",-3982059.42,
            LENGTHUNIT["metre",1],
            ID["EPSG",8837]],
        PARAMETER["Geocentric Y of topocentric origin",3331314.88,
            LENGTHUNIT["metre",1],
            ID["EPSG",8838]],
        PARAMETER["Geocentric Z of topocentric origin",3692463.58,
            LENGTHUNIT["metre",1],
            ID["EPSG",8839]]],
    CS[Cartesian,3],
        AXIS["topocentric East (U)",east,
            ORDER[1],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["topocentric North (V)",north,
            ORDER[2],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]],
        AXIS["topocentric Up (W)",up,
            ORDER[3],
            LENGTHUNIT["metre",1,
                ID["EPSG",9001]]]]"#
    );
}

const TEMPORAL_EXTENT_JSON: &str = r#"{
  "$schema": "foo",
  "type": "GeodeticReferenceFrame",
  "name": "World Geodetic System 1984",
  "ellipsoid": {
    "name": "WGS 84",
    "semi_major_axis": 6378137,
    "inverse_flattening": 298.257223563
  },
  "temporal_extent": {
    "start": "my start",
    "end": "my end"
  }
}"#;

const PARAMETRIC_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "ParametricCRS",
  "name": "WMO standard atmosphere layer 0",
  "datum": {
    "name": "Mean Sea Level",
    "anchor": "1013.25 hPa at 15°C"
  },
  "coordinate_system": {
    "subtype": "parametric",
    "axis": [
      {
        "name": "Pressure",
        "abbreviation": "hPa",
        "direction": "up",
        "unit": {
          "type": "ParametricUnit",
          "name": "HectoPascal",
          "conversion_factor": 100
        }
      }
    ]
  }
}"#;

const ENGINEERING_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "EngineeringCRS",
  "name": "Engineering CRS",
  "datum": {
    "name": "Engineering datum"
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": "metre"
      }
    ]
  }
}"#;

const TEMPORAL_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "TemporalCRS",
  "name": "Temporal CRS",
  "datum": {
    "name": "Gregorian calendar",
    "calendar": "proleptic Gregorian",
    "time_origin": "0000-01-01"
  },
  "coordinate_system": {
    "subtype": "TemporalDateTime",
    "axis": [
      {
        "name": "Time",
        "abbreviation": "T",
        "direction": "future"
      }
    ]
  }
}"#;

const DERIVED_GEODETIC_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedGeodeticCRS",
  "name": "Derived geodetic CRS",
  "base_crs": {
    "type": "GeographicCRS",
    "name": "WGS 84",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "World Geodetic System 1984",
      "ellipsoid": {
        "name": "WGS 84",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257223563
      }
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Latitude",
          "abbreviation": "lat",
          "direction": "north",
          "unit": "degree"
        },
        {
          "name": "Longitude",
          "abbreviation": "lon",
          "direction": "east",
          "unit": "degree"
        }
      ]
    }
  },
  "conversion": {
    "name": "Some conversion",
    "method": {
      "name": "Some method"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Geocentric X",
        "abbreviation": "X",
        "direction": "geocentricX",
        "unit": "metre"
      },
      {
        "name": "Geocentric Y",
        "abbreviation": "Y",
        "direction": "geocentricY",
        "unit": "metre"
      },
      {
        "name": "Geocentric Z",
        "abbreviation": "Z",
        "direction": "geocentricZ",
        "unit": "metre"
      }
    ]
  }
}"#;

const DERIVED_GEOGRAPHIC_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedGeographicCRS",
  "name": "WMO Atlantic Pole",
  "base_crs": {
    "type": "GeographicCRS",
    "name": "WGS 84",
    "datum": {
      "type": "GeodeticReferenceFrame",
      "name": "World Geodetic System 1984",
      "ellipsoid": {
        "name": "WGS 84",
        "semi_major_axis": 6378137,
        "inverse_flattening": 298.257223563
      }
    },
    "coordinate_system": {
      "subtype": "ellipsoidal",
      "axis": [
        {
          "name": "Latitude",
          "abbreviation": "lat",
          "direction": "north",
          "unit": "degree"
        },
        {
          "name": "Longitude",
          "abbreviation": "lon",
          "direction": "east",
          "unit": "degree"
        }
      ]
    }
  },
  "conversion": {
    "name": "Atlantic pole",
    "method": {
      "name": "Pole rotation"
    },
    "parameters": [
      {
        "name": "Latitude of rotated pole",
        "value": 52,
        "unit": "degree"
      },
      {
        "name": "Longitude of rotated pole",
        "value": -30,
        "unit": "degree"
      },
      {
        "name": "Axis rotation",
        "value": -25,
        "unit": "degree"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "ellipsoidal",
    "axis": [
      {
        "name": "Latitude",
        "abbreviation": "lat",
        "direction": "north",
        "unit": "degree"
      },
      {
        "name": "Longitude",
        "abbreviation": "lon",
        "direction": "east",
        "unit": "degree"
      }
    ]
  }
}"#;

const DERIVED_PROJECTED_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedProjectedCRS",
  "name": "derived projectedCRS",
  "base_crs": {
    "type": "ProjectedCRS",
    "name": "WGS 84 / UTM zone 31N",
    "base_crs": {
      "type": "GeographicCRS",
      "name": "WGS 84",
      "datum": {
        "type": "GeodeticReferenceFrame",
        "name": "World Geodetic System 1984",
        "ellipsoid": {
          "name": "WGS 84",
          "semi_major_axis": 6378137,
          "inverse_flattening": 298.257223563
        }
      },
      "coordinate_system": {
        "subtype": "ellipsoidal",
        "axis": [
          {
            "name": "Latitude",
            "abbreviation": "lat",
            "direction": "north",
            "unit": "degree"
          },
          {
            "name": "Longitude",
            "abbreviation": "lon",
            "direction": "east",
            "unit": "degree"
          }
        ]
      }
    },
    "conversion": {
      "name": "UTM zone 31N",
      "method": {
        "name": "Transverse Mercator",
        "id": {
          "authority": "EPSG",
          "code": 9807
        }
      },
      "parameters": [
        {
          "name": "Latitude of natural origin",
          "value": 0,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8801
          }
        },
        {
          "name": "Longitude of natural origin",
          "value": 3,
          "unit": "degree",
          "id": {
            "authority": "EPSG",
            "code": 8802
          }
        },
        {
          "name": "Scale factor at natural origin",
          "value": 0.9996,
          "unit": "unity",
          "id": {
            "authority": "EPSG",
            "code": 8805
          }
        },
        {
          "name": "False easting",
          "value": 500000,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8806
          }
        },
        {
          "name": "False northing",
          "value": 0,
          "unit": "metre",
          "id": {
            "authority": "EPSG",
            "code": 8807
          }
        }
      ]
    },
    "coordinate_system": {
      "subtype": "Cartesian",
      "axis": [
        {
          "name": "Easting",
          "abbreviation": "E",
          "direction": "east",
          "unit": "metre"
        },
        {
          "name": "Northing",
          "abbreviation": "N",
          "direction": "north",
          "unit": "metre"
        }
      ]
    }
  },
  "conversion": {
    "name": "unnamed",
    "method": {
      "name": "PROJ unimplemented"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": "metre"
      }
    ]
  }
}"#;

const DERIVED_VERTICAL_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedVerticalCRS",
  "name": "Derived vertCRS",
  "base_crs": {
    "type": "VerticalCRS",
    "name": "ODN height",
    "datum": {
      "type": "VerticalReferenceFrame",
      "name": "Ordnance Datum Newlyn"
    },
    "coordinate_system": {
      "subtype": "vertical",
      "axis": [
        {
          "name": "Gravity-related height",
          "abbreviation": "H",
          "direction": "up",
          "unit": "metre"
        }
      ]
    }
  },
  "conversion": {
    "name": "unnamed",
    "method": {
      "name": "PROJ unimplemented"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  }
}"#;

const DERIVED_VERTICAL_EPSG_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedVerticalCRS",
  "name": "Derived vertCRS",
  "base_crs": {
    "type": "VerticalCRS",
    "name": "ODN height",
    "datum": {
      "type": "VerticalReferenceFrame",
      "name": "Ordnance Datum Newlyn",
      "id": {
        "authority": "EPSG",
        "code": 5101
      }
    },
    "coordinate_system": {
      "subtype": "vertical",
      "axis": [
        {
          "name": "Gravity-related height",
          "abbreviation": "H",
          "direction": "up",
          "unit": "metre"
        }
      ]
    }
  },
  "conversion": {
    "name": "Conv Vertical Offset and Slope",
    "method": {
      "name": "Vertical Offset and Slope",
      "id": {
        "authority": "EPSG",
        "code": 1046
      }
    },
    "parameters": [
      {
        "name": "Ordinate 1 of evaluation point",
        "value": 40.5,
        "unit": "degree",
        "id": {
          "authority": "EPSG",
          "code": 8617
        }
      },
      {
        "name": "EPSG code for Horizontal CRS",
        "value": 4277,
        "id": {
          "authority": "EPSG",
          "code": 1037
        }
      }
    ]
  },
  "coordinate_system": {
    "subtype": "vertical",
    "axis": [
      {
        "name": "Gravity-related height",
        "abbreviation": "H",
        "direction": "up",
        "unit": "metre"
      }
    ]
  }
}"#;

const DERIVED_ENGINEERING_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedEngineeringCRS",
  "name": "Derived EngineeringCRS",
  "base_crs": {
    "type": "EngineeringCRS",
    "name": "Engineering CRS",
    "datum": {
      "name": "Engineering datum"
    },
    "coordinate_system": {
      "subtype": "Cartesian",
      "axis": [
        {
          "name": "Easting",
          "abbreviation": "E",
          "direction": "east",
          "unit": "metre"
        },
        {
          "name": "Northing",
          "abbreviation": "N",
          "direction": "north",
          "unit": "metre"
        }
      ]
    }
  },
  "conversion": {
    "name": "unnamed",
    "method": {
      "name": "PROJ unimplemented"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "Cartesian",
    "axis": [
      {
        "name": "Easting",
        "abbreviation": "E",
        "direction": "east",
        "unit": "metre"
      },
      {
        "name": "Northing",
        "abbreviation": "N",
        "direction": "north",
        "unit": "metre"
      }
    ]
  }
}"#;

const DERIVED_PARAMETRIC_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedParametricCRS",
  "name": "Derived ParametricCRS",
  "base_crs": {
    "type": "ParametricCRS",
    "name": "Parametric CRS",
    "datum": {
      "name": "Parametric datum"
    },
    "coordinate_system": {
      "subtype": "parametric",
      "axis": [
        {
          "name": "unknown parametric",
          "abbreviation": "",
          "direction": "unspecified",
          "unit": {
            "type": "ParametricUnit",
            "name": "unknown",
            "conversion_factor": 1
          }
        }
      ]
    }
  },
  "conversion": {
    "name": "unnamed",
    "method": {
      "name": "PROJ unimplemented"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "parametric",
    "axis": [
      {
        "name": "Pressure",
        "abbreviation": "hPa",
        "direction": "up",
        "unit": {
          "type": "ParametricUnit",
          "name": "HectoPascal",
          "conversion_factor": 100
        }
      }
    ]
  }
}"#;

const DERIVED_TEMPORAL_CRS_JSON: &str = r#"{
  "$schema": "foo",
  "type": "DerivedTemporalCRS",
  "name": "Derived TemporalCRS",
  "base_crs": {
    "type": "TemporalCRS",
    "name": "Temporal CRS",
    "datum": {
      "name": "Gregorian calendar",
      "calendar": "proleptic Gregorian",
      "time_origin": "0000-01-01"
    },
    "coordinate_system": {
      "subtype": "TemporalDateTime",
      "axis": [
        {
          "name": "unknown temporal",
          "abbreviation": "",
          "direction": "future",
          "unit": {
            "type": "TimeUnit",
            "name": "unknown",
            "conversion_factor": 1
          }
        }
      ]
    }
  },
  "conversion": {
    "name": "unnamed",
    "method": {
      "name": "PROJ unimplemented"
    },
    "parameters": [
      {
        "name": "foo",
        "value": 1,
        "unit": "metre"
      }
    ]
  },
  "coordinate_system": {
    "subtype": "TemporalDateTime",
    "axis": [
      {
        "name": "Time",
        "abbreviation": "T",
        "direction": "future"
      }
    ]
  }
}"#;

fn db() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

fn schema_foo() -> JsonFormat {
    JsonFormat::default().with_schema("foo")
}

fn round_trip_crs(json: &str) -> Crs {
    let database = db();
    let crs = create_from_user_input(json, &database).expect("import");
    let exported = crs
        .export_to_json_with(&database, &schema_foo())
        .expect("export");
    assert_eq!(exported, json);
    crs
}

fn reimport_datum(json: &str, matches_datum: impl FnOnce(&UserObject) -> bool) {
    let database = db();
    let object = create_object_from_user_input(json, &database).expect("datum");
    assert!(matches_datum(&object), "{json}");
}

#[test]
fn json_import_temporal_extent() {
    let database = db();
    let object = create_object_from_user_input(TEMPORAL_EXTENT_JSON, &database).expect("datum");
    assert!(matches!(object, UserObject::GeodeticDatum(_)));
    let exported = object
        .export_to_json(&database, &schema_foo())
        .expect("export");
    assert_eq!(exported, TEMPORAL_EXTENT_JSON);
}

#[test]
fn json_import_parametric_crs() {
    let crs = round_trip_crs(PARAMETRIC_CRS_JSON);
    let Crs::Parametric(crs) = crs else {
        panic!("parametric");
    };
    let json = UserObject::ParametricDatum(crs.datum().clone())
        .export_to_json(&db(), &schema_foo())
        .expect("datum");
    reimport_datum(&json, |object| {
        matches!(object, UserObject::ParametricDatum(_))
    });
}

#[test]
fn json_import_engineering_crs() {
    let crs = round_trip_crs(ENGINEERING_CRS_JSON);
    let Crs::Engineering(crs) = crs else {
        panic!("engineering");
    };
    let json = UserObject::EngineeringDatum(crs.datum().clone())
        .export_to_json(&db(), &schema_foo())
        .expect("datum");
    reimport_datum(&json, |object| {
        matches!(object, UserObject::EngineeringDatum(_))
    });
}

#[test]
fn json_import_temporal_crs() {
    let crs = round_trip_crs(TEMPORAL_CRS_JSON);
    let Crs::Temporal(crs) = crs else {
        panic!("temporal");
    };
    let json = UserObject::TemporalDatum(crs.datum().clone())
        .export_to_json(&db(), &schema_foo())
        .expect("datum");
    reimport_datum(&json, |object| {
        matches!(object, UserObject::TemporalDatum(_))
    });
}

#[test]
fn json_import_derived_geodetic_crs() {
    let crs = round_trip_crs(DERIVED_GEODETIC_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn json_import_derived_geographic_crs() {
    let crs = round_trip_crs(DERIVED_GEOGRAPHIC_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn json_import_derived_projected_crs() {
    let crs = round_trip_crs(DERIVED_PROJECTED_CRS_JSON);
    assert!(matches!(crs, Crs::DerivedProjected(_)));
}

#[test]
fn json_import_derived_vertical_crs() {
    let crs = round_trip_crs(DERIVED_VERTICAL_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn json_import_derived_vertical_crs_epsg_code_for_horizontal_crs() {
    let database = db();
    let crs = create_from_user_input(DERIVED_VERTICAL_EPSG_JSON, &database).expect("import");
    let Crs::Derived(derived) = &crs else {
        panic!("derived vertical");
    };
    assert_eq!(derived.deriving_conversion().parameters().len(), 1);
    assert!(derived.deriving_conversion().interpolation_crs().is_some());
    let exported = crs
        .export_to_json_with(&database, &schema_foo())
        .expect("export");
    assert_eq!(exported, DERIVED_VERTICAL_EPSG_JSON);
}

#[test]
fn json_import_derived_engineering_crs() {
    let crs = round_trip_crs(DERIVED_ENGINEERING_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn json_import_derived_parametric_crs() {
    let crs = round_trip_crs(DERIVED_PARAMETRIC_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn json_import_derived_temporal_crs() {
    let crs = round_trip_crs(DERIVED_TEMPORAL_CRS_JSON);
    assert!(matches!(crs, Crs::Derived(_)));
}

#[test]
fn io_createfromuserinput_ogc_auto() {
    let projected = |text: &str| {
        let crs = create_from_user_input(text, &db()).expect(text);
        let Crs::Projected(crs) = crs else {
            panic!("{text} is not projected");
        };
        crs
    };
    let utm_north = projected("AUTO:42001,-117,33");
    assert_eq!(utm_north.deriving_conversion().name(), "UTM zone 11N");
    assert_eq!(
        utm_north.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=11 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let utm_south = projected("AUTO:42001,-117,-33");
    assert_eq!(utm_south.deriving_conversion().name(), "UTM zone 11S");
    assert_eq!(
        utm_south.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=11 +south +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let metre = projected("AUTO:42001,9001,-117,33");
    assert_eq!(metre.deriving_conversion().name(), "UTM zone 11N");
    assert_eq!(
        metre.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=11 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let foot = projected("AUTO:42001,9002,-117,33");
    assert_eq!(foot.deriving_conversion().name(), "UTM zone 11N");
    assert_eq!(
        foot.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=11 +datum=WGS84 +units=ft +no_defs +type=crs"
    );
    let us_foot = projected("AUTO:42001,9003,-117,33");
    assert_eq!(us_foot.deriving_conversion().name(), "UTM zone 11N");
    assert_eq!(
        us_foot.export_to_proj_string().expect("proj"),
        "+proj=utm +zone=11 +datum=WGS84 +units=us-ft +no_defs +type=crs"
    );
    let database = db();
    for text in [
        "AUTO:42001,0,-117,33",
        "AUTO:42001,-180.01,33",
        "AUTO:42001,180,33",
        "AUTO:42001",
        "AUTO:42999,1,0",
    ] {
        assert!(
            create_from_user_input(text, &database).is_err(),
            "{text} should be refused"
        );
    }
    assert!(create_from_user_input("AUTO:42001,-180,33", &database).is_ok());
    assert!(create_from_user_input("AUTO:42001,179.999,33", &database).is_ok());
    let tmerc = projected("AUTO:42002,1,2");
    assert_eq!(tmerc.deriving_conversion().name(), "Transverse Mercator");
    assert_eq!(
        tmerc.export_to_proj_string().expect("proj"),
        "+proj=tmerc +lat_0=0 +lon_0=1 +k=0.9996 +x_0=500000 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    let tmerc_south = projected("AUTO:42002,1,-2");
    assert_eq!(
        tmerc_south.deriving_conversion().name(),
        "Transverse Mercator"
    );
    assert_eq!(
        tmerc_south.export_to_proj_string().expect("proj"),
        "+proj=tmerc +lat_0=0 +lon_0=1 +k=0.9996 +x_0=500000 +y_0=10000000 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(
        projected("AUTO:42003,1,2")
            .export_to_proj_string()
            .expect("proj"),
        "+proj=ortho +lat_0=2 +lon_0=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(
        projected("AUTO:42004,1,0").export_to_proj_string().expect("proj"),
        "+proj=eqc +lat_ts=0 +lat_0=0 +lon_0=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(
        projected("AUTO:42005,1")
            .export_to_proj_string()
            .expect("proj"),
        "+proj=moll +lon_0=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(
        projected("AUTO:42005,9001,1")
            .export_to_proj_string()
            .expect("proj"),
        "+proj=moll +lon_0=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs +type=crs"
    );
    assert_eq!(
        projected("urn:ogc:def:crs:OGC::AUTO42001:-117:33")
            .export_to_proj_string()
            .expect("proj"),
        "+proj=utm +zone=11 +datum=WGS84 +units=m +no_defs +type=crs"
    );
}
