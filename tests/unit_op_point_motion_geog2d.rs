//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, operations between two epochs of a
//! dynamic geographic 2D CRS with a point motion velocity grid.

use proj_rs::iso19111::{
    AuthorityFactory, CoordinateMetadata, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, GridAvailabilityUse, ProjLines, SpatialCriterion,
};

/// `TEST(operation, createOperation_point_motion_operation_geog2D)`
#[test]
fn operation_point_motion_operation_geog2d() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("8255")
        .expect("8255");
    let crs_2002 = CoordinateMetadata::create(&crs, 2002.0, &db).expect("2002");
    let crs_2010 = CoordinateMetadata::create(&crs, 2010.0, &db).expect("2010");
    let mut ctx = CoordinateOperationContext::create(&db, "");
    ctx.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2002, &crs_2010, &ctx)
        .expect("operations");
    assert_eq!(list.len(), 2);
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=set +v_4=2002 +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m +step +proj=cart +ellps=GRS80 +step +proj=set +v_4=2002 +omit_fwd +step +proj=deformation +dt=8 +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 +step +proj=set +v_4=2010 +omit_inv +step +inv +proj=cart +ellps=GRS80 +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1 +step +proj=set +v_4=2010"
    );
    assert!(list[1].has_ballpark_transformation());
    assert_eq!(
        list[1]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=noop"
    );
}
