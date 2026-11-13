//! PROJ `test/unit/test_network.cpp`, the grid cache cases: `cache_basic`,
//! `proj_grid_cache_clear`, `cache_saturation`, `cache_ttl` and `cache_lock`.
//!
//! The C++ cases read `https://cdn.proj.org/fr_ign_ntf_r93.tif` through the cache. Here the grid
//! is served by the local server (`tests/net_support`), and the cache is a file of the test's
//! own directory. The expected values are PROJ 9.9.0 `cct -d 10` (the same hgridshift pipeline)
//! on the same grid bytes, not the CDN values of the C++ test.
//!
//! The database layout is read with `rusqlite` directly, as the C++ cases read it with
//! `sqlite3`. The two cases that change the environment (`PROJ_GRID_CACHE_MAX_SIZE_BYTES`,
//! `PROJ_LOCK_MAX_ITERS`) hold `ENV_LOCK`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use proj_rs::{GridCacheMode, GridCacheTtl, ProjBuilder};
use rusqlite::Connection;

#[path = "net_support/grids.rs"]
#[expect(dead_code, reason = "this file serves the cache grid only")]
mod grids;
#[path = "net_support/mod.rs"]
#[expect(dead_code, reason = "this file serves with spawn_with_headers only")]
mod net_support;

use net_support::{LocalServer, Route};

/// Serialises the two cases that set process environment variables.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// The hgridshift pipeline of the C++ cases, with the grid at `url`.
fn pipeline(url: &str) -> String {
    format!(
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids={url} \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg"
    )
}

/// The grid of the cache tests on the local server, with the validators a CDN sends.
fn server() -> LocalServer {
    LocalServer::spawn_with_headers(
        vec![Route {
            path: "/fr_ign_ntf_r93.tif",
            status: 200,
            body: grids::cache_grid(),
        }],
        vec![
            ("Last-Modified", "Wed, 21 Oct 2015 07:28:00 GMT"),
            ("ETag", "\"proj-rs-test\""),
        ],
    )
}

/// A builder with the network on and the cache database at `db`.
fn builder(db: &Path) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.enable_network(true).expect("network on");
    builder
        .set_grid_cache_enabled(GridCacheMode::Enabled)
        .expect("cache on");
    builder.set_grid_cache_path(db).expect("cache path");
    builder
}

/// A directory for one test, removed when the test ends.
struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("proj-rs-cache-{name}-{}", std::process::id()));
        let _removed = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        Self(dir)
    }

    fn db(&self) -> PathBuf {
        self.0.join("cache.db")
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// The trans of PROJ's test point (`lon 2, lat 49`), in degrees.
fn assert_test_point(proj: &proj_rs::Proj) {
    let (lon, lat, _h) = proj.convert_xyz(2.0, 49.0, 0.0).expect("point");
    assert!((lon - 1.999_777_777_8).abs() < 1e-10, "lon {lon}");
    assert!((lat - 49.000_270_833_3).abs() < 1e-10, "lat {lat}");
}

fn open_read_only(db: &Path) -> Connection {
    Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("database opens")
}

/// `networking.cache_basic`: a first build stores the grid, its first chunk is at the head
/// of the LRU list, and a second build makes no network request.
#[test]
fn cache_basic() {
    let dir = TestDir::new("basic");
    let server = server();
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    let proj = builder(&dir.db())
        .proj(&pipeline(&url))
        .expect("first build");
    assert_test_point(&proj);
    drop(proj);
    assert!(dir.db().is_file(), "the database exists after a download");

    let (head_url, head_offset): (String, i64) = open_read_only(&dir.db())
        .query_row(
            "SELECT url, offset FROM chunks WHERE id = (\
             SELECT chunk_id FROM linked_chunks WHERE id = (\
             SELECT head FROM linked_chunks_head_tail))",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("head chunk");
    assert_eq!(head_url, url);
    assert_eq!(head_offset, 0);

    let before = server.requests().len();
    let again = builder(&dir.db())
        .proj(&pipeline(&url))
        .expect("second build");
    assert_test_point(&again);
    assert_eq!(server.requests().len(), before, "{:?}", server.requests());
}

/// `networking.proj_grid_cache_clear`: the database is created by a build, and removed by the
/// clear.
#[test]
fn proj_grid_cache_clear() {
    let dir = TestDir::new("clear");
    let server = server();
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    let configured = builder(&dir.db());
    assert_eq!(configured.grid_cache_path(), dir.db());
    configured.clear_grid_cache().expect("clear an empty cache");

    let proj = configured.proj(&pipeline(&url)).expect("build");
    drop(proj);
    assert!(dir.db().is_file(), "the database exists");

    builder(&dir.db()).clear_grid_cache().expect("clear");
    assert!(!dir.db().exists(), "the database is removed");
}

/// `networking.cache_saturation`: with a limit of two chunks, the database never keeps more
/// than two, and the transform is still right.
#[test]
fn cache_saturation() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let dir = TestDir::new("saturation");
    let server = server();
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    let mut limited = builder(&dir.db());
    limited.clear_grid_cache().expect("clear");
    std::env::set_var("PROJ_GRID_CACHE_MAX_SIZE_BYTES", "32768");
    limited
        .set_grid_cache_max_size_mb(Some(0))
        .expect("limit from the environment");
    std::env::remove_var("PROJ_GRID_CACHE_MAX_SIZE_BYTES");

    let proj = limited.proj(&pipeline(&url)).expect("build");
    assert_test_point(&proj);
    drop(proj);

    let connection = open_read_only(&dir.db());
    for table in ["chunk_data", "chunks", "linked_chunks"] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 2, "{table}");
    }
}

/// `networking.cache_ttl`: an entry whose check time is the epoch is not trusted when the
/// time-to-live is positive, and is downloaded again. With no expiry the junk stored in the
/// chunks is used, and the build fails.
#[test]
fn cache_ttl() {
    let dir = TestDir::new("ttl");
    let server = server();
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    builder(&dir.db())
        .proj(&pipeline(&url))
        .expect("first build");

    let connection = Connection::open(dir.db()).expect("database opens");
    connection
        .execute_batch(
            "UPDATE properties SET lastChecked = 0, lastModified = 'foo', etag = 'bar'; \
             UPDATE chunk_data SET data = zeroblob(16384);",
        )
        .expect("expire and corrupt");
    drop(connection);

    let mut never = builder(&dir.db());
    never
        .set_grid_cache_ttl(GridCacheTtl::Never)
        .expect("never expires");
    assert!(
        never.proj(&pipeline(&url)).is_err(),
        "junk chunks do not build"
    );

    let mut expiring = builder(&dir.db());
    expiring
        .set_grid_cache_ttl(GridCacheTtl::Seconds(86_400))
        .expect("one day");
    let proj = expiring.proj(&pipeline(&url)).expect("refreshed build");
    assert_test_point(&proj);
    drop(proj);

    let (last_checked, last_modified, etag): (i64, String, String) = open_read_only(&dir.db())
        .query_row(
            "SELECT lastChecked, lastModified, etag FROM properties",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("properties");
    assert_ne!(last_checked, 0);
    assert_eq!(last_modified, "Wed, 21 Oct 2015 07:28:00 GMT");
    assert_eq!(etag, "\"proj-rs-test\"");
}

/// `networking.cache_lock`: another connection holds the exclusive lock. The build still
/// succeeds, without the cache, after the retries of `PROJ_LOCK_MAX_ITERS = 25`.
///
/// The retries sleep 5 ms (ten times), 10 ms (ten times) and 100 ms (five times): 650 ms.
#[test]
fn cache_lock() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let dir = TestDir::new("lock");
    let server = server();
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    builder(&dir.db())
        .proj(&pipeline(&url))
        .expect("first build");

    let holder = Connection::open(dir.db()).expect("database opens");
    holder
        .execute_batch("BEGIN EXCLUSIVE")
        .expect("exclusive lock");

    std::env::set_var("PROJ_LOCK_MAX_ITERS", "25");
    let start = Instant::now();
    let proj = builder(&dir.db()).proj(&pipeline(&url));
    let elapsed = start.elapsed();
    std::env::remove_var("PROJ_LOCK_MAX_ITERS");

    assert_test_point(&proj.expect("build without the locked cache"));
    assert!(elapsed >= Duration::from_millis(650), "{elapsed:?}");
    holder.execute_batch("COMMIT").expect("release");
}
