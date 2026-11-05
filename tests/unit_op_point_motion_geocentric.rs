//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `createOperation_point_motion_operation_geocentric`.

use proj_rs::iso19111::CoordinateMetadata;
use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion,
};

/// `TEST(operation, createOperation_point_motion_operation_geocentric)`
#[test]
fn operation_create_operation_point_motion_operation_geocentric() {
    let db = DatabaseContext::create().expect("proj.db");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("8253")
        .expect("8253");
    let crs_2002 = CoordinateMetadata::create(&crs, 2002.0, &db).expect("2002");
    let crs_2010 = CoordinateMetadata::create(&crs, 2010.0, &db).expect("2010");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2002, &crs_2010, &ctxt)
        .expect("operations");
    assert_eq!(list.len(), 2);
    assert!(!list[0].has_ballpark_transformation());
    let proj = |operation: &proj_rs::CoordinateOperation| {
        operation
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string")
    };
    assert_eq!(
        proj(&list[0]),
        "+proj=pipeline +step +proj=set +v_4=2002 \
         +step +proj=deformation +dt=8 +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 \
         +step +proj=set +v_4=2010"
    );
    assert_eq!(
        proj(&list[0].inverse()),
        "+proj=pipeline +step +proj=set +v_4=2010 \
         +step +proj=deformation +dt=-8 +grids=ca_nrc_NAD83v70VG.tif +ellps=GRS80 \
         +step +proj=set +v_4=2002"
    );
    assert!(list[1].has_ballpark_transformation());
    assert_eq!(
        proj(&list[1]),
        "+proj=pipeline +step +proj=set +v_4=2002 +step +proj=set +v_4=2010"
    );
}
