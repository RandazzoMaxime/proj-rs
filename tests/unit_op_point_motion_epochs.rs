//! Operations between two epochs of one dynamic CRS, as `projinfo -s X --s_epoch E1 -t X --t_epoch
//! E2 --hide-ballpark` builds them. The expected names, identifiers and counts are the output of
//! the PROJ 9.9.0 `projinfo` binary (`--summary`), not of this crate.

use proj_rs::iso19111::CoordinateMetadata;
use proj_rs::{
    AuthorityFactory, BallparkUse, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

/// The context of `projinfo --spatial-test intersects --hide-ballpark`.
fn hidden_ballpark_context(db: &DatabaseContext) -> CoordinateOperationContext {
    let mut ctxt = CoordinateOperationContext::create(db, "");
    ctxt.set_spatial_criterion(SpatialCriterion::PartialIntersection)
        .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
        .set_ballpark_use(BallparkUse::Forbid);
    ctxt
}

fn metadata(db: &DatabaseContext, code: &str, epoch: f64) -> CoordinateMetadata {
    let crs = AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect("CRS");
    CoordinateMetadata::create(&crs, epoch, db).expect("epoch")
}

/// `projinfo -s "NAD83(CSRS)v7" --s_epoch 1997 -t "NAD83(CSRS)v7" --t_epoch 2010`: one candidate,
/// the null offset to 3D, the velocity grid between the epochs, and the null offset back.
#[test]
fn geographic_2d_epochs_concatenate_the_velocity_grid() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = metadata(&db, "8255", 1997.0);
    let target = metadata(&db, "8255", 2010.0);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&source, &target, &hidden_ballpark_context(&db))
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].name(),
        "Null geographic offset from NAD83(CSRS)v7 (geog2D) to NAD83(CSRS)v7 (geog3D) + \
         Canada velocity grid v7 from epoch 1997 to epoch 2010 + Null geographic offset from \
         NAD83(CSRS)v7 (geog3D) to NAD83(CSRS)v7 (geog2D)"
    );
    assert!(matches!(list[0], CoordinateOperation::Concatenated(_)));
}

/// `projinfo -s EPSG:8254 --s_epoch 1997 -t EPSG:8254 --t_epoch 2010`: the velocity grid itself,
/// with the identifier PROJ gives to the grid it reads from its own name (`DERIVED_FROM`).
#[test]
fn geographic_3d_epochs_give_the_velocity_grid_with_derived_identifier() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = metadata(&db, "8254", 1997.0);
    let target = metadata(&db, "8254", 2010.0);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&source, &target, &hidden_ballpark_context(&db))
        .expect("operations");
    assert_eq!(list.len(), 1);
    let CoordinateOperation::Transformation(velocity) = &list[0] else {
        panic!("a point motion operation is a transformation");
    };
    assert_eq!(
        velocity.name(),
        "Canada velocity grid v7 from epoch 1997 to epoch 2010"
    );
    assert_eq!(velocity.identifiers()[0].code_space(), "DERIVED_FROM(EPSG)");
    assert_eq!(velocity.identifiers()[0].code(), "9483");
    assert!(velocity.is_point_motion());
}

/// `projinfo -s "NAD83(CSRS)v7" --s_epoch 2010 -t "NAD83(CSRS)v7" --t_epoch 1997`: the same
/// operation read in reverse, with the epochs in the other order.
#[test]
fn reverse_epochs_name_the_velocity_grid_in_reverse() {
    let db = DatabaseContext::create().expect("proj.db");
    let source = metadata(&db, "8255", 2010.0);
    let target = metadata(&db, "8255", 1997.0);
    let list = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&source, &target, &hidden_ballpark_context(&db))
        .expect("operations");
    assert_eq!(list.len(), 1);
    assert!(list[0]
        .name()
        .contains("Canada velocity grid v7 from epoch 2010 to epoch 1997"));
}

/// `projinfo -s "NAD83(CSRS)v7" --s_epoch 2010 -t "NAD83(CSRS)v7" --t_epoch 2010`: equal epochs
/// give the operations of the CRS alone, as without epochs.
#[test]
fn equal_epochs_give_the_operation_without_epochs() {
    let db = DatabaseContext::create().expect("proj.db");
    let ctxt = hidden_ballpark_context(&db);
    let source = metadata(&db, "8255", 2010.0);
    let target = metadata(&db, "8255", 2010.0);
    let with_epochs = CoordinateOperationFactory::create()
        .create_operations_between_metadata(&source, &target, &ctxt)
        .expect("operations");
    let crs = AuthorityFactory::create(&db, "EPSG")
        .create_coordinate_reference_system("8255")
        .expect("CRS");
    let without = CoordinateOperationFactory::create()
        .create_operations(&crs, &crs, &ctxt)
        .expect("operations");
    let names = |list: &[CoordinateOperation]| {
        list.iter()
            .map(|operation| operation.name().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&with_epochs), names(&without));
    assert_eq!(
        names(&with_epochs),
        ["Null geographic offset from NAD83(CSRS)v7 to NAD83(CSRS)v7"]
    );
}
