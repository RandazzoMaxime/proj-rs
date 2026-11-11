//! PROJ 9.9.0 `test/unit/test_crs.cpp`: `TEST(crs, normalizeForVisualization_derivedprojected_operation)`.
//!
//! The derived projected CRS is `createDerivedProjectedCRSNorthingEasting` of the C++ test: the
//! UTM zone 31N base, a "PROJ unimplemented" deriving conversion, and northing-easting axes in
//! foot. Expected PROJ strings are copied from the C++ test.

use proj_rs::{
    CartesianCS, Conversion, CoordinateOperationFactory, CoordinateSystem, Crs, DatabaseContext,
    DerivedProjectedCRS, GeodeticCRS, GeographicCRS, OpProps, ProjLines, ProjectedCRS,
    UnitOfMeasure,
};

fn unimplemented_conversion(name: &str) -> Conversion {
    Conversion::create_with_method("PROJ unimplemented", "", "", Vec::new()).with_name(name)
}

/// `createProjected()` of the C++ test: WGS 84 / UTM zone 31N, EPSG:32631.
fn utm_31n() -> ProjectedCRS {
    let conversion = Conversion::create_utm_north(&OpProps::new(), 31).expect("utm");
    ProjectedCRS::create(
        "WGS 84 / UTM zone 31N",
        GeodeticCRS::Geographic(GeographicCRS::epsg_4326()),
        conversion,
        CartesianCS::create_easting_northing(UnitOfMeasure::metre()),
    )
    .with_identifier("EPSG", "32631")
}

/// `createDerivedProjectedCRSNorthingEasting()` of the C++ test.
fn derived_projected_northing_easting() -> DerivedProjectedCRS {
    DerivedProjectedCRS::create(
        "derived projectedCRS",
        utm_31n(),
        unimplemented_conversion("unnamed"),
        CoordinateSystem::Cartesian(CartesianCS::create_northing_easting(UnitOfMeasure::foot())),
    )
}

/// `TEST(crs, normalizeForVisualization_derivedprojected_operation)`: EPSG:4326 to the derived
/// projected CRS, before and after normalization.
#[test]
fn crs_normalize_for_visualization_derivedprojected_operation() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = Crs::DerivedProjected(derived_projected_northing_easting());
    let op = CoordinateOperationFactory::create()
        .create_operation(&Crs::Geographic(GeographicCRS::epsg_4326()), &crs)
        .expect("operation")
        .expect("an operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=unimplemented \
         +step +proj=unitconvert +xy_in=m +xy_out=ft \
         +step +proj=axisswap +order=2,1"
    );

    let normalized = op.normalize_for_visualization().expect("normalized");
    assert_eq!(
        normalized
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=unimplemented \
         +step +proj=unitconvert +xy_in=m +xy_out=ft"
    );
}
