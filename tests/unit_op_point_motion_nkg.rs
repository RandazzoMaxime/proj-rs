//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `createOperation_point_motion_operation_nkg`.

use proj_rs::iso19111::CoordinateMetadata;
use proj_rs::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, ProjLines, SpatialCriterion,
};

/// `TEST(operation, createOperation_point_motion_operation_nkg)`
#[test]
fn operation_create_operation_point_motion_operation_nkg() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("8403")
        .expect("8403");
    let crs_2020 = CoordinateMetadata::create(&source, 2020.0, &db).expect("2020");
    let crs_2025 = CoordinateMetadata::create(&source, 2025.0, &db).expect("2025");
    let mut ctxt = CoordinateOperationContext::create(&db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&crs_2020, &crs_2025, &ctxt)
        .expect("operations");
    assert!(!list.is_empty());
    assert!(!list[0].has_ballpark_transformation());
    assert_eq!(
        list[0]
            .export_to_proj_string_with_database(&db, ProjLines::Single)
            .expect("PROJ string"),
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +z_in=m +xy_out=rad +z_out=m \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=set +v_4=2020 +omit_fwd \
         +step +proj=deformation +dt=5 +grids=eur_nkg_nkgrf17vel.tif +ellps=GRS80 \
         +step +proj=set +v_4=2025 +omit_inv \
         +step +inv +proj=cart +ellps=GRS80 \
         +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m \
         +step +proj=axisswap +order=2,1"
    );
}
