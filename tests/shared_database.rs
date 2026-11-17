//! The contexts of one thread read the embedded `proj.db` through one connection, and a
//! context that writes takes a copy of its own. See `docs/PORTING-memory.md`.

use proj_rs::iso19111::DatabaseContext;

fn db() -> DatabaseContext {
    DatabaseContext::create().expect("proj.db")
}

#[test]
fn a_write_stays_in_the_context_that_made_it_and_its_clones() {
    let before = db();
    let writer = db();
    writer
        .execute_sql("CREATE TABLE written_by_one_context(x INTEGER)")
        .expect("a context can write");
    let clone = writer.clone();
    assert!(
        clone
            .execute_sql("CREATE TABLE written_by_one_context(x INTEGER)")
            .is_err(),
        "a clone shares the writes of its context"
    );
    assert!(
        writer
            .execute_sql("CREATE TABLE written_by_one_context(x INTEGER)")
            .is_err(),
        "the context still sees its own write"
    );
    for other in [before, db()] {
        other
            .execute_sql("CREATE TABLE written_by_one_context(x INTEGER)")
            .expect("another context does not see the table");
    }
}

#[test]
fn a_context_reads_after_it_has_written() {
    let context = db();
    context
        .execute_sql("CREATE TABLE scratch(x INTEGER); INSERT INTO scratch VALUES (7)")
        .expect("write");
    let crs = proj_rs::iso19111::AuthorityFactory::create(&context, "EPSG")
        .create_coordinate_reference_system("4326")
        .expect("the embedded rows are still read");
    assert_eq!(crs.name(), "WGS 84");
}

#[test]
fn contexts_made_while_another_is_alive_agree_with_it() {
    let first = db();
    let second = db();
    let name = |context: &DatabaseContext| {
        proj_rs::iso19111::AuthorityFactory::create(context, "EPSG")
            .create_coordinate_reference_system("2154")
            .expect("EPSG:2154")
            .name()
            .to_owned()
    };
    assert_eq!(name(&first), name(&second));
    drop(first);
    assert_eq!(name(&second), "ETRS89-FRA [RGF93 v1] / Lambert-93");
}
