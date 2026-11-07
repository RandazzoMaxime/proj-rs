//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, operations between two epochs of a
//! dynamic CRS with a point motion velocity grid, between a geocentric and a geographic 3D CRS.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. The expected names and PROJ
//! strings are copied from that file. The C++ `ComparePROJString` helper is mirrored by
//! [`assert_proj_eq`], except that a token with no `=` is compared exactly.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateMetadata, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
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

fn epsg_crs(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn point_motion_context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctx = CoordinateOperationContext::create(db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctx
}

/// `TEST(operation, createOperation_point_motion_operation_geocentric_to_geog3D)`
#[test]
fn operation_point_motion_operation_geocentric_to_geog3d() {
    let db = database();
    // "NAD83(CSRS)v7"
    let crs_2002 = CoordinateMetadata::create(&epsg_crs(&db, "8253"), 2002.0, &db).expect("2002");
    // "NAD83(CSRS)v7" geographic 3D
    let crs_2010 = CoordinateMetadata::create(&epsg_crs(&db, "8254"), 2010.0, &db).expect("2010");
    let ctx = point_motion_context(&db);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2002, &crs_2010, &ctx)
        .expect("operations");
    assert_eq!(list.len(), 2);
    assert!(!list[0].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline ",
            "+step +proj=set +v_4=2002 ",
            "+step +proj=deformation +dt=8 +grids=ca_nrc_NAD83v70VG.tif ",
            "+ellps=GRS80 ",
            "+step +proj=set +v_4=2010 +omit_inv ",
            "+step +inv +proj=cart +ellps=GRS80 ",
            "+step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=set +v_4=2010"
        ),
    );
    assert!(list[1].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[1]),
        concat!(
            "+proj=pipeline ",
            "+step +inv +proj=cart +ellps=GRS80 ",
            "+step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m ",
            "+step +proj=axisswap +order=2,1"
        ),
    );
}

/// `TEST(operation, createOperation_point_motion_operation_NAD83_CSRS_v7_TO_ITRF2014)`
#[test]
fn operation_point_motion_operation_nad83_csrs_v7_to_itrf2014() {
    let db = database();
    // "NAD83(CSRS)v7"
    let crs_2002 = CoordinateMetadata::create(&epsg_crs(&db, "8254"), 2002.0, &db).expect("2002");
    // ITRF2014
    let crs_2005 = CoordinateMetadata::create(&epsg_crs(&db, "7912"), 2005.0, &db).expect("2005");
    let ctx = point_motion_context(&db);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2002, &crs_2005, &ctx)
        .expect("operations");
    assert!(list.len() >= 2);
    assert!(!list[0].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline ",
            "+step +proj=set +v_4=2002 ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m ",
            "+step +proj=cart +ellps=GRS80 ",
            "+step +proj=set +v_4=2002 +omit_fwd ",
            "+step +proj=deformation +dt=3 +grids=ca_nrc_NAD83v70VG.tif ",
            "+ellps=GRS80 ",
            "+step +proj=set +v_4=2005 +omit_inv ",
            "+step +inv +proj=helmert ",
            "+x=1.0053 +y=-1.90921 +z=-0.54157 +rx=-0.02678138 ",
            "+ry=0.00042027 +rz=-0.01093206 +s=0.00036891 +dx=0.00079 +dy=-0.0006 ",
            "+dz=-0.00144 +drx=-6.667e-05 +dry=0.00075744 +drz=5.133e-05 ",
            "+ds=-7.201e-05 +t_epoch=2010 +convention=position_vector ",
            "+step +inv +proj=cart +ellps=GRS80 ",
            "+step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=set +v_4=2005"
        ),
    );
    assert!(list[1].has_ballpark_transformation());
}

/// `TEST(operation, createOperation_point_motion_operation_ITRF2014_to_NAD83_CSRS_v7)`
#[test]
fn operation_point_motion_operation_itrf2014_to_nad83_csrs_v7() {
    let db = database();
    // ITRF2014
    let crs_2005 = CoordinateMetadata::create(&epsg_crs(&db, "7912"), 2005.0, &db).expect("2005");
    // "NAD83(CSRS)v7"
    let crs_2002 = CoordinateMetadata::create(&epsg_crs(&db, "8254"), 2002.0, &db).expect("2002");
    let ctx = point_motion_context(&db);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2005, &crs_2002, &ctx)
        .expect("operations");
    assert!(list.len() >= 2);
    assert!(!list[0].has_ballpark_transformation());
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline ",
            "+step +proj=set +v_4=2005 ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m ",
            "+step +proj=cart +ellps=GRS80 ",
            "+step +proj=helmert +x=1.0053 +y=-1.90921 +z=-0.54157 +rx=-0.02678138 ",
            "+ry=0.00042027 +rz=-0.01093206 +s=0.00036891 +dx=0.00079 +dy=-0.0006 ",
            "+dz=-0.00144 +drx=-6.667e-05 +dry=0.00075744 +drz=5.133e-05 ",
            "+ds=-7.201e-05 +t_epoch=2010 +convention=position_vector ",
            "+step +proj=set +v_4=2005 +omit_fwd ",
            "+step +proj=deformation +dt=-3 +grids=ca_nrc_NAD83v70VG.tif ",
            "+ellps=GRS80 ",
            "+step +proj=set +v_4=2002 +omit_inv ",
            "+step +inv +proj=cart +ellps=GRS80 ",
            "+step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=set +v_4=2002"
        ),
    );
    assert!(list[1].has_ballpark_transformation());
}
