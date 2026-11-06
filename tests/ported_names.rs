//! Every name in [`proj_rs::PORTED`] builds. The bench times this list.

mod common;

#[test]
fn every_ported_operation_builds() {
    common::use_fixture_search_path();
    let mut failed = Vec::new();
    for name in proj_rs::PORTED {
        let definition = proj_rs::bench_definition(name);
        if let Err(error) = proj_rs::Operation::create(&definition) {
            failed.push(format!("{name}: {definition}: {error}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}
