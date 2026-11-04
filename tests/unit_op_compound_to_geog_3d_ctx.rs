//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: compound CRS to a geographic 3D CRS in the
//! EPSG context.

use proj_rs::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

fn context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut context = CoordinateOperationContext::create(db, "EPSG");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    context
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect("EPSG CRS")
}

fn proj(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_with_3D_helmert_context)`
#[test]
fn operation_compound_to_geog_3d_with_3d_helmert_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input("EPSG:4326+5773", &db).expect("WGS 84 + EGM96");
    let list = CoordinateOperationFactory::create()
        .create_operations(
            &source,
            &epsg(&db, "4150").promote_to_3d(Some(&db)),
            &context(&db),
        )
        .expect("operations");
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of WGS 84 to EGM96 height (1) + Inverse of CH1903+ to WGS 84 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=vgridshift +grids=us_nga_egm96_15.tif +multiplier=1 \
         +step +proj=cart +ellps=WGS84 \
         +step +proj=helmert +x=-674.374 +y=-15.056 +z=-405.346 \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
