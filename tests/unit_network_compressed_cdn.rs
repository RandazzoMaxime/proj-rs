//! The GeoTIFF grid `nz_linz_nzgd2000-c120100904-grid01.tif` of `https://cdn.proj.org`, a
//! Deflate grid with planar bands and the floating point predictor, read through the network
//! callbacks as PROJ reads it (`NetworkFile::read`, `GTiffGrid::valueAt`).
//!
//! The callbacks answer each `open` and `read_range` with an HTTP `Range` request to the CDN, so
//! the bytes are the CDN's. The call sequences and the values are PROJ 9.9.0's, measured with
//! `libproj` 9.9.0 on the same file, with the grid named by `+grids=` (no URL in the string):
//! creation `open(url, 0, 16384)`, three headers, `close`; `(169, -45)` `open(url, 16384, 16384)`,
//! three headers; `(171.9, -43.55)` `read_range(344064, 131072)` on the same handle, three headers;
//! `proj_destroy` `close`. The value literals are the shortest that read back as the doubles
//! `libproj` prints with `%.17g`.
//!
//! The case is guarded by the `initial_check` probe (`tests/net_cdn`): when the CDN does not answer
//! `206`, the case returns, as PROJ's `if (!networkAccessOK) return;` does.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use proj_rs::{
    GridCacheMode, NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder,
};

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::{cdn_reachable, tunnel_to_cdn};

const NAME: &str = "nz_linz_nzgd2000-c120100904-grid01.tif";
const URL: &str = "https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif";
const CDN_HOST: &str = "cdn.proj.org";
/// PROJ's degree to radian factor as the probe writes it: the inputs are the same doubles.
const DEG: f64 = 0.017453292519943295;

/// The bytes `first..first + size` of the CDN file, with the size of the file from
/// `Content-Range`. `None` when the CDN does not answer `206` with the bytes asked for.
fn cdn_range(first: u64, size: usize) -> Option<(Vec<u8>, u64)> {
    let stream = tunnel_to_cdn()?;
    let connector = native_tls::TlsConnector::new().ok()?;
    let mut tls = connector.connect(CDN_HOST, stream).ok()?;
    let last = first
        .checked_add(u64::try_from(size).ok()?)?
        .checked_sub(1)?;
    let request = format!(
        "GET /{NAME} HTTP/1.1\r\nHost: {CDN_HOST}\r\nRange: bytes={first}-{last}\r\n\
         User-Agent: proj-rs-test\r\nConnection: close\r\n\r\n"
    );
    tls.write_all(request.as_bytes()).ok()?;
    let mut reply = Vec::new();
    // The server closes the connection at the end of the reply, which TLS may report as an EOF
    // error after the bytes: the bytes read are the reply.
    let _read = tls.read_to_end(&mut reply);
    let split = reply.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let head = String::from_utf8_lossy(&reply[..split]).to_ascii_lowercase();
    if !head.starts_with("http/1.1 206") {
        return None;
    }
    let total = head
        .lines()
        .find_map(|line| line.strip_prefix("content-range: bytes "))?
        .split_once('/')?
        .1
        .trim()
        .parse::<u64>()
        .ok()?;
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length: "))?
        .trim()
        .parse::<usize>()
        .ok()?;
    let body = reply[split..].to_vec();
    (body.len() == length).then_some((body, total))
}

/// Answers each `open` and `read_range` with the CDN's bytes, and journals every call in order.
/// The header replies are PROJ's probe's: the size of the last reply from `Content-Range`, and fixed
/// validators, so the file does not look changed between two calls.
#[derive(Default)]
struct CdnJournal {
    calls: Mutex<Vec<String>>,
    total: Mutex<u64>,
}

impl CdnJournal {
    fn record(&self, call: String) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(call);
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("journal lock").clone()
    }

    fn reply(&self, first: u64, size: usize) -> Result<Vec<u8>, NetworkError> {
        let (bytes, total) = cdn_range(first, size).ok_or_else(|| {
            NetworkError::ReadRange("the CDN did not answer the range".to_owned())
        })?;
        if let Ok(mut last) = self.total.lock() {
            *last = total;
        }
        Ok(bytes)
    }
}

impl NetworkCallbacks for CdnJournal {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        self.record(format!("open {url} {offset} {size_to_read}"));
        if url != URL {
            return Err(NetworkError::Open("Cannot open file".to_owned()));
        }
        let data = self
            .reply(offset, size_to_read)
            .map_err(|_| NetworkError::Open("the CDN did not answer the range".to_owned()))?;
        Ok(OpenReply {
            handle: NetworkHandle(1),
            data,
        })
    }

    fn header(&self, _handle: NetworkHandle, name: &str) -> Option<String> {
        self.record(format!("header {name}"));
        match name {
            "Content-Range" => {
                let total = *self.total.lock().ok()?;
                Some(format!("bytes 0-{}/{total}", total.saturating_sub(1)))
            }
            "Last-Modified" => Some("some_date".to_owned()),
            "ETag" => Some("some_etag".to_owned()),
            _ => None,
        }
    }

    fn read_range(
        &self,
        _handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        self.record(format!("read_range {offset} {size_to_read}"));
        self.reply(offset, size_to_read)
    }

    fn close(&self, _handle: NetworkHandle) {
        self.record("close".to_owned());
    }
}

/// The calls of creation, as PROJ 9.9.0 makes them.
fn creation_calls() -> Vec<String> {
    vec![
        format!("open {URL} 0 16384"),
        "header Content-Range".to_owned(),
        "header Last-Modified".to_owned(),
        "header ETag".to_owned(),
        "close".to_owned(),
    ]
}

/// The three header calls after each call to the callbacks.
fn headers() -> Vec<String> {
    vec![
        "header Content-Range".to_owned(),
        "header Last-Modified".to_owned(),
        "header ETag".to_owned(),
    ]
}

/// A temporary folder for this process, never inside the repository.
fn user_dir() -> PathBuf {
    env::temp_dir().join(format!("proj-rs-cdn-compressed-{}", std::process::id()))
}

#[test]
fn creation_and_points_read_the_cdn_grid_as_proj_does() {
    if !cdn_reachable() {
        eprintln!("creation_and_points_read_the_cdn_grid_as_proj_does: the CDN does not answer, the case returns");
        return;
    }
    let dir = user_dir();
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let journal = Arc::new(CdnJournal::default());
    let mut builder = ProjBuilder::new();
    builder
        .set_grid_cache_enabled(GridCacheMode::Disabled)
        .expect("grid cache off");
    builder
        .set_user_writable_directory(&dir)
        .expect("user-writable directory");
    builder.enable_network(true).expect("network on");
    let shared: Arc<dyn NetworkCallbacks> = Arc::<CdnJournal>::clone(&journal);
    builder.set_network_callbacks(shared);

    let proj = builder
        .proj(&format!("proj=vgridshift grids={NAME} multiplier=1"))
        .expect("the grid is opened at creation");
    assert_eq!(journal.calls(), creation_calls(), "creation");

    let (_, _, z) = proj
        .convert_xyz(169.0 * DEG, -45.0 * DEG, 0.0)
        .expect("(169, -45) is in the grid");
    assert_eq!(
        z, -0.0006999999750405533,
        "libproj 9.9.0 value at (169, -45)"
    );
    // Creation closed its handle, so the first point opens the file again.
    let mut after_first_point = creation_calls();
    after_first_point.push(format!("open {URL} 16384 16384"));
    after_first_point.extend(headers());
    assert_eq!(
        journal.calls(),
        after_first_point,
        "calls of the first point"
    );

    let (_, _, z) = proj
        .convert_xyz(171.9 * DEG, -43.55 * DEG, 0.0)
        .expect("(171.9, -43.55) is in the grid");
    assert_eq!(
        z, 0.3900079886118898,
        "libproj 9.9.0 value at (171.9, -43.55)"
    );
    let mut after_second_point = after_first_point;
    after_second_point.push("read_range 344064 131072".to_owned());
    after_second_point.extend(headers());
    assert_eq!(
        journal.calls(),
        after_second_point,
        "calls of the second point"
    );

    drop(proj);
    let mut destroyed = after_second_point;
    destroyed.push("close".to_owned());
    assert_eq!(journal.calls(), destroyed, "proj_destroy closes the handle");

    let _gone = fs::remove_dir_all(&dir);
}
