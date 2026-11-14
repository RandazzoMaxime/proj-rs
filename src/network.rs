//! On-demand grid download, following PROJ 9.9 `pj_load_ini` and
//! `FileManager::open_resource_file`.
//!
//! The default is network off and `cdn_endpoint = https://cdn.proj.org`.
//! `PROJ_NETWORK` and `PROJ_NETWORK_ENDPOINT` override `proj.ini`. A missing
//! file is fetched only when [`decide_download`] says so.

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CDN_ENDPOINT: &str = "https://cdn.proj.org";
pub(crate) const MAX_GRID_BYTES: u64 = 512 * 1024 * 1024;

/// An open remote file, as the application's callbacks name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NetworkHandle(pub u64);

/// What `open` gives back: the handle, and the bytes of the first range it was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenReply {
    /// The handle later `header`, `read_range` and `close` calls take.
    pub handle: NetworkHandle,
    /// The bytes read, at most the size asked for.
    pub data: Vec<u8>,
}

/// A remote read that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NetworkError {
    /// `open` failed. The text is the application's message.
    Open(String),
    /// `read_range` failed. The text is the application's message.
    ReadRange(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(message) => write!(formatter, "cannot open the remote file: {message}"),
            Self::ReadRange(message) => write!(formatter, "cannot read the remote file: {message}"),
        }
    }
}

impl std::error::Error for NetworkError {}

/// Replaces the built-in HTTP client for grids that are not on disk, as PROJ's
/// `proj_context_set_network_callbacks`.
///
/// A grid is fetched by one `open(url, 0, 16384)`, then `read_range` calls of 16384 bytes
/// until the size given by the `Content-Range` header (`header`) is reached, then `close`.
/// Each call is made on the thread that builds or transforms.
pub trait NetworkCallbacks: Send + Sync {
    /// Open `url` and read `size_to_read` bytes from `offset`.
    ///
    /// # Errors
    ///
    /// [`NetworkError::Open`] when the file cannot be opened.
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError>;

    /// The value of a response header of an open file, `None` when it is absent.
    fn header(&self, handle: NetworkHandle, name: &str) -> Option<String> {
        let _unused = (handle, name);
        None
    }

    /// Read `size_to_read` bytes of an open file from `offset`.
    ///
    /// # Errors
    ///
    /// [`NetworkError::ReadRange`] when the read fails.
    fn read_range(
        &self,
        handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError>;

    /// Release an open file. Called once per `open`, also after an error.
    fn close(&self, handle: NetworkHandle);
}

/// The callbacks a builder registered, as the context holds them, with the chunks they have
/// served (`network_chunks`). Clones share the callbacks and the chunks.
#[derive(Clone)]
pub(crate) struct Callbacks {
    imp: Arc<dyn NetworkCallbacks>,
    pub(crate) chunks: Arc<Mutex<crate::network_chunks::ChunkCache>>,
}

impl Callbacks {
    pub(crate) fn new(imp: Arc<dyn NetworkCallbacks>) -> Self {
        Self {
            imp,
            chunks: Arc::new(Mutex::new(crate::network_chunks::ChunkCache::default())),
        }
    }

    pub(crate) fn imp(&self) -> &dyn NetworkCallbacks {
        self.imp.as_ref()
    }
}

impl fmt::Debug for Callbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callbacks")
    }
}
/// `gridChunkCache.max_size` default in PROJ 9.9.0 (`cache_size_MB = 300` in `proj.ini`).
const DEFAULT_CACHE_MAX_BYTES: u64 = 300 * 1024 * 1024;
/// `gridChunkCache.ttl` default in PROJ 9.9.0 (`cache_ttl_sec = 86400`).
const DEFAULT_CACHE_TTL_SECONDS: i64 = crate::network_cache::DEFAULT_TTL_SECONDS;

thread_local! {
    static ACTIVE: RefCell<Option<GridContext>> = const { RefCell::new(None) };
    static DEFER_GRIDS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAST_NOT_FOUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the last HTTP fetch of this thread was answered `404` or `410`: the file is not on
/// the server, which is not the same as the server being out of reach. A grid that is not there
/// drops its candidate when the candidates are built; one that cannot be reached is kept and
/// reported by the first transform (`proj_create_crs_to_crs` with `ONLY_BEST` and the network).
pub(crate) fn last_fetch_not_found() -> bool {
    LAST_NOT_FOUND.with(std::cell::Cell::get)
}

/// Whether a grid that is not on disk may be left for the first transform: only while the
/// candidates of a CRS pair are built. An explicit operation string fails at creation, as PROJ
/// does with an unreachable endpoint (`proj_create` gives `NULL`).
pub(crate) fn grids_may_be_deferred() -> bool {
    DEFER_GRIDS.with(std::cell::Cell::get)
}

/// Run `body` with [`grids_may_be_deferred`] on, then restore the previous setting.
pub(crate) fn with_deferred_grids<T>(body: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            DEFER_GRIDS.with(|flag| flag.set(self.0));
        }
    }
    let previous = DEFER_GRIDS.with(|flag| flag.replace(true));
    let _restore = Restore(previous);
    body()
}

/// Network, cache and `ONLY_BEST` default settings for one builder, or the process default.
///
/// The `ONLY_BEST` default is a `pj_load_ini` setting in PROJ, which keeps it on the context
/// (`ctx->errorIfBestTransformationNotAvailableDefault`) beside the network settings.
#[derive(Clone, Debug)]
pub(crate) struct GridContext {
    pub(crate) enabled: bool,
    pub(crate) endpoint: String,
    pub(crate) cache_enabled: bool,
    /// `Some(true)` is `ONLY_BEST=YES`, `Some(false)` is `ONLY_BEST=NO`, and `None` is no
    /// setting: `crs_to_crs` then keeps PROJ's warning default.
    pub(crate) only_best_default: Option<bool>,
    /// The grid chunk database, `proj_grid_cache_set_filename`. `None` is `cache.db` in the
    /// user-writable directory.
    pub(crate) cache_path: Option<PathBuf>,
    /// The size limit of the chunk database in bytes; `None` is no limit.
    pub(crate) cache_max_bytes: Option<u64>,
    /// Seconds before a cached file is checked again; zero or less never expires.
    pub(crate) cache_ttl_seconds: i64,
    cache_dir: Option<PathBuf>,
    search_paths: Vec<PathBuf>,
    ca_bundle: Option<PathBuf>,
    file_finder: Option<FileFinder>,
    callbacks: Option<Callbacks>,
}

/// The caller's file lookup, consulted before the search directories.
#[derive(Clone)]
pub(crate) struct FileFinder(pub(crate) std::sync::Arc<FinderFn>);

/// The callable a [`FileFinder`] holds.
pub(crate) type FinderFn = dyn Fn(&str) -> Option<PathBuf> + Send + Sync;

impl std::fmt::Debug for FileFinder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FileFinder")
    }
}

/// A grid the caller should fetch. The URL is `endpoint + "/" + filename`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Download {
    pub(crate) url: String,
}

/// Whether a missing grid is fetched.
///
/// `None` when the network is off, the name is a filesystem path, or the file
/// is already on disk. The caller then keeps the missing-file error and does
/// not invent a coordinate. `Some` is the CDN URL after `grid_alternatives`
/// renames a short name such as `alaska`.
#[must_use]
pub(crate) fn decide_download(name: &str) -> Option<Download> {
    let ctx = context();
    if !ctx.enabled || crate::gridpath::resolve(name).is_some() {
        return None;
    }
    let url = remote_url_in(&ctx, name)?;
    // A URL with no file name at its end is not a grid to fetch.
    cache_file_name_for(&url, name)?;
    Some(Download { url })
}

/// Download the grid [`decide_download`] selected. With the cache on, the grid is read from
/// the chunk database when its entry is fresh, else downloaded and stored there
/// ([`crate::network_cache::fetch_grid`]).
pub(crate) fn fetch_missing(name: &str) -> Option<Vec<u8>> {
    let download = decide_download(name)?;
    let ctx = context();
    if let Some(callbacks) = &ctx.callbacks {
        return fetch_with_callbacks(callbacks, &download.url);
    }
    if !ctx.cache_enabled {
        return http_get(&download.url);
    }
    crate::network_cache::fetch_grid(&ctx.cache_settings(), &download.url, || {
        http_get_validated(&download.url)
    })
}

/// The whole file at `url`, read through the application's callbacks in 16384-byte chunks
/// (`network_chunks`).
///
/// The total size comes from the `Content-Range` header (`bytes 0-16383/4153510`, or
/// `bytes=0-16383/...` in the C++ tests). Without it, the first reply is the whole file.
/// Any failure gives `None`, and the handle is closed in every case.
fn fetch_with_callbacks(callbacks: &Callbacks, url: &str) -> Option<Vec<u8>> {
    let mut file = crate::network_chunks::RemoteFile::new(url, callbacks.clone());
    let bytes = file.read_whole(MAX_GRID_BYTES);
    file.release();
    bytes
}

/// The total size after the `/` of a `Content-Range` value, `bytes 0-16383/4153510`.
pub(crate) fn content_range_total(value: &str) -> Option<u64> {
    value.rsplit_once('/')?.1.trim().parse().ok()
}

/// `proj_grid_name` for an old short name such as `alaska`.
pub(crate) fn proj_grid_filename(name: &str) -> Option<String> {
    if !bare_short_name(name) {
        return None;
    }
    crate::projdb::with_db(|connection| {
        connection.query_row(
            "SELECT proj_grid_name FROM grid_alternatives \
             WHERE old_proj_grid_name = ?1 AND proj_grid_name <> ''",
            [name],
            |row| row.get(0),
        )
    })
    .ok()
}

/// `old_proj_grid_name` for a PROJ grid name such as `de_adv_BETA2007.tif`.
///
/// As PROJ 9.9.0 `DatabaseContext::getOldProjGridName` (`factory.cpp:3575`):
/// the first row for `proj_grid_name`, and nothing when that row has no old
/// name. Only a name containing `.tif` is looked up, as
/// `FileManager::open_resource_file` does (`filemanager.cpp:1833-1857`).
pub(crate) fn old_grid_filename(name: &str) -> Option<String> {
    if !plain_name(name) || !name.contains(".tif") {
        return None;
    }
    let old = crate::projdb::with_db(|connection| {
        connection.query_row(
            "SELECT old_proj_grid_name FROM grid_alternatives WHERE proj_grid_name = ?1",
            [name],
            |row| row.get::<_, Option<String>>(0),
        )
    })
    .ok()
    .flatten();
    old.filter(|old| !old.is_empty())
}

pub(crate) fn search_paths() -> Vec<PathBuf> {
    context().search_paths
}

/// The default search path as `proj_info` reports it: the directories added to the
/// default context, or else the data directories when none were added.
pub(crate) fn default_searchpath() -> String {
    let added = search_paths();
    if added.is_empty() {
        join_directories(&data_directories())
    } else {
        join_directories(&added)
    }
}

/// Directories joined by the search path separator of the platform: `;` on
/// Windows, `:` elsewhere. A directory that is not UTF-8 is left out.
fn join_directories(directories: &[PathBuf]) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    directories
        .iter()
        .filter_map(|path| path.to_str())
        .collect::<Vec<_>>()
        .join(separator)
}

/// The data directories, read from the environment on each lookup as PROJ does.
///
/// `PROJ_DATA`, else the legacy `PROJ_LIB`, replaces the default data
/// directory, which is PROJ's compile-time share directory
/// (`pj_get_default_searchpaths`). An empty value counts as unset.
pub(crate) fn data_directories() -> Vec<PathBuf> {
    match proj_data_env() {
        Some(value) => split_directories(&value),
        None => vec![default_data_directory()],
    }
}

fn proj_data_env() -> Option<OsString> {
    data_path_value(std::env::var_os("PROJ_DATA"), std::env::var_os("PROJ_LIB"))
}

/// `PROJ_DATA` if it is set, else `PROJ_LIB`, when the value is not empty.
/// An empty `PROJ_DATA` hides `PROJ_LIB`, as in `getProjDataEnvVar`.
fn data_path_value(proj_data: Option<OsString>, proj_lib: Option<OsString>) -> Option<OsString> {
    proj_data.or(proj_lib).filter(|value| !value.is_empty())
}

/// The default data directory: `data/grids`, which holds the grids a user
/// places in the tree (git-ignored).
fn default_data_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("grids")
}

/// Split a search path the way PROJ does: `:` on Unix, `;` on Windows. `;` and
/// `|` are accepted everywhere. Empty entries are dropped.
fn split_directories(value: &OsStr) -> Vec<PathBuf> {
    let text = value.to_string_lossy();
    let separators: &[char] = if cfg!(windows) {
        &[';', '|']
    } else {
        &[':', ';', '|']
    };
    text.split(separators)
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// A dictionary or other file looked up by name: the caller's file finder first,
/// then the search directories (`pj_open_lib`).
pub(crate) fn find_file(name: &str) -> Option<PathBuf> {
    let ctx = context();
    if let Some(finder) = &ctx.file_finder {
        if let Some(path) = (finder.0)(name) {
            return Some(path);
        }
    }
    crate::gridpath::resolve(name)
}

pub(crate) fn cache_dir_if_enabled() -> Option<PathBuf> {
    let ctx = context();
    // The skip covers the default directory only. A directory set on the
    // builder is read, as PROJ does for an explicit context.
    let skip_default = ctx.cache_dir.is_none()
        && nonempty_env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY")
            .is_some_and(|value| truthy(&value));
    if ctx.cache_enabled && !skip_default {
        Some(ctx.cache_directory())
    } else {
        None
    }
}

pub(crate) fn context() -> GridContext {
    ACTIVE
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(default_context)
}

fn default_context() -> GridContext {
    static DEFAULT: OnceLock<GridContext> = OnceLock::new();
    DEFAULT.get_or_init(GridContext::load).clone()
}

pub(crate) fn with_context<T>(ctx: &GridContext, body: impl FnOnce() -> T) -> T {
    struct Restore(Option<GridContext>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            ACTIVE.with(|slot| *slot.borrow_mut() = previous);
        }
    }
    let previous = ACTIVE.with(|slot| slot.replace(Some(ctx.clone())));
    let _restore = Restore(previous);
    body()
}

impl GridContext {
    pub(crate) fn load() -> Self {
        let mut ctx = Self {
            enabled: false,
            endpoint: String::new(),
            cache_enabled: true,
            only_best_default: None,
            cache_path: None,
            cache_max_bytes: Some(DEFAULT_CACHE_MAX_BYTES),
            cache_ttl_seconds: DEFAULT_CACHE_TTL_SECONDS,
            cache_dir: None,
            search_paths: Vec::new(),
            ca_bundle: None,
            file_finder: None,
            callbacks: None,
        };
        let env_network = nonempty_env("PROJ_NETWORK");
        let env_endpoint = nonempty_env("PROJ_NETWORK_ENDPOINT");
        let env_only_best = std::env::var_os("PROJ_ONLY_BEST_DEFAULT");
        ctx.only_best_default = only_best_from_env(env_only_best.as_deref());
        if let Some(text) = read_proj_ini() {
            apply_ini(
                &text,
                &mut ctx,
                &IniSkip {
                    network: env_network.is_some(),
                    endpoint: env_endpoint.is_some(),
                    only_best: env_only_best.is_some(),
                },
            );
        }
        if let Some(value) = env_network {
            ctx.enabled = truthy(&value);
        }
        if let Some(value) = env_endpoint {
            ctx.endpoint = value;
        }
        if ctx.endpoint.is_empty() {
            ctx.endpoint = CDN_ENDPOINT.to_owned();
        }
        ctx
    }

    pub(crate) fn cache_directory(&self) -> PathBuf {
        self.cache_dir.clone().unwrap_or_else(default_user_dir)
    }

    /// The chunk database this context reads and writes: the path set on the builder, else
    /// `cache.db` in the user-writable directory (`pj_context_get_grid_cache_filename`).
    pub(crate) fn cache_settings(&self) -> crate::network_cache::Settings {
        let path = self.cache_path.clone().unwrap_or_else(|| {
            self.cache_directory()
                .join(crate::network_cache::DEFAULT_FILE_NAME)
        });
        crate::network_cache::Settings {
            path,
            max_bytes: self.cache_max_bytes,
            ttl_seconds: self.cache_ttl_seconds,
        }
    }

    pub(crate) fn searchpath(&self) -> String {
        join_directories(&self.search_paths)
    }

    pub(crate) fn push_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    pub(crate) fn set_cache_dir(&mut self, path: PathBuf) {
        self.cache_dir = Some(path);
    }

    /// The directories added by `set_search_paths`, in order.
    pub(crate) fn added_search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// The CA bundle set by `set_ca_bundle_path`, if any.
    pub(crate) fn ca_bundle(&self) -> Option<&Path> {
        self.ca_bundle.as_deref()
    }

    pub(crate) fn set_ca_bundle(&mut self, path: PathBuf) {
        self.ca_bundle = Some(path);
    }

    /// Forget the directory set by `set_user_writable_directory`: the default is used again.
    pub(crate) fn clear_cache_dir(&mut self) {
        self.cache_dir = None;
    }

    pub(crate) fn set_file_finder(&mut self, finder: FileFinder) {
        self.file_finder = Some(finder);
    }

    pub(crate) fn clear_file_finder(&mut self) {
        self.file_finder = None;
    }

    pub(crate) fn set_callbacks(&mut self, callbacks: Callbacks) {
        self.callbacks = Some(callbacks);
    }

    /// The callbacks of this context, shared with its clones, when the application set some.
    pub(crate) fn callbacks(&self) -> Option<Callbacks> {
        self.callbacks.clone()
    }
}

fn remote_url_in(ctx: &GridContext, name: &str) -> Option<String> {
    if is_explicit_path(name) {
        return None;
    }
    if is_http(name) {
        return Some(name.to_owned());
    }
    let file_name = proj_grid_filename(name).unwrap_or_else(|| name.to_owned());
    if let Some(url) = direct_url(&file_name) {
        if let Some(rest) = url.strip_prefix("https://cdn.proj.org/") {
            return join_endpoint(&ctx.endpoint, rest);
        }
        if is_http(&url) {
            return Some(url);
        }
    }
    join_endpoint(&ctx.endpoint, &file_name)
}

fn direct_url(name: &str) -> Option<String> {
    let (url, direct): (Option<String>, Option<i64>) = crate::projdb::with_db(|connection| {
        connection.query_row(
            "SELECT url, direct_download FROM grid_alternatives \
             WHERE proj_grid_name = ?1 OR old_proj_grid_name = ?1 LIMIT 1",
            [name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    })
    .ok()?;
    if direct == Some(1) {
        url.filter(|value| !value.is_empty())
    } else {
        None
    }
}

fn cache_file_name_for(url: &str, name: &str) -> Option<String> {
    if is_http(name) {
        cache_file_name(url)
    } else {
        let file = proj_grid_filename(name).unwrap_or_else(|| name.to_owned());
        cache_file_name(&file)
    }
}

fn join_endpoint(endpoint: &str, file: &str) -> Option<String> {
    if endpoint.is_empty() || file.is_empty() || file.contains('\0') {
        return None;
    }
    if endpoint.ends_with('/') {
        Some(format!("{endpoint}{file}"))
    } else {
        Some(format!("{endpoint}/{file}"))
    }
}

fn bare_short_name(name: &str) -> bool {
    plain_name(name) && !name.contains(".tif")
}

/// A name that is neither a path nor a URL, as PROJ's
/// `is_rel_or_absolute_filename` and `starts_with(name, "http")` tests.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !is_http(name)
        && !is_explicit_path(name)
}

fn is_http(name: &str) -> bool {
    name.starts_with("http://") || name.starts_with("https://")
}

fn is_explicit_path(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| *byte == b'/' || *byte == b'\\')
    {
        return true;
    }
    if name.starts_with("./")
        || name.starts_with(".\\")
        || name.starts_with("../")
        || name.starts_with("..\\")
        || name.starts_with("~/")
        || name.starts_with("~\\")
    {
        return true;
    }
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn cache_file_name(name: &str) -> Option<String> {
    if name.is_empty() || name.contains('\0') || name.split(['/', '\\']).any(|part| part == "..") {
        return None;
    }
    let file = name.rsplit(['/', '\\', '?']).next()?;
    if file.is_empty() || file == "." || file == ".." {
        None
    } else {
        Some(file.to_owned())
    }
}

fn http_get(url: &str) -> Option<Vec<u8>> {
    http_get_validated(url).map(|fetched| fetched.body)
}

/// A `GET` that follows the redirects, with the `Last-Modified` and `ETag` of the reply.
pub(crate) fn http_get_validated(url: &str) -> Option<crate::network_cache::Fetched> {
    LAST_NOT_FOUND.with(|flag| flag.set(false));
    let mut url = url.to_owned();
    for _ in 0..4 {
        let target = parse_http_url(&url)?;
        let reply = http_exchange(&target)?;
        if reply.status == 200 {
            if reply.body.is_empty() {
                return None;
            }
            let validator = |name| header_value(&reply.headers, name).unwrap_or("").to_owned();
            return Some(crate::network_cache::Fetched {
                last_modified: validator("last-modified"),
                etag: validator("etag"),
                body: reply.body,
            });
        }
        if !matches!(reply.status, 301 | 302 | 303 | 307 | 308) {
            LAST_NOT_FOUND.with(|flag| flag.set(matches!(reply.status, 404 | 410)));
            return None;
        }
        let location = header_value(&reply.headers, "location")?;
        url = resolve_location(&target, location)?;
    }
    None
}

struct HttpTarget {
    https: bool,
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> Option<HttpTarget> {
    let (https, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else {
        (false, url.strip_prefix("http://")?)
    };
    if rest.is_empty() || rest.contains(['\r', '\n', ' ']) {
        return None;
    }
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_owned()),
    };
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let (host, port) = split_host_port(authority, if https { 443 } else { 80 })?;
    Some(HttpTarget {
        https,
        host,
        port,
        path,
    })
}

fn split_host_port(authority: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        if host.is_empty() {
            return None;
        }
        let port = if let Some(port) = tail.strip_prefix(':') {
            if port.is_empty() {
                return None;
            }
            port.parse().ok()?
        } else if tail.is_empty() {
            default_port
        } else {
            return None;
        };
        return Some((host.to_owned(), port));
    }
    if let Some((host, port)) = authority.rsplit_once(':') {
        if !host.is_empty() && !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some((host.to_owned(), port.parse().ok()?));
        }
    }
    if authority.is_empty() {
        return None;
    }
    Some((authority.to_owned(), default_port))
}

fn resolve_location(current: &HttpTarget, location: &str) -> Option<String> {
    if is_http(location) {
        return Some(location.to_owned());
    }
    let path = location.strip_prefix('/')?;
    let scheme = if current.https { "https" } else { "http" };
    Some(format!(
        "{scheme}://{}:{}/{}",
        current.host, current.port, path
    ))
}

struct HttpReply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn http_exchange(target: &HttpTarget) -> Option<HttpReply> {
    let stream = TcpStream::connect((target.host.as_str(), target.port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .ok()?;
    if target.https {
        let connector = native_tls::TlsConnector::new().ok()?;
        let tls = connector.connect(&target.host, stream).ok()?;
        read_http(tls, target)
    } else {
        read_http(stream, target)
    }
}

fn read_http(mut stream: impl Read + Write, target: &HttpTarget) -> Option<HttpReply> {
    let host = if (target.https && target.port == 443) || (!target.https && target.port == 80) {
        target.host.clone()
    } else {
        format!("{}:{}", target.host, target.port)
    };
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: proj-rs\r\nConnection: close\r\n\r\n",
        path = target.path
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = Vec::new();
    stream
        .take(MAX_GRID_BYTES.saturating_add(65_536))
        .read_to_end(&mut raw)
        .ok()?;
    let reply = split_http(&raw)?;
    if u64::try_from(reply.body.len()).ok()? > MAX_GRID_BYTES {
        return None;
    }
    Some(reply)
}

fn split_http(raw: &[u8]) -> Option<HttpReply> {
    let end = raw.windows(4).position(|mark| mark == b"\r\n\r\n")?;
    let header_text = std::str::from_utf8(&raw[..end]).ok()?;
    let mut lines = header_text.split("\r\n");
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    let mut body = raw[end + 4..].to_vec();
    if header_value(&headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        body = decode_chunks(&body)?;
    } else if let Some(length) = header_value(&headers, "content-length") {
        let length = length.parse::<usize>().ok()?;
        if body.len() < length {
            return None;
        }
        body.truncate(length);
    }
    Some(HttpReply {
        status,
        headers,
        body,
    })
}

fn decode_chunks(mut bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        let line_end = bytes.windows(2).position(|mark| mark == b"\r\n")?;
        let size_text = std::str::from_utf8(&bytes[..line_end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        bytes = &bytes[line_end + 2..];
        if size == 0 {
            break;
        }
        if bytes.len() < size + 2 {
            return None;
        }
        out.extend_from_slice(&bytes[..size]);
        if &bytes[size..size + 2] != b"\r\n" {
            return None;
        }
        bytes = &bytes[size + 2..];
    }
    Some(out)
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn default_user_dir() -> PathBuf {
    if let Some(dir) = nonempty_env("PROJ_USER_WRITABLE_DIRECTORY") {
        return PathBuf::from(dir);
    }
    if let Some(local) = nonempty_env("LOCALAPPDATA") {
        return PathBuf::from(local).join("proj");
    }
    std::env::temp_dir().join("proj")
}

fn read_proj_ini() -> Option<String> {
    let mut dirs = proj_data_env()
        .map(|value| split_directories(&value))
        .unwrap_or_default();
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));
    for dir in dirs {
        let path = dir.join("proj.ini");
        if let Ok(text) = std::fs::read_to_string(&path) {
            if text.len() <= 100 * 1024 {
                return Some(text);
            }
        }
    }
    None
}

struct IniSkip {
    network: bool,
    endpoint: bool,
    /// `PROJ_ONLY_BEST_DEFAULT` is set, even empty, so `proj.ini` does not override it.
    only_best: bool,
}

fn apply_ini(text: &str, ctx: &mut GridContext, skip: &IniSkip) {
    for line in text.split(['\n', '\r']) {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "cdn_endpoint" if !skip.endpoint => ctx.endpoint = value.to_owned(),
            "network" if !skip.network => ctx.enabled = truthy(value),
            "cache_enabled" => ctx.cache_enabled = truthy(value),
            // filemanager.cpp:2083-2086: a positive size in MB, else unlimited.
            "cache_size_MB" => {
                ctx.cache_max_bytes = value
                    .parse::<u64>()
                    .ok()
                    .filter(|megabytes| *megabytes > 0)
                    .map(|megabytes| megabytes.saturating_mul(1024 * 1024));
            }
            "cache_ttl_sec" => ctx.cache_ttl_seconds = value.parse().unwrap_or(0),
            "only_best_default" if !skip.only_best => ctx.only_best_default = Some(truthy(value)),
            _ => {}
        }
    }
}

/// `PROJ_ONLY_BEST_DEFAULT` as `pj_load_ini` reads it (`filemanager.cpp:2025-2031`). An empty
/// value is no setting. Any other value is `ONLY_BEST=YES` when it is YES, ON or TRUE, and
/// `ONLY_BEST=NO` otherwise.
fn only_best_from_env(value: Option<&OsStr>) -> Option<bool> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| truthy(&value.to_string_lossy()))
}

fn truthy(value: &str) -> bool {
    value.eq_ignore_ascii_case("on")
        || value.eq_ignore_ascii_case("yes")
        || value.eq_ignore_ascii_case("true")
}

fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_ini, data_path_value, decide_download, only_best_from_env, remote_url_in,
        split_directories, with_context, GridContext, IniSkip,
    };
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    #[test]
    fn only_best_env_follows_pj_load_ini() {
        // filemanager.cpp:2025-2031. Empty is no setting. YES, ON or TRUE (any case) is
        // ONLY_BEST=YES, and every other value is ONLY_BEST=NO. Oracle: scenarios s02-s09.
        let env = |value: &str| only_best_from_env(Some(OsStr::new(value)));
        assert_eq!(only_best_from_env(None), None);
        assert_eq!(env(""), None);
        assert_eq!(env("YES"), Some(true));
        assert_eq!(env("on"), Some(true));
        assert_eq!(env("TRUE"), Some(true));
        assert_eq!(env("NO"), Some(false));
        assert_eq!(env("maybe"), Some(false));
        assert_eq!(env(" yes"), Some(false));
        assert_eq!(env("0"), Some(false));
    }

    #[test]
    fn only_best_ini_key_follows_pj_load_ini() {
        // filemanager.cpp:2101-2107. The key is exact after trimming, and any value other than
        // ON, YES or TRUE is ONLY_BEST=NO. Oracle: scenarios s10-s15, s23-s26, s29-s31.
        let ini = |text: &str| {
            let mut ctx = GridContext::load();
            ctx.only_best_default = None;
            let skip = IniSkip {
                network: false,
                endpoint: false,
                only_best: false,
            };
            apply_ini(text, &mut ctx, &skip);
            ctx.only_best_default
        };
        assert_eq!(ini("only_best_default=on\n"), Some(true));
        assert_eq!(ini("only_best_default=YES\n"), Some(true));
        assert_eq!(ini("only_best_default = true \n"), Some(true));
        assert_eq!(ini("foo=1\r\nonly_best_default=on\r\n"), Some(true));
        assert_eq!(ini("only_best_default=off\n"), Some(false));
        assert_eq!(ini("only_best_default=\n"), Some(false));
        assert_eq!(ini("only_best_default=bogus\n"), Some(false));
        assert_eq!(ini("only_best_default=\"on\"\n"), Some(false));
        assert_eq!(
            ini("only_best_default=on\nonly_best_default=off\n"),
            Some(false)
        );
        assert_eq!(ini("ONLY_BEST_DEFAULT=on\n"), None);
        assert_eq!(ini("=on\n"), None);
        assert_eq!(ini("#only_best_default=on\n"), None);
        assert_eq!(ini("only_best=on\n"), None);
    }

    #[test]
    fn only_best_env_hides_the_ini_key_even_when_empty() {
        // filemanager.cpp:2101 tests `getenv(...) == nullptr`, so an empty variable hides the key
        // as well. Oracle: scenarios s17 and s28.
        let mut ctx = GridContext::load();
        ctx.only_best_default = only_best_from_env(Some(OsStr::new("")));
        let skip = IniSkip {
            network: false,
            endpoint: false,
            only_best: true,
        };
        apply_ini("only_best_default=on\n", &mut ctx, &skip);
        assert_eq!(ctx.only_best_default, None);
    }

    #[test]
    fn proj_data_replaces_proj_lib_and_empty_hides_it() {
        let data = || Some(OsString::from("/data"));
        let lib = || Some(OsString::from("/lib"));
        assert_eq!(data_path_value(data(), lib()), data());
        assert_eq!(data_path_value(None, lib()), lib());
        assert_eq!(data_path_value(Some(OsString::new()), lib()), None);
        assert_eq!(data_path_value(None, None), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn search_path_is_split_on_colon_and_drops_empty_entries() {
        assert_eq!(
            split_directories(&OsString::from("a:b;c|::d:")),
            ["a", "b", "c", "d"].map(PathBuf::from).to_vec()
        );
    }

    #[cfg(windows)]
    #[test]
    fn search_path_keeps_drive_letters_on_windows() {
        // `:` is part of a drive letter on Windows: only `;` and `|` split.
        assert_eq!(
            split_directories(&OsString::from(r"C:\a;D:\b|;")),
            [r"C:\a", r"D:\b"].map(PathBuf::from).to_vec()
        );
    }

    #[test]
    fn decide_download_requires_the_network() {
        let mut ctx = GridContext::load();
        ctx.endpoint = "http://127.0.0.1:9".to_owned();
        ctx.enabled = false;
        let off = with_context(&ctx, || decide_download("proj_rs_no_such_grid.tif"));
        assert!(off.is_none());
        ctx.enabled = true;
        let on =
            with_context(&ctx, || decide_download("proj_rs_no_such_grid.tif")).expect("network on");
        assert_eq!(on.url, "http://127.0.0.1:9/proj_rs_no_such_grid.tif");
    }

    #[test]
    fn alaska_short_name_is_resolved_before_the_cdn() {
        crate::gridpath::use_fixture_search_path();
        let mut ctx = GridContext::load();
        ctx.enabled = true;
        ctx.endpoint = "http://127.0.0.1:9".to_owned();
        let url = with_context(&ctx, || remote_url_in(&ctx, "alaska")).expect("alaska");
        assert_eq!(url, "http://127.0.0.1:9/us_noaa_alaska.tif");
        let decision = with_context(&ctx, || decide_download("alaska"));
        assert!(
            decision.is_none(),
            "the vendored alaska grid is not fetched"
        );
    }
}

#[cfg(test)]
mod callback_tests {
    use super::{
        content_range_total, fetch_with_callbacks, Callbacks, NetworkCallbacks, NetworkError,
        NetworkHandle, OpenReply,
    };
    use std::sync::{Arc, Mutex};

    /// A remote file served by the test, which records each call. The callbacks are the boundary
    /// of the crate, so they are the only thing faked here.
    struct Served {
        body: Vec<u8>,
        opens: Mutex<Vec<(u64, usize)>>,
        reads: Mutex<Vec<(u64, usize)>>,
        closes: Mutex<usize>,
    }

    impl Served {
        fn slice(&self, offset: u64, size: usize) -> Vec<u8> {
            let start = usize::try_from(offset)
                .unwrap_or(usize::MAX)
                .min(self.body.len());
            let end = start.saturating_add(size).min(self.body.len());
            self.body[start..end].to_vec()
        }
    }

    impl NetworkCallbacks for Served {
        fn open(
            &self,
            _url: &str,
            offset: u64,
            size_to_read: usize,
        ) -> Result<OpenReply, NetworkError> {
            if let Ok(mut opens) = self.opens.lock() {
                opens.push((offset, size_to_read));
            }
            Ok(OpenReply {
                handle: NetworkHandle(7),
                data: self.slice(offset, size_to_read),
            })
        }

        fn header(&self, _handle: NetworkHandle, name: &str) -> Option<String> {
            (name == "Content-Range").then(|| format!("bytes 0-16383/{}", self.body.len()))
        }

        fn read_range(
            &self,
            _handle: NetworkHandle,
            offset: u64,
            size_to_read: usize,
        ) -> Result<Vec<u8>, NetworkError> {
            if let Ok(mut reads) = self.reads.lock() {
                reads.push((offset, size_to_read));
            }
            Ok(self.slice(offset, size_to_read))
        }

        fn close(&self, _handle: NetworkHandle) {
            if let Ok(mut closes) = self.closes.lock() {
                *closes += 1;
            }
        }
    }

    /// The whole file comes back in 16384-byte ranges after the first one, and the handle is
    /// closed once.
    #[test]
    fn whole_file_is_read_in_16384_byte_ranges() {
        let body: Vec<u8> = (0..40_000_u32)
            .map(|value| u8::try_from(value % 251).unwrap_or(0))
            .collect();
        let served = Arc::new(Served {
            body: body.clone(),
            opens: Mutex::new(Vec::new()),
            reads: Mutex::new(Vec::new()),
            closes: Mutex::new(0),
        });
        let imp: Arc<dyn NetworkCallbacks> = Arc::<Served>::clone(&served);
        let callbacks = Callbacks::new(imp);
        let bytes = fetch_with_callbacks(&callbacks, "https://foo/my.tif").expect("fetched");
        assert_eq!(bytes, body);
        assert_eq!(*served.opens.lock().expect("lock"), vec![(0, 16384)]);
        assert_eq!(
            *served.reads.lock().expect("lock"),
            vec![(16384, 16384), (32768, 7232)]
        );
        assert_eq!(*served.closes.lock().expect("lock"), 1);
    }

    #[test]
    fn content_range_gives_the_total_size() {
        assert_eq!(
            content_range_total("bytes 0-16383/4153510"),
            Some(4_153_510)
        );
        assert_eq!(
            content_range_total("bytes=0-16383/10000000"),
            Some(10_000_000)
        );
        assert_eq!(content_range_total("bytes */*"), None);
    }
}
