//! A compressed GeoTIFF grid read through the network callbacks, as PROJ reads it
//! (`GTiffGrid::valueAt`, `TIFFReadEncodedStrip` and `TIFFReadEncodedTile`, `NetworkFile::read`).
//!
//! Two grids, made by `tools/make-planar-deflate-grid.py` and both in `data/tests`: planar bands
//! (`PlanarConfiguration` 2), Deflate (`Compression` 8), the floating point predictor (`Predictor`
//! 3). `planar_deflate_predictor3.tif` has one strip per band, as the grids of `cdn.proj.org` do;
//! its strips lie beyond the first 16 KiB, so a grid read whole is told apart from a grid read by
//! strip. `planar_deflate_predictor3_tiled.tif` has 32 by 32 tiles on a 100 by 70 grid, so its
//! right and bottom tiles are partial.
//!
//! The callbacks serve the file under one URL and journal every call. The call sequences and the
//! values are PROJ 9.9.0's, measured with `libproj` 9.9.0 on the same bytes: callbacks that serve
//! the file and journal the calls, `proj=vgridshift grids=<url> multiplier=1`, `proj_trans(PJ_FWD)`.
//! `vgridshift` takes the band described `vertical_offset`, the third one. The value literals are
//! the shortest that read back as the doubles `libproj` prints with `%.17g`.

use std::sync::{Arc, Mutex};

use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder};

const STRIPS_URL: &str = "https://foo/planar.tif";
const TILES_URL: &str = "https://foo/planar_tiled.tif";
/// PROJ's degree to radian factor as the probe writes it: the inputs are the same doubles.
const DEG: f64 = 0.017453292519943295;

/// Serves the bytes of one file under `url`, and journals every call in order. The header replies
/// are PROJ's probe's: the size from `Content-Range`, and fixed validators, so the file does not
/// look changed between two calls.
struct Journal {
    url: &'static str,
    bytes: Vec<u8>,
    calls: Mutex<Vec<String>>,
}

impl Journal {
    fn new(url: &'static str, file: &str) -> Self {
        let bytes = std::fs::read(format!("{}/data/tests/{file}", env!("CARGO_MANIFEST_DIR")))
            .expect("the planar deflate fixture");
        Self {
            url,
            bytes,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn record(&self, call: String) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(call);
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("journal lock").clone()
    }

    /// Up to `size` bytes of the file from `offset`, fewer at its end.
    fn slice(&self, offset: u64, size: usize) -> Vec<u8> {
        let len = self.bytes.len();
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(len);
        let end = start.saturating_add(size).min(len);
        self.bytes[start..end].to_vec()
    }
}

impl NetworkCallbacks for Journal {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        self.record(format!("open {url} {offset} {size_to_read}"));
        if url != self.url {
            return Err(NetworkError::Open("Cannot open file".to_owned()));
        }
        Ok(OpenReply {
            handle: NetworkHandle(1),
            data: self.slice(offset, size_to_read),
        })
    }

    fn header(&self, _handle: NetworkHandle, name: &str) -> Option<String> {
        self.record(format!("header {name}"));
        match name {
            "Content-Range" => Some(format!(
                "bytes 0-{}/{}",
                self.bytes.len() - 1,
                self.bytes.len()
            )),
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
        Ok(self.slice(offset, size_to_read))
    }

    fn close(&self, _handle: NetworkHandle) {
        self.record("close".to_owned());
    }
}

/// The calls of creation, as PROJ 9.9.0 makes them: `open(url, 0, 16384)`, the three headers,
/// `close`.
fn creation_calls(url: &str) -> Vec<String> {
    vec![
        format!("open {url} 0 16384"),
        "header Content-Range".to_owned(),
        "header Last-Modified".to_owned(),
        "header ETag".to_owned(),
        "close".to_owned(),
    ]
}

/// The three header calls that follow each call to the callbacks, in PROJ's order.
fn headers() -> Vec<String> {
    vec![
        "header Content-Range".to_owned(),
        "header Last-Modified".to_owned(),
        "header ETag".to_owned(),
    ]
}

/// A builder whose callbacks serve the file and journal the calls.
fn builder_with(journal: &Arc<Journal>) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let shared: Arc<dyn NetworkCallbacks> = Arc::<Journal>::clone(journal);
    builder.set_network_callbacks(shared);
    builder
}

/// Creation reads the first chunk only. The strips lie beyond it: a grid read whole would make
/// `read_range` calls after the `open`, which is what this test refuses.
#[test]
fn creation_reads_only_the_first_chunk() {
    let journal = Arc::new(Journal::new(STRIPS_URL, "planar_deflate_predictor3.tif"));
    let builder = builder_with(&journal);

    let _proj = builder
        .proj(&format!("proj=vgridshift grids={STRIPS_URL} multiplier=1"))
        .expect("the grid is opened");

    assert_eq!(journal.calls(), creation_calls(STRIPS_URL));
}

/// A point reads the strip that holds its sample, once. The third band (`vertical_offset`) of
/// `(20, 50)` is at file offset 23018, 8687 bytes, all in chunk 1: PROJ asks
/// `open(url, 16384, 16384)` and the three headers. `(41, 37)` lies in the same chunk and reads
/// nothing. `proj_destroy` closes the handle. The values are libproj 9.9.0's for the same bytes.
#[test]
fn a_point_reads_the_strip_it_needs() {
    let journal = Arc::new(Journal::new(STRIPS_URL, "planar_deflate_predictor3.tif"));
    let builder = builder_with(&journal);

    let proj = builder
        .proj(&format!("proj=vgridshift grids={STRIPS_URL} multiplier=1"))
        .expect("the grid is opened");
    assert_eq!(journal.calls(), creation_calls(STRIPS_URL), "creation");

    let (_, _, z) = proj
        .convert_xyz(20.0 * DEG, 50.0 * DEG, 0.0)
        .expect("(20, 50) is in the grid");
    assert_eq!(z, 0.8661957234144211, "libproj 9.9.0 value at (20, 50)");
    // Creation closed its handle, so the first point opens the file again.
    let mut after_first_point = creation_calls(STRIPS_URL);
    after_first_point.push(format!("open {STRIPS_URL} 16384 16384"));
    after_first_point.extend(headers());
    assert_eq!(
        journal.calls(),
        after_first_point,
        "calls of the first point"
    );

    let (_, _, z) = proj
        .convert_xyz(41.0 * DEG, 37.0 * DEG, 0.0)
        .expect("(41, 37) is in the grid");
    assert_eq!(z, 0.6664930284023288, "libproj 9.9.0 value at (41, 37)");
    assert_eq!(
        journal.calls(),
        after_first_point,
        "the second point reads nothing"
    );

    drop(proj);
    let mut destroyed = after_first_point;
    destroyed.push("close".to_owned());
    assert_eq!(journal.calls(), destroyed, "proj_destroy closes the handle");
}

/// The tiled grid: `(20.125, 49.875)` is in the interior tile of band 2 that holds pixel (40, 40),
/// which lies in chunk 1, so PROJ asks `open(url, 16384, 16384)`. `(34.625, 42.875)` is in the
/// partial tile at the right edge (pixels 96 to 99 of 100, rows 64 to 69 of 70), which the first
/// read has in chunk 1 already: no call. The values are libproj 9.9.0's, and the edge value is the
/// test of the rows of a partial tile, which the decoder gives `tile_w` pixels apart.
#[test]
fn a_tiled_grid_reads_the_tile_of_each_point() {
    let journal = Arc::new(Journal::new(
        TILES_URL,
        "planar_deflate_predictor3_tiled.tif",
    ));
    let builder = builder_with(&journal);

    let proj = builder
        .proj(&format!("proj=vgridshift grids={TILES_URL} multiplier=1"))
        .expect("the grid is opened");
    assert_eq!(journal.calls(), creation_calls(TILES_URL), "creation");

    let (_, _, z) = proj
        .convert_xyz(20.125 * DEG, 49.875 * DEG, 0.0)
        .expect("(20.125, 49.875) is in the grid");
    assert_eq!(
        z, 0.859148681163788,
        "libproj 9.9.0 value in the interior tile"
    );
    let mut after_first_point = creation_calls(TILES_URL);
    after_first_point.push(format!("open {TILES_URL} 16384 16384"));
    after_first_point.extend(headers());
    assert_eq!(
        journal.calls(),
        after_first_point,
        "calls of the first point"
    );

    let (_, _, z) = proj
        .convert_xyz(34.625 * DEG, 42.875 * DEG, 0.0)
        .expect("(34.625, 42.875) is in the grid");
    assert_eq!(
        z, 0.880941569805145,
        "libproj 9.9.0 value in the partial tile"
    );
    assert_eq!(
        journal.calls(),
        after_first_point,
        "the second point reads nothing"
    );

    drop(proj);
    let mut destroyed = after_first_point;
    destroyed.push("close".to_owned());
    assert_eq!(journal.calls(), destroyed, "proj_destroy closes the handle");
}
