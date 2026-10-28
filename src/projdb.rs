//! Embedded PROJ 9.9.0 `proj.db`, opened once from the crate bytes.

use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

const PROJ_DB: &[u8] = include_bytes!("../data/proj.db");

/// Opening the embedded database failed, or a query did.
#[derive(Debug)]
pub(crate) enum DbFail {
    /// `deserialize_bytes` or the in-memory connection failed.
    Open(String),
    /// A statement failed.
    Sql(rusqlite::Error),
}

fn open_embedded() -> Result<Mutex<Connection>, String> {
    let mut connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .deserialize_bytes(rusqlite::MAIN_DB, PROJ_DB)
        .map_err(|error| error.to_string())?;
    Ok(Mutex::new(connection))
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
    let guard = match slot() {
        Ok(mutex) => mutex.lock().unwrap_or_else(|poison| poison.into_inner()),
        Err(message) => return Err(DbFail::Open(message.clone())),
    };
    body(&guard).map_err(DbFail::Sql)
}
