//! Embedded PROJ 9.9.0 `proj.db`, opened once from the crate bytes.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};

const PROJ_DB: &[u8] = include_bytes!("../data/proj.db");
/// The 38 triggers of `proj.db`, left out of the image: SQLite parses them at every load of the
/// schema, about 2.7 MiB of heap for a read. A writable copy replays them (`tools/proj-db`).
const PROJ_TRIGGERS: &str = include_str!("../data/proj-triggers.sql");

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
    read_only_image()
        .and_then(with_aux)
        .inspect(keep_statements)
        .map(Mutex::new)
        .map_err(|error| error.to_string())
}

/// The embedded bytes as a read-only image. On Linux the connection reads them from the
/// executable file with `pread` (`projdb_exe`), so the pages stay out of the resident set. Else
/// it reads the pages of the binary in place: `mmap_size` keeps SQLite from copying each page it
/// reads into its cache.
fn read_only_image() -> Result<Connection, rusqlite::Error> {
    #[cfg(target_os = "linux")]
    if let Some(connection) = crate::projdb_exe::open(PROJ_DB) {
        return Ok(connection);
    }
    let mut connection = Connection::open_in_memory()?;
    connection.deserialize_bytes(rusqlite::MAIN_DB, PROJ_DB)?;
    connection.pragma_update(None, "mmap_size", MMAP_SIZE)?;
    Ok(connection)
}

const MMAP_SIZE: i64 = 1 << 30;

/// Compiled statements a connection keeps for its queries. A search repeats them (`EPSG:4267 ->
/// EPSG:4326`: 2 699 compilations for 30 distinct statements); 30 pairs of CRS use 72 in all.
/// 16 already holds most of a search, 64 leaves room for a search that uses more.
pub(crate) const STATEMENT_CACHE: usize = 64;

/// Let `connection` keep [`STATEMENT_CACHE`] compiled statements for `prepare_cached`.
pub(crate) fn keep_statements(connection: &Connection) {
    connection.set_prepared_statement_cache_capacity(STATEMENT_CACHE);
}

/// `connection` with the auxiliary databases of `PROJ_AUX_DB` attached: the paths are separated
/// by `;` on Windows and by `:` elsewhere, as `DatabaseContext::create` reads them.
fn with_aux(connection: Connection) -> Result<Connection, rusqlite::Error> {
    attach_aux(&connection)?;
    Ok(connection)
}

/// `DatabaseContext::attachExtraDatabases`, for the connections of this process: each auxiliary
/// database of `PROJ_AUX_DB` is attached, and its rows of every base table are copied into the
/// table of the same name in the main database. A row of the auxiliary database replaces the
/// main row with the same key. The views of the main database then read both, as PROJ's
/// queries do. `metadata` is not copied: each database's layout is checked on its own.
fn attach_aux(connection: &Connection) -> Result<(), rusqlite::Error> {
    let Ok(paths) = std::env::var("PROJ_AUX_DB") else {
        return Ok(());
    };
    let mut aliases = Vec::new();
    let separator = if cfg!(windows) { ';' } else { ':' };
    for (index, path) in paths
        .split(separator)
        .filter(|path| !path.is_empty())
        .enumerate()
    {
        let alias = format!("aux{index}");
        let quoted = path.replace('\'', "''");
        connection.execute_batch(&format!("ATTACH DATABASE '{quoted}' AS {alias}"))?;
        aliases.push(alias);
    }
    let tables: Vec<String> = connection
        .prepare(
            "SELECT name FROM main.sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    // `usage` refers to the objects of every other table (`usage_insert_trigger`), so it is
    // copied last.
    let mut order: Vec<&String> = tables
        .iter()
        .filter(|table| table.as_str() != "metadata" && table.as_str() != "usage")
        .collect();
    order.extend(tables.iter().filter(|table| table.as_str() == "usage"));
    for table in order {
        let columns: Vec<String> = connection
            .prepare(&format!("PRAGMA main.table_info(\"{table}\")"))?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<_, _>>()?;
        let list = columns
            .iter()
            .map(|column| format!("\"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        for alias in &aliases {
            let present: Option<String> = connection
                .query_row(
                    &format!(
                        "SELECT name FROM {alias}.sqlite_master WHERE type = 'table' AND name = ?1"
                    ),
                    [table],
                    |row| row.get(0),
                )
                .ok();
            if present.is_some() {
                connection.execute_batch(&format!(
                    "INSERT OR REPLACE INTO main.\"{table}\" ({list}) SELECT {list} FROM {alias}.\"{table}\""
                ))?;
            }
        }
    }
    Ok(())
}

/// Copy the main database of `source` into `target`, which stays writable. An
/// insert session writes to it and rolls the writes back.
fn copy_into(source: &Connection, mut target: Connection) -> Result<Connection, rusqlite::Error> {
    Backup::new(source, &mut target)?.run_to_completion(i32::MAX, Duration::ZERO, None)?;
    target.execute_batch(PROJ_TRIGGERS)?;
    Ok(target)
}

/// A writable in-memory copy of the embedded `proj.db`.
fn open_memory() -> Result<Connection, rusqlite::Error> {
    copy_into(&read_only_image()?, Connection::open_in_memory()?).and_then(with_aux)
}

thread_local! {
    static READER: RefCell<Weak<Mutex<Connection>>> = const { RefCell::new(Weak::new()) };
}

/// A read-only connection to the embedded `proj.db` that reads the bytes of the binary in
/// place, shared by every caller of this thread while one of them holds it: the schema of
/// `proj.db` costs about 2.4 MiB of SQLite heap for each connection. The last holder to drop
/// closes it. `None` when `PROJ_AUX_DB` is set: the auxiliary rows go into the main database,
/// so each caller needs [`open_connection`].
pub(crate) fn shared_read_only() -> Result<Option<Arc<Mutex<Connection>>>, DbFail> {
    if std::env::var_os("PROJ_AUX_DB").is_some() {
        return Ok(None);
    }
    READER.with(|reader| {
        if let Some(open) = reader.borrow().upgrade() {
            return Ok(Some(open));
        }
        let image = read_only_image().map_err(|error| DbFail::Open(error.to_string()))?;
        keep_statements(&image);
        let open = Arc::new(Mutex::new(image));
        *reader.borrow_mut() = Arc::downgrade(&open);
        Ok(Some(open))
    })
}

/// A connection of its own for a nested read: the image of the binary, or the copy that holds
/// the auxiliary rows.
fn open_lazy_or_copy() -> Result<Connection, DbFail> {
    if std::env::var_os("PROJ_AUX_DB").is_some() {
        return open_connection();
    }
    read_only_image().map_err(|error| DbFail::Open(error.to_string()))
}

/// A read-only connection of its own to the embedded `proj.db`, with no auxiliary database.
pub(crate) fn read_only_connection() -> Result<Connection, DbFail> {
    read_only_image().map_err(|error| DbFail::Open(error.to_string()))
}

/// The bytes of the embedded `proj.db`.
pub(crate) fn embedded_bytes() -> &'static [u8] {
    PROJ_DB
}

/// A private, writable connection to the embedded `proj.db`.
///
/// Each call copies the bytes. The shared slot used by [`with_db`] stays
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
        copy_into(&read_only_image()?, target).and_then(with_aux)
    };
    open().map_err(|_| DbFail::Open("Open of proj.db failed".to_owned()))
}

/// The database file at `path`, opened read-only as PROJ opens `proj.db`,
/// through the SQLite VFS `vfs` or SQLite's default when `None`.
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
        let connection = open_lazy_or_copy()?;
        return body(&connection).map_err(DbFail::Sql);
    }
    let shared = shared_read_only()?;
    let guard = match &shared {
        Some(open) => open.lock().unwrap_or_else(|poison| poison.into_inner()),
        None => match slot() {
            Ok(mutex) => mutex.lock().unwrap_or_else(|poison| poison.into_inner()),
            Err(message) => return Err(DbFail::Open(message.clone())),
        },
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

#[cfg(test)]
mod trigger_tests {
    use super::*;

    fn triggers(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'",
                [],
                |row| row.get(0),
            )
            .expect("count")
    }

    #[test]
    fn the_read_image_has_no_triggers_and_a_writable_copy_has_them_all() {
        let image = read_only_image().expect("image");
        assert_eq!(triggers(&image), 0);
        let copy = open_memory().expect("copy");
        let written =
            i64::try_from(PROJ_TRIGGERS.matches("CREATE TRIGGER").count()).expect("count");
        assert_eq!(triggers(&copy), written);
    }
}
