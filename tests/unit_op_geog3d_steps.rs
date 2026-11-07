//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS to geographic 3D, the step
//! order, the horizontal push/pop pair and the z units of the unitconvert steps. Expected PROJ
//! strings are copied from that file and were checked against the PROJ 9.9.0 reference.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

fn ctx_partial_ignore(db: &DatabaseContext, authority: &str) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, authority);
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    ctxt.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctxt
}

/// `createFromUserInput("EPSG:<code>")`, which also reads compound codes such as `4150+5773`.
fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    create_from_user_input(&format!("EPSG:{code}"), db).expect(code)
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctxt: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctxt)
        .expect("operations")
}

/// `exportToPROJString` with the database for the grid names.
fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_with_3D_helmert_same_geog_src_target_context)`
#[test]
fn operation_compound_to_geog_3d_with_3d_helmert_same_geog_src_target_context() {
    let db = DatabaseContext::create().expect("proj.db");
    // CH1903+ + EGM96 height
    let src = epsg(&db, "4150+5773");
    let dst = epsg(&db, "4150").promote_to_3d(Some(&db));
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    let list = operations(&src, &dst, &ctxt);
    assert!(!list.is_empty());
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=push +v_1 +v_2 \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=push +v_3 \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346 \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=pop +v_3 \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=-674.374 +y=-15.056 +z=-405.346 \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1 \
         +step +proj=pop +v_1 +v_2"
    );
}

/// `TEST(operation, compoundCRS_to_geogCRS_with_height_depth_reversal)`
#[test]
fn operation_compound_to_geog_with_height_depth_reversal() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    // NAD83(2011) + NAVD88 depth
    let src = epsg(&db, "6318+6357");
    // NAD83(2011) 3D
    let dst = epsg(&db, "6319");
    let list_depth = operations(&src, &dst, &ctxt);
    assert!(!list_depth.is_empty());
    // NAD83(2011) + NAVD88 height
    let src_metre = epsg(&db, "6318+5703");
    let list_metre = operations(&src_metre, &dst, &ctxt);
    assert_eq!(list_depth.len(), list_metre.len());
    assert_eq!(
        list_depth[0].name(),
        format!(
            "Conversion from NAVD88 depth to NAVD88 height + {}",
            list_metre[0].name()
        )
    );
    assert_eq!(
        proj(&db, &list_depth[0]),
        proj(&db, &list_metre[0]).replace(
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad",
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=axisswap +order=1,2,-3"
        )
    );
    // Check reverse path
    let list_rev = operations(&dst, &src, &ctxt);
    assert_eq!(list_rev.len(), list_depth.len());
}

/// `TEST(operation, compoundCRS_to_geogCRS_with_vertical_unit_change_and_height_depth_reversal)`
#[test]
fn operation_compound_to_geog_with_vertical_unit_change_and_height_depth_reversal() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = ctx_partial_ignore(&db, "EPSG");
    // NAD83(2011) + NAVD88 depth (ftUS)
    let src = epsg(&db, "6318+6358");
    // NAD83(2011) 3D
    let dst = epsg(&db, "6319");
    let list_depth = operations(&src, &dst, &ctxt);
    assert!(!list_depth.is_empty());
    // NAD83(2011) + NAVD88 height
    let src_metre = epsg(&db, "6318+5703");
    let list_metre = operations(&src_metre, &dst, &ctxt);
    assert_eq!(list_depth.len(), list_metre.len());
    assert_eq!(
        list_depth[0].name(),
        format!(
            "Conversion from NAVD88 depth (ftUS) to NAVD88 height + {}",
            list_metre[0].name()
        )
    );
    assert_eq!(
        proj(&db, &list_depth[0]),
        proj(&db, &list_metre[0]).replace(
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad",
            "+step +proj=unitconvert +xy_in=deg +xy_out=rad \
             +step +proj=affine +s33=-0.304800609601219"
        )
    );
    // Check reverse path
    let list_rev = operations(&dst, &src, &ctxt);
    assert_eq!(list_rev.len(), list_depth.len());
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

/// `TEST(operation, geogCRS_3D_to_projCRS_with_2D_geocentric_translation)`: WGS 84 3D to Azores
/// Central 1948 / UTM zone 26N, and the reverse. The unitconvert of the 3D side keeps its
/// `+z_in=m +z_out=m` terms.
#[test]
fn operation_geog_crs_3d_to_proj_crs_with_2d_geocentric_translation() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctx = CoordinateOperationContext::create(&db, "EPSG");
    let src = epsg(&db, "4979");
    let dst = epsg(&db, "2189");

    let list = operations(&src, &dst, &ctx);
    assert!(!list.is_empty());
    assert_proj_eq(
        &proj(&db, &list[0]),
        concat!(
            "+proj=pipeline ",
            "+step +proj=axisswap +order=2,1 ",
            "+step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m ",
            "+step +proj=push +v_3 ",
            "+step +proj=cart +ellps=WGS84 ",
            "+step +proj=helmert +x=104 +y=-167 +z=38 ",
            "+step +inv +proj=cart +ellps=intl ",
            "+step +proj=pop +v_3 ",
            "+step +proj=utm +zone=26 +ellps=intl"
        ),
    );

    let list_reverse = operations(&dst, &src, &ctx);
    assert!(!list_reverse.is_empty());
    assert_proj_eq(
        &proj(&db, &list_reverse[0]),
        concat!(
            "+proj=pipeline ",
            "+step +inv +proj=utm +zone=26 +ellps=intl ",
            "+step +proj=push +v_3 ",
            "+step +proj=cart +ellps=intl ",
            "+step +proj=helmert +x=-104 +y=167 +z=-38 ",
            "+step +inv +proj=cart +ellps=WGS84 ",
            "+step +proj=pop +v_3 ",
            "+step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m ",
            "+step +proj=axisswap +order=2,1"
        ),
    );
}
