//! PROJ 9.9.0 `test/unit/test_c_api.cpp` cases about the context and the insert
//! session. Each function keeps the C++ `TEST_F(CApi, ...)` name, with the
//! `c_api_` prefix of the other ported files. Expected values come from the C++
//! source, not from the code under test.
//!
//! The process default (`Context::set_default_*`) is shared by every test in
//! this binary, so the tests that use it take [`global_guard`].

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use proj_rs::{
    suggests_code_for, AuthorityFactory, AuthorityObject, CodeForm, ComponentCodes, Context,
    DatabaseContext, DatabaseSource, FactoryError, InitRules, InsertSession, WktParser,
};

static GLOBAL_DEFAULTS: Mutex<()> = Mutex::new(());

/// Serializes the tests that read or change the process default, and puts it
/// back to the built-in settings.
fn global_guard() -> MutexGuard<'static, ()> {
    let guard = GLOBAL_DEFAULTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Context::set_default_database(DatabaseSource::Embedded);
    Context::set_default_sqlite_vfs_name(None);
    Context::set_default_init_rules(InitRules::Iso19111);
    guard
}

/// A copy of the `proj.db` the crate embeds, as a file of the temporary
/// directory. The caller removes it.
fn copy_of_proj_db(name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/proj.db");
    let target = std::env::temp_dir().join(format!("proj-rs-{name}-{}.db", std::process::id()));
    std::fs::copy(source, &target).expect("copy proj.db");
    target
}

/// Whether `database` resolves `EPSG:4326`, as `proj_create(ctx, "EPSG:4326")`.
fn knows_epsg_4326(database: &DatabaseContext) -> bool {
    AuthorityFactory::create(database, "EPSG")
        .create_geodetic_crs("4326")
        .is_ok()
}

#[test]
fn c_api_proj_context_copy_from_default() {
    let _guard = global_guard();
    let copy = copy_of_proj_db("copy-from-default");

    // The new context takes the database of the default context.
    Context::set_default_database(DatabaseSource::File(copy.clone()));
    let context = Context::new();
    Context::set_default_database(DatabaseSource::Embedded);

    assert_eq!(context.database_path(), Some(copy.as_path()));
    let database = context.database().expect("database from the copy");
    assert!(knows_epsg_4326(&database));
    std::fs::remove_file(&copy).expect("remove copy");
}

#[test]
fn c_api_proj_context_clone() {
    let _guard = global_guard();
    let mut context = Context::new();
    let new_rules = match Context::default_init_rules() {
        InitRules::Iso19111 => InitRules::Proj4,
        InitRules::Proj4 => InitRules::Iso19111,
    };
    context.set_init_rules(new_rules);

    let mut clone = context.clone();
    assert_eq!(clone.init_rules(), context.init_rules());
    assert_ne!(Context::new().init_rules(), clone.init_rules());

    // The clone is a copy: changing it leaves the original alone.
    clone.set_init_rules(Context::default_init_rules());
    assert_eq!(context.init_rules(), new_rules);
}

#[test]
fn c_api_proj_context_set_sqlite3_vfs_name() {
    let _guard = global_guard();
    let mut context = Context::new();

    // An unknown VFS is an error, and the message is PROJ's.
    context.set_sqlite_vfs_name(Some("dummy_vfs_name"));
    match context.database() {
        Err(FactoryError::Sql(message)) => assert_eq!(message, "Open of proj.db failed"),
        other => panic!("expected an open error, got {other:?}"),
    }

    // Restoring SQLite's default VFS opens the database again.
    context.set_sqlite_vfs_name(None);
    let database = context.database().expect("database");
    assert!(knows_epsg_4326(&database));
}

#[test]
fn c_api_proj_context_set_sqlite3_vfs_name_from_global_context() {
    let _guard = global_guard();

    Context::set_default_sqlite_vfs_name(Some("dummy_vfs_name"));
    let context = Context::new();
    assert!(context.database().is_err());

    Context::set_default_sqlite_vfs_name(None);
    let context = Context::new();
    assert!(knows_epsg_4326(&context.database().expect("database")));
}

#[test]
fn c_api_use_proj4_init_rules_from_global_context() {
    let _guard = global_guard();

    Context::set_default_init_rules(InitRules::Proj4);
    let proj4 = Context::new();
    assert_eq!(proj4.init_rules(), InitRules::Proj4);

    Context::set_default_init_rules(InitRules::Iso19111);
    let iso = Context::new();
    assert_eq!(iso.init_rules(), InitRules::Iso19111);
    // A context keeps the rules it was created with.
    assert_eq!(proj4.init_rules(), InitRules::Proj4);
}

#[cfg(target_os = "linux")]
fn open_file_descriptors() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .expect("list the open file descriptors")
        .count()
}

#[test]
fn c_api_open_plenty_of_contexts() {
    let _guard = global_guard();
    let copy = copy_of_proj_db("plenty-of-contexts");

    // Every context keeps one connection to the database file, and nothing
    // more: its lookups do not open the file again.
    #[cfg(target_os = "linux")]
    let before = open_file_descriptors();
    let mut databases = Vec::new();
    for _ in 0..100 {
        let context = Context::builder()
            .database(DatabaseSource::File(copy.clone()))
            .build();
        let database = context.database().expect("database");
        assert!(knows_epsg_4326(&database));
        databases.push(database);
    }
    #[cfg(target_os = "linux")]
    assert_eq!(open_file_descriptors() - before, 100);

    drop(databases);
    std::fs::remove_file(&copy).expect("remove copy");
}

#[test]
fn c_api_proj_get_insert_statements() {
    let wkt = concat!(
        "GEOGCRS[\"myGDA2020\",\n",
        "    DATUM[\"GDA2020\",\n",
        "        ELLIPSOID[\"GRS_1980\",6378137,298.257222101,\n",
        "            LENGTHUNIT[\"metre\",1]]],\n",
        "    PRIMEM[\"Greenwich\",0,\n",
        "        ANGLEUNIT[\"Degree\",0.0174532925199433]],\n",
        "    CS[ellipsoidal,2],\n",
        "        AXIS[\"geodetic latitude (Lat)\",north,\n",
        "            ORDER[1],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]],\n",
        "        AXIS[\"geodetic longitude (Lon)\",east,\n",
        "            ORDER[2],\n",
        "            ANGLEUNIT[\"degree\",0.0174532925199433]]]"
    );
    let db = DatabaseContext::create().expect("proj.db");
    let parsed = WktParser::new().create_from_wkt(wkt).expect("wkt");
    let crs = AuthorityObject::Crs(parsed.crs().expect("crs").clone());
    // `NULL` allowed authorities in PROJ is {"EPSG", "PROJ"}.
    let default_allowed = ["EPSG".to_owned(), "PROJ".to_owned()];

    // Suggested codes: the name in capitals, and the next number.
    assert_eq!(
        suggests_code_for(&db, &crs, "HOBU", CodeForm::FromName).expect("code"),
        "MYGDA2020"
    );
    assert_eq!(
        suggests_code_for(&db, &crs, "HOBU", CodeForm::Numeric).expect("code"),
        "1"
    );

    // A session, as proj_insert_object_session_create().
    let session = InsertSession::start(&db).expect("session");
    // Only one session at a time: the second create returns NULL in PROJ.
    assert!(InsertSession::start(&db).is_err());

    let list = session
        .statements_for(
            &crs,
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &default_allowed,
        )
        .expect("statements");
    assert_eq!(
        list[0],
        "INSERT INTO geodetic_datum VALUES('HOBU',\
         'GEODETIC_DATUM_XXXX','GDA2020','','EPSG','7019',\
         'EPSG','8901',NULL,NULL,NULL,NULL,NULL,0);"
    );
    assert_eq!(list.len(), 4);

    // The objects of the first call are in the session: nothing to insert again.
    let again = session
        .statements_for(
            &crs,
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &default_allowed,
        )
        .expect("statements");
    assert!(again.is_empty());

    // Stopping the session discards what it inserted.
    drop(session);
    let session = InsertSession::start(&db).expect("second session");
    let list = session
        .statements_for(
            &crs,
            "HOBU",
            "XXXX",
            ComponentCodes::FromParentCode,
            &default_allowed,
        )
        .expect("statements");
    assert_eq!(list.len(), 4);
    drop(session);

    // Each allowed list is checked in a session of its own, as the C++ test
    // does with temporary sessions. Without EPSG the ellipsoid and the prime
    // meridian cannot be reused.
    let count = |allowed: &[String]| {
        let session = InsertSession::start(&db).expect("session");
        session
            .statements_for(
                &crs,
                "HOBU",
                "XXXX",
                ComponentCodes::FromParentCode,
                allowed,
            )
            .expect("statements")
            .len()
    };
    assert_eq!(count(&[]), 6);
    assert_eq!(count(&["PROJ".to_owned()]), 6);
    assert_eq!(count(&["EPSG".to_owned()]), 4);
}
