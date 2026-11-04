//! PROJ 9.9.0 `test/unit/test_crs.cpp`, a geodetic CRS with several identifiers.

use proj_rs::{
    CartesianCS, CoordinateSystem, Crs, GeodeticCRS, GeodeticReferenceFrame, UnitOfMeasure,
    WktConvention, WktFormatter,
};

/// `TEST(crs, multiple_ID)`
#[test]
fn crs_multiple_id() {
    let crs = GeodeticCRS::create(
        "WGS 84",
        Some(GeodeticReferenceFrame::epsg_6326()),
        None,
        CoordinateSystem::Cartesian(CartesianCS::create_geocentric(UnitOfMeasure::metre())),
    )
    .expect("geocentric")
    .with_identifier("authorityA", "codeA")
    .with_identifier("authorityB", "codeB");
    let got = Crs::Geodetic(crs)
        .export_to_wkt(&mut WktFormatter::create(
            WktConvention::Wkt2_2019Simplified,
        ))
        .expect("wkt");
    assert_eq!(
        got,
        "GEODCRS[\"WGS 84\",\n    DATUM[\"World Geodetic System 1984\",\n        ELLIPSOID[\"WGS 84\",6378137,298.257223563]],\n    CS[Cartesian,3],\n        AXIS[\"(X)\",geocentricX],\n        AXIS[\"(Y)\",geocentricY],\n        AXIS[\"(Z)\",geocentricZ],\n        UNIT[\"metre\",1],\n    ID[\"authorityA\",\"codeA\"],\n    ID[\"authorityB\",\"codeB\"]]"
    );
}
