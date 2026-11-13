//! Embedded PROJ 9.9.0 `proj.db`, opened once from the crate bytes.

use std::cell::Cell;
use std::fmt;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};

const PROJ_DB: &[u8] = include_bytes!("../data/proj.db");

/// Opening the embedded database failed, or a query did.
#[derive(Debug)]
pub(crate) enum DbFail {
    /// `deserialize_bytes` or the in-memory connection failed.
    Open(String),
    /// A statement failed.
    Sql(rusqlite::Error),
}

impl fmt::Display for DbFail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(message) => formatter.write_str(message),
            Self::Sql(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for DbFail {}

fn open_embedded() -> Result<Mutex<Connection>, String> {
    open_memory()
        .map(Mutex::new)
        .map_err(|error| error.to_string())
}

/// The embedded bytes as a read-only in-memory image.
fn read_only_image() -> Result<Connection, rusqlite::Error> {
    let mut connection = Connection::open_in_memory()?;
    connection.deserialize_bytes(rusqlite::MAIN_DB, PROJ_DB)?;
    Ok(connection)
}

/// Copy the main database of `source` into `target`, which stays writable: an
/// insert session writes to it and rolls the writes back.
fn copy_into(source: &Connection, mut target: Connection) -> Result<Connection, rusqlite::Error> {
    Backup::new(source, &mut target)?.run_to_completion(i32::MAX, Duration::ZERO, None)?;
    Ok(target)
}

fn open_memory() -> Result<Connection, rusqlite::Error> {
    copy_into(&read_only_image()?, Connection::open_in_memory()?)
}

/// A private connection to the embedded `proj.db`.
///
/// Each call deserializes the bytes. The shared slot used by [`with_db`] stays
/// free, so an ISO 19111 factory can nest queries without taking that lock.
pub(crate) fn open_connection() -> Result<Connection, DbFail> {
    open_memory().map_err(|error| DbFail::Open(error.to_string()))
}

/// The embedded `proj.db` opened through the SQLite VFS `vfs`.
///
/// SQLite looks the VFS up before it reads anything, so an unknown name fails
/// here as it would for a file. The message is PROJ's `Open of <path> failed`.
pub(crate) fn open_embedded_with_vfs(vfs: &str) -> Result<Connection, DbFail> {
    let open = || -> Result<Connection, rusqlite::Error> {
        let target = Connection::open_with_flags_and_vfs(":memory:", OpenFlags::default(), vfs)?;
        copy_into(&read_only_image()?, target)
    };
    open().map_err(|_| DbFail::Open("Open of proj.db failed".to_owned()))
}

/// The database file at `path`, opened read-only as PROJ opens `proj.db`.
///
/// `vfs` names the SQLite VFS to open it through, or `None` for SQLite's
/// default.
pub(crate) fn open_file(path: &Path, vfs: Option<&str>) -> Result<Connection, DbFail> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
    let opened = match vfs {
        Some(name) => Connection::open_with_flags_and_vfs(path, flags, name),
        None => Connection::open_with_flags(path, flags),
    };
    opened.map_err(|_| DbFail::Open(format!("Open of {} failed", path.display())))
}

fn slot() -> &'static Result<Mutex<Connection>, String> {
    static DB: OnceLock<Result<Mutex<Connection>, String>> = OnceLock::new();
    DB.get_or_init(open_embedded)
}

/// Run `body` against the embedded database.
///
/// # Errors
///
/// [`DbFail::Open`] when the bytes cannot be deserialized, or [`DbFail::Sql`]
/// from `body`.
pub(crate) fn with_db<T>(
    body: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
) -> Result<T, DbFail> {
    if HELD.with(Cell::get) {
        let connection = open_connection()?;
        return body(&connection).map_err(DbFail::Sql);
    }
    let guard = match slot() {
        Ok(mutex) => mutex.lock().unwrap_or_else(|poison| poison.into_inner()),
        Err(message) => return Err(DbFail::Open(message.clone())),
    };
    let _held = Held::set();
    body(&guard).map_err(DbFail::Sql)
}

thread_local! {
    static HELD: Cell<bool> = const { Cell::new(false) };
}

/// Marks the shared connection as taken by this thread, until dropped.
struct Held;

impl Held {
    fn set() -> Self {
        HELD.with(|held| held.set(true));
        Self
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        HELD.with(|held| held.set(false));
    }
}
