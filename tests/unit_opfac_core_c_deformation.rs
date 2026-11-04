//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`: the deformation and
//! velocity-grid cases of `createOperation(s)` that this crate reproduces.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected PROJ strings are
//! copied from that file. The other cases of the lot are listed in
//! `docs/PORTING-lotc-deformation.md`.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, ProjLines,
    SpatialCriterion,
};

fn database() -> DatabaseContext {
    DatabaseContext::create().expect("database")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect(code)
}

fn parse(db: &DatabaseContext, text: &str) -> Crs {
    create_from_user_input(text, db).expect(text)
}

/// Context on every authority, partial intersection, grids taken as available.
fn partial_ignore_grids(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctx = CoordinateOperationContext::create(db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctx
}

/// Context on every authority, grids taken as available, the default spatial criterion.
fn ignore_grids(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctx = CoordinateOperationContext::create(db, "");
    ctx.set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctx
}

fn operations(
    source: &Crs,
    target: &Crs,
    ctx: &CoordinateOperationContext,
) -> Vec<CoordinateOperation> {
    CoordinateOperationFactory::create()
        .create_operations(source, target, ctx)
        .expect("operations")
}

/// `exportToPROJString`, with the PROJ grid names of the database.
fn exported(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// TEST(operation, createOperation_Geographic3D_Offset_by_velocity_grid)
#[test]
fn operation_create_operation_geographic3d_offset_by_velocity_grid() {
    let db = database();
    let ctx = partial_ignore_grids(&db);
    // NAD83(CSRS)v7
    let source = epsg(&db, "8254");
    // NAD83(CSRS)v3
    let target = epsg(&db, "8239");

    let list = operations(&source, &target, &ctx);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        exported(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 \
         +step +inv +proj=deformation +dt=13 \
         +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}

/// TEST(operation, createOperation_ITRF2014_to_ETRS89_DNK)
#[test]
fn operation_create_operation_itrf2014_to_etrs89_dnk() {
    let db = database();
    let ctx = partial_ignore_grids(&db);
    // ITRF2014
    let source = epsg(&db, "7789");
    // ETRS89-DNK
    let target = epsg(&db, "10890");

    let list = operations(&source, &target, &ctx);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        exported(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=helmert +x=0 +y=0 +z=0 +rx=0.001785 \
         +ry=0.011151 +rz=-0.01617 +s=0 +dx=0 +dy=0 +dz=0 +drx=8.5e-05 \
         +dry=0.000531 +drz=-0.00077 +ds=0 +t_epoch=2010 \
         +convention=position_vector \
         +step +inv +proj=deformation +t_epoch=2000 \
         +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80 \
         +step +proj=helmert +x=0.66818 +y=0.04453 +z=-0.45049 \
         +rx=0.00312883 +ry=-0.02373423 +rz=0.00442969 +s=-0.003136 \
         +convention=position_vector \
         +step +proj=deformation +dt=15.829 \
         +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80"
    );
}

/// TEST(operation, createOperation_test_createOperationsWithDatumPivot_iter_1)
#[test]
fn operation_create_operation_test_create_operations_with_datum_pivot_iter_1() {
    let db = database();
    // Partial intersection is not set: it would give the direct operations.
    let ctx = ignore_grids(&db);
    // NAD83(CSRS)v2 (2D)
    let source = epsg(&db, "8237");
    // NAD83(CSRS)v3 (2D)
    let target = epsg(&db, "8240");

    let list = operations(&source, &target, &ctx);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0].name(),
        "Conversion from NAD83(CSRS)v2 (geog2D) to \
         NAD83(CSRS)v2 (geocentric) + \
         NAD83(CSRS)v2 to NAD83(CSRS)v3 (3) + \
         Conversion from NAD83(CSRS)v3 (geocentric) to \
         NAD83(CSRS)v3 (geog2D)"
    );
    assert_eq!(exported(&db, &list[0]), "+proj=noop");
}

/// TEST(operation, createOperation_Vrtical_Offset_by_velocity_grid)
#[test]
fn operation_create_operation_vrtical_offset_by_velocity_grid() {
    let db = database();
    let ctx = partial_ignore_grids(&db);
    let source = parse(&db, "NAD83(CSRS)v7 + CGVD2013a(2002) height");
    let target = parse(&db, "NAD83(CSRS)v7 + CGVD2013a(2010) height");

    let list = operations(&source, &target, &ctx);
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        exported(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=push +v_1 +v_2 \
         +step +proj=cart +ellps=GRS80 \
         +step +inv +proj=deformation +dt=-8 \
         +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=pop +v_1 +v_2 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1"
    );
}
