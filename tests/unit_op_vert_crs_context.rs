//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: `TEST(operation, vertCRS_to_geogCRS_context)`.
//! The expected strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, ProjLines, SpatialCriterion,
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

/// `TEST(operation, vertCRS_to_geogCRS_context)`
#[test]
fn operation_vert_crs_to_geog_crs_context() {
    let db = database();
    let egm2008 = epsg(&db, "3855"); // EGM2008 height
    let wgs84 = epsg(&db, "4979"); // WGS 84
    {
        let mut ctx = CoordinateOperationContext::create(&db, "EPSG");
        ctx.set_alternative_grid_names(AlternativeGridNames::Official);
        let list = operations(&egm2008, &wgs84, &ctx);
        assert_eq!(list.len(), 3);
        assert_proj_eq(
            &proj(&db, &list[1]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
    {
        let ctx = CoordinateOperationContext::create(&db, "EPSG");
        let list = operations(&egm2008, &wgs84, &ctx);
        assert_eq!(list.len(), 3);
        assert_proj_eq(
            &proj(&db, &list[0]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +proj=vgridshift +grids=us_nga_egm08_25.tif +multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
    {
        let ctx = CoordinateOperationContext::create(&db, "EPSG");
        let list = operations(&wgs84, &egm2008, &ctx);
        assert_eq!(list.len(), 2);
        assert_proj_eq(
            &proj(&db, &list[0]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +inv +proj=vgridshift +grids=us_nga_egm08_25.tif ",
                "+multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
    {
        let ctx = CoordinateOperationContext::create(&db, "EPSG");
        // NGVD29 depth (ftUS)
        let list = operations(&epsg(&db, "6359"), &epsg(&db, "4326"), &ctx);
        assert_eq!(list.len(), 1);
        assert_proj_eq(&proj(&db, &list[0]), "+proj=affine +s33=-0.304800609601219");
    }
    {
        let ctx = CoordinateOperationContext::create(&db, "EPSG");
        // NZVD2016 height, NZGD2000
        let list = operations(&epsg(&db, "7839"), &epsg(&db, "4959"), &ctx);
        assert_eq!(list.len(), 2);
        assert_proj_eq(
            &proj(&db, &list[0]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +proj=vgridshift +grids=nz_linz_nzgeoid2016.tif ",
                "+multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
    {
        // Baltic 1957 height to ETRS89, through the records of EPSG:1088 (the
        // 'Geog3D to Geog2D+GravityRelatedHeight (gtx)' method), with every authority.
        let db = database();
        let mut ctx = CoordinateOperationContext::create(&db, "");
        ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        let list = operations(&epsg(&db, "8357"), &epsg(&db, "4937"), &ctx);
        assert!(list.len() >= 2);
        assert_proj_eq(
            &proj(&db, &list[1]),
            concat!(
                "+proj=pipeline ",
                "+step +proj=axisswap +order=2,1 ",
                "+step +proj=unitconvert +xy_in=deg +xy_out=rad ",
                "+step +proj=vgridshift ",
                "+grids=sk_gku_Slovakia_ETRS89h_to_Baltic1957.tif ",
                "+multiplier=1 ",
                "+step +proj=unitconvert +xy_in=rad +xy_out=deg ",
                "+step +proj=axisswap +order=2,1"
            ),
        );
    }
}
