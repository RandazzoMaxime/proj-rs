//! `DatabaseContext::open` and `DatabaseContext::create_with_auxiliary`, the
//! Rust side of PROJ 9.9.0 `DatabaseContext::create(path, auxiliary)`.
//!
//! Expected values come from PROJ, never from the code under test:
//! - `WGS 84` for EPSG:4326 is what PROJ 9.9.0 `projinfo` prints for the same
//!   `proj.db`;
//! - the `OTHER_4326` row is the literal inserted by PROJ's
//!   `test/unit/test_factory.cpp` `TEST(factory, attachExtraDatabases_auxiliary)`;
//! - the error texts are those of `src/iso19111/factory.cpp`
//!   (`SQLiteHandle::checkDatabaseLayout`, `DatabaseContext::Private::open`,
//!   `attachExtraDatabases`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};

use proj_rs::iso19111::{AuthorityFactory, DatabaseContext, FactoryError};
use rusqlite::Connection;

/// A fresh directory under the system temporary directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("proj-rs-open-{}-{index}-{tag}", process::id()));
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

/// The `proj.db` shipped in the crate, the one PROJ 9.9.0 reads.
fn shipped_db() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/proj.db")
}

fn copy_shipped_db(scratch: &Scratch, name: &str) -> PathBuf {
    let path = scratch.file(name);
    fs::copy(shipped_db(), &path).expect("copy proj.db");
    path
}

/// `createStructure` of PROJ's `test_factory.cpp`: the tables and views of the
/// embedded schema, then its layout rows, in an empty database at `path`.
/// Triggers are left out, as PROJ's auxiliary-database test does, so the rows
/// inserted afterwards are not checked by them.
fn create_empty_structure(path: &Path) {
    let reference = DatabaseContext::create().expect("embedded proj.db");
    let conn = Connection::open(path).expect("create database");
    for sql in reference.database_structure().expect("structure") {
        if sql.contains("CREATE TRIGGER") {
            continue;
        }
        conn.execute_batch(&sql).expect("structure statement");
    }
}

/// An auxiliary database holding one geodetic CRS that the main database lacks.
fn auxiliary_with_other_crs(scratch: &Scratch) -> PathBuf {
    let path = scratch.file("other.db");
    create_empty_structure(&path);
    let conn = Connection::open(&path).expect("open auxiliary");
    // rusqlite turns foreign keys on; PROJ's test connection leaves them off.
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF; INSERT INTO geodetic_crs VALUES('OTHER','OTHER_4326','WGS 84',NULL,\
         'geographic 2D','EPSG','6422','EPSG','6326',NULL,0);",
    )
    .expect("insert OTHER_4326");
    path
}

fn error_text(error: &FactoryError) -> &str {
    match error {
        FactoryError::Sql(text) => text,
        other => panic!("expected a database error, got {other:?}"),
    }
}

fn geographic_name(db: &DatabaseContext, authority: &str, code: &str) -> String {
    let factory = AuthorityFactory::create(db, authority);
    factory
        .create_geographic_crs(code)
        .expect("geographic CRS")
        .name()
        .to_owned()
}

#[test]
fn open_reads_the_shipped_database_as_proj_does() {
    let scratch = Scratch::new("open");
    let db = DatabaseContext::open(&copy_shipped_db(&scratch, "proj.db")).expect("open");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
}

#[test]
fn open_missing_file_is_an_error() {
    let scratch = Scratch::new("missing");
    let error = DatabaseContext::open(&scratch.file("absent.db")).expect_err("missing file");
    assert!(error_text(&error).contains("Open of "), "{error}");
}

#[test]
fn open_file_that_is_not_sqlite_is_an_error() {
    let scratch = Scratch::new("not-sqlite");
    let path = scratch.file("text.db");
    fs::write(&path, b"this is not an SQLite database").expect("write");
    assert!(DatabaseContext::open(&path).is_err());
}

#[test]
fn open_without_layout_rows_is_an_error() {
    let scratch = Scratch::new("no-layout");
    let path = scratch.file("bare.db");
    Connection::open(&path)
        .expect("create")
        .execute_batch("CREATE TABLE metadata(key TEXT, value TEXT);")
        .expect("metadata table");
    let error = DatabaseContext::open(&path).expect_err("no layout rows");
    assert!(
        error_text(&error).contains(
            "lacks DATABASE.LAYOUT.VERSION.MAJOR / DATABASE.LAYOUT.VERSION.MINOR metadata"
        ),
        "{error}"
    );
}

#[test]
fn open_rejects_a_minor_version_below_7() {
    let scratch = Scratch::new("minor-6");
    let path = copy_shipped_db(&scratch, "proj.db");
    Connection::open(&path)
        .expect("open copy")
        .execute(
            "UPDATE metadata SET value = '6' WHERE key = 'DATABASE.LAYOUT.VERSION.MINOR'",
            [],
        )
        .expect("update");
    let error = DatabaseContext::open(&path).expect_err("minor 6");
    assert!(
        error_text(&error).contains(
            "contains DATABASE.LAYOUT.VERSION.MINOR = 6 whereas a number >= 7 is expected"
        ),
        "{error}"
    );
}

#[test]
fn open_rejects_a_major_version_other_than_1() {
    let scratch = Scratch::new("major-2");
    let path = copy_shipped_db(&scratch, "proj.db");
    Connection::open(&path)
        .expect("open copy")
        .execute(
            "UPDATE metadata SET value = '2' WHERE key = 'DATABASE.LAYOUT.VERSION.MAJOR'",
            [],
        )
        .expect("update");
    let error = DatabaseContext::open(&path).expect_err("major 2");
    assert!(
        error_text(&error)
            .contains("contains DATABASE.LAYOUT.VERSION.MAJOR = 2 whereas 1 is expected"),
        "{error}"
    );
}

#[test]
fn no_auxiliary_database_is_the_embedded_database() {
    let db = DatabaseContext::create_with_auxiliary(None, &[]).expect("embedded");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
}

#[test]
fn no_auxiliary_database_on_a_file_is_open() {
    let scratch = Scratch::new("no-aux-file");
    let path = copy_shipped_db(&scratch, "proj.db");
    let db = DatabaseContext::create_with_auxiliary(Some(&path), &[]).expect("file");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
}

#[test]
fn auxiliary_objects_are_found_next_to_the_main_ones() {
    let scratch = Scratch::new("aux-found");
    let other = auxiliary_with_other_crs(&scratch);
    let embedded_structure = DatabaseContext::create()
        .expect("embedded")
        .database_structure()
        .expect("structure");

    let db = DatabaseContext::create_with_auxiliary(None, &[other]).expect("attach");
    assert_eq!(geographic_name(&db, "OTHER", "OTHER_4326"), "WGS 84");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
    // PROJ: `dbStructure` after attaching equals the structure before.
    assert_eq!(
        db.database_structure().expect("structure"),
        embedded_structure
    );
}

#[test]
fn auxiliary_is_attached_to_a_file_main_database_too() {
    let scratch = Scratch::new("aux-file-main");
    let main = copy_shipped_db(&scratch, "proj.db");
    let other = auxiliary_with_other_crs(&scratch);
    let db = DatabaseContext::create_with_auxiliary(Some(&main), &[other]).expect("attach");
    assert_eq!(geographic_name(&db, "OTHER", "OTHER_4326"), "WGS 84");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
}

/// `TEST(factory, attachExtraDatabases_auxiliary)`, second block: the file
/// auxiliary and an empty `:memory:` one together. Both lookups succeed.
#[test]
fn auxiliary_file_and_memory_auxiliary_are_both_attached() {
    let scratch = Scratch::new("aux-and-memory");
    let other = auxiliary_with_other_crs(&scratch);
    let db = DatabaseContext::create_with_auxiliary(None, &[other, PathBuf::from(":memory:")])
        .expect("attach file and :memory:");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
    assert_eq!(geographic_name(&db, "OTHER", "OTHER_4326"), "WGS 84");
}

/// `TEST(factory, attachExtraDatabases_auxiliary)`, third block: an empty
/// `:memory:` auxiliary. The main object is found, the auxiliary one is a
/// `FactoryException` in PROJ, a `NotFound` here.
#[test]
fn memory_auxiliary_database_is_empty() {
    let db = DatabaseContext::create_with_auxiliary(None, &[PathBuf::from(":memory:")])
        .expect("attach :memory:");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
    let factory = AuthorityFactory::create(&db, "OTHER");
    assert!(matches!(
        factory.create_geographic_crs("OTHER_4326"),
        Err(FactoryError::NotFound { .. })
    ));
}

#[test]
fn missing_auxiliary_database_is_an_error_and_is_not_created() {
    let name = "i_dont_exist_db";
    let error = DatabaseContext::create_with_auxiliary(None, &[PathBuf::from(name)])
        .expect_err("missing auxiliary");
    assert!(
        error_text(&error).contains("Open of i_dont_exist_db failed"),
        "{error}"
    );
    assert!(!Path::new(name).exists());
}

#[test]
fn auxiliary_with_a_layout_different_from_the_main_one_is_an_error() {
    let scratch = Scratch::new("aux-minor-8");
    let other = scratch.file("other8.db");
    create_empty_structure(&other);
    Connection::open(&other)
        .expect("open auxiliary")
        .execute(
            "UPDATE metadata SET value = '8' WHERE key = 'DATABASE.LAYOUT.VERSION.MINOR'",
            [],
        )
        .expect("update");
    let error = DatabaseContext::create_with_auxiliary(None, &[other]).expect_err("layout 1.8");
    assert!(
        error_text(&error).contains(
            "contains a DATABASE.LAYOUT.VERSION =  1.8 which is different from the one from the main database"
        ),
        "{error}"
    );
}

#[test]
fn auxiliary_with_minor_below_7_is_an_error() {
    let scratch = Scratch::new("aux-minor-6");
    let other = scratch.file("other6.db");
    create_empty_structure(&other);
    Connection::open(&other)
        .expect("open auxiliary")
        .execute(
            "UPDATE metadata SET value = '6' WHERE key = 'DATABASE.LAYOUT.VERSION.MINOR'",
            [],
        )
        .expect("update");
    let error = DatabaseContext::create_with_auxiliary(None, &[other]).expect_err("minor 6");
    assert!(
        error_text(&error).contains("contains DATABASE.LAYOUT.VERSION.MINOR = 6"),
        "{error}"
    );
}

#[test]
fn auxiliary_without_metadata_is_accepted_as_sparse() {
    let scratch = Scratch::new("sparse");
    let sparse = scratch.file("sparse.db");
    Connection::open(&sparse)
        .expect("create")
        .execute_batch("CREATE TABLE unrelated(x INTEGER);")
        .expect("table");
    let db = DatabaseContext::create_with_auxiliary(None, &[sparse]).expect("sparse accepted");
    assert_eq!(geographic_name(&db, "EPSG", "4326"), "WGS 84");
}
