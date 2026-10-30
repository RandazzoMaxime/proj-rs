//! PROJ `test/unit/test_operation.cpp` checks that build or export an operation.
//!
//! Each function is one `TEST()` name. WKT and PROJ strings are the expected text.

use proj_rs::{
    Conversion, Crs, Equivalence, GeographicCRS, OpProps, OperationMethod, OperationParameter,
    OperationParameterValue, ParameterValue, Transformation, WktConvention, WktFormatter,
    WktParser,
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

#[test]
fn operation_tped_export() {
    let conv = Conversion::create_two_point_equidistant(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
        .expect("tped");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=tpeqd +lat_1=1 +lon_1=2 +lat_2=3 +lon_2=4 +x_0=5 +y_0=6"
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    assert_eq!(
        conv.export_to_wkt(&mut formatter).expect("wkt2"),
        "\
CONVERSION[\"Two Point Equidistant\",
    METHOD[\"Two Point Equidistant\"],
    PARAMETER[\"Latitude of 1st point\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Longitude of 1st point\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Latitude of 2nd point\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Longitude of 2nd point\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"False easting\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Two_Point_Equidistant\"],
PARAMETER[\"Latitude_Of_1st_Point\",1],
PARAMETER[\"Longitude_Of_1st_Point\",2],
PARAMETER[\"Latitude_Of_2nd_Point\",3],
PARAMETER[\"Longitude_Of_2nd_Point\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_tmg_export() {
    let conv = Conversion::create_tunisia_mining_grid(&props(), 1.0, 2.0, 3.0, 4.0).expect("tmg");
    assert!(conv.validate_parameters().is_empty());
    assert!(conv.export_to_proj_string().is_err());
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Tunisia Mining Grid\",
    METHOD[\"Tunisia Mining Grid\",
        ID[\"EPSG\",9816]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Easting at false origin\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Tunisia_Mining_Grid\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_aea_export() {
    let conv =
        Conversion::create_albers_equal_area(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0).expect("aea");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=aea +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Albers Equal Area\",
    METHOD[\"Albers Equal Area\",
        ID[\"EPSG\",9822]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Latitude of 1st standard parallel\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Latitude of 2nd standard parallel\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8824]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Albers_Conic_Equal_Area\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"standard_parallel_1\",3],
PARAMETER[\"standard_parallel_2\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_azimuthal_equidistant_export() {
    let conv =
        Conversion::create_azimuthal_equidistant(&props(), 1.0, 2.0, 3.0, 4.0).expect("aeqd");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=aeqd +lat_0=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Azimuthal Equidistant\",
    METHOD[\"Azimuthal Equidistant\",
        ID[\"EPSG\",1125]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Azimuthal_Equidistant\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_guam_projection_export() {
    let conv = Conversion::create_guam_projection(&props(), 1.0, 2.0, 3.0, 4.0).expect("guam");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=aeqd +guam +lat_0=1 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Guam Projection\",
    METHOD[\"Guam Projection\",
        ID[\"EPSG\",9831]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert!(conv
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

#[test]
fn operation_lambert_cylindrical_equal_area_spherical_export() {
    let conv =
        Conversion::create_lambert_cylindrical_equal_area_spherical(&props(), 1.0, 2.0, 3.0, 4.0)
            .expect("cea sphere");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=cea +R_A +lat_ts=1 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Cylindrical Equal Area (Spherical)\",
    METHOD[\"Lambert Cylindrical Equal Area (Spherical)\",
        ID[\"EPSG\",9834]],
    PARAMETER[\"Latitude of 1st standard parallel\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Cylindrical_Equal_Area\"],
PARAMETER[\"standard_parallel_1\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_lambert_cylindrical_equal_area_export() {
    let conv = Conversion::create_lambert_cylindrical_equal_area(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("cea");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=cea +lat_ts=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Cylindrical Equal Area\",
    METHOD[\"Lambert Cylindrical Equal Area\",
        ID[\"EPSG\",9835]],
    PARAMETER[\"Latitude of 1st standard parallel\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Cylindrical_Equal_Area\"],
PARAMETER[\"standard_parallel_1\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_lcc1sp_export() {
    let conv = Conversion::create_lambert_conic_conformal_1sp(&props(), 1.0, 2.0, 3.0, 4.0, 5.0)
        .expect("lcc1sp");
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
fn operation_lcc1sp_variant_b_export() {
    let conv = Conversion::create_lambert_conic_conformal_1sp_variant_b(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
    )
    .expect("lcc1sp b");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=lcc +lat_1=1 +k_0=2 +lat_0=3 +lon_0=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Conic Conformal (1SP variant B)\",
    METHOD[\"Lambert Conic Conformal (1SP variant B)\",
        ID[\"EPSG\",1102]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Scale factor at natural origin\",2,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8805]],
    PARAMETER[\"Latitude of false origin\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
}

#[test]
fn operation_lcc2sp_export() {
    let conv =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
            .expect("lcc2sp");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=lcc +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Conic Conformal (2SP)\",
    METHOD[\"Lambert Conic Conformal (2SP)\",
        ID[\"EPSG\",9802]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Latitude of 1st standard parallel\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Latitude of 2nd standard parallel\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8824]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Lambert_Conformal_Conic_2SP\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"standard_parallel_1\",3],
PARAMETER[\"standard_parallel_2\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_lcc2sp_is_equivalent_to_parallels_switched() {
    let conv1 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
            .expect("lcc2sp");
    let conv2 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 4.0, 3.0, 5.0, 6.0)
            .expect("lcc2sp switched");
    assert!(conv1.is_equivalent_to_with(&conv2, Equivalence::Equivalent));
    let conv3 =
        Conversion::create_lambert_conic_conformal_2sp(&props(), 1.0, 2.0, 3.0, 3.0, 5.0, 6.0)
            .expect("lcc2sp equal parallels");
    assert!(!conv1.is_equivalent_to_with(&conv3, Equivalence::Equivalent));
}

#[test]
fn operation_lcc2sp_michigan_export() {
    let conv = Conversion::create_lambert_conic_conformal_2sp_michigan(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("michigan");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=lcc +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6 +k_0=7"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Conic Conformal (2SP Michigan)\",
    METHOD[\"Lambert Conic Conformal (2SP Michigan)\",
        ID[\"EPSG\",1051]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Latitude of 1st standard parallel\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Latitude of 2nd standard parallel\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8824]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]],
    PARAMETER[\"Ellipsoid scaling factor\",7,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",1038]]]"
    );
    assert!(conv
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt1Gdal))
        .is_err());
}

#[test]
fn operation_lcc2sp_belgium_export() {
    let conv = Conversion::create_lambert_conic_conformal_2sp_belgium(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
    )
    .expect("belgium");
    assert_eq!(
        proj(&conv),
        "+proj=lcc +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Conic Conformal (2SP Belgium)\",
    METHOD[\"Lambert Conic Conformal (2SP Belgium)\",
        ID[\"EPSG\",9803]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Latitude of 1st standard parallel\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Latitude of 2nd standard parallel\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8824]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Lambert_Conformal_Conic_2SP_Belgium\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"standard_parallel_1\",3],
PARAMETER[\"standard_parallel_2\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_cassini_soldner_export() {
    let conv = Conversion::create_cassini_soldner(&props(), 1.0, 2.0, 4.0, 5.0).expect("cass");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=cass +lat_0=1 +lon_0=2 +x_0=4 +y_0=5");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Cassini-Soldner\",
    METHOD[\"Cassini-Soldner\",
        ID[\"EPSG\",9806]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Cassini_Soldner\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_equidistant_conic_export() {
    let conv =
        Conversion::create_equidistant_conic(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0).expect("eqdc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=eqdc +lat_0=1 +lon_0=2 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Equidistant Conic\",
    METHOD[\"Equidistant Conic\",
        ID[\"EPSG\",1119]],
    PARAMETER[\"Latitude of false origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8821]],
    PARAMETER[\"Longitude of false origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8822]],
    PARAMETER[\"Latitude of 1st standard parallel\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Latitude of 2nd standard parallel\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8824]],
    PARAMETER[\"Easting at false origin\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8826]],
    PARAMETER[\"Northing at false origin\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8827]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Equidistant_Conic\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"standard_parallel_1\",3],
PARAMETER[\"standard_parallel_2\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_eckert_export() {
    let numbers = ["", "1", "2", "3", "4", "5", "6"];
    let latin = ["", "I", "II", "III", "IV", "V", "VI"];
    for n in 1..=6_u8 {
        let conv = Conversion::create_eckert(&props(), n, 1.0, 2.0, 3.0).expect("eckert");
        let i = usize::from(n);
        assert!(conv.validate_parameters().is_empty());
        assert_eq!(
            proj(&conv),
            format!("+proj=eck{} +lon_0=1 +x_0=2 +y_0=3", numbers[i])
        );
        assert_eq!(
            wkt2(&conv),
            format!(
                "\
CONVERSION[\"Eckert {r}\",
    METHOD[\"Eckert {r}\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",2,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]",
                r = latin[i]
            )
        );
        assert_eq!(
            wkt1(&conv),
            format!(
                "\
PROJECTION[\"Eckert_{r}\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"false_easting\",2],
PARAMETER[\"false_northing\",3]",
                r = latin[i]
            )
        );
    }
}

#[test]
fn operation_create_equidistant_cylindrical() {
    let conv =
        Conversion::create_equidistant_cylindrical(&props(), 1.0, 2.0, 3.0, 4.0).expect("eqc");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=eqc +lat_ts=1 +lat_0=0 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Equidistant Cylindrical\",
    METHOD[\"Equidistant Cylindrical\",
        ID[\"EPSG\",1028]],
    PARAMETER[\"Latitude of 1st standard parallel\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Equirectangular\"],
PARAMETER[\"standard_parallel_1\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_create_equidistant_cylindrical_spherical() {
    let conv = Conversion::create_equidistant_cylindrical_spherical(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("eqc sphere");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=eqc +lat_ts=1 +lat_0=0 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Equidistant Cylindrical (Spherical)\",
    METHOD[\"Equidistant Cylindrical (Spherical)\",
        ID[\"EPSG\",1029]],
    PARAMETER[\"Latitude of 1st standard parallel\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Equirectangular\"],
PARAMETER[\"standard_parallel_1\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_gall_export() {
    let conv = Conversion::create_gall(&props(), 1.0, 2.0, 3.0).expect("gall");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=gall +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Gall Stereographic\",
    METHOD[\"Gall Stereographic\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",2,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Gall_Stereographic\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"false_easting\",2],
PARAMETER[\"false_northing\",3]"
    );
}

#[test]
fn operation_goode_homolosine_export() {
    let conv = Conversion::create_goode_homolosine(&props(), 1.0, 2.0, 3.0).expect("goode");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=goode +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Goode Homolosine\",
    METHOD[\"Goode Homolosine\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",2,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Goode_Homolosine\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"false_easting\",2],
PARAMETER[\"false_northing\",3]"
    );
}

#[test]
fn operation_interrupted_goode_homolosine_export() {
    let conv =
        Conversion::create_interrupted_goode_homolosine(&props(), 1.0, 2.0, 3.0).expect("igh");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=igh +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Interrupted Goode Homolosine\",
    METHOD[\"Interrupted Goode Homolosine\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",2,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Interrupted_Goode_Homolosine\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"false_easting\",2],
PARAMETER[\"false_northing\",3]"
    );
}

#[test]
fn operation_geostationary_satellite_sweep_y_export() {
    let conv = Conversion::create_geostationary_satellite_sweep_y(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("geos y");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=geos +lon_0=1 +h=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Geostationary Satellite (Sweep Y)\",
    METHOD[\"Geostationary Satellite (Sweep Y)\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Satellite Height\",2,
        LENGTHUNIT[\"metre\",1,
            ID[\"EPSG\",9001]]],
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
PROJECTION[\"Geostationary_Satellite\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"satellite_height\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_gnomonic_export() {
    let conv = Conversion::create_gnomonic(&props(), 1.0, 2.0, 4.0, 5.0).expect("gnom");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=gnom +lat_0=1 +lon_0=2 +x_0=4 +y_0=5");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Gnomonic\",
    METHOD[\"Gnomonic\"],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Gnomonic\"],
PARAMETER[\"latitude_of_origin\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_hotine_oblique_mercator_variant_a_export() {
    let conv = Conversion::create_hotine_oblique_mercator_variant_a(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom a");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=omerc +no_uoff +lat_0=1 +lonc=2 +alpha=3 +gamma=4 +k=5 +x_0=6 +y_0=7"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Hotine Oblique Mercator (variant A)\",
    METHOD[\"Hotine Oblique Mercator (variant A)\",
        ID[\"EPSG\",9812]],
    PARAMETER[\"Latitude of projection centre\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Longitude of projection centre\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8812]],
    PARAMETER[\"Azimuth at projection centre\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8813]],
    PARAMETER[\"Angle from Rectified to Skew Grid\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8814]],
    PARAMETER[\"Scale factor at projection centre\",5,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8815]],
    PARAMETER[\"False easting\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",7,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Hotine_Oblique_Mercator\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"azimuth\",3],
PARAMETER[\"rectified_grid_angle\",4],
PARAMETER[\"scale_factor\",5],
PARAMETER[\"false_easting\",6],
PARAMETER[\"false_northing\",7]"
    );
}

#[test]
fn operation_hotine_oblique_mercator_variant_a_export_swiss_mercator() {
    let conv = Conversion::create_hotine_oblique_mercator_variant_a(
        &props(),
        1.0,
        2.0,
        90.0,
        90.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom a swiss");
    assert_eq!(
        proj(&conv),
        "+proj=somerc +lat_0=1 +lon_0=2 +k_0=5 +x_0=6 +y_0=7"
    );
}

#[test]
fn operation_hotine_oblique_mercator_variant_b_export() {
    let conv = Conversion::create_hotine_oblique_mercator_variant_b(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom b");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=omerc +lat_0=1 +lonc=2 +alpha=3 +gamma=4 +k=5 +x_0=6 +y_0=7"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Hotine Oblique Mercator (variant B)\",
    METHOD[\"Hotine Oblique Mercator (variant B)\",
        ID[\"EPSG\",9815]],
    PARAMETER[\"Latitude of projection centre\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Longitude of projection centre\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8812]],
    PARAMETER[\"Azimuth at projection centre\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8813]],
    PARAMETER[\"Angle from Rectified to Skew Grid\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8814]],
    PARAMETER[\"Scale factor at projection centre\",5,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8815]],
    PARAMETER[\"Easting at projection centre\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8816]],
    PARAMETER[\"Northing at projection centre\",7,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8817]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Hotine_Oblique_Mercator_Azimuth_Center\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"azimuth\",3],
PARAMETER[\"rectified_grid_angle\",4],
PARAMETER[\"scale_factor\",5],
PARAMETER[\"false_easting\",6],
PARAMETER[\"false_northing\",7]"
    );
}

#[test]
fn operation_hotine_oblique_mercator_variant_b_export_swiss_mercator() {
    let conv = Conversion::create_hotine_oblique_mercator_variant_b(
        &props(),
        1.0,
        2.0,
        90.0,
        90.0,
        5.0,
        6.0,
        7.0,
    )
    .expect("hom b swiss");
    assert_eq!(
        proj(&conv),
        "+proj=somerc +lat_0=1 +lon_0=2 +k_0=5 +x_0=6 +y_0=7"
    );
}

#[test]
fn operation_hotine_oblique_mercator_two_point_natural_origin_export() {
    let conv = Conversion::create_hotine_oblique_mercator_two_point_natural_origin(
        &props(),
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
    )
    .expect("hom 2pt");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=omerc +lat_0=1 +lat_1=2 +lon_1=3 +lat_2=4 +lon_2=5 +k=6 +x_0=7 +y_0=8"
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    assert_eq!(
        conv.export_to_wkt(&mut formatter).expect("wkt2"),
        "\
CONVERSION[\"Hotine Oblique Mercator Two Point Natural Origin\",
    METHOD[\"Hotine Oblique Mercator Two Point Natural Origin\"],
    PARAMETER[\"Latitude of projection centre\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Latitude of 1st point\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Longitude of 1st point\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Latitude of 2nd point\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Longitude of 2nd point\",5,
        ANGLEUNIT[\"degree\",0.0174532925199433]],
    PARAMETER[\"Scale factor at projection centre\",6,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8815]],
    PARAMETER[\"Easting at projection centre\",7,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8816]],
    PARAMETER[\"Northing at projection centre\",8,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8817]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Hotine_Oblique_Mercator_Two_Point_Natural_Origin\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"latitude_of_point_1\",2],
PARAMETER[\"longitude_of_point_1\",3],
PARAMETER[\"latitude_of_point_2\",4],
PARAMETER[\"longitude_of_point_2\",5],
PARAMETER[\"scale_factor\",6],
PARAMETER[\"false_easting\",7],
PARAMETER[\"false_northing\",8]"
    );
}

#[test]
fn operation_laborde_oblique_mercator_export() {
    let conv = Conversion::create_laborde_oblique_mercator(&props(), 1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
        .expect("laborde");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=labrd +lat_0=1 +lon_0=2 +azi=3 +k=4 +x_0=5 +y_0=6"
    );
    let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
    formatter.simul_cur_node_has_id();
    assert_eq!(
        conv.export_to_wkt(&mut formatter).expect("wkt2"),
        "\
CONVERSION[\"Laborde Oblique Mercator\",
    METHOD[\"Laborde Oblique Mercator\",
        ID[\"EPSG\",9813]],
    PARAMETER[\"Latitude of projection centre\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Longitude of projection centre\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8812]],
    PARAMETER[\"Azimuth at projection centre\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8813]],
    PARAMETER[\"Scale factor at projection centre\",4,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8815]],
    PARAMETER[\"False easting\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Laborde_Oblique_Mercator\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"azimuth\",3],
PARAMETER[\"scale_factor\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_imw_polyconic_export() {
    let conv =
        Conversion::create_international_map_world_polyconic(&props(), 1.0, 3.0, 4.0, 5.0, 6.0)
            .expect("imw");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=imw_p +lon_0=1 +lat_1=3 +lat_2=4 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"International Map of the World Polyconic\",
    METHOD[\"International Map of the World Polyconic\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"Latitude of 1st point\",3,
        ANGLEUNIT[\"degree\",0.0174532925199433,
            ID[\"EPSG\",9122]]],
    PARAMETER[\"Latitude of 2nd point\",4,
        ANGLEUNIT[\"degree\",0.0174532925199433,
            ID[\"EPSG\",9122]]],
    PARAMETER[\"False easting\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"International_Map_of_the_World_Polyconic\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"Latitude_Of_1st_Point\",3],
PARAMETER[\"Latitude_Of_2nd_Point\",4],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_krovak_north_oriented_export() {
    let conv = Conversion::create_krovak_north_oriented(
        &props(),
        49.5,
        42.5,
        30.288_139_752_777_8,
        78.5,
        0.9999,
        5.0,
        6.0,
    )
    .expect("krovak north");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397527778 +k=0.9999 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Krovak (North Orientated)\",
    METHOD[\"Krovak (North Orientated)\",
        ID[\"EPSG\",1041]],
    PARAMETER[\"Latitude of projection centre\",49.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Longitude of origin\",42.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8833]],
    PARAMETER[\"Co-latitude of cone axis\",30.2881397527778,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",1036]],
    PARAMETER[\"Latitude of pseudo standard parallel\",78.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8818]],
    PARAMETER[\"Scale factor on pseudo standard parallel\",0.9999,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8819]],
    PARAMETER[\"False easting\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Krovak\"],
PARAMETER[\"latitude_of_center\",49.5],
PARAMETER[\"longitude_of_center\",42.5],
PARAMETER[\"azimuth\",30.2881397527778],
PARAMETER[\"pseudo_standard_parallel_1\",78.5],
PARAMETER[\"scale_factor\",0.9999],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_krovak_export() {
    let conv = Conversion::create_krovak(
        &props(),
        49.5,
        42.5,
        30.288_139_752_777_8,
        78.5,
        0.9999,
        5.0,
        6.0,
    )
    .expect("krovak");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=krovak +axis=swu +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397527778 +k=0.9999 +x_0=5 +y_0=6"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Krovak\",
    METHOD[\"Krovak\",
        ID[\"EPSG\",9819]],
    PARAMETER[\"Latitude of projection centre\",49.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8811]],
    PARAMETER[\"Longitude of origin\",42.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8833]],
    PARAMETER[\"Co-latitude of cone axis\",30.2881397527778,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",1036]],
    PARAMETER[\"Latitude of pseudo standard parallel\",78.5,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8818]],
    PARAMETER[\"Scale factor on pseudo standard parallel\",0.9999,
        SCALEUNIT[\"unity\",1],
        ID[\"EPSG\",8819]],
    PARAMETER[\"False easting\",5,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",6,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Krovak\"],
PARAMETER[\"latitude_of_center\",49.5],
PARAMETER[\"longitude_of_center\",42.5],
PARAMETER[\"azimuth\",30.2881397527778],
PARAMETER[\"pseudo_standard_parallel_1\",78.5],
PARAMETER[\"scale_factor\",0.9999],
PARAMETER[\"false_easting\",5],
PARAMETER[\"false_northing\",6]"
    );
}

#[test]
fn operation_lambert_azimuthal_equal_area_export() {
    let conv = Conversion::create_lambert_azimuthal_equal_area(&props(), 1.0, 2.0, 3.0, 4.0)
        .expect("laea");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=laea +lat_0=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Lambert Azimuthal Equal Area\",
    METHOD[\"Lambert Azimuthal Equal Area\",
        ID[\"EPSG\",9820]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Lambert_Azimuthal_Equal_Area\"],
PARAMETER[\"latitude_of_center\",1],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_miller_cylindrical_export() {
    let conv = Conversion::create_miller_cylindrical(&props(), 2.0, 3.0, 4.0).expect("miller");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=mill +R_A +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Miller Cylindrical\",
    METHOD[\"Miller Cylindrical\"],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Miller_Cylindrical\"],
PARAMETER[\"longitude_of_center\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_wkt1_import_mercator_variant_a() {
    let obj = WktParser::new()
        .create_from_wkt(concat!(
            "PROJCS[\"test\",\n",
            "    GEOGCS[\"WGS 84\",\n",
            "        DATUM[\"WGS 1984\",\n",
            "            SPHEROID[\"WGS 84\",6378137,298.257223563]],\n",
            "        PRIMEM[\"Greenwich\",0],\n",
            "        UNIT[\"degree\",0.0174532925199433]],\n",
            "    PROJECTION[\"Mercator_1SP\"],\n",
            "    PARAMETER[\"central_meridian\",1],\n",
            "    PARAMETER[\"scale_factor\",2],\n",
            "    PARAMETER[\"false_easting\",3],\n",
            "    PARAMETER[\"false_northing\",4],\n",
            "    UNIT[\"metre\",1]]"
        ))
        .expect("wkt");
    let conversion = obj.projected().expect("projected").deriving_conversion();
    let conv_ref =
        Conversion::create_mercator_variant_a(&props().name("unknown"), 0.0, 1.0, 2.0, 3.0, 4.0)
            .expect("merc a");
    assert_eq!(wkt2(conversion), wkt2(&conv_ref));
}

#[test]
fn operation_wkt1_import_mercator_variant_a_that_is_variant_b() {
    let obj = WktParser::new()
        .create_from_wkt(concat!(
            "PROJCS[\"test\",\n",
            "    GEOGCS[\"WGS 84\",\n",
            "        DATUM[\"WGS 1984\",\n",
            "            SPHEROID[\"WGS 84\",6378137,298.257223563]],\n",
            "        PRIMEM[\"Greenwich\",0],\n",
            "        UNIT[\"degree\",0.0174532925199433]],\n",
            "    PROJECTION[\"Mercator_1SP\"],\n",
            "    PARAMETER[\"latitude_of_origin\",-1],\n",
            "    PARAMETER[\"central_meridian\",2],\n",
            "    PARAMETER[\"scale_factor\",1],\n",
            "    PARAMETER[\"false_easting\",3],\n",
            "    PARAMETER[\"false_northing\",4],\n",
            "    UNIT[\"metre\",1]]"
        ))
        .expect("wkt");
    let conversion = obj.projected().expect("projected").deriving_conversion();
    let conv_ref =
        Conversion::create_mercator_variant_b(&props().name("unnamed"), -1.0, 2.0, 3.0, 4.0)
            .expect("merc b");
    assert!(conversion.is_equivalent_to_with(&conv_ref, Equivalence::Equivalent));
}

#[test]
fn operation_mercator_variant_b_export() {
    let conv = Conversion::create_mercator_variant_b(&props(), 1.0, 2.0, 3.0, 4.0).expect("merc b");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=merc +lat_ts=1 +lon_0=2 +x_0=3 +y_0=4");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Mercator (variant B)\",
    METHOD[\"Mercator (variant B)\",
        ID[\"EPSG\",9805]],
    PARAMETER[\"Latitude of 1st standard parallel\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8823]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
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
PROJECTION[\"Mercator_2SP\"],
PARAMETER[\"standard_parallel_1\",1],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",3],
PARAMETER[\"false_northing\",4]"
    );
}

#[test]
fn operation_odd_mercator_1sp_with_non_null_latitude() {
    let obj = WktParser::new()
        .create_from_wkt(concat!(
            "PROJCS[\"unnamed\",\n",
            "    GEOGCS[\"WGS 84\",\n",
            "        DATUM[\"WGS_1984\",\n",
            "            SPHEROID[\"WGS 84\",6378137,298.257223563,\n",
            "                AUTHORITY[\"EPSG\",\"7030\"]],\n",
            "            AUTHORITY[\"EPSG\",\"6326\"]],\n",
            "        PRIMEM[\"Greenwich\",0,\n",
            "            AUTHORITY[\"EPSG\",\"8901\"]],\n",
            "        UNIT[\"degree\",0.0174532925199433,\n",
            "            AUTHORITY[\"EPSG\",\"9122\"]],\n",
            "        AUTHORITY[\"EPSG\",\"4326\"]],\n",
            "    PROJECTION[\"Mercator_1SP\"],\n",
            "    PARAMETER[\"latitude_of_origin\",30],\n",
            "    PARAMETER[\"central_meridian\",0],\n",
            "    PARAMETER[\"scale_factor\",0.99],\n",
            "    PARAMETER[\"false_easting\",0],\n",
            "    PARAMETER[\"false_northing\",0],\n",
            "    UNIT[\"metre\",1],\n",
            "    AXIS[\"Easting\",EAST],\n",
            "    AXIS[\"Northing\",NORTH]]"
        ))
        .expect("wkt");
    let crs = obj.projected().expect("projected");
    assert!(crs.export_to_proj_string().is_err());
}

#[test]
fn operation_odd_mercator_2sp_with_latitude_of_origin() {
    let obj = WktParser::new()
        .create_from_wkt(concat!(
            "PROJCS[\"unnamed\",\n",
            "    GEOGCS[\"WGS 84\",\n",
            "        DATUM[\"WGS_1984\",\n",
            "            SPHEROID[\"WGS 84\",6378137,298.257223563,\n",
            "                AUTHORITY[\"EPSG\",\"7030\"]],\n",
            "            AUTHORITY[\"EPSG\",\"6326\"]],\n",
            "        PRIMEM[\"Greenwich\",0,\n",
            "            AUTHORITY[\"EPSG\",\"8901\"]],\n",
            "        UNIT[\"degree\",0.0174532925199433,\n",
            "            AUTHORITY[\"EPSG\",\"9122\"]],\n",
            "        AUTHORITY[\"EPSG\",\"4326\"]],\n",
            "    PROJECTION[\"Mercator_2SP\"],\n",
            "    PARAMETER[\"standard_parallel_1\",30],\n",
            "    PARAMETER[\"latitude_of_origin\",40],\n",
            "    PARAMETER[\"central_meridian\",0],\n",
            "    PARAMETER[\"false_easting\",0],\n",
            "    PARAMETER[\"false_northing\",0],\n",
            "    UNIT[\"metre\",1],\n",
            "    AXIS[\"Easting\",EAST],\n",
            "    AXIS[\"Northing\",NORTH]]"
        ))
        .expect("wkt");
    let crs = obj.projected().expect("projected");
    assert!(crs.export_to_proj_string().is_err());
}

#[test]
fn operation_mercator_spherical_export() {
    let conv = Conversion::create_mercator_spherical(&props(), 1.0, 2.0, 3.0, 4.0).expect("merc s");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=merc +R_C +lat_0=1 +lon_0=2 +x_0=3 +y_0=4"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Mercator (Spherical)\",
    METHOD[\"Mercator (Spherical)\",
        ID[\"EPSG\",1026]],
    PARAMETER[\"Latitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8801]],
    PARAMETER[\"Longitude of natural origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",4,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
}

#[test]
fn operation_mollweide_export() {
    let conv = Conversion::create_mollweide(&props(), 1.0, 2.0, 3.0).expect("moll");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(proj(&conv), "+proj=moll +lon_0=1 +x_0=2 +y_0=3");
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Mollweide\",
    METHOD[\"Mollweide\"],
    PARAMETER[\"Longitude of natural origin\",1,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8802]],
    PARAMETER[\"False easting\",2,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8806]],
    PARAMETER[\"False northing\",3,
        LENGTHUNIT[\"metre\",1],
        ID[\"EPSG\",8807]]]"
    );
    assert_eq!(
        wkt1(&conv),
        "\
PROJECTION[\"Mollweide\"],
PARAMETER[\"central_meridian\",1],
PARAMETER[\"false_easting\",2],
PARAMETER[\"false_northing\",3]"
    );
}

#[test]
fn operation_polar_stereographic_variant_a_export() {
    let conv = Conversion::create_polar_stereographic_variant_a(&props(), 90.0, 2.0, 3.0, 4.0, 5.0)
        .expect("stere a");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=stere +lat_0=90 +lon_0=2 +k=3 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Polar Stereographic (variant A)\",
    METHOD[\"Polar Stereographic (variant A)\",
        ID[\"EPSG\",9810]],
    PARAMETER[\"Latitude of natural origin\",90,
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
PROJECTION[\"Polar_Stereographic\"],
PARAMETER[\"latitude_of_origin\",90],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"scale_factor\",3],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_polar_stereographic_variant_b_export_positive_lat() {
    let conv = Conversion::create_polar_stereographic_variant_b(&props(), 70.0, 2.0, 4.0, 5.0)
        .expect("stere b");
    assert!(conv.validate_parameters().is_empty());
    assert_eq!(
        proj(&conv),
        "+proj=stere +lat_0=90 +lat_ts=70 +lon_0=2 +x_0=4 +y_0=5"
    );
    assert_eq!(
        wkt2(&conv),
        "\
CONVERSION[\"Polar Stereographic (variant B)\",
    METHOD[\"Polar Stereographic (variant B)\",
        ID[\"EPSG\",9829]],
    PARAMETER[\"Latitude of standard parallel\",70,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8832]],
    PARAMETER[\"Longitude of origin\",2,
        ANGLEUNIT[\"degree\",0.0174532925199433],
        ID[\"EPSG\",8833]],
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
PROJECTION[\"Polar_Stereographic\"],
PARAMETER[\"latitude_of_origin\",70],
PARAMETER[\"central_meridian\",2],
PARAMETER[\"false_easting\",4],
PARAMETER[\"false_northing\",5]"
    );
}

#[test]
fn operation_polar_stereographic_variant_b_export_negative_lat() {
    let conv = Conversion::create_polar_stereographic_variant_b(&props(), -70.0, 2.0, 4.0, 5.0)
        .expect("stere b south");
    assert_eq!(
        proj(&conv),
        "+proj=stere +lat_0=-90 +lat_ts=-70 +lon_0=2 +x_0=4 +y_0=5"
    );
}
