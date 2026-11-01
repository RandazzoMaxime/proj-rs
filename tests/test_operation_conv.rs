//! PROJ `test/unit/test_operation.cpp` checks on `Conversion::convertToOtherMethod`:
//! Mercator variants A and B, and Lambert Conic Conformal 1SP and 2SP.
//!
//! Each function is one `TEST()` name. A PROJ `nullptr` is `None` here, and an
//! `isEquivalentTo` with `EQUIVALENT` across two methods is
//! `Conversion::is_equivalent_to_with_source`, which takes the base CRS.
//! The expected numbers and tolerances are PROJ's.

use proj_rs::iso19111::{
    CartesianCS, Conversion, Ellipsoid, EllipsoidalCS, Equivalence, GeodeticCRS,
    GeodeticReferenceFrame, GeographicCRS, OpProps, ParameterValue, PrimeMeridian, ProjectedCRS,
    UnitOfMeasure, WktParser,
};

/// `GeographicCRS::EPSG_4269`: something using GRS80.
fn grs80() -> GeodeticCRS {
    GeodeticCRS::Geographic(GeographicCRS::epsg_4269())
}

/// `GeographicCRS::EPSG_4326`.
fn wgs84() -> GeodeticCRS {
    GeodeticCRS::Geographic(GeographicCRS::epsg_4326())
}

/// PROJ `geographicCRSInvalidEccentricity()`: a flattened sphere with inverse flattening 0.1.
fn invalid_eccentricity() -> GeodeticCRS {
    GeodeticCRS::Geographic(
        GeographicCRS::create(
            "",
            Some(GeodeticReferenceFrame::create(
                "",
                Ellipsoid::create_flattened_sphere("", 6_378_137.0, 0.1),
                PrimeMeridian::greenwich(),
            )),
            None,
            EllipsoidalCS::create_latitude_longitude(UnitOfMeasure::degree()),
        )
        .expect("geographic CRS with invalid eccentricity"),
    )
}

fn props() -> OpProps {
    OpProps::new()
}

/// `ProjectedCRS::create(PropertyMap(), base, conv, EastingNorthing(metre))`.
fn projected(base: GeodeticCRS, conversion: Conversion) -> ProjectedCRS {
    ProjectedCRS::create(
        "",
        base,
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
}

/// `parameterValueNumeric(code, unit)`: the stored value, read in `unit`.
fn numeric(conversion: &Conversion, code: i32, unit: &UnitOfMeasure) -> f64 {
    let code = code.to_string();
    let parameter = conversion
        .parameters()
        .iter()
        .find(|parameter| parameter.code() == code.as_str())
        .unwrap_or_else(|| panic!("parameter {code} is missing"));
    match parameter.value() {
        ParameterValue::Measure(measure) => measure.si_value() / unit.conversion_to_si(),
        other => panic!("parameter {code} is not numeric: {other:?}"),
    }
}

fn degree() -> UnitOfMeasure {
    UnitOfMeasure::degree()
}

fn metre() -> UnitOfMeasure {
    UnitOfMeasure::metre()
}

fn scale_unity() -> UnitOfMeasure {
    UnitOfMeasure::unity()
}

/// `convertToOtherMethod` on the deriving conversion of `crs`.
fn convert(crs: &ProjectedCRS, method: i32) -> Option<Conversion> {
    crs.deriving_operation().convert_to_other_method(method)
}

const METHOD_MERCATOR_VARIANT_A: i32 = 9804;
const METHOD_MERCATOR_VARIANT_B: i32 = 9805;
const METHOD_LCC_1SP: i32 = 9801;
const METHOD_LCC_2SP: i32 = 9802;

const PARAMETER_LATITUDE_1ST_STD_PARALLEL: i32 = 8823;
const PARAMETER_LATITUDE_2ND_STD_PARALLEL: i32 = 8824;
const PARAMETER_LATITUDE_OF_NATURAL_ORIGIN: i32 = 8801;
const PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN: i32 = 8802;
const PARAMETER_SCALE_FACTOR_AT_NATURAL_ORIGIN: i32 = 8805;
const PARAMETER_FALSE_EASTING: i32 = 8806;
const PARAMETER_FALSE_NORTHING: i32 = 8807;
const PARAMETER_LATITUDE_FALSE_ORIGIN: i32 = 8821;
const PARAMETER_LONGITUDE_FALSE_ORIGIN: i32 = 8822;
const PARAMETER_EASTING_FALSE_ORIGIN: i32 = 8826;
const PARAMETER_NORTHING_FALSE_ORIGIN: i32 = 8827;

#[test]
fn operation_mercator_variant_a_to_variant_b() {
    let crs = projected(
        wgs84(),
        Conversion::create_mercator_variant_a(&props(), 0.0, 1.0, 0.9, 3.0, 4.0)
            .expect("mercator variant A"),
    );
    let conv = crs.deriving_operation();

    let same = conv
        .convert_to_other_method(METHOD_MERCATOR_VARIANT_A)
        .expect("same method");
    assert!(same.is_equivalent_to(conv.conversion()));

    let target = conv
        .convert_to_other_method(METHOD_MERCATOR_VARIANT_B)
        .expect("variant B");

    // PROJ compares these floats with EXPECT_EQ, which is exact. The last digit of
    // acos/sqrt/sin differs between glibc libm and the Windows CRT, so the comparison
    // uses a relative tolerance of 1e-14 here instead.
    let lat_1 = numeric(&target, PARAMETER_LATITUDE_1ST_STD_PARALLEL, &degree());
    assert!(
        (lat_1 - 25.917_499_691_810_534).abs() <= 1e-14 * 25.917_499_691_810_534,
        "{lat_1}"
    );
    assert_eq!(
        numeric(&target, PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN, &degree()),
        1.0
    );
    assert_eq!(numeric(&target, PARAMETER_FALSE_EASTING, &metre()), 3.0);
    assert_eq!(numeric(&target, PARAMETER_FALSE_NORTHING, &metre()), 4.0);

    assert!(!conv
        .conversion()
        .is_equivalent_to_with(&target, Equivalence::Strict));
    assert!(conv.conversion().is_equivalent_to_with_source(
        &target,
        conv.source_crs(),
        Equivalence::Equivalent
    ));
    assert!(target.is_equivalent_to_with_source(
        conv.conversion(),
        conv.source_crs(),
        Equivalence::Equivalent
    ));
}

#[test]
fn operation_mercator_variant_a_to_variant_b_scale_1() {
    let crs = projected(
        wgs84(),
        Conversion::create_mercator_variant_a(&props(), 0.0, 1.0, 1.0, 3.0, 4.0)
            .expect("mercator variant A"),
    );
    let target = convert(&crs, METHOD_MERCATOR_VARIANT_B).expect("variant B");
    assert_eq!(
        numeric(&target, PARAMETER_LATITUDE_1ST_STD_PARALLEL, &degree()),
        0.0
    );
}

#[test]
fn operation_mercator_variant_a_to_variant_b_invalid_scale() {
    let crs = projected(
        wgs84(),
        Conversion::create_mercator_variant_a(&props(), 0.0, 1.0, 0.0, 3.0, 4.0)
            .expect("mercator variant A"),
    );
    assert!(convert(&crs, METHOD_MERCATOR_VARIANT_B).is_none());
}

#[test]
fn operation_mercator_variant_a_to_variant_b_invalid_eccentricity() {
    let crs = projected(
        invalid_eccentricity(),
        Conversion::create_mercator_variant_a(&props(), 0.0, 1.0, 1.0, 3.0, 4.0)
            .expect("mercator variant A"),
    );
    assert!(convert(&crs, METHOD_MERCATOR_VARIANT_B).is_none());
}

#[test]
fn operation_mercator_variant_b_to_variant_a() {
    let crs = projected(
        wgs84(),
        Conversion::create_mercator_variant_b(&props(), 25.917_499_691_810_534, 1.0, 3.0, 4.0)
            .expect("mercator variant B"),
    );
    let target = convert(&crs, METHOD_MERCATOR_VARIANT_A).expect("variant A");

    assert_eq!(
        numeric(&target, PARAMETER_LATITUDE_OF_NATURAL_ORIGIN, &degree()),
        0.0
    );
    assert_eq!(
        numeric(&target, PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN, &degree()),
        1.0
    );
    let k_0 = numeric(
        &target,
        PARAMETER_SCALE_FACTOR_AT_NATURAL_ORIGIN,
        &scale_unity(),
    );
    // Same reason as the lat_1 check in operation_mercator_variant_a_to_variant_b: PROJ
    // compares exactly, and the last digit of the libm result differs on Windows.
    assert!((k_0 - 0.9).abs() <= 1e-14 * 0.9, "{k_0}");
    assert_eq!(numeric(&target, PARAMETER_FALSE_EASTING, &metre()), 3.0);
    assert_eq!(numeric(&target, PARAMETER_FALSE_NORTHING, &metre()), 4.0);
}

#[test]
fn operation_mercator_variant_b_to_variant_a_invalid_std1() {
    let crs = projected(
        wgs84(),
        Conversion::create_mercator_variant_b(&props(), 100.0, 1.0, 3.0, 4.0)
            .expect("mercator variant B"),
    );
    assert!(convert(&crs, METHOD_MERCATOR_VARIANT_A).is_none());
}

#[test]
fn operation_mercator_variant_b_to_variant_a_invalid_eccentricity() {
    let crs = projected(
        invalid_eccentricity(),
        Conversion::create_mercator_variant_b(&props(), 0.0, 1.0, 3.0, 4.0)
            .expect("mercator variant B"),
    );
    assert!(convert(&crs, METHOD_MERCATOR_VARIANT_A).is_none());
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "the y_0 literal is PROJ's expected value, kept verbatim; the 1e-7 tolerance makes the extra digits irrelevant"
)]
fn operation_lcc2sp_to_lcc1sp() {
    let crs = projected(
        grs80(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            49.0,
            44.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("lcc 2SP"),
    );
    let conv = crs.deriving_operation();
    let target = conv
        .convert_to_other_method(METHOD_LCC_1SP)
        .expect("LCC 1SP");

    let lat_0 = numeric(&target, PARAMETER_LATITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lat_0 - 46.519_430_223_986_866).abs() <= 1e-12, "{lat_0}");
    let lon_0 = numeric(&target, PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let k_0 = numeric(
        &target,
        PARAMETER_SCALE_FACTOR_AT_NATURAL_ORIGIN,
        &scale_unity(),
    );
    assert!((k_0 - 0.999_051_028_637_469_2).abs() <= 1e-15, "{k_0}");
    let x_0 = numeric(&target, PARAMETER_FALSE_EASTING, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&target, PARAMETER_FALSE_NORTHING, &metre());
    assert!((y_0 - 6_602_157.838_810_336_8).abs() <= 1e-7, "{y_0}");

    let target_crs = projected(grs80(), target.clone());
    let target_conv = target_crs.deriving_operation();
    let two_sp = target_conv
        .convert_to_other_method(METHOD_LCC_2SP)
        .expect("LCC 2SP from 1SP");

    let lat_0 = numeric(&two_sp, PARAMETER_LATITUDE_FALSE_ORIGIN, &degree());
    assert!((lat_0 - 46.5).abs() <= 1e-15, "{lat_0}");
    let lon_0 = numeric(&two_sp, PARAMETER_LONGITUDE_FALSE_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let lat_1 = numeric(&two_sp, PARAMETER_LATITUDE_1ST_STD_PARALLEL, &degree());
    assert!((lat_1 - 49.0).abs() <= 1e-15, "{lat_1}");
    let lat_2 = numeric(&two_sp, PARAMETER_LATITUDE_2ND_STD_PARALLEL, &degree());
    assert!((lat_2 - 44.0).abs() <= 1e-15, "{lat_2}");
    let x_0 = numeric(&two_sp, PARAMETER_EASTING_FALSE_ORIGIN, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&two_sp, PARAMETER_NORTHING_FALSE_ORIGIN, &metre());
    assert!((y_0 - 6_600_000.0).abs() <= 1e-15, "{y_0}");

    assert!(!conv
        .conversion()
        .is_equivalent_to_with(&target, Equivalence::Strict));
    assert!(conv.conversion().is_equivalent_to_with_source(
        &target,
        conv.source_crs(),
        Equivalence::Equivalent
    ));
    assert!(target.is_equivalent_to_with_source(
        conv.conversion(),
        conv.source_crs(),
        Equivalence::Equivalent
    ));
}

#[test]
fn operation_lcc2sp_to_lcc1sp_phi0_eq_phi1_eq_phi2() {
    let crs = projected(
        grs80(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            46.5,
            46.5,
            700_000.0,
            6_600_000.0,
        )
        .expect("lcc 2SP"),
    );
    let conv = crs.deriving_operation();
    let target = conv
        .convert_to_other_method(METHOD_LCC_1SP)
        .expect("LCC 1SP");

    let lat_0 = numeric(&target, PARAMETER_LATITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lat_0 - 46.5).abs() <= 1e-15, "{lat_0}");
    let lon_0 = numeric(&target, PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let k_0 = numeric(
        &target,
        PARAMETER_SCALE_FACTOR_AT_NATURAL_ORIGIN,
        &scale_unity(),
    );
    assert!((k_0 - 1.0).abs() <= 1e-15, "{k_0}");
    let x_0 = numeric(&target, PARAMETER_FALSE_EASTING, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&target, PARAMETER_FALSE_NORTHING, &metre());
    assert!((y_0 - 6_600_000.0).abs() <= 1e-15, "{y_0}");

    let target_crs = projected(grs80(), target.clone());
    let two_sp = target_crs
        .deriving_operation()
        .convert_to_other_method(METHOD_LCC_2SP)
        .expect("LCC 2SP from 1SP");

    let lat_0 = numeric(&two_sp, PARAMETER_LATITUDE_FALSE_ORIGIN, &degree());
    assert!((lat_0 - 46.5).abs() <= 1e-15, "{lat_0}");
    let lon_0 = numeric(&two_sp, PARAMETER_LONGITUDE_FALSE_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let lat_1 = numeric(&two_sp, PARAMETER_LATITUDE_1ST_STD_PARALLEL, &degree());
    assert!((lat_1 - 46.5).abs() <= 1e-15, "{lat_1}");
    let lat_2 = numeric(&two_sp, PARAMETER_LATITUDE_2ND_STD_PARALLEL, &degree());
    assert!((lat_2 - 46.5).abs() <= 1e-15, "{lat_2}");
    let x_0 = numeric(&two_sp, PARAMETER_EASTING_FALSE_ORIGIN, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&two_sp, PARAMETER_NORTHING_FALSE_ORIGIN, &metre());
    assert!((y_0 - 6_600_000.0).abs() <= 1e-15, "{y_0}");

    assert!(conv.conversion().is_equivalent_to_with_source(
        &target,
        conv.source_crs(),
        Equivalence::Equivalent
    ));
    assert!(target.is_equivalent_to_with_source(
        conv.conversion(),
        conv.source_crs(),
        Equivalence::Equivalent
    ));
}

#[test]
fn operation_lcc2sp_to_lcc1sp_phi0_diff_phi1_and_phi1_eq_phi2() {
    let crs = projected(
        grs80(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.123,
            3.0,
            46.4567,
            46.4567,
            700_000.0,
            6_600_000.0,
        )
        .expect("lcc 2SP"),
    );
    let conv = crs.deriving_operation();
    let target = conv
        .convert_to_other_method(METHOD_LCC_1SP)
        .expect("LCC 1SP");

    let lat_0 = numeric(&target, PARAMETER_LATITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lat_0 - 46.4567).abs() <= 1e-14, "{lat_0}");
    let lon_0 = numeric(&target, PARAMETER_LONGITUDE_OF_NATURAL_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let k_0 = numeric(
        &target,
        PARAMETER_SCALE_FACTOR_AT_NATURAL_ORIGIN,
        &scale_unity(),
    );
    assert!((k_0 - 1.0).abs() <= 1e-15, "{k_0}");
    let x_0 = numeric(&target, PARAMETER_FALSE_EASTING, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&target, PARAMETER_FALSE_NORTHING, &metre());
    assert!((y_0 - 6_637_093.292_952_879).abs() <= 1e-8, "{y_0}");

    let target_crs = projected(grs80(), target.clone());
    let two_sp = target_crs
        .deriving_operation()
        .convert_to_other_method(METHOD_LCC_2SP)
        .expect("LCC 2SP from 1SP");

    let lat_0 = numeric(&two_sp, PARAMETER_LATITUDE_FALSE_ORIGIN, &degree());
    assert!((lat_0 - 46.4567).abs() <= 1e-14, "{lat_0}");
    let lon_0 = numeric(&two_sp, PARAMETER_LONGITUDE_FALSE_ORIGIN, &degree());
    assert!((lon_0 - 3.0).abs() <= 1e-15, "{lon_0}");
    let lat_1 = numeric(&two_sp, PARAMETER_LATITUDE_1ST_STD_PARALLEL, &degree());
    assert!((lat_1 - 46.4567).abs() <= 1e-14, "{lat_1}");
    let lat_2 = numeric(&two_sp, PARAMETER_LATITUDE_2ND_STD_PARALLEL, &degree());
    assert!((lat_2 - 46.4567).abs() <= 1e-14, "{lat_2}");
    let x_0 = numeric(&two_sp, PARAMETER_EASTING_FALSE_ORIGIN, &metre());
    assert!((x_0 - 700_000.0).abs() <= 1e-15, "{x_0}");
    let y_0 = numeric(&two_sp, PARAMETER_NORTHING_FALSE_ORIGIN, &metre());
    assert!((y_0 - 6_637_093.292_952_879).abs() <= 1e-8, "{y_0}");

    assert!(conv.conversion().is_equivalent_to_with_source(
        &target,
        conv.source_crs(),
        Equivalence::Equivalent
    ));
    assert!(target.is_equivalent_to_with_source(
        conv.conversion(),
        conv.source_crs(),
        Equivalence::Equivalent
    ));

    assert!(conv.conversion().is_equivalent_to_with_source(
        &two_sp,
        conv.source_crs(),
        Equivalence::Equivalent
    ));
}

#[test]
fn operation_lcc1sp_to_lcc2sp_invalid_eccentricity() {
    let crs = projected(
        invalid_eccentricity(),
        Conversion::create_lambert_conic_conformal_1sp(&props(), 40.0, 1.0, 0.99, 3.0, 4.0)
            .expect("LCC 1SP"),
    );
    assert!(convert(&crs, METHOD_LCC_2SP).is_none());
}

#[test]
fn operation_lcc1sp_to_lcc2sp_invalid_scale() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_1sp(&props(), 40.0, 1.0, 0.0, 3.0, 4.0)
            .expect("LCC 1SP"),
    );
    assert!(convert(&crs, METHOD_LCC_2SP).is_none());
}

#[test]
fn operation_lcc1sp_to_lcc2sp_invalid_lat0() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_1sp(&props(), 100.0, 1.0, 0.99, 3.0, 4.0)
            .expect("LCC 1SP"),
    );
    assert!(convert(&crs, METHOD_LCC_2SP).is_none());
}

#[test]
fn operation_lcc1sp_to_lcc2sp_null_lat0() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_1sp(&props(), 0.0, 1.0, 0.99, 3.0, 4.0)
            .expect("LCC 1SP"),
    );
    assert!(convert(&crs, METHOD_LCC_2SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_lat0() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            100.0,
            3.0,
            44.0,
            49.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_lat1() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            100.0,
            49.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_lat2() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            44.0,
            100.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_lat1_opposite_lat2() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            -49.0,
            49.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_lat1_and_lat2_close_to_zero() {
    let crs = projected(
        wgs84(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            0.000_000_000_000_000_1,
            0.000_000_000_000_000_2,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

#[test]
fn operation_lcc2sp_to_lcc1sp_invalid_eccentricity() {
    let crs = projected(
        invalid_eccentricity(),
        Conversion::create_lambert_conic_conformal_2sp(
            &props(),
            46.5,
            3.0,
            44.0,
            49.0,
            700_000.0,
            6_600_000.0,
        )
        .expect("LCC 2SP"),
    );
    assert!(convert(&crs, METHOD_LCC_1SP).is_none());
}

/// `WKTParser().createFromWKT(wkt)` as a projected CRS.
fn projected_from_wkt(wkt: &str) -> ProjectedCRS {
    WktParser::new()
        .create_from_wkt(wkt)
        .expect("wkt")
        .projected()
        .expect("projected")
        .clone()
}

fn equivalent(left: &ProjectedCRS, right: &ProjectedCRS) -> bool {
    left.is_equivalent_to_with(right, Equivalence::Equivalent)
}

#[test]
fn operation_conversion_missing_parameter_scale() {
    const HEAD: &str = r#"PROJCS["test",GEOGCS["WGS 84",DATUM["WGS 1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Mercator_1SP"],PARAMETER["latitude_of_origin",-1],PARAMETER["central_meridian",2],"#;
    const TAIL: &str =
        r#"PARAMETER["false_easting",3],PARAMETER["false_northing",4],UNIT["metre",1]]"#;
    let crs1 = projected_from_wkt(&format!(r#"{HEAD}PARAMETER["scale_factor",1],{TAIL}"#));
    // scale_factor missing
    let crs2 = projected_from_wkt(&format!("{HEAD}{TAIL}"));
    // scale_factor set to -1
    let crs3 = projected_from_wkt(&format!(r#"{HEAD}PARAMETER["scale_factor",-1],{TAIL}"#));

    assert!(equivalent(&crs1, &crs2));
    assert!(equivalent(&crs2, &crs1));

    assert!(!equivalent(&crs1, &crs3));
    assert!(!equivalent(&crs3, &crs1));
    assert!(!equivalent(&crs2, &crs3));
    assert!(!equivalent(&crs3, &crs2));
}
