//! PROJ 9.9.0 `test/unit/test_c_api.cpp`: `proj_crs_alter_cs_linear_unit`,
//! `proj_crs_alter_cs_angular_unit`, `proj_crs_alter_parameters_linear_unit`, and
//! `proj_crs_alter_geodetic_crs`, on the WGS 84 / UTM zone 31N and EPSG:4326 fixtures.

use proj_rs::{
    create_from_user_input, Axis, Conversion, Crs, DatabaseContext, Equivalence, GeodeticCRS,
    GeographicCRS, OpProps, ParameterLinearUnit, ProjectedCRS, UnitKind, UnitOfMeasure,
    WktConvention, WktFormatter, WktLines,
};

fn utm_31n() -> ProjectedCRS {
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        Conversion::create_utm_north(&OpProps::new(), 31).expect("utm"),
        proj_rs::CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

fn my_unit(kind: UnitKind) -> UnitOfMeasure {
    UnitOfMeasure::new("my unit", 2.0, kind).with_identifier("my auth", "my code")
}

fn axes(crs: &Crs) -> Vec<Axis> {
    match crs {
        Crs::Projected(projected) => projected.coordinate_system().axes().to_vec(),
        Crs::Geographic(geographic) => geographic.coordinate_system().axes().to_vec(),
        other => panic!("expected a projected or geographic CRS: {other:?}"),
    }
}

/// `TEST_F(CApi, proj_crs_alter_cs_linear_unit)`
#[test]
fn capi_crs_alter_cs_linear_unit() {
    let altered = Crs::Projected(utm_31n()).alter_cs_linear_unit(&UnitOfMeasure::new(
        "my unit",
        2.0,
        UnitKind::Linear,
    ));
    let unit = axes(&altered)[0].unit().clone();
    assert_eq!(unit.name(), "my unit");
    assert_eq!(unit.conversion_to_si(), 2.0);

    let altered = Crs::Projected(utm_31n()).alter_cs_linear_unit(&my_unit(UnitKind::Linear));
    let unit = axes(&altered)[0].unit().clone();
    assert_eq!(unit.conversion_to_si(), 2.0);
    assert_eq!(unit.name(), "my unit");
    assert_eq!(unit.code_space(), "my auth");
    assert_eq!(unit.code(), "my code");
}

/// `TEST_F(CApi, proj_crs_alter_cs_angular_unit)`
#[test]
fn capi_crs_alter_cs_angular_unit() {
    let epsg_4326 = Crs::Geographic(GeographicCRS::epsg_4326());
    let altered =
        epsg_4326.alter_cs_angular_unit(&UnitOfMeasure::new("my unit", 2.0, UnitKind::Angular));
    let unit = axes(&altered)[0].unit().clone();
    assert_eq!(unit.conversion_to_si(), 2.0);
    assert_eq!(unit.name(), "my unit");

    let altered = epsg_4326.alter_cs_angular_unit(&my_unit(UnitKind::Angular));
    let unit = axes(&altered)[0].unit().clone();
    assert_eq!(unit.conversion_to_si(), 2.0);
    assert_eq!(unit.name(), "my unit");
    assert_eq!(unit.code_space(), "my auth");
    assert_eq!(unit.code(), "my code");
}

/// `TEST_F(CApi, proj_crs_alter_parameters_linear_unit)`
#[test]
fn capi_crs_alter_parameters_linear_unit() {
    let wkt = |crs: &ProjectedCRS| {
        let mut formatter = WktFormatter::create(WktConvention::Wkt2_2019);
        formatter.set_lines(WktLines::Single);
        Crs::Projected(crs.clone())
            .export_to_wkt(&mut formatter)
            .expect("wkt")
    };

    let relabeled = utm_31n()
        .alter_parameters_linear_unit(ParameterLinearUnit::Relabel(my_unit(UnitKind::Linear)));
    let text = wkt(&relabeled);
    assert!(text.contains("500000"), "{text}");
    assert!(text.contains("\"my unit\",2"), "{text}");

    let converted = utm_31n()
        .alter_parameters_linear_unit(ParameterLinearUnit::Convert(my_unit(UnitKind::Linear)));
    let text = wkt(&converted);
    assert!(text.contains("250000"), "{text}");
    assert!(text.contains("\"my unit\",2"), "{text}");
}

/// `TEST_F(CApi, proj_crs_alter_geodetic_crs)`
#[test]
fn capi_crs_alter_geodetic_crs() {
    let db = DatabaseContext::create().expect("proj.db");
    let new_crs = create_from_user_input("+proj=longlat +type=crs", &db).expect("longlat");
    let Crs::Geographic(new_geographic) = new_crs.clone() else {
        panic!("expected a geographic CRS: {new_crs:?}");
    };
    let new_geodetic = GeodeticCRS::Geographic(new_geographic);

    let projected = utm_31n();
    let geodetic = Crs::Geodetic(projected.base_crs().clone());
    let altered = geodetic.alter_geodetic_crs(&new_geodetic);
    assert!(altered.is_equivalent_to_with(&new_crs, Equivalence::Strict));

    let altered = Crs::Projected(projected.clone()).alter_geodetic_crs(&new_geodetic);
    let Crs::Projected(altered_projected) = &altered else {
        panic!("expected a projected CRS: {altered:?}");
    };
    assert!(Crs::Geodetic(altered_projected.base_crs().clone())
        .is_equivalent_to_with(&new_crs, Equivalence::Strict));

    let deprecated = Crs::Projected(projected).alter_name("new name (deprecated)");
    let altered = deprecated.alter_geodetic_crs(&new_geodetic);
    assert_eq!(altered.name(), "new name");
    assert!(altered.is_deprecated());
}
