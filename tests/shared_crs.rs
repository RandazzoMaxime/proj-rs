//! The candidates of one search share their CRS instead of each owning a copy. See
//! `docs/PORTING-memory.md`.

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, Crs, DatabaseContext, GridAvailabilityUse, SpatialCriterion,
};

fn candidates(from: &str, to: &str) -> Vec<CoordinateOperation> {
    let db = DatabaseContext::create().expect("proj.db");
    let source = create_from_user_input(from, &db).expect("source");
    let target = create_from_user_input(to, &db).expect("target");
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("candidates")
}

fn distinct<'a>(crs: impl Iterator<Item = Option<&'a Crs>>) -> usize {
    let mut seen: Vec<&Crs> = Vec::new();
    for crs in crs.flatten() {
        if !seen.iter().any(|known| std::ptr::eq(*known, crs)) {
            seen.push(crs);
        }
    }
    seen.len()
}

#[test]
fn every_candidate_between_two_crs_points_to_the_same_two_objects() {
    let operations = candidates("EPSG:4267", "EPSG:4326");
    assert!(operations.len() > 10, "{} candidates", operations.len());
    assert_eq!(
        distinct(operations.iter().map(CoordinateOperation::source_crs)),
        1
    );
    assert_eq!(
        distinct(operations.iter().map(CoordinateOperation::target_crs)),
        1
    );
}

#[test]
fn a_cloned_candidate_shares_its_crs() {
    let operations = candidates("EPSG:4267", "EPSG:4326");
    let copy = operations[0].clone();
    let pair = [&operations[0], &copy];
    assert_eq!(distinct(pair.iter().map(|op| op.source_crs())), 1);
    assert_eq!(distinct(pair.iter().map(|op| op.target_crs())), 1);
}

#[test]
fn the_public_types_keep_their_thread_bounds() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Crs>();
    assert_send_sync::<proj_rs::Crs>();
    assert_send_sync::<CoordinateOperation>();
    assert_send_sync::<DatabaseContext>();
    assert_send_sync::<proj_rs::Context>();
    fn assert_send<T: Send>() {}
    assert_send::<proj_rs::Transformer>();
    assert_send::<proj_rs::unstable::Proj>();
}

#[test]
fn a_write_to_the_database_is_seen_by_the_next_search() {
    let db = DatabaseContext::create().expect("proj.db");
    let search = |db: &DatabaseContext| {
        let source = create_from_user_input("EPSG:4267", db).expect("source");
        let target = create_from_user_input("EPSG:4326", db).expect("target");
        let mut context = CoordinateOperationContext::create(db, "");
        context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
        CoordinateOperationFactory::create()
            .create_operations(&source, &target, &context)
            .expect("candidates")
    };
    let names = |operations: &[CoordinateOperation]| -> Vec<String> {
        operations
            .iter()
            .filter_map(|operation| operation.source_crs().map(|crs| crs.name().to_owned()))
            .collect()
    };
    assert!(names(&search(&db)).iter().all(|name| name == "NAD27"));
    db.execute_sql(
        "UPDATE geodetic_crs SET name = 'Renamed NAD27' WHERE auth_name = 'EPSG' AND code = '4267'",
    )
    .expect("update");
    let after = names(&search(&db));
    assert!(!after.is_empty());
    assert!(
        after.iter().all(|name| name == "Renamed NAD27"),
        "{after:?}"
    );
}
