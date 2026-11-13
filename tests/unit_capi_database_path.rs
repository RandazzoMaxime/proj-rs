//! The database-path cases of PROJ 9.9.0 `test/unit/test_c_api.cpp`:
//! `CApi.proj_context_set_database_path_null`, `_aux`, `_error_1` and `_error_2`.
//!
//! A C `PJ_CONTEXT` holds the database that `proj_create_from_database` reads.
//! In this port that database is the `DatabaseContext` itself, built by
//! `DatabaseContext::create_with_auxiliary`: its `Err` is the C `FALSE`, and
//! `AuthorityFactory::create(&db, ...)` is `proj_create_from_database` on it.
//! A failed call leaves no state behind, so the "default database still works"
//! step of each C test is `DatabaseContext::create()`.
//!
//! Expected values come from the C++ tests (`EPSG:4326` is `WGS 84`, and the
//! `OTHER_4326` row is the literal the C++ test inserts), never from the code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};

use proj_rs::iso19111::{AuthorityFactory, DatabaseContext};
use rusqlite::Connection;

/// A fresh directory under the system temporary directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("proj-rs-capi-path-{}-{index}-{tag}", process::id()));
        fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `proj_create_from_database(ctx, authority, code, PJ_CATEGORY_CRS, ...)`:
/// the CRS is found, and it is a geographic CRS named `name`.
fn assert_crs_found(db: &DatabaseContext, authority: &str, code: &str, name: &str) {
    let crs = AuthorityFactory::create(db, authority)
        .create_geographic_crs(code)
        .unwrap_or_else(|error| panic!("{authority}:{code} not found: {error}"));
    assert_eq!(crs.name(), name);
}

/// The default context: the embedded `proj.db`. `EPSG:4326` is WGS 84.
fn assert_default_database_works() {
    let db = DatabaseContext::create().expect("default database");
    assert_crs_found(&db, "EPSG", "4326", "WGS 84");
}

/// `dbAux` of `proj_context_set_database_path_aux`: every statement of the
/// embedded structure, triggers included, as the C++ test runs them, then the
/// `OTHER_4326` row.
fn aux_with_other_crs(path: &Path) {
    let reference = DatabaseContext::create().expect("default database");
    let conn = Connection::open(path).expect("create auxiliary");
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("foreign keys off");
    for sql in reference.database_structure().expect("structure") {
        conn.execute_batch(&sql).expect("structure statement");
    }
    conn.execute_batch(
        "INSERT INTO geodetic_crs VALUES('OTHER','OTHER_4326','WGS 84',NULL,'geographic 2D',\
         'EPSG','6422','EPSG','6326',NULL,0);",
    )
    .expect("insert OTHER_4326");
}

/// `CApi.proj_context_set_database_path_null`: `proj_context_set_database_path`
/// with no path and no auxiliary list returns true, and EPSG:4326 is found.
#[test]
fn proj_context_set_database_path_null() {
    let db = DatabaseContext::create_with_auxiliary(None, &[]).expect("set path to null");
    assert_crs_found(&db, "EPSG", "4326", "WGS 84");
}

/// `CApi.proj_context_set_database_path_aux`: an auxiliary list of one database
/// with the same structure and an `OTHER_4326` row. Both CRSs are found.
#[test]
fn proj_context_set_database_path_aux() {
    let scratch = Scratch::new("aux");
    let aux = scratch.file("proj_test_aux.db");
    aux_with_other_crs(&aux);

    let db = DatabaseContext::create_with_auxiliary(None, &[aux]).expect("set aux list");
    assert_crs_found(&db, "EPSG", "4326", "WGS 84");
    assert_crs_found(&db, "OTHER", "OTHER_4326", "WGS 84");
}

/// `CApi.proj_context_set_database_path_error_1`: a path that does not exist
/// returns false. The default database still answers EPSG:4326.
#[test]
fn proj_context_set_database_path_error_1() {
    let missing = PathBuf::from("i_do_not_exist.db");
    assert!(DatabaseContext::create_with_auxiliary(Some(&missing), &[]).is_err());
    assert_default_database_works();
}

/// `CApi.proj_context_set_database_path_error_2`: an auxiliary list naming a
/// file that does not exist returns false. The default database still answers.
#[test]
fn proj_context_set_database_path_error_2() {
    let missing = PathBuf::from("i_do_not_exist.db");
    assert!(DatabaseContext::create_with_auxiliary(None, &[missing]).is_err());
    assert_default_database_works();
}
