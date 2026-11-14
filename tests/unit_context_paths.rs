//! PROJ `test/unit/proj_context_test.cpp`: the context setters that store a
//! path and read it back (`proj_context_set_ca_bundle_path`,
//! `proj_context_set_user_writable_directory`, `proj_context_set_search_paths`).
//! The C++ `+init=` dictionary lookups are not part of this test.

use std::fs;
use std::path::PathBuf;

use proj_rs::ProjBuilder;

#[test]
fn proj_context_set_ca_bundle_path() {
    let path = "/tmp/dummmy/path/cacert.pem";
    let mut builder = ProjBuilder::new();
    builder.set_ca_bundle_path(path).expect("set ca bundle");
    assert_eq!(builder.ca_bundle_path(), Some(PathBuf::from(path)));
}

#[test]
fn proj_context_set_user_writable_directory() {
    let dir = std::env::temp_dir().join(format!("proj-rs-temp-dic4-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let mut builder = ProjBuilder::new();
    builder
        .set_user_writable_directory(&dir)
        .expect("create writable dir");
    assert!(dir.is_dir());
    assert_eq!(builder.user_writable_directory(), dir.clone());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn proj_context_set_search_paths() {
    let dir = std::env::temp_dir().join(format!("proj-rs-temp-dic2-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("search dir");
    let mut builder = ProjBuilder::new();
    builder.set_search_paths(&dir).expect("search path");
    assert_eq!(builder.search_paths(), vec![dir.clone()]);
    let _ = fs::remove_dir_all(&dir);
}
