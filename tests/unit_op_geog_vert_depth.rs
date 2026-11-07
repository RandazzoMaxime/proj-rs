//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, operations between a geographic or
//! vertical CRS and a vertical CRS, with a geoid model or a depth: the `vgridshift` and
//! `affine` steps of each candidate.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The expected names and PROJ
//! strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, ProjLines, SpatialCriterion,
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

/// `TEST(operation, geog3DCRS_to_vertCRS_depth_context)`
#[test]
fn operation_geog3d_crs_to_vert_crs_depth_context() {
    let db = database();
    let etrs89 = epsg(&db, "4937");
    let cd_norway_depth = epsg(&db, "9672"); // CD Norway deph
    {
        let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
        ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        let list = operations(&etrs89, &cd_norway_depth, &ctx);
        assert!(!list.is_empty());
        assert_proj_eq(
            &proj(&db, &list[0]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +inv +proj=vgridshift ",
                "+grids=no_kv_CD_above_Ell_ETRS89_v2023b.tif +multiplier=1 ",
                "+step +proj=axisswap +order=1,2,-3 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
    {
        let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
        ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        let list = operations(&cd_norway_depth, &etrs89, &ctx);
        assert!(!list.is_empty());
        assert_proj_eq(
            &proj(&db, &list[0]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +proj=axisswap +order=1,2,-3 ",
                "+step +proj=vgridshift ",
                "+grids=no_kv_CD_above_Ell_ETRS89_v2023b.tif +multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
}

/// `TEST(operation, geog3DCRS_to_geog2DCRS_plus_vertCRS_depth_context)`
#[test]
fn operation_geog3d_crs_to_geog2d_crs_plus_vert_crs_depth_context() {
    let check = |all_authorities: bool, src_code: &str| {
        let db = database();
        let authority = if all_authorities { "" } else { "EPSG" };
        {
            let mut ctx = CoordinateOperationContext::create(&db, authority);
            ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
            // ETRS89-NOR [EUREF89] + CD Norway deph
            let list = operations(&epsg(&db, src_code), &epsg(&db, "9883"), &ctx);
            assert!(!list.is_empty());
            assert_proj_eq(
                &proj(&db, &list[0]),
                concat!(
                    "+proj=pipeline ",
                    "+step +proj=axisswap +order=2,1 ",
                    "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                    "+step +inv +proj=vgridshift ",
                    "+grids=no_kv_CD_above_Ell_ETRS89_v2023b.tif +multiplier=1 ",
                    "+step +proj=axisswap +order=1,2,-3 ",
                    "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                    "+step +proj=axisswap +order=2,1"
                ),
            );
        }
        {
            let mut ctx = CoordinateOperationContext::create(&db, authority);
            ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
            // ETRS89-NOR [EUREF89] + CD Norway deph
            let list = operations(&epsg(&db, "9883"), &epsg(&db, src_code), &ctx);
            assert!(!list.is_empty());
            assert_proj_eq(
                &proj(&db, &list[0]),
                concat!(
                    "+proj=pipeline ",
                    "+step +proj=axisswap +order=2,1 ",
                    "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                    "+step +proj=axisswap +order=1,2,-3 ",
                    "+step +proj=vgridshift ",
                    "+grids=no_kv_CD_above_Ell_ETRS89_v2023b.tif +multiplier=1 ",
                    "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                    "+step +proj=axisswap +order=2,1"
                ),
            );
        }
    };

    check(false, "4937"); // ETRS89 3D
    check(true, "4937"); // ETRS89 3D
    check(false, "10874"); // ETRS89-NOR [EUREF89] 3D
}
