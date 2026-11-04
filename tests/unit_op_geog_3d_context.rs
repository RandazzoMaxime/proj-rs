//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: geographic 3D and geocentric CRS in the
//! EPSG context.

use proj_rs::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, GridAvailabilityUse, ProjLines, SpatialCriterion,
};

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect("EPSG CRS")
}

fn context(
    db: &DatabaseContext,
    authority: &str,
    grid_use: GridAvailabilityUse,
) -> CoordinateOperationContext {
    let mut context = CoordinateOperationContext::create(db, authority);
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(grid_use);
    context
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
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// `TEST(operation, geogCRS_3D_to_geogCRS_3D_different_datum_context)`
#[test]
fn operation_geog_3d_to_geog_3d_different_datum_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let context = context(&db, "", GridAvailabilityUse::UseForSorting);
    let list = operations(
        &epsg(&db, "4965"),
        &epsg(&db, "4150").promote_to_3d(Some(&db)),
        &context,
    );
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of ETRS89 to ETRS89-FRA [RGF93 v1] + Inverse of CH1903+ to ETRS89 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=-674.374 +y=-15.056 +z=-405.346 \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, &list[0].inverse()),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

/// `TEST(operation, geocentric_to_geogCRS_3D_different_datum_context)`
#[test]
fn operation_geocentric_to_geog_3d_different_datum_context() {
    let db = DatabaseContext::create().expect("proj.db");
    let context = context(&db, "", GridAvailabilityUse::UseForSorting);
    let list = operations(
        &epsg(&db, "4964"),
        &epsg(&db, "4150").promote_to_3d(Some(&db)),
        &context,
    );
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Conversion from ETRS89-FRA [RGF93 v1] (geocentric) to ETRS89-FRA [RGF93 v1] (geog3D) + \
         Inverse of ETRS89 to ETRS89-FRA [RGF93 v1] + Inverse of CH1903+ to ETRS89 (1)"
    );
    assert_eq!(
        proj(&db, &list[0]),
        "+proj=pipeline +step +proj=helmert +x=-674.374 +y=-15.056 +z=-405.346 \
         +step +inv +proj=cart +ellps=bessel \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        proj(&db, &list[0].inverse()),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=bessel \
         +step +proj=helmert +x=674.374 +y=15.056 +z=405.346"
    );
}
