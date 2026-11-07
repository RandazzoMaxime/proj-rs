//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, operations from an ESRI projected CRS to a
//! geographic 3D CRS through an ITRF intermediate: the pipeline PROJ writes for it.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The expected names and PROJ
//! strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, ProjLines,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn operations(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, context)
        .expect("operations")
}

fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    match operation {
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_none() || conversion.target_crs().is_none() =>
        {
            conversion.export_to_proj_string()
        }
        _ => operation.export_to_proj_string_with_database(db, ProjLines::Single),
    }
    .expect("PROJ string")
}

/// PROJ's `ComparePROJString`: tokens split on spaces, numeric `key=value` terms compared to a
/// relative 1e-14, other terms compared as text. Tokens without `=` are compared exactly here.
fn assert_proj_eq(actual: &str, expected: &str) {
    let actual_tokens: Vec<&str> = actual.split(' ').collect();
    let expected_tokens: Vec<&str> = expected.split(' ').collect();
    assert_eq!(actual_tokens.len(), expected_tokens.len(), "{actual}");
    for (a, e) in actual_tokens.iter().zip(&expected_tokens) {
        match (a.split_once('='), e.split_once('=')) {
            (Some((ak, av)), Some((ek, ev))) => {
                assert_eq!(ak, ek, "{actual}");
                match (av.parse::<f64>(), ev.parse::<f64>()) {
                    (Ok(x), Ok(y)) => assert!((x - y).abs() <= 1e-14 * x.abs(), "{a} vs {e}"),
                    (Err(_), Err(_)) => assert_eq!(av, ev, "{actual}"),
                    _ => panic!("{a} vs {e}"),
                }
            }
            _ => assert_eq!(a, e, "{actual}"),
        }
    }
}

/// `TEST(operation, esri_projectedCRS_to_geogCRS_with_ITRF_intermediate_context)`
#[test]
fn operation_esri_projected_crs_to_geog_crs_with_itrf_intermediate_context() {
    let db = database();
    let esri = AuthorityFactory::create(&db, "ESRI");
    let ctx = CoordinateOperationContext::create(&db, "EPSG");
    // NAD_1983_CORS96_StatePlane_North_Carolina_FIPS_3200_Ft_US (projected)
    let src = esri
        .create_coordinate_reference_system("103501")
        .expect("ESRI:103501");
    // ITRF2005 (geog3D)
    let dst = epsg(&db, "7910");
    let list = operations(&src, &dst, &ctx);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        concat!(
            "Inverse of NAD_1983_CORS96_StatePlane_North_Carolina_",
            "FIPS_3200_Ft_US + ",
            "Conversion from NAD83(CORS96) (geog2D) to NAD83(CORS96) ",
            "(geocentric) + Inverse of ITRF2000 to NAD83(CORS96) (1) + ",
            "ITRF2000 to ITRF2005 (1) + ",
            "Conversion from ITRF2005 (geocentric) to ITRF2005 (geog3D)"
        ),
    );
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline +step +proj=unitconvert +xy_in=us-ft ",
            "+xy_out=m +step +inv +proj=lcc +lat_0=33.75 +lon_0=-79 ",
            "+lat_1=34.3333333333333 +lat_2=36.1666666666667 ",
            "+x_0=609601.219202438 +y_0=0 +ellps=GRS80 +step +proj=cart ",
            "+ellps=GRS80 +step +inv +proj=helmert +x=0.9956 +y=-1.9013 ",
            "+z=-0.5215 +rx=0.025915 +ry=0.009426 +rz=0.011599 +s=0.00062 ",
            "+dx=0.0007 +dy=-0.0007 +dz=0.0005 +drx=6.7e-05 +dry=-0.000757 ",
            "+drz=-5.1e-05 +ds=-0.00018 +t_epoch=1997 ",
            "+convention=coordinate_frame +step +proj=helmert +x=-0.0001 ",
            "+y=0.0008 +z=0.0058 +rx=0 +ry=0 +rz=0 +s=-0.0004 +dx=0.0002 ",
            "+dy=-0.0001 +dz=0.0018 +drx=0 +dry=0 +drz=0 +ds=-8e-05 ",
            "+t_epoch=2000 +convention=position_vector +step +inv +proj=cart ",
            "+ellps=GRS80 +step +proj=unitconvert +xy_in=rad +z_in=m ",
            "+xy_out=deg +z_out=m +step +proj=axisswap +order=2,1"
        ),
    );
}
