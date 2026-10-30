//! On-demand grid download, following PROJ 9.9 `pj_load_ini` and
//! `FileManager::open_resource_file`.
//!
//! The default is network off and `cdn_endpoint = https://cdn.proj.org`.
//! `PROJ_NETWORK` and `PROJ_NETWORK_ENDPOINT` override `proj.ini`. A missing
//! file is fetched only when [`decide_download`] says so.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

const CDN_ENDPOINT: &str = "https://cdn.proj.org";
const MAX_GRID_BYTES: u64 = 512 * 1024 * 1024;

thread_local! {
    static ACTIVE: RefCell<Option<GridContext>> = const { RefCell::new(None) };
}

/// Network and cache settings for one builder, or the process default.
#[derive(Clone, Debug)]
pub(crate) struct GridContext {
    pub(crate) enabled: bool,
    pub(crate) endpoint: String,
    pub(crate) cache_enabled: bool,
    cache_dir: Option<PathBuf>,
    search_paths: Vec<PathBuf>,
}

/// A grid the caller should fetch. The URL is `endpoint + "/" + filename`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Download {
    pub(crate) url: String,
    pub(crate) file_name: String,
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
    let file_name = cache_file_name_for(&url, name)?;
    Some(Download { url, file_name })
}

/// Download the grid [`decide_download`] selected and, when the cache is on,
/// write it under the user-writable directory.
pub(crate) fn fetch_missing(name: &str) -> Option<Vec<u8>> {
    let download = decide_download(name)?;
    let bytes = http_get(&download.url)?;
    let ctx = context();
    if ctx.cache_enabled {
        let _stored = store_cache(&ctx, &download.file_name, &bytes);
    }
    Some(bytes)
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

pub(crate) fn search_paths() -> Vec<PathBuf> {
    context().search_paths
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
            cache_dir: None,
            search_paths: Vec::new(),
        };
        let env_network = nonempty_env("PROJ_NETWORK");
        let env_endpoint = nonempty_env("PROJ_NETWORK_ENDPOINT");
        if let Some(text) = read_proj_ini() {
            apply_ini(
                &text,
                &mut ctx,
                &IniSkip {
                    network: env_network.is_some(),
                    endpoint: env_endpoint.is_some(),
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

    pub(crate) fn searchpath(&self) -> String {
        let separator = if cfg!(windows) { ";" } else { ":" };
        self.search_paths
            .iter()
            .filter_map(|path| path.to_str())
            .collect::<Vec<_>>()
            .join(separator)
    }

    pub(crate) fn push_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    pub(crate) fn set_cache_dir(&mut self, path: PathBuf) {
        self.cache_dir = Some(path);
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
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(".tif")
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

fn store_cache(ctx: &GridContext, file_name: &str, bytes: &[u8]) -> Option<PathBuf> {
    let dir = ctx.cache_directory();
    std::fs::create_dir_all(&dir).ok()?;
    let dest = dir.join(file_name);
    let temporary = dir.join(format!(".{file_name}.download"));
    std::fs::write(&temporary, bytes).ok()?;
    if dest.is_file() {
        let _removed = std::fs::remove_file(&dest);
    }
    if std::fs::rename(&temporary, &dest).is_err() {
        let _removed = std::fs::remove_file(&temporary);
        return None;
    }
    Some(dest)
}

fn http_get(url: &str) -> Option<Vec<u8>> {
    let mut url = url.to_owned();
    for _ in 0..4 {
        let target = parse_http_url(&url)?;
        let reply = http_exchange(&target)?;
        if reply.status == 200 {
            return Some(reply.body);
        }
        if !matches!(reply.status, 301 | 302 | 303 | 307 | 308) {
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
    if u64::try_from(reply.body.len()).ok()? > MAX_GRID_BYTES || reply.body.is_empty() {
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
    let mut dirs = Vec::new();
    for key in ["PROJ_DATA", "PROJ_LIB"] {
        if let Some(value) = std::env::var_os(key) {
            push_split(&mut dirs, &value);
        }
    }
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

fn push_split(dirs: &mut Vec<PathBuf>, value: &std::ffi::OsStr) {
    let text = value.to_string_lossy();
    for directory in text.split([';', '|']) {
        if !directory.is_empty() {
            dirs.push(PathBuf::from(directory));
        }
    }
}

struct IniSkip {
    network: bool,
    endpoint: bool,
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
            _ => {}
        }
    }
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
    use super::{decide_download, remote_url_in, with_context, GridContext};

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
        assert_eq!(on.file_name, "proj_rs_no_such_grid.tif");
    }

    #[test]
    fn alaska_short_name_is_resolved_before_the_cdn() {
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
