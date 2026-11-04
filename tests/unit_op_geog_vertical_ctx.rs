//! PROJ 9.9.0 `test/unit/test_operationfactory.cpp`: a compound CRS and WGS 84 3D, both ways.

use proj_rs::{
    AuthorityFactory, CoordinateOperation, CoordinateOperationContext, CoordinateOperationFactory,
    Crs, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

fn operations(db: &DatabaseContext, source: &Crs, target: &Crs) -> Vec<CoordinateOperation> {
    let mut context = CoordinateOperationContext::create(db, "EPSG");
    context
        .set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability);
    CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .expect("operations")
}

fn epsg(db: &DatabaseContext, code: &str) -> Crs {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect("EPSG CRS")
}

/// `TEST(operation, compoundCRS_to_geogCRS_3D_WGS84_to_GDA2020_AHD_Height)`
#[test]
fn operation_compound_to_geog_3d_wgs84_to_gda2020_ahd_height() {
    let db = DatabaseContext::create().expect("proj.db");
    let list = operations(&db, &epsg(&db, "9463"), &epsg(&db, "4979"));
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of GDA2020 to AHD height (1) + GDA2020 to WGS 84 (2)"
    );
    let list = operations(&db, &epsg(&db, "4979"), &epsg(&db, "9463"));
    assert!(!list.is_empty());
    assert_eq!(
        list[0].name(),
        "Inverse of GDA2020 to WGS 84 (2) + GDA2020 to AHD height (1)"
    );
}
