//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, lot `opfac-c`: the `CoordinateMetadata`
//! cases, the time-dependent Helmert and the New Zealand deformation model.
//!
//! Each function keeps the C++ `TEST()` name in its doc comment. Expected PROJ strings are
//! copied from that file. The `compound_to_compound_with_epoch` cases are not here: the
//! compound to compound chain they need is not built, see `docs/PORTING-lotc-metadata.md`.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateMetadata, CoordinateOperation, CoordinateOperationContext,
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

/// Context on no authority, partial intersection, grids taken as available.
fn partial_ignore_ctx(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctx = CoordinateOperationContext::create(db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    ctx
}

/// `exportToPROJString(PROJStringFormatter::create())`, with the PROJ grid names of `db`.
fn plain(db: &DatabaseContext, operation: &CoordinateOperation) -> String {
    operation
        .export_to_proj_string_with_database(db, ProjLines::Single)
        .expect("PROJ string")
}

/// TEST(operation, createOperation_time_dependent_helmert_directly_from_database)
#[test]
fn operation_time_dependent_helmert_directly_from_database() {
    let db = database();
    // ITRF2014 geocentric, at epoch 2026
    let itrf2014_at_2026 =
        CoordinateMetadata::create(&epsg(&db, "7789"), 2026.0, &db).expect("ITRF2014 at 2026");
    let gda2020 = epsg(&db, "7842");
    let ctx = partial_ignore_ctx(&db);
    let factory = CoordinateOperationFactory::create();

    let list = factory
        .create_operations_from_metadata(&itrf2014_at_2026, &gda2020, &ctx)
        .expect("operations");
    assert!(!list.is_empty());

    assert_eq!(
        plain(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=set +v_4=2026 \
         +step +proj=helmert +x=0 +y=0 +z=0 +rx=0 +ry=0 +rz=0 +s=0 \
         +dx=0 +dy=0 +dz=0 \
         +drx=0.00150379 +dry=0.00118346 +drz=0.00120716 +ds=0 \
         +t_epoch=2020 +convention=coordinate_frame \
         +step +proj=set +v_4=2026"
    );
    assert_eq!(
        plain(&db, &list[0].inverse()),
        "+proj=pipeline \
         +step +proj=set +v_4=2026 \
         +step +inv +proj=helmert +x=0 +y=0 +z=0 +rx=0 +ry=0 +rz=0 +s=0 \
         +dx=0 +dy=0 +dz=0 \
         +drx=0.00150379 +dry=0.00118346 +drz=0.00120716 +ds=0 \
         +t_epoch=2020 +convention=coordinate_frame \
         +step +proj=set +v_4=2026"
    );

    let list2 = factory
        .create_operations_to_metadata(&gda2020, &itrf2014_at_2026, &ctx)
        .expect("operations");
    assert!(!list2.is_empty());
    assert_eq!(
        plain(&db, &list2[0]),
        "+proj=pipeline \
         +step +proj=set +v_4=2026 \
         +step +inv +proj=helmert +x=0 +y=0 +z=0 +rx=0 +ry=0 +rz=0 +s=0 \
         +dx=0 +dy=0 +dz=0 \
         +drx=0.00150379 +dry=0.00118346 +drz=0.00120716 +ds=0 \
         +t_epoch=2020 +convention=coordinate_frame \
         +step +proj=set +v_4=2026"
    );
    assert_eq!(
        plain(&db, &list2[0].inverse()),
        "+proj=pipeline \
         +step +proj=set +v_4=2026 \
         +step +proj=helmert +x=0 +y=0 +z=0 +rx=0 +ry=0 +rz=0 +s=0 \
         +dx=0 +dy=0 +dz=0 \
         +drx=0.00150379 +dry=0.00118346 +drz=0.00120716 +ds=0 \
         +t_epoch=2020 +convention=coordinate_frame \
         +step +proj=set +v_4=2026"
    );
}

/// TEST(operation, createOperation_defmodel_from_database)
#[test]
fn operation_defmodel_from_database() {
    let db = database();
    // ITRF96, at epoch 2026
    let itrf96_at_2026 =
        CoordinateMetadata::create(&epsg(&db, "7907"), 2026.0, &db).expect("ITRF96 at 2026");
    let nzgd2000 = epsg(&db, "4959");
    let ctx = partial_ignore_ctx(&db);
    let factory = CoordinateOperationFactory::create();

    let list = factory
        .create_operations_from_metadata(&itrf96_at_2026, &nzgd2000, &ctx)
        .expect("operations");
    assert!(!list.is_empty());

    assert_eq!(
        plain(&db, &list[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=set +v_4=2026 \
         +step +inv +proj=defmodel +model=nz_linz_nzgd2000-20180701.json \
         +step +proj=set +v_4=2026 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&db, &list[0].inverse()),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=set +v_4=2026 \
         +step +proj=defmodel +model=nz_linz_nzgd2000-20180701.json \
         +step +proj=set +v_4=2026 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );

    let list2 = factory
        .create_operations_to_metadata(&nzgd2000, &itrf96_at_2026, &ctx)
        .expect("operations");
    assert!(!list2.is_empty());
    assert_eq!(
        plain(&db, &list2[0]),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=set +v_4=2026 \
         +step +proj=defmodel +model=nz_linz_nzgd2000-20180701.json \
         +step +proj=set +v_4=2026 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
    assert_eq!(
        plain(&db, &list2[0].inverse()),
        "+proj=pipeline \
         +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=set +v_4=2026 \
         +step +inv +proj=defmodel +model=nz_linz_nzgd2000-20180701.json \
         +step +proj=set +v_4=2026 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}
