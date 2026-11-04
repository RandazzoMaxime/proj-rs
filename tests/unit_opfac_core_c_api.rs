//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`, the cases that need an
//! API this crate exposes only with the `isPROJInstantiable` and canonical bound CRS work:
//! `isPROJInstantiable` and `createOperation_on_crs_with_canonical_bound_crs`.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The other cases of the lot
//! are listed in `docs/PORTING-lotc-api.md`.

use proj_rs::iso19111::{
    AuthorityFactory, BoundCRS, CoordinateOperation, CoordinateOperationFactory, Crs,
    DatabaseContext, GridAvailabilityUse, OpProps, OperationMethod, OperationParameter,
    ParameterValue, Transformation, WktConvention, WktFormatter,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn wkt2_2019(operation: &CoordinateOperation) -> String {
    operation
        .export_to_wkt(&mut WktFormatter::create(WktConvention::Wkt2_2019))
        .expect("WKT")
}

/// TEST(operation, isPROJInstantiable)
#[test]
fn operation_is_proj_instantiable() {
    let db = database();
    let policy = GridAvailabilityUse::UseForSorting;

    // Geocentric translations need no grid.
    let geocentric = Transformation::create_with_operation_method(
        &OpProps::new(),
        epsg(&db, "4269"),
        epsg(&db, "4326"),
        None,
        OperationMethod::create(
            "Geocentric translations (geog2D domain)",
            vec![
                OperationParameter::create("X-axis translation").with_identifier("EPSG", "8605"),
                OperationParameter::create("Y-axis translation").with_identifier("EPSG", "8606"),
                OperationParameter::create("Z-axis translation").with_identifier("EPSG", "8607"),
            ],
        )
        .with_identifier("EPSG", "9603"),
        vec![
            ParameterValue::length(1.0),
            ParameterValue::length(2.0),
            ParameterValue::length(3.0),
        ],
        None,
    )
    .expect("geocentric translations");
    assert!(CoordinateOperation::Transformation(geocentric).is_proj_instantiable(&db, policy));

    // Missing grid.
    let ntv2 = Transformation::create_with_operation_method(
        &OpProps::new(),
        epsg(&db, "4807"),
        epsg(&db, "4326"),
        None,
        OperationMethod::create(
            "NTv2",
            vec![
                OperationParameter::create("Latitude and longitude difference file")
                    .with_identifier("EPSG", "8656"),
            ],
        )
        .with_identifier("EPSG", "9615"),
        vec![ParameterValue::filename("foo.gsb")],
        None,
    )
    .expect("NTv2");
    assert!(!CoordinateOperation::Transformation(ntv2).is_proj_instantiable(&db, policy));

    // Unsupported method.
    let unsupported = Transformation::create(
        &OpProps::new(),
        epsg(&db, "4269"),
        epsg(&db, "4326"),
        None,
        "",
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("unsupported method");
    assert!(!CoordinateOperation::Transformation(unsupported).is_proj_instantiable(&db, policy));
}

/// TEST(operation, createOperation_on_crs_with_canonical_bound_crs)
#[test]
fn operation_create_operation_on_crs_with_canonical_bound_crs() {
    let db = database();
    let bound = BoundCRS::from_towgs84(epsg(&db, "4267"), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0])
        .expect("bound CRS");
    let crs = bound.base_crs_with_canonical_bound_crs();
    let factory = CoordinateOperationFactory::create();
    let transformation = bound.transformation();
    {
        let op = factory
            .create_operation(&crs, &epsg(&db, "4326"))
            .expect("operations")
            .expect("operation");
        assert!(op.is_equivalent_to(transformation));
        assert_eq!(wkt2_2019(&op), wkt2_2019(transformation));
    }
    {
        let op = factory
            .create_operation(&epsg(&db, "4326"), &crs)
            .expect("operations")
            .expect("operation");
        let inverse = transformation.inverse();
        assert!(op.is_equivalent_to(&inverse));
        assert_eq!(wkt2_2019(&op), wkt2_2019(&inverse));
    }
}
