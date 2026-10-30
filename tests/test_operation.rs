//! PROJ `test/unit/test_operation.cpp` checks that build or export an operation.
//!
//! Each function is one `TEST()` name. WKT and PROJ strings are the expected text.

use proj_rs::{
    Conversion, Crs, Equivalence, GeographicCRS, OpProps, OperationMethod, OperationParameter,
    OperationParameterValue, ParameterValue, Transformation, WktConvention, WktFormatter,
};

fn props() -> OpProps {
    OpProps::new()
}

fn wkt2(conv: &Conversion) -> String {
    conv.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("wkt2")
}

fn wkt1(conv: &Conversion) -> String {
    conv.export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .expect("wkt1")
}

fn proj(conv: &Conversion) -> String {
    conv.export_to_proj_string().expect("proj")
}

#[test]
fn operation_method() {
    let method = OperationMethod::create("", Vec::new());
    assert!(method.is_equivalent_to(&method));
    let other = OperationMethod::create("", vec![OperationParameter::create("paramName")]);
    assert!(other.is_equivalent_to(&other));
    assert!(!method.is_equivalent_to(&other));
    let other2 = OperationMethod::create("", vec![OperationParameter::create("paramName2")]);
    assert!(!other.is_equivalent_to(&other2));
    assert!(!other.is_equivalent_to_with(&other2, Equivalence::Equivalent));
}

#[test]
fn operation_method_parameter_different_order() {
    let method1 = OperationMethod::create(
        "",
        vec![
            OperationParameter::create("paramName"),
            OperationParameter::create("paramName2"),
        ],
    );
    let method2 = OperationMethod::create(
        "",
        vec![
            OperationParameter::create("paramName2"),
            OperationParameter::create("paramName"),
        ],
    );
    let method3 = OperationMethod::create(
        "",
        vec![
            OperationParameter::create("paramName3"),
            OperationParameter::create("paramName"),
        ],
    );
    assert!(!method1.is_equivalent_to(&method2));
    assert!(method1.is_equivalent_to_with(&method2, Equivalence::Equivalent));
    assert!(!method1.is_equivalent_to_with(&method3, Equivalence::Equivalent));
}

#[test]
fn operation_parameter_value() {
    let string1 = ParameterValue::string("str1");
    let string2 = ParameterValue::string("str2");
    assert!(string1.is_equivalent_to(&string1));
    assert!(!string1.is_equivalent_to(&string2));

    let measure = ParameterValue::angle(-90.0);
    let measure_eps = ParameterValue::angle(-90.0 - 1e-11);
    let measure2 = ParameterValue::angle(-89.0);
    assert!(measure.is_equivalent_to(&measure));
    assert!(measure.is_equivalent_to_with(&measure, Equivalence::Equivalent));
    assert!(!measure.is_equivalent_to(&measure_eps));
    assert!(measure.is_equivalent_to_with(&measure_eps, Equivalence::Equivalent));
    assert!(!measure.is_equivalent_to(&string1));
    assert!(!measure.is_equivalent_to(&measure2));
    assert!(!measure.is_equivalent_to_with(&measure2, Equivalence::Equivalent));

    let int1 = ParameterValue::integer(1);
    let int2 = ParameterValue::integer(2);
    assert!(int1.is_equivalent_to(&int1));
    assert!(!int1.is_equivalent_to(&int2));

    let yes = ParameterValue::boolean_true();
    let no = ParameterValue::boolean_false();
    assert!(yes.is_equivalent_to(&yes));
    assert!(!yes.is_equivalent_to(&no));
}

#[test]
fn operation_operation_parameter() {
    let left = OperationParameter::create("paramName");
    let right = OperationParameter::create("paramName2");
    assert!(left.is_equivalent_to(&left));
    assert!(!left.is_equivalent_to(&right));
}

#[test]
fn operation_operation_parameter_value() {
    let param1 = OperationParameter::create("paramName");
    let param2 = OperationParameter::create("paramName2");
    let value1 = ParameterValue::string("str1");
    let value2 = ParameterValue::string("str2");
    let pair11 = OperationParameterValue::create(param1.clone(), value1.clone());
    assert!(pair11.is_equivalent_to(&pair11));
    let pair12 = OperationParameterValue::create(param1, value2);
    assert!(!pair11.is_equivalent_to(&pair12));
    let pair21 = OperationParameterValue::create(param2, value1);
    assert!(!pair11.is_equivalent_to(&pair21));
}

fn sample_transformation(method_param: &str, file_name: &str) -> Transformation {
    Transformation::create(
        &props(),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Crs::Geographic(GeographicCRS::epsg_4807()),
        Some(Crs::Geographic(GeographicCRS::epsg_4979())),
        "",
        vec![OperationParameter::create(method_param)],
        vec![ParameterValue::filename(file_name)],
        Some(0.1),
    )
    .expect("transformation")
}

#[test]
fn operation_single_operation() {
    let sop1 = sample_transformation("paramName", "foo.bin");
    assert!(sop1.is_equivalent_to(&sop1));
    let inverse = sop1.inverse();
    assert!(inverse.is_equivalent_to(&inverse.clone()));
    let sop2 = sample_transformation("paramName2", "foo.bin");
    assert!(!sop1.is_equivalent_to(&sop2));
    let sop3 = Transformation::create(
        &props(),
        Crs::Geographic(GeographicCRS::epsg_4326()),
        Crs::Geographic(GeographicCRS::epsg_4807()),
        Some(Crs::Geographic(GeographicCRS::epsg_4979())),
        "",
        vec![
            OperationParameter::create("paramName"),
            OperationParameter::create("paramName2"),
        ],
        vec![
            ParameterValue::filename("foo.bin"),
            ParameterValue::filename("foo2.bin"),
        ],
        Some(0.1),
    )
    .expect("sop3");
    assert!(!sop1.is_equivalent_to(&sop3));
    let sop4 = sample_transformation("paramName", "foo2.bin");
    assert!(!sop1.is_equivalent_to(&sop4));
}

#[test]
fn operation_single_operation_different_order() {
    let build = |name: &str, params: [&str; 2], files: [&str; 2]| {
        Transformation::create(
            &props().name(name),
            Crs::Geographic(GeographicCRS::epsg_4326()),
            Crs::Geographic(GeographicCRS::epsg_4807()),
            None,
            "",
            vec![
                OperationParameter::create(params[0]),
                OperationParameter::create(params[1]),
            ],
            vec![
                ParameterValue::filename(files[0]),
                ParameterValue::filename(files[1]),
            ],
            None,
        )
        .expect("transformation")
    };
    let sop1 = build(
        "ignored1",
        ["paramName", "paramName2"],
        ["foo.bin", "foo2.bin"],
    );
    let sop2 = build(
        "ignored2",
        ["paramName2", "paramName"],
        ["foo2.bin", "foo.bin"],
    );
    let sop3 = build(
        "ignored3",
        ["paramName", "paramName2"],
        ["foo2.bin", "foo.bin"],
    );
    assert!(!sop1.is_equivalent_to(&sop2));
    assert!(sop1.is_equivalent_to_with(&sop2, Equivalence::Equivalent));
    assert!(!sop1.is_equivalent_to_with(&sop3, Equivalence::Equivalent));
}

#[test]
fn operation_utm_export() {
    let conv = Conversion::create_utm_south(&props(), 1).expect("utm");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=utm +zone=1 +south");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"UTM zone 1S\",
    METHOD[\"Transverse Mercator\",
        ID[\"EPSG\",9807]],
    PARAMETER[\"Latitude of natural origin\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",-177,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Scale factor at natural origin\",0.9996,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"False easting\",500000,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",10000000,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]],
    ID[\"EPSG\",16101]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Transverse_Mercator\"],
PARAMETER[\"latitude_of_origin\",0],
PARAMETER[\"central_meridian\",-177],
PARAMETER[\"scale_factor\",0.9996],
PARAMETER[\"false_easting\",500000],
PARAMETER[\"false_northing\",10000000]"
    );
}

#[test]
fn operation_tmerc_export() {
    let conv =
        Conversion::create_transverse_mercator(&props(), 1.0, 2.0, 3.0, 4.0, 5.0).expect("tmerc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=tmerc +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        conv.export_to_proj_string_approx_tmerc().expect("approx"),
        "+proj=tmerc +approx +lat_0=1 +lon_0=2 +k=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Transverse Mercator\",
    METHOD[\"Transverse Mercator\",
        ID[\"EPSG\",9807]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Scale factor at natural origin\",3,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"False easting\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Transverse_Mercator\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"scale_factor\",3],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_gstmerc_export() {
    let conv =
        Conversion::create_gauss_schreiber_transverse_mercator(&props(), 1.0, 2.0, 3.0, 4.0, 5.0)
            .expect("gstmerc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=gstmerc +lat_0=1 +lon_0=2 +k_0=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Gauss Schreiber Transverse Mercator\",
    METHOD[\"Gauss Schreiber Transverse Mercator\"],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Scale factor at natural origin\",3,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"False easting\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Gauss_Schreiber_Transverse_Mercator\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"scale_factor\",3],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_tmerc_south_oriented_export() {
    let conv =
        Conversion::create_transverse_mercator_south_oriented(&props(), 1.0, 2.0, 3.0, 4.0, 5.0)
            .expect("tmso");
    assert!(conv.validate_parameters().is_empty());
    assert!(conv.export_to_proj_string().is_err());
    assert!(wkt2(&conv).contains("Transverse Mercator (South Orientated)"));
    assert!(wkt2(&conv).contains("ID[\"EPSG\",9808]"));
    assert!(wkt1(&conv).starts_with("PROJECTION[\"Transverse_Mercator_South_Orientated\"]"));
}

#[test]
fn operation_mercator_variant_a_export() {
    let conv =
        Conversion::create_mercator_variant_a(&props(), 0.0, 1.0, 2.0, 3.0, 4.0).expect("merc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=merc +lon_0=1 +k=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Mercator (variant A)\",
    METHOD[\"Mercator (variant A)\",
        ID[\"EPSG\",9804]],
    PARAMETER[\"Latitude of natural origin\",0,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Scale factor at natural origin\",2,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"False easting\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Mercator_1SP\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"scale_factor\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_mercator_variant_a_export_latitude_origin_non_zero() {
    let conv =
        Conversion::create_mercator_variant_a(&props(), 10.0, 1.0, 2.0, 3.0, 4.0).expect("merc");
    assert!(conv.export_to_proj_string().is_err());
}

#[test]
fn operation_lcc1sp_export() {
    let conv = Conversion::create_lambert_conic_conformal_1sp(&props(), 1.0, 2.0, 3.0, 4.0, 5.0)
        .expect("lcc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=lcc +lat_1=1 +lat_0=1 +lon_0=2 +k_0=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Conic Conformal (1SP)\",
    METHOD[\"Lambert Conic Conformal (1SP)\",
        ID[\"EPSG\",9801]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Scale factor at natural origin\",3,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"False easting\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Lambert_Conformal_Conic_1SP\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"scale_factor\",3],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_lcc2sp_is_equivalent_to_parallels_switched() {
    let conv1 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
            .expect("lcc");
    let conv2 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 4.0, 3.0, 5.0, 6.0)
            .expect("lcc");
    assert!(conv1.is_equivalent_to_with(&conv2, Equivalence::Equivalent));
    let conv3 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 3.0, 3.0, 5.0, 6.0)
            .expect("lcc");
    assert!(!conv1.is_equivalent_to_with(&conv3, Equivalence::Equivalent));
}

#[test]
fn operation_create_change_vertical_unit() {
    let conv = Conversion::create_change_vertical_unit(&props(), 1.0).expect("unit");
    assert!(conv.validate_parameters().is_empty());
}

#[test]
fn operation_create_change_vertical_unit_no_conv_factor() {
    let conv = Conversion::create_change_vertical_unit_no_conv_factor(&props()).expect("unit");
    assert!(conv.validate_parameters().is_empty());
}

#[test]
fn operation_create_geographic_geocentric() {
    let conv = Conversion::create_geographic_geocentric(&props()).expect("geog");
    assert!(conv.validate_parameters().is_empty());
}

#[test]
fn operation_validate_parameters() {
    let unknown = Conversion::create_with_method("unknown", "", "", Vec::new());
    assert_eq!(
        unknown.validate_parameters(),
        vec!["Unknown method unknown".to_owned()]
    );

    let loose = Conversion::create_with_method("change of vertical unit", "", "", Vec::new());
    assert_eq!(
        loose.validate_parameters(),
        vec![
            "Method name change of vertical unit is equivalent to official Change of Vertical Unit but not strictly equal".to_owned(),
            "Cannot find expected parameter Unit conversion scalar".to_owned(),
        ]
    );

    let wrong_code =
        Conversion::create_with_method("Change of Vertical Unit", "EPSG", "1234", Vec::new());
    assert_eq!(
        wrong_code.validate_parameters(),
        vec![
            "Method of EPSG code 1234 does not match official code (1069)".to_owned(),
            "Cannot find expected parameter Unit conversion scalar".to_owned(),
        ]
    );

    let fancy = Conversion::create_with_method("some fancy name", "EPSG", "1069", Vec::new());
    assert_eq!(
        fancy.validate_parameters(),
        vec![
            "Method name some fancy name, matched to Change of Vertical Unit, through its EPSG code has not an equivalent name".to_owned(),
            "Cannot find expected parameter Unit conversion scalar".to_owned(),
        ]
    );

    let soft_name = Conversion::create_with_method(
        "Change of Vertical Unit",
        "",
        "",
        vec![Conversion::parameter(
            "unit conversion scalar",
            "",
            "",
            ParameterValue::scale(1.0),
        )],
    );
    assert_eq!(
        soft_name.validate_parameters(),
        vec![
            "Parameter name unit conversion scalar is equivalent to official Unit conversion scalar but not strictly equal".to_owned(),
        ]
    );

    let fancy_param = Conversion::create_with_method(
        "Change of Vertical Unit",
        "",
        "",
        vec![Conversion::parameter(
            "fancy name",
            "EPSG",
            "1051",
            ParameterValue::scale(1.0),
        )],
    );
    assert_eq!(
        fancy_param.validate_parameters(),
        vec![
            "Parameter name fancy name, matched to Unit conversion scalar, through its EPSG code has not an equivalent name".to_owned(),
        ]
    );

    let extra = Conversion::create_with_method(
        "Change of Vertical Unit",
        "",
        "",
        vec![Conversion::parameter(
            "extra param",
            "",
            "",
            ParameterValue::scale(1.0),
        )],
    );
    assert_eq!(
        extra.validate_parameters(),
        vec![
            "Cannot find expected parameter Unit conversion scalar".to_owned(),
            "Parameter extra param found but not expected for this method".to_owned(),
        ]
    );
}

#[test]
fn operation_webmerc_export_conversion() {
    let conv =
        Conversion::create_popular_visualisation_pseudo_mercator(&props(), 0.0, 2.0, 3.0, 4.0)
            .expect("webmerc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=webmerc +lat_0=0 +lon_0=2 +x_0=3 +y_0=4");
    assert!(wkt2(&conv).contains("Popular Visualisation Pseudo Mercator"));
    assert!(wkt2(&conv).contains("ID[\"EPSG\",1024]"));
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Mercator_1SP\"],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"scale_factor\",1],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}
