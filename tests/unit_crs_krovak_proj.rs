//! PROJ 9.9.0 `test/unit/test_crs.cpp`, the PROJ string of the Krovak projected CRS.

use proj_rs::{
    create_crs_from_proj_string, AuthorityFactory, CoordinateOperationFactory, Crs,
    DatabaseContext, ProjLines,
};

/// `TEST(crs, projectedCRS_Krovak_EPSG_5221_as_PROJ_string)`
#[test]
fn crs_projected_crs_krovak_epsg_5221_as_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let projected = AuthorityFactory::create(&db, "EPSG")
        .create_projected_crs("5221")
        .expect("5221");
    let base = Crs::Geodetic(projected.base_crs().clone());
    let op = CoordinateOperationFactory::create()
        .create_operation(&base, &Crs::Projected(projected))
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=bessel +pm=ferro +step +proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397527778 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro"
    );
}

/// `TEST(crs, projectedCRS_Krovak_with_approximate_alpha_as_PROJ_string)`
#[test]
fn crs_projected_crs_krovak_with_approximate_alpha_as_proj_string() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = create_crs_from_proj_string(
        "+proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.28813972222222 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro +units=m +no_defs +type=crs",
    )
    .expect("crs");
    let Crs::Projected(projected) = crs else {
        panic!("expected a projected CRS");
    };
    let base = Crs::Geodetic(projected.base_crs().clone());
    let op = CoordinateOperationFactory::create()
        .create_operation(&base, &Crs::Projected(projected))
        .expect("operation")
        .expect("some operation");
    assert_eq!(
        op.export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("export"),
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +inv +proj=longlat +ellps=bessel +pm=ferro +step +proj=krovak +lat_0=49.5 +lon_0=42.5 +alpha=30.2881397222222 +k=0.9999 +x_0=0 +y_0=0 +ellps=bessel +pm=ferro"
    );
}
