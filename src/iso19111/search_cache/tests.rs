use super::*;
use crate::iso19111::factory::{
    AuthorityFactory, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
    GridAvailabilityUse, SpatialCriterion,
};
use crate::iso19111::json::create_from_user_input;
use crate::iso19111::model::CoordinateOperation;

fn operations(db: &DatabaseContext, from: &str, to: &str) -> Vec<CoordinateOperation> {
    let source = create_from_user_input(from, db).expect("source");
    let target = create_from_user_input(to, db).expect("target");
    let mut context = CoordinateOperationContext::create(db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    context.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("candidates")
}

fn read_crs(db: &DatabaseContext, code: &str) -> String {
    AuthorityFactory::create(db, "EPSG")
        .create_coordinate_reference_system(code)
        .expect("CRS")
        .name()
        .to_owned()
}

#[test]
fn nothing_is_kept_while_no_search_runs() {
    let db = DatabaseContext::create().expect("proj.db");
    read_crs(&db, "4267");
    assert_eq!(db.search_cache().entries(), 0);
}

#[test]
fn what_a_search_reads_is_kept_until_its_last_scope_ends() {
    let db = DatabaseContext::create().expect("proj.db");
    let cache = db.search_cache();
    cache.begin();
    cache.begin();
    read_crs(&db, "4267");
    read_crs(&db, "4267");
    assert!(cache.entries() > 0);
    assert!(cache.hits() > 0);
    cache.end();
    assert!(cache.entries() > 0, "an inner search must not empty it");
    cache.end();
    assert_eq!(cache.entries(), 0);
}

#[test]
fn a_finished_search_leaves_the_context_empty() {
    let db = DatabaseContext::create().expect("proj.db");
    let found = operations(&db, "EPSG:4267", "EPSG:4326");
    assert!(found.len() > 10);
    assert!(db.search_cache().hits() > 100, "the search must use it");
    assert_eq!(db.search_cache().entries(), 0);
    let again = operations(&db, "EPSG:4326", "EPSG:2154");
    assert!(!again.is_empty());
    assert_eq!(db.search_cache().entries(), 0);
}

#[test]
fn a_write_empties_it_and_the_next_read_sees_the_write() {
    let db = DatabaseContext::create().expect("proj.db");
    let cache = db.search_cache();
    cache.begin();
    assert_eq!(read_crs(&db, "4326"), "WGS 84");
    assert!(cache.entries() > 0);
    db.execute_sql(
        "UPDATE geodetic_crs SET name = 'renamed' WHERE auth_name = 'EPSG' AND code = '4326'",
    )
    .expect("update");
    assert_eq!(cache.entries(), 0);
    assert_eq!(read_crs(&db, "4326"), "renamed");
    cache.end();
    assert_eq!(cache.entries(), 0);
}

#[test]
fn a_read_that_a_write_overtook_is_not_kept() {
    let cache = SearchCache::default();
    cache.begin();
    let ticket = cache.ticket();
    cache.clear();
    cache.store_rows(ticket, "SELECT 1", &[], &[vec![Cell::Text("1".to_owned())]]);
    assert_eq!(cache.entries(), 0);
    let ticket = cache.ticket();
    cache.store_rows(ticket, "SELECT 1", &[], &[vec![Cell::Text("1".to_owned())]]);
    assert_eq!(cache.entries(), 1);
    cache.end();
}

#[test]
fn a_long_result_and_a_full_budget_are_left_out() {
    let cache = SearchCache::default();
    cache.begin();
    let row = vec![Cell::Text("x".repeat(1000))];
    let long = vec![row.clone(); MAX_ROWS + 1];
    cache.store_rows(cache.ticket(), "long", &[], &long);
    assert_eq!(cache.entries(), 0);
    for index in 0..ROW_BUDGET / 1000 + 100 {
        let params = [index.to_string()];
        cache.store_rows(cache.ticket(), "wide", &params, std::slice::from_ref(&row));
    }
    let kept = cache.entries();
    assert!(kept > 0 && kept < ROW_BUDGET / 1000 + 100, "{kept} kept");
    cache.end();
}

#[test]
fn the_candidates_are_the_same_with_and_without_the_cache() {
    let pairs = [
        ("EPSG:4267", "EPSG:4326"),
        ("EPSG:4326", "EPSG:2154"),
        ("EPSG:2154", "EPSG:32631"),
        ("EPSG:4277", "EPSG:4258"),
        ("EPSG:27700", "EPSG:2154"),
        ("EPSG:7405", "EPSG:4979"),
    ];
    for (from, to) in pairs {
        OFF.with(|off| off.set(true));
        let db = DatabaseContext::create().expect("proj.db");
        let plain = format!("{:#?}", operations(&db, from, to));
        assert_eq!(db.search_cache().hits(), 0);
        OFF.with(|off| off.set(false));
        let db = DatabaseContext::create().expect("proj.db");
        let cached = format!("{:#?}", operations(&db, from, to));
        assert!(db.search_cache().hits() > 0, "{from} to {to}");
        assert!(plain == cached, "{from} to {to}: the candidates differ");
    }
}
