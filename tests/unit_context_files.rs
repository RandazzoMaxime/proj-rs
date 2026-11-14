//! PROJ `test/unit/proj_context_test.cpp`: the file finder, the `+init=file:section`
//! dictionaries read from the search paths and the user-writable directory, and
//! the getter and reset of the user-writable directory. The fixture is the
//! C++ `createTmpFile` content, and the expectations are PROJ's.
//!
//! Divergence: PROJ's finder callback receives the context and a `user_data`
//! pointer. The Rust finder is a closure over the file name, so it receives no
//! context and needs no user data.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use proj_rs::ProjBuilder;

/// The dictionary `createTmpFile` writes.
const DICTIONARY: &str = "<MY_PIPELINE> +proj=pipeline +step +proj=utm +zone=31 +ellps=GRS80\n";

fn scratch_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("proj-rs-{tag}-{}", std::process::id()))
}

fn write_dictionary(dir: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("dictionary dir");
    let path = dir.join(name);
    fs::write(&path, DICTIONARY).expect("write dictionary");
    path
}

#[test]
fn proj_context_set_file_finder() {
    let dir = scratch_dir("finder");
    let path = write_dictionary(&dir, "temp_proj_dic1");
    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
    let asked_by_finder = Arc::clone(&asked);
    let mut builder = ProjBuilder::new();
    builder.set_file_finder(move |name| {
        asked_by_finder
            .lock()
            .expect("finder log")
            .push(name.to_owned());
        (name == "temp_proj_dic1").then(|| path.clone())
    });
    let proj = builder.proj("+init=temp_proj_dic1:MY_PIPELINE");
    assert!(proj.is_ok(), "{:?}", proj.err());
    assert!(asked
        .lock()
        .expect("finder log")
        .iter()
        .any(|name| name == "temp_proj_dic1"));
    // Without the finder and without a search path, the dictionary is not found.
    let mut cleared = ProjBuilder::new();
    cleared.set_file_finder(|_| None);
    cleared.clear_file_finder();
    assert!(cleared.proj("+init=temp_proj_dic1:MY_PIPELINE").is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn proj_context_set_search_paths_reads_dictionary() {
    let dir = scratch_dir("search");
    write_dictionary(&dir, "temp_proj_dic2");
    let mut builder = ProjBuilder::new();
    builder.set_search_paths(&dir).expect("search path");
    assert_eq!(builder.search_paths(), vec![dir.clone()]);
    assert!(builder.proj("+init=temp_proj_dic2:MY_PIPELINE").is_ok());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn read_dictionary_from_user_writable_directory() {
    let dir = scratch_dir("writable");
    write_dictionary(&dir, "temp_proj_dic3");
    let mut builder = ProjBuilder::new();
    builder
        .set_user_writable_directory(&dir)
        .expect("writable directory");
    assert!(builder.proj("+init=temp_proj_dic3:MY_PIPELINE").is_ok());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn proj_context_user_writable_directory_default_and_reset() {
    let mut builder = ProjBuilder::new();
    let default = builder.user_writable_directory();
    assert!(!default.as_os_str().is_empty());
    let new_path = default.join("temp_proj_dic4");
    builder
        .set_user_writable_directory(&new_path)
        .expect("new writable directory");
    assert_eq!(builder.user_writable_directory(), new_path);
    builder.clear_user_writable_directory();
    assert_eq!(builder.user_writable_directory(), default);
    let _ = fs::remove_dir_all(&new_path);
}

/// The negative of `read_grid_from_user_writable_directory`: the dictionary is written where the
/// context does not look, so `+init=` fails. Once the directory is set, the same text reads.
#[test]
fn dictionary_absent_before_user_writable_directory_is_set() {
    let dir = scratch_dir("writable-negative");
    write_dictionary(&dir, "temp_proj_dic5");
    assert!(
        ProjBuilder::new()
            .proj("+init=temp_proj_dic5:MY_PIPELINE")
            .is_err(),
        "the dictionary is not in the default search path"
    );
    let mut builder = ProjBuilder::new();
    builder
        .set_user_writable_directory(&dir)
        .expect("writable directory");
    assert!(builder.proj("+init=temp_proj_dic5:MY_PIPELINE").is_ok());
    let _ = fs::remove_dir_all(&dir);
}
