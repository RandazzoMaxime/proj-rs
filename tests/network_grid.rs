//! PROJ `test/unit/test_network.cpp`, against a local server.
//!
//! Nothing in this file calls `https://cdn.proj.org`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use proj_rs::ProjBuilder;

#[test]
fn missing_grid_is_fetched_only_when_the_network_is_on() {
    let fixture = std::fs::read(vendored_hgrid()).expect("fixture");
    let server = Server::spawn(fixture.clone());
    assert!(!server.url.contains("cdn.proj.org"), "{}", server.url);
    let cache = TempDir::new("cache");
    let definition = "+proj=gridshift +grids=proj_rs_missing_hgrid.tif";

    let mut off = ProjBuilder::new();
    off.set_url_endpoint(&server.url).expect("endpoint");
    off.set_user_writable_directory(&cache.0).expect("cache");
    off.grid_cache_enable(true).expect("cache on");
    assert!(!off.network_enabled());
    assert!(
        off.proj(definition).is_err(),
        "network off keeps the missing-file error"
    );
    assert_eq!(server.hits(), 0);
    assert!(!cache.0.join("proj_rs_missing_hgrid.tif").is_file());

    let mut on = ProjBuilder::new();
    on.enable_network(true).expect("enable");
    assert_eq!(on.enable_network(true).expect("still on"), 1);
    on.set_url_endpoint(&server.url).expect("endpoint");
    on.set_user_writable_directory(&cache.0).expect("cache");
    on.grid_cache_enable(true).expect("cache on");
    assert!(on.network_enabled());
    let fetched = on.proj(definition).expect("downloaded grid");
    assert!(fetched.network_enabled());
    assert_eq!(fetched.get_url_endpoint().expect("endpoint"), server.url);
    assert_eq!(server.hits(), 1);
    assert_eq!(
        server.paths(),
        vec!["/proj_rs_missing_hgrid.tif".to_owned()]
    );
    let cached = std::fs::read(cache.0.join("proj_rs_missing_hgrid.tif")).expect("cached bytes");
    assert_eq!(cached, fixture);

    let local = proj_rs::Proj::new("+proj=gridshift +grids=tests/test_hgrid_with_subgrid.tif")
        .expect("vendored grid");
    let lon = -115.541_666_7_f64.to_radians();
    let lat = 51.166_666_7_f64.to_radians();
    let from_cache = fetched.project((lon, lat), false).expect("fetched point");
    let from_disk = local.project((lon, lat), false).expect("local point");
    assert!((from_cache.0 - from_disk.0).abs() < 1e-12);
    assert!((from_cache.1 - from_disk.1).abs() < 1e-12);

    let mut again = ProjBuilder::new();
    again.enable_network(true).expect("enable");
    again.set_url_endpoint(&server.url).expect("endpoint");
    again.set_user_writable_directory(&cache.0).expect("cache");
    again.grid_cache_enable(true).expect("cache on");
    again.proj(definition).expect("cache hit");
    assert_eq!(server.hits(), 1, "a cached file is not fetched again");

    let mut bad = ProjBuilder::new();
    bad.enable_network(true).expect("enable");
    bad.set_url_endpoint(&server.url).expect("endpoint");
    bad.set_user_writable_directory(&cache.0).expect("cache");
    assert!(
        bad.proj("+proj=gridshift +grids=proj_rs_bad_grid.tif")
            .is_err(),
        "a download that is not a grid is not a coordinate"
    );

    let mut uncached = ProjBuilder::new();
    uncached.enable_network(true).expect("enable");
    uncached.set_url_endpoint(&server.url).expect("endpoint");
    uncached
        .set_user_writable_directory(&cache.0)
        .expect("cache");
    uncached.grid_cache_enable(false).expect("cache off");
    uncached
        .proj("+proj=gridshift +grids=proj_rs_missing_hgrid_nocache.tif")
        .expect("fetched without a cache file");
    assert!(!cache.0.join("proj_rs_missing_hgrid_nocache.tif").is_file());
    assert!(server.hits() > 1);
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

fn vendored_hgrid() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("gie")
        .join("tests")
        .join("test_hgrid_with_subgrid.tif")
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

struct Server {
    url: String,
    hits: Arc<AtomicUsize>,
    paths: Arc<Mutex<Vec<String>>>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn spawn(body: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let hits = Arc::new(AtomicUsize::new(0));
        let paths = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let hits_thread = Arc::clone(&hits);
        let paths_thread = Arc::clone(&paths);
        let shutdown_thread = Arc::clone(&shutdown);
        let handle = thread::spawn(move || {
            serve(listener, body, hits_thread, paths_thread, shutdown_thread);
        });
        Self {
            url: format!("http://127.0.0.1:{port}"),
            hits,
            paths,
            shutdown,
            handle: Some(handle),
        }
    }

    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }

    fn paths(&self) -> Vec<String> {
        self.paths
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Ok(mut stream) = TcpStream::connect(self.url.trim_start_matches("http://")) {
            let _ = stream.write_all(b"GET /shutdown HTTP/1.0\r\n\r\n");
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

fn serve(
    listener: TcpListener,
    body: Vec<u8>,
    hits: Arc<AtomicUsize>,
    paths: Arc<Mutex<Vec<String>>>,
    shutdown: Arc<AtomicBool>,
) {
    listener.set_nonblocking(false).expect("blocking listener");
    while !shutdown.load(Ordering::SeqCst) {
        let Ok(mut stream) = listener.accept().map(|(stream, _)| stream) else {
            break;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut buffer = [0_u8; 2048];
        let read = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
        if !request.starts_with("GET ") {
            continue;
        }
        hits.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = paths.lock() {
            guard.push(path.clone());
        }
        if path == "/proj_rs_bad_grid.tif" {
            let payload = b"not-a-grid";
            let _ = write_response(&mut stream, 200, payload);
        } else if path.ends_with(".tif") {
            let _ = write_response(&mut stream, 200, &body);
        } else {
            let _ = write_response(&mut stream, 404, b"");
        }
    }
}

fn write_response(stream: &mut TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
