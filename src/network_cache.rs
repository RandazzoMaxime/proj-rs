//! The grid chunk cache, `cache.db`: the remote grids in 16 KiB chunks, in a SQLite database.
//!
//! A port of PROJ 9.9.0 `src/networkfilemanager.cpp`: `DiskChunkCache` (the exclusive lock, the
//! schema, the LRU links), `NetworkChunkCache` (insert, reuse of the least recently used chunk,
//! the size limit) and `NetworkFilePropertiesCache` (the time-to-live of a file). The table and
//! column names are PROJ's, so a `cache.db` written by one library is read by the other.
//!
//! Where this differs from PROJ, the reason is in `docs/PORTING-network-cache.md`: a download is
//! stored whole, once it is complete, and an expired entry is downloaded again in full (no
//! conditional request).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, ErrorCode, OptionalExtension};

/// `DOWNLOAD_CHUNK_SIZE` in PROJ: every chunk is this long, the last one of a file may be shorter.
pub(crate) const CHUNK_SIZE: u64 = 16 * 1024;

/// The file name of the database in the user-writable directory (`pj_context_get_grid_cache_filename`).
pub(crate) const DEFAULT_FILE_NAME: &str = "cache.db";

/// The time-to-live of a checked file, `cache_ttl_sec = 86400` in `proj.ini`.
pub(crate) const DEFAULT_TTL_SECONDS: i64 = 86_400;

/// Lock attempts before the cache is given up, `PROJ_LOCK_MAX_ITERS` otherwise (`filemanager`
/// default 30, about a second).
const DEFAULT_LOCK_ITERATIONS: u32 = 30;

/// The URL given to a chunk that is no longer valid. Such a chunk is reused first.
const INVALIDATED: &str = "invalidated";

/// `cache_db_structure_sql` of PROJ 9.9.0, byte for byte.
const SCHEMA: &str = "CREATE TABLE properties(\
 url          TEXT PRIMARY KEY NOT NULL,\
 lastChecked  TIMESTAMP NOT NULL,\
 fileSize     INTEGER NOT NULL,\
 lastModified TEXT,\
 etag         TEXT\
);\
CREATE TABLE downloaded_file_properties(\
 url          TEXT PRIMARY KEY NOT NULL,\
 lastChecked  TIMESTAMP NOT NULL,\
 fileSize     INTEGER NOT NULL,\
 lastModified TEXT,\
 etag         TEXT\
);\
CREATE TABLE chunk_data(\
 id        INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),\
 data      BLOB NOT NULL\
);\
CREATE TABLE chunks(\
 id        INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),\
 url       TEXT NOT NULL,\
 offset    INTEGER NOT NULL,\
 data_id   INTEGER NOT NULL,\
 data_size INTEGER NOT NULL,\
 CONSTRAINT fk_chunks_url FOREIGN KEY (url) REFERENCES properties(url),\
 CONSTRAINT fk_chunks_data FOREIGN KEY (data_id) REFERENCES chunk_data(id)\
);\
CREATE INDEX idx_chunks ON chunks(url, offset);\
CREATE TABLE linked_chunks(\
 id        INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),\
 chunk_id  INTEGER NOT NULL,\
 prev      INTEGER,\
 next      INTEGER,\
 CONSTRAINT fk_links_chunkid FOREIGN KEY (chunk_id) REFERENCES chunks(id),\
 CONSTRAINT fk_links_prev FOREIGN KEY (prev) REFERENCES linked_chunks(id),\
 CONSTRAINT fk_links_next FOREIGN KEY (next) REFERENCES linked_chunks(id)\
);\
CREATE INDEX idx_linked_chunks_chunk_id ON linked_chunks(chunk_id);\
CREATE TABLE linked_chunks_head_tail(\
  head       INTEGER,\
  tail       INTEGER,\
  CONSTRAINT lht_head FOREIGN KEY (head) REFERENCES linked_chunks(id),\
  CONSTRAINT lht_tail FOREIGN KEY (tail) REFERENCES linked_chunks(id)\
);\
INSERT INTO linked_chunks_head_tail VALUES (NULL, NULL);";

/// What the cache keeps about a remote file: `FileProperties` in PROJ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FileProperties {
    /// Length of the file in bytes.
    pub(crate) size: u64,
    /// Unix time of the last check, in seconds.
    pub(crate) last_checked: i64,
    /// The `Last-Modified` header of the reply, empty when there was none.
    pub(crate) last_modified: String,
    /// The `ETag` header of the reply, empty when there was none.
    pub(crate) etag: String,
}

/// A `GET` reply: the body and the two validators the cache keeps.
#[derive(Debug)]
pub(crate) struct Fetched {
    pub(crate) body: Vec<u8>,
    pub(crate) last_modified: String,
    pub(crate) etag: String,
}

/// The cache settings of one builder, read when a grid is fetched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    /// The database file.
    pub(crate) path: PathBuf,
    /// The size limit in bytes. `None`, or `Some(0)`, is no limit (`max_size` of PROJ).
    pub(crate) max_bytes: Option<u64>,
    /// Seconds before a checked file is checked again. Zero or less never expires (PROJ
    /// expires only when `ttl > 0`).
    pub(crate) ttl_seconds: i64,
}

/// The open database, holding the exclusive lock until it is dropped (`DiskChunkCache`).
#[derive(Debug)]
pub(crate) struct DiskChunkCache {
    conn: Connection,
}

impl DiskChunkCache {
    /// Open or create the database at `path`, and take its exclusive lock.
    ///
    /// The lock is retried on the schedule of PROJ (5 ms, 10 ms, then 100 ms), up to
    /// `PROJ_LOCK_MAX_ITERS` attempts. `None` when the file cannot be opened or the lock is
    /// not taken: the caller then fetches without the cache, as PROJ does.
    pub(crate) fn open(path: &Path) -> Option<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _created = fs::create_dir_all(parent);
            }
        }
        let conn = Connection::open(path).ok()?;
        // PROJ's SQLite leaves foreign keys off, and so does the cache: the `invalidated` URL
        // and the order of the inserts rely on it. The bundled build turns them on.
        conn.pragma_update(None, "foreign_keys", false).ok()?;
        // The lock is retried below on PROJ's schedule, so SQLite must not wait by itself.
        conn.busy_timeout(Duration::ZERO).ok()?;
        let max_attempts = lock_max_iterations();
        let mut attempt = 0_u32;
        loop {
            match conn.execute_batch("BEGIN EXCLUSIVE") {
                Ok(()) => break,
                Err(error) if is_busy(&error) => {
                    if attempt >= max_attempts {
                        return None;
                    }
                    thread::sleep(retry_delay(attempt));
                    attempt += 1;
                }
                Err(_) => return None,
            }
        }
        let cache = Self { conn };
        let has_structure = cache
            .conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE name = 'properties'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .ok()?
            .is_some();
        if !has_structure {
            cache.conn.execute_batch(SCHEMA).ok()?;
        }
        Some(cache)
    }

    /// The properties of `url`, if the cache has them.
    pub(crate) fn properties(&self, url: &str) -> Option<FileProperties> {
        self.conn
            .query_row(
                "SELECT lastChecked, fileSize, lastModified, etag FROM properties WHERE url = ?1",
                [url],
                |row| {
                    Ok(FileProperties {
                        last_checked: row.get(0)?,
                        size: u64::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
                        last_modified: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        etag: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    })
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Store the properties of `url`, replacing those it had.
    fn set_properties(&self, url: &str, props: &FileProperties) -> rusqlite::Result<()> {
        let size = i64::try_from(props.size).unwrap_or(i64::MAX);
        self.conn.execute(
            "INSERT INTO properties (url, lastChecked, fileSize, lastModified, etag) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(url) DO UPDATE SET lastChecked = excluded.lastChecked, \
             fileSize = excluded.fileSize, lastModified = excluded.lastModified, \
             etag = excluded.etag",
            params![
                url,
                props.last_checked,
                size,
                props.last_modified,
                props.etag
            ],
        )?;
        Ok(())
    }

    /// The record of a whole-file download of `url` (`downloaded_file_properties`).
    pub(crate) fn download_properties(&self, url: &str) -> Option<FileProperties> {
        self.conn
            .query_row(
                "SELECT lastChecked, fileSize, lastModified, etag \
                 FROM downloaded_file_properties WHERE url = ?1",
                [url],
                |row| {
                    Ok(FileProperties {
                        last_checked: row.get(0)?,
                        size: u64::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
                        last_modified: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        etag: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    })
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Record a whole-file download of `url`, replacing the record it had.
    pub(crate) fn set_download_properties(
        &self,
        url: &str,
        props: &FileProperties,
    ) -> rusqlite::Result<()> {
        let size = i64::try_from(props.size).unwrap_or(i64::MAX);
        self.conn.execute(
            "INSERT INTO downloaded_file_properties (url, lastChecked, fileSize, lastModified, etag) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(url) DO UPDATE SET lastChecked = excluded.lastChecked, \
             fileSize = excluded.fileSize, lastModified = excluded.lastModified, \
             etag = excluded.etag",
            params![url, props.last_checked, size, props.last_modified, props.etag],
        )?;
        Ok(())
    }

    /// The chunk `index` of `url`, with its real length, and moved to the head of the LRU list.
    pub(crate) fn chunk(&self, url: &str, index: u64) -> Option<Vec<u8>> {
        let offset = i64::try_from(index.checked_mul(CHUNK_SIZE)?).ok()?;
        let (chunk_id, size, data): (i64, i64, Vec<u8>) = self
            .conn
            .query_row(
                "SELECT chunks.id, chunks.data_size, chunk_data.data FROM chunks \
                 JOIN chunk_data ON chunks.data_id = chunk_data.id \
                 WHERE chunks.url = ?1 AND chunks.offset = ?2",
                params![url, offset],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .ok()
            .flatten()?;
        let size = usize::try_from(size).ok()?;
        if u64::try_from(size).ok()? > CHUNK_SIZE || data.len() < size {
            return None;
        }
        let _moved = self.move_to_head(chunk_id);
        Some(data[..size].to_vec())
    }

    /// The whole file of `size` bytes from its chunks, or `None` when one is missing or short.
    pub(crate) fn assemble(&self, url: &str, size: u64) -> Option<Vec<u8>> {
        if size == 0 {
            return None;
        }
        let capacity = usize::try_from(size).ok()?;
        let mut out = Vec::with_capacity(capacity);
        for index in 0..size.div_ceil(CHUNK_SIZE) {
            let expected = (size - index * CHUNK_SIZE).min(CHUNK_SIZE);
            let chunk = self.chunk(url, index)?;
            if u64::try_from(chunk.len()).ok()? != expected {
                return None;
            }
            out.extend_from_slice(&chunk);
        }
        Some(out)
    }

    /// Store a complete download of `url`: the old chunks of `url` are invalidated, the new ones
    /// are inserted, and the properties are replaced.
    pub(crate) fn store(
        &self,
        url: &str,
        body: &[u8],
        props: &FileProperties,
        max_bytes: Option<u64>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE chunks SET url = ?1 WHERE url = ?2",
            params![INVALIDATED, url],
        )?;
        // From the last chunk to the first, so the first one ends at the head of the LRU list:
        // the head is what a reader touches last, as `cache_basic` expects.
        for (index, chunk) in body.chunks(to_usize(CHUNK_SIZE)).enumerate().rev() {
            self.store_chunk(
                url,
                u64::try_from(index).unwrap_or(u64::MAX),
                chunk,
                max_bytes,
            )?;
        }
        self.set_properties(url, props)
    }

    /// Insert one chunk (`NetworkChunkCache::insert`): refresh it when it is in the cache, reuse
    /// the tail of the LRU list when it is invalidated or the cache is full, else append it.
    fn store_chunk(
        &self,
        url: &str,
        index: u64,
        data: &[u8],
        max_bytes: Option<u64>,
    ) -> rusqlite::Result<()> {
        let offset = i64::try_from(index.saturating_mul(CHUNK_SIZE)).unwrap_or(i64::MAX);
        // Always CHUNK_SIZE bytes, to avoid fragmentation, as PROJ does.
        let mut blob = data.to_vec();
        blob.resize(to_usize(CHUNK_SIZE), 0);
        let size = i64::try_from(data.len()).unwrap_or(i64::MAX);

        let existing: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT id, data_id FROM chunks WHERE url = ?1 AND offset = ?2",
                params![url, offset],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((chunk_id, data_id)) = existing {
            self.conn.execute(
                "UPDATE chunk_data SET data = ?1 WHERE id = ?2",
                params![blob, data_id],
            )?;
            return self.move_to_head(chunk_id);
        }

        let tail: Option<(i64, i64, String)> = self
            .conn
            .query_row(
                "SELECT chunks.id, chunks.data_id, chunks.url FROM chunks \
                 JOIN linked_chunks ON linked_chunks.chunk_id = chunks.id \
                 WHERE linked_chunks.id = (SELECT tail FROM linked_chunks_head_tail)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let full = match max_bytes {
            Some(max) if max > 0 => {
                let count: i64 = self
                    .conn
                    .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
                u64::try_from(count).unwrap_or(0).saturating_mul(CHUNK_SIZE) >= max
            }
            _ => false,
        };
        if let Some((chunk_id, data_id, tail_url)) = tail {
            if tail_url == INVALIDATED || full {
                self.conn.execute(
                    "UPDATE chunk_data SET data = ?1 WHERE id = ?2",
                    params![blob, data_id],
                )?;
                self.conn.execute(
                    "UPDATE chunks SET url = ?1, offset = ?2, data_size = ?3, data_id = ?4 \
                     WHERE id = ?5",
                    params![url, offset, size, data_id, chunk_id],
                )?;
                return self.move_to_head(chunk_id);
            }
        }

        self.conn
            .execute("INSERT INTO chunk_data(data) VALUES (?1)", params![blob])?;
        let data_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "INSERT INTO chunks(url, offset, data_id, data_size) VALUES (?1, ?2, ?3, ?4)",
            params![url, offset, data_id, size],
        )?;
        let chunk_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "INSERT INTO linked_chunks(chunk_id, prev, next) VALUES (?1, NULL, NULL)",
            params![chunk_id],
        )?;
        self.move_to_head(chunk_id)
    }

    /// Move the link of `chunk_id` to the head of the LRU list (`move_to_head`).
    fn move_to_head(&self, chunk_id: i64) -> rusqlite::Result<()> {
        let (link, prev, next): (i64, Option<i64>, Option<i64>) = self.conn.query_row(
            "SELECT id, prev, next FROM linked_chunks WHERE chunk_id = ?1",
            params![chunk_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let (head, tail): (Option<i64>, Option<i64>) = self.conn.query_row(
            "SELECT head, tail FROM linked_chunks_head_tail",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if head == Some(link) {
            return Ok(());
        }
        if let Some(previous) = prev {
            self.conn.execute(
                "UPDATE linked_chunks SET next = ?1 WHERE id = ?2",
                params![next, previous],
            )?;
        }
        if let Some(following) = next {
            self.conn.execute(
                "UPDATE linked_chunks SET prev = ?1 WHERE id = ?2",
                params![prev, following],
            )?;
        }
        let tail = if tail == Some(link) { prev } else { tail };
        self.conn.execute(
            "UPDATE linked_chunks SET prev = NULL, next = ?1 WHERE id = ?2",
            params![head, link],
        )?;
        if let Some(old_head) = head {
            self.conn.execute(
                "UPDATE linked_chunks SET prev = ?1 WHERE id = ?2",
                params![link, old_head],
            )?;
        }
        self.conn.execute(
            "UPDATE linked_chunks_head_tail SET head = ?1, tail = ?2",
            params![link, tail.or(Some(link))],
        )?;
        Ok(())
    }
}

impl Drop for DiskChunkCache {
    fn drop(&mut self) {
        // The exclusive lock ends with the commit. A failure rolls back when the connection closes.
        let _committed = self.conn.execute_batch("COMMIT");
    }
}

/// The grid of `url`, from the cache when its entry is fresh and complete, else downloaded with
/// `download` and stored (`NetworkChunkCache::get` and `NetworkFilePropertiesCache::tryGet`).
///
/// When the cache cannot be opened, the download is returned and nothing is stored.
pub(crate) fn fetch_grid(
    settings: &Settings,
    url: &str,
    download: impl FnOnce() -> Option<Fetched>,
) -> Option<Vec<u8>> {
    let Some(cache) = DiskChunkCache::open(&settings.path) else {
        return download().map(|fetched| fetched.body);
    };
    let now = unix_now();
    if let Some(props) = cache.properties(url) {
        if !expired(&props, settings.ttl_seconds, now) {
            if let Some(bytes) = cache.assemble(url, props.size) {
                return Some(bytes);
            }
        }
    }
    let fetched = download()?;
    let props = FileProperties {
        size: u64::try_from(fetched.body.len()).unwrap_or(u64::MAX),
        last_checked: now,
        last_modified: fetched.last_modified.clone(),
        etag: fetched.etag.clone(),
    };
    // A failed store still gives the grid: the download is good, only the cache is not updated.
    let _stored = cache.store(url, &fetched.body, &props, settings.max_bytes);
    Some(fetched.body)
}

/// Remove the database and its journal files (`proj_grid_cache_clear`).
pub(crate) fn clear(path: &Path) -> io::Result<()> {
    for suffix in ["", "-journal", "-wal", "-shm"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        match fs::remove_file(PathBuf::from(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// PROJ expires an entry only when its time-to-live is positive and has run out.
fn expired(props: &FileProperties, ttl_seconds: i64, now: i64) -> bool {
    ttl_seconds > 0 && now > props.last_checked.saturating_add(ttl_seconds)
}

/// Whether a record checked at `props.last_checked` has run out now.
pub(crate) fn is_expired(props: &FileProperties, ttl_seconds: i64) -> bool {
    expired(props, ttl_seconds, unix_now())
}

fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(code, _) if code.code == ErrorCode::DatabaseBusy
    )
}

/// The wait before lock attempt `attempt`: 5 ms for the first ten, 10 ms for the next ten,
/// then 100 ms.
fn retry_delay(attempt: u32) -> Duration {
    Duration::from_millis(if attempt < 10 {
        5
    } else if attempt < 20 {
        10
    } else {
        100
    })
}

/// `PROJ_LOCK_MAX_ITERS`, read as `atoi` reads it, or the default of PROJ.
fn lock_max_iterations() -> u32 {
    match std::env::var("PROJ_LOCK_MAX_ITERS") {
        Ok(value) if !value.is_empty() => value.trim().parse().unwrap_or(0),
        _ => DEFAULT_LOCK_ITERATIONS,
    }
}

pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
