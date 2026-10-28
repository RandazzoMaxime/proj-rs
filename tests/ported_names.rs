//! Every name in [`proj_rs::PORTED`] builds. The bench times this list.

#[test]
fn every_ported_operation_builds() {
    let mut failed = Vec::new();
    for name in proj_rs::PORTED {
        let definition = proj_rs::bench_definition(name);
        if let Err(error) = proj_rs::Operation::create(&definition) {
            failed.push(format!("{name}: {definition}: {error}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}
