//! PROJ `test/unit/test_network.cpp`, against a local server that answers `Range` requests, as the
//! CDN does. The built-in client reads a grid by ranges, as PROJ's curl client.
//!
//! Nothing in this file calls `https://cdn.proj.org`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use proj_rs::ProjBuilder;

mod common;
#[path = "net_support/mod.rs"]
#[expect(dead_code, reason = "this file serves with spawn only")]
mod net_support;

use net_support::{LocalServer, Route};

const GRID: &str = "/proj_rs_missing_hgrid.tif";
const NOCACHE: &str = "/proj_rs_missing_hgrid_nocache.tif";
const BAD: &str = "/proj_rs_bad_grid.tif";

#[test]
fn missing_grid_is_fetched_only_when_the_network_is_on() {
    common::use_fixture_search_path();
    let fixture = std::fs::read(vendored_hgrid()).expect("fixture");
    let route = |path, body: &[u8]| Route {
        path,
        status: 200,
        body: body.to_vec(),
    };
    let server = LocalServer::spawn(vec![
        route(GRID, &fixture),
        route(NOCACHE, &fixture),
        route(BAD, b"not-a-grid"),
    ]);
    let cache = TempDir::new("cache");
    let definition = "+proj=gridshift +grids=proj_rs_missing_hgrid.tif";

    let mut off = ProjBuilder::new();
    off.set_url_endpoint(server.url()).expect("endpoint");
    off.set_user_writable_directory(&cache.0).expect("cache");
    off.grid_cache_enable(true).expect("cache on");
    assert!(!off.network_enabled());
    assert!(
        off.proj(definition).is_err(),
        "network off keeps the missing-file error"
    );
    assert!(server.requests().is_empty());

    let on = network_builder(server.url(), &cache.0);
    assert!(on.network_enabled());
    let fetched = on.proj(definition).expect("downloaded grid");
    assert!(fetched.network_enabled());
    assert_eq!(fetched.get_url_endpoint().expect("endpoint"), server.url());
    // The grid is smaller than one 16 KiB chunk: one ranged request reads it all.
    assert_eq!(server.requests(), vec![GRID.to_owned()]);
    // The grid is in the chunk database `cache.db`, not a file of the directory, as in PROJ 9.9.0.
    assert!(!cache.0.join("proj_rs_missing_hgrid.tif").is_file());
    let database = rusqlite::Connection::open(cache.0.join("cache.db")).expect("chunk database");
    let url = format!("{}{GRID}", server.url());
    let size: i64 = database
        .query_row(
            "SELECT fileSize FROM properties WHERE url = ?1",
            [url],
            |row| row.get(0),
        )
        .expect("cached properties");
    assert_eq!(usize::try_from(size).ok(), Some(fixture.len()));

    let local = proj_rs::Proj::new("+proj=gridshift +grids=tests/test_hgrid_with_subgrid.tif")
        .expect("vendored grid");
    let lon = -115.541_666_7_f64.to_radians();
    let lat = 51.166_666_7_f64.to_radians();
    let from_cache = fetched.project((lon, lat), false).expect("fetched point");
    let from_disk = local.project((lon, lat), false).expect("local point");
    assert!((from_cache.0 - from_disk.0).abs() < 1e-12);
    assert!((from_cache.1 - from_disk.1).abs() < 1e-12);

    network_builder(server.url(), &cache.0)
        .proj(definition)
        .expect("cache hit");
    assert_eq!(
        server.requests().len(),
        1,
        "a cached grid is not fetched again"
    );

    assert!(
        network_builder(server.url(), &cache.0)
            .proj("+proj=gridshift +grids=proj_rs_bad_grid.tif")
            .is_err(),
        "a download that is not a grid is not a coordinate"
    );

    let mut uncached = network_builder(server.url(), &cache.0);
    uncached.grid_cache_enable(false).expect("cache off");
    uncached
        .proj("+proj=gridshift +grids=proj_rs_missing_hgrid_nocache.tif")
        .expect("fetched without a cache file");
    assert!(server.requests().iter().any(|path| path == NOCACHE));
}

/// A server that ignores `Range` sends no `Content-Range`: PROJ's open fails on that
/// (`NetworkFile::get_props_from_headers`), and so does this one.
#[test]
fn server_without_ranges_is_not_read() {
    let fixture = std::fs::read(vendored_hgrid()).expect("fixture");
    let server = Scripted::spawn(vec![Reply::Whole(fixture)]);
    let cache = TempDir::new("whole");
    assert!(network_builder(&server.url, &cache.0)
        .proj("+proj=gridshift +grids=proj_rs_whole.tif")
        .is_err());
    assert_eq!(server.finish(), 1);
}

/// A `503` is retried after a wait, as PROJ's curl client does (`GetNewRetryDelay`: 500 ms
/// doubled, plus up to a quarter at random); a `404` is not.
#[test]
fn transient_error_is_retried() {
    let fixture = std::fs::read(vendored_hgrid()).expect("fixture");
    let server = Scripted::spawn(vec![Reply::Status(503), Reply::Ranged(fixture)]);
    let cache = TempDir::new("retry");
    let start = Instant::now();
    network_builder(&server.url, &cache.0)
        .proj("+proj=gridshift +grids=proj_rs_retry.tif")
        .expect("read after the retry");
    let waited = start.elapsed();
    assert_eq!(server.finish(), 2);
    assert!(waited >= Duration::from_millis(1000), "{waited:?}");

    let server = Scripted::spawn(vec![Reply::Status(404)]);
    let cache = TempDir::new("gone");
    assert!(network_builder(&server.url, &cache.0)
        .proj("+proj=gridshift +grids=proj_rs_gone.tif")
        .is_err());
    assert_eq!(server.finish(), 1);
}

/// A connection reset is retried after the same wait as a `503`, as PROJ's curl client does for
/// « Connection reset by peer » (`GetNewRetryDelay`).
#[test]
fn reset_connection_is_retried() {
    let fixture = std::fs::read(vendored_hgrid()).expect("fixture");
    let server = Scripted::spawn(vec![Reply::Reset, Reply::Ranged(fixture)]);
    let cache = TempDir::new("reset");
    let start = Instant::now();
    network_builder(&server.url, &cache.0)
        .proj("+proj=gridshift +grids=proj_rs_reset.tif")
        .expect("read after the reset");
    assert_eq!(server.finish(), 2);
    assert!(start.elapsed() >= Duration::from_millis(1000));
}

#[test]
fn search_path_opens_a_grid_without_the_network() {
    let source = vendored_hgrid();
    let dir = TempDir::new("search");
    let name = "proj_rs_search_path_grid.tif";
    std::fs::copy(&source, dir.0.join(name)).expect("copy");
    let mut builder = ProjBuilder::new();
    assert!(!builder.network_enabled());
    builder.set_search_paths(&dir.0).expect("path");
    let definition = format!("+proj=gridshift +grids={name}");
    builder.proj(&definition).expect("search path");
}

fn network_builder(url: &str, cache: &Path) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    assert_eq!(builder.enable_network(true).expect("enable"), 1);
    builder.set_url_endpoint(url).expect("endpoint");
    builder.set_user_writable_directory(cache).expect("cache");
    builder.grid_cache_enable(true).expect("cache on");
    builder
}

fn vendored_hgrid() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("gie")
        .join("tests")
        .join("test_hgrid_with_subgrid.tif")
}

/// One reply of a [`Scripted`] server.
enum Reply {
    /// The status, with no body.
    Status(u16),
    /// `200` and the whole body, whatever range was asked.
    Whole(Vec<u8>),
    /// `206`, the asked range of the body and its `Content-Range`.
    Ranged(Vec<u8>),
    /// No reply: the connection is closed with the request unread, so the client sees a reset.
    Reset,
}

/// A server that sends its replies in order, one per connection, then stops.
struct Scripted {
    url: String,
    handle: JoinHandle<usize>,
}

impl Scripted {
    fn spawn(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("non-blocking");
        let url = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("addr").port()
        );
        let handle = thread::spawn(move || {
            let mut served = 0;
            let deadline = Instant::now() + Duration::from_secs(20);
            let mut replies = replies.into_iter();
            while Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let Some(reply) = replies.next() else {
                    break;
                };
                let _ = stream.set_nonblocking(false);
                if matches!(reply, Reply::Reset) {
                    // Closing a socket with received bytes unread sends a reset, not a FIN.
                    thread::sleep(Duration::from_millis(200));
                    served += 1;
                    drop(stream);
                    continue;
                }
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let head = String::from_utf8_lossy(&buffer[..read]).into_owned();
                served += 1;
                let _ = stream.write_all(&answer(&reply, &head));
                if replies.len() == 0 {
                    break;
                }
            }
            served
        });
        Self { url, handle }
    }

    /// The number of requests served.
    fn finish(self) -> usize {
        self.handle.join().expect("server thread")
    }
}

fn answer(reply: &Reply, head: &str) -> Vec<u8> {
    let (status, extra, body) = match reply {
        Reply::Status(status) => (*status, String::new(), Vec::new()),
        Reply::Reset => return Vec::new(),
        Reply::Whole(body) => (200, String::new(), body.clone()),
        Reply::Ranged(body) => {
            let (first, last) = range(head).expect("a range request");
            let last = last.min(body.len() - 1);
            let header = format!("Content-Range: bytes {first}-{last}/{}\r\n", body.len());
            (206, header, body[first..=last].to_vec())
        }
    };
    let mut out = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    )
    .into_bytes();
    out.extend(body);
    out
}

fn range(head: &str) -> Option<(usize, usize)> {
    let value = head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range").then(|| value.trim())
    })?;
    let (first, last) = value.strip_prefix("bytes=")?.split_once('-')?;
    Some((first.parse().ok()?, last.parse().ok()?))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "proj-rs-net-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        std::fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}
