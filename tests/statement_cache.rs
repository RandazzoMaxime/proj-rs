//! The compiled statements a connection keeps for its queries see what the context writes, and
//! one connection's statements never answer for another. See `docs/PORTING-memory.md`.

use proj_rs::iso19111::{
    create_from_user_input, AuthorityFactory, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext,
};

fn name_of_4326(context: &DatabaseContext) -> String {
    AuthorityFactory::create(context, "EPSG")
        .create_coordinate_reference_system("4326")
        .expect("EPSG:4326")
        .name()
        .to_owned()
}

fn rename_4326(context: &DatabaseContext, name: &str) {
    context
        .execute_sql(&format!(
            "UPDATE geodetic_crs SET name = '{name}' WHERE auth_name = 'EPSG' AND code = '4326'"
        ))
        .expect("update");
}

#[test]
fn a_repeated_query_sees_the_rows_its_context_writes() {
    let context = DatabaseContext::create().expect("proj.db");
    assert_eq!(name_of_4326(&context), "WGS 84");
    assert_eq!(name_of_4326(&context), "WGS 84");
    rename_4326(&context, "first");
    assert_eq!(name_of_4326(&context), "first");
    assert_eq!(name_of_4326(&context), "first");
    rename_4326(&context, "second");
    assert_eq!(name_of_4326(&context), "second");
}

#[test]
fn a_write_of_one_context_does_not_reach_the_statements_of_the_others() {
    let reader = DatabaseContext::create().expect("proj.db");
    assert_eq!(name_of_4326(&reader), "WGS 84");
    let writer = DatabaseContext::create().expect("proj.db");
    assert_eq!(name_of_4326(&writer), "WGS 84");
    rename_4326(&writer, "written");
    assert_eq!(name_of_4326(&writer), "written");
    assert_eq!(name_of_4326(&reader), "WGS 84");
    assert_eq!(
        name_of_4326(&DatabaseContext::create().expect("proj.db")),
        "WGS 84"
    );
    let clone = writer.clone();
    assert_eq!(name_of_4326(&clone), "written");
    rename_4326(&clone, "by the clone");
    assert_eq!(name_of_4326(&writer), "by the clone");
}

#[test]
fn a_repeated_query_runs_again_after_the_schema_changes() {
    let context = DatabaseContext::create().expect("proj.db");
    assert_eq!(name_of_4326(&context), "WGS 84");
    context
        .execute_sql("ALTER TABLE geodetic_crs ADD COLUMN added_after_first_read TEXT")
        .expect("alter");
    assert_eq!(name_of_4326(&context), "WGS 84");
    rename_4326(&context, "renamed");
    assert_eq!(name_of_4326(&context), "renamed");
}

#[test]
fn a_search_leaves_the_context_reading_what_it_wrote_before_and_after() {
    let context = DatabaseContext::create().expect("proj.db");
    let search = |context: &DatabaseContext| {
        let source = create_from_user_input("EPSG:4267", context).expect("source");
        let target = create_from_user_input("EPSG:4326", context).expect("target");
        let search = CoordinateOperationContext::create(context, "");
        CoordinateOperationFactory::create()
            .create_operations(&source, &target, &search)
            .expect("candidates")
            .len()
    };
    let before = search(&context);
    assert!(before > 0);
    assert_eq!(search(&context), before);
    rename_4326(&context, "renamed");
    assert_eq!(name_of_4326(&context), "renamed");
    assert_eq!(search(&context), before);
    assert_eq!(name_of_4326(&context), "renamed");
}
