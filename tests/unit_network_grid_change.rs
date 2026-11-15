//! A grid whose file changes while it is open, for the steps `hgridshift`, `gridshift` and
//! `xyzgridshift`. PROJ 9.9.0 detects the change in `NetworkFile::read` (`m_hasChanged`), and the
//! step loads the grid again (`reopen`, then the point is run again): `pj_hgrid_apply_internal`
//! and `pj_hgrid_value` for `hgridshift`, `grid_apply_internal` for `gridshift`
//! (`transformations/gridshift.cpp`). `vgridshift` is `simul_file_change_while_opened` in
//! `unit_network_core.rs`.
//!
//! The callbacks serve two versions of one file: version A until the `change_at`-th open, then
//! version B, with the `Last-Modified` of a changed file. Each call is journaled in the form of the
//! C probe that measured the sequences (`libproj` 9.9.0, the same two files, the same
//! headers), so the expected journals are the probe's output, not the port's.
//!
//! `xyzgridshift` is the exception: PROJ 9.9.0 never reloads it. `pj_bilinear_interpolation_three_samples`
//! asks for a retry, and `get_grid_values` runs the same grid again, which reports the change
//! again: the probe exits with SIGSEGV (exit 139) after the `read_range` that found the change.
//! The port reloads it like the other steps, and the test checks the values of the changed grid.

use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

#[path = "net_support/grids.rs"]
#[allow(dead_code)]
mod grids;

use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder};

const HGRID_URL: &str = "https://foo/h.tif";
const GRIDSHIFT_URL: &str = "https://foo/g.tif";
const XYZ_URL: &str = "https://foo/x.tif";
const GRIDSHIFT_TYPE: &str =
    "<GDALMetadata>\n  <Item name=\"TYPE\">HORIZONTAL_OFFSET</Item>\n</GDALMetadata>";

/// The file of one step: 64 x 64 pixels of 0.5 degree, north-west corner (0, 32), so that the
/// samples of one row do not fit the first 16 KiB chunk and a point in the south reads past it.
fn grid_file(samples_per_pixel: u16, values: &[f32], metadata: Option<&str>) -> Vec<u8> {
    let samples: Vec<f32> = values.repeat(64 * 64);
    match metadata {
        Some(text) => {
            grids::geotiff_gdal(64, 64, (0.0, 32.0), 0.5, samples_per_pixel, &samples, text)
        }
        None => grids::geotiff(64, 64, 0.0, 32.0, 0.5, samples_per_pixel, &samples),
    }
}

/// The bytes `data[offset..offset + size]`, fewer at the end of the file.
fn slice(data: &[u8], offset: u64, size: usize) -> Vec<u8> {
    let start = usize::try_from(offset).map_or(data.len(), |offset| offset.min(data.len()));
    let end = start.saturating_add(size).min(data.len());
    data[start..end].to_vec()
}

#[derive(Default)]
struct JournalState {
    /// Opens so far, which `change_at` counts.
    opens: usize,
    /// Set by the open that reaches `change_at`: the handles opened after it see version B.
    changed: bool,
    /// Whether each handle, numbered from 1, was opened on the changed file.
    handles: Vec<bool>,
    lines: Vec<String>,
}

/// Serves `before` until the `change_at`-th open, `after` from it on, and journals each call.
/// Headers are asked by the port only while the handle has not seen the change, as PROJ does.
struct Journal {
    before: Vec<u8>,
    after: Vec<u8>,
    change_at: usize,
    state: Mutex<JournalState>,
}

impl Journal {
    fn new(before: Vec<u8>, after: Vec<u8>, change_at: usize) -> Self {
        Self {
            before,
            after,
            change_at,
            state: Mutex::new(JournalState::default()),
        }
    }

    fn version(&self, changed: bool) -> &[u8] {
        if changed {
            &self.after
        } else {
            &self.before
        }
    }

    fn lines(&self) -> Vec<String> {
        self.state.lock().expect("journal").lines.clone()
    }

    fn journal(&self, line: String) {
        self.state.lock().expect("journal").lines.push(line);
    }
}

impl NetworkCallbacks for Journal {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        let mut state = self.state.lock().expect("journal");
        state.opens += 1;
        if self.change_at > 0 && state.opens >= self.change_at {
            state.changed = true;
        }
        let changed = state.changed;
        state.handles.push(changed);
        let id = state.handles.len();
        let data = slice(self.version(changed), offset, size_to_read);
        state.lines.push(format!(
            "open({url}, {offset}, {size_to_read}) -> #{id} size_read={}",
            data.len()
        ));
        Ok(OpenReply {
            handle: NetworkHandle(u64::try_from(id).unwrap_or(u64::MAX)),
            data,
        })
    }

    fn header(&self, handle: NetworkHandle, name: &str) -> Option<String> {
        let mut state = self.state.lock().expect("journal");
        let id = usize::try_from(handle.0).unwrap_or(0);
        let changed = state
            .handles
            .get(id.wrapping_sub(1))
            .copied()
            .unwrap_or(false);
        let total = self.version(changed).len();
        let value = match name {
            "Content-Range" => Some(format!("bytes 0-16383/{total}")),
            "Last-Modified" => Some(
                if changed {
                    "some_date CHANGED!!!!"
                } else {
                    "some_date"
                }
                .to_owned(),
            ),
            "ETag" => Some("some_etag".to_owned()),
            _ => None,
        };
        state.lines.push(format!(
            "header #{id} {name} = {}",
            value.as_deref().unwrap_or("(null)")
        ));
        value
    }

    fn read_range(
        &self,
        handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        let mut state = self.state.lock().expect("journal");
        let id = usize::try_from(handle.0).unwrap_or(0);
        let changed = state
            .handles
            .get(id.wrapping_sub(1))
            .copied()
            .unwrap_or(false);
        let data = slice(self.version(changed), offset, size_to_read);
        state.lines.push(format!(
            "read_range #{id} ({offset}, {size_to_read}) -> {}",
            data.len()
        ));
        Ok(data)
    }

    fn close(&self, handle: NetworkHandle) {
        self.journal(format!("close #{}", usize::try_from(handle.0).unwrap_or(0)));
    }
}

/// A builder that reads its grids through `callbacks`, the network on, the cache off.
fn builder_with(journal: &Arc<Journal>) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let callbacks: Arc<dyn NetworkCallbacks> = Arc::<Journal>::clone(journal);
    builder.set_network_callbacks(callbacks);
    builder
}

fn assert_point(actual: (f64, f64), expected: (f64, f64)) {
    assert!(
        (actual.0 - expected.0).abs() < 1e-12 && (actual.1 - expected.1).abs() < 1e-12,
        "{actual:?} against PROJ {expected:?}"
    );
}

/// Radians of a point in degrees, the units of a raw operation.
fn radians(lon: f64, lat: f64) -> (f64, f64) {
    (lon / 180.0 * PI, lat / 180.0 * PI)
}

/// The creation of `hgridshift` on version A, then one forward transform of (3, 2) degrees.
/// The first read past the head finds the file changed; the grid is loaded again, and the point
/// is run on version B. Values: PROJ 9.9.0's, `x` and `y` in radians.
#[test]
fn hgridshift_forward_reloads_the_grid_when_its_file_changed() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], None),
        grid_file(2, &[3.0, 6.0], None),
        2,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=hgridshift +grids={HGRID_URL}"))
        .expect("the grid is read at creation");
    let (x, y, _) = proj
        .convert_xyz(radians(3.0, 2.0).0, radians(3.0, 2.0).1, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_388_966_379_986_059, 0.034_921_129_449_964_679),
    );
    drop(proj);
    assert_in_order(&journal.lines(), HGRID_RELOAD);
}

/// The inverse of `hgridshift`: the same grid load and reload, the point iterated on version B.
#[test]
fn hgridshift_inverse_reloads_the_grid_when_its_file_changed() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], None),
        grid_file(2, &[3.0, 6.0], None),
        2,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=hgridshift +grids={HGRID_URL}"))
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y) = proj
        .project((lon, lat), true)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_330_788_739_673_707, 0.034_892_040_629_808_503),
    );
    drop(proj);
    assert_in_order(&journal.lines(), HGRID_RELOAD);
}

/// Two operations from one builder. The first is created before the change and reads nothing
/// else. The second's transform finds the change and reloads its own grid. The first then
/// transforms without a call: the chunk the reload read is in the shared cache, so it sees
/// version B, as PROJ does, and it is not reloaded.
#[test]
fn hgridshift_operation_made_before_the_change_reads_the_changed_chunk() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], None),
        grid_file(2, &[3.0, 6.0], None),
        2,
    ));
    let builder = builder_with(&journal);
    let definition = format!("+proj=hgridshift +grids={HGRID_URL}");
    let first = builder
        .clone()
        .proj(&definition)
        .expect("the first creation");
    let second = builder.proj(&definition).expect("the second creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y, _) = second
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_388_966_379_986_059, 0.034_921_129_449_964_679),
    );
    let calls_before = journal.lines().len();
    let (x, y, _) = first
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_388_966_379_986_059, 0.034_921_129_449_964_679),
    );
    assert_eq!(
        journal.lines().len(),
        calls_before,
        "the stale operation makes no call"
    );
    drop(first);
    drop(second);
    assert_in_order(&journal.lines(), HGRID_RELOAD);
}

/// An unchanged file is read as before, with no reload.
#[test]
fn hgridshift_unchanged_file_is_not_reloaded() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], None),
        grid_file(2, &[3.0, 6.0], None),
        0,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=hgridshift +grids={HGRID_URL}"))
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y, _) = proj
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_369_573_833_821_605, 0.034_911_433_176_882_452),
    );
    drop(proj);
    assert_eq!(journal.lines(), HGRID_UNCHANGED);
}

/// `gridshift` (`GenericShiftGridSet::reopen`): the same sequence as `hgridshift`, with the
/// `TYPE` of the grid in its GDAL metadata.
#[test]
fn gridshift_forward_reloads_the_grid_when_its_file_changed() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], Some(GRIDSHIFT_TYPE)),
        grid_file(2, &[3.0, 6.0], Some(GRIDSHIFT_TYPE)),
        2,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=gridshift +grids={GRIDSHIFT_URL}"))
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y, _) = proj
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_388_966_380_696_456, 0.034_921_129_450_319_874),
    );
    drop(proj);
    assert_in_order(&journal.lines(), GRIDSHIFT_RELOAD);
}

/// The inverse of `gridshift`, iterated on version B after the reload.
#[test]
fn gridshift_inverse_reloads_the_grid_when_its_file_changed() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], Some(GRIDSHIFT_TYPE)),
        grid_file(2, &[3.0, 6.0], Some(GRIDSHIFT_TYPE)),
        2,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=gridshift +grids={GRIDSHIFT_URL}"))
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y) = proj
        .project((lon, lat), true)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_330_788_738_963_31, 0.034_892_040_629_453_308),
    );
    drop(proj);
    assert_in_order(&journal.lines(), GRIDSHIFT_RELOAD);
}

/// `gridshift` on an unchanged file: one chunk read past the head, no reload.
#[test]
fn gridshift_unchanged_file_is_not_reloaded() {
    let journal = Arc::new(Journal::new(
        grid_file(2, &[1.0, 2.0], Some(GRIDSHIFT_TYPE)),
        grid_file(2, &[3.0, 6.0], Some(GRIDSHIFT_TYPE)),
        0,
    ));
    let proj = builder_with(&journal)
        .proj(&format!("+proj=gridshift +grids={GRIDSHIFT_URL}"))
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y, _) = proj
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_369_573_833_452_074, 0.034_911_433_176_697_683),
    );
    drop(proj);
    assert_eq!(journal.lines(), GRIDSHIFT_UNCHANGED);
}

/// `xyzgridshift` in a geocentric pipeline, the grid of version A read, the change found by the
/// first read past the head, and the grid loaded again. PROJ 9.9.0 has no reload for this step
/// and exits with SIGSEGV on the same file (see the module comment); the expected value is PROJ's
/// for version B read without a change.
#[test]
fn xyzgridshift_reloads_the_grid_when_its_file_changed() {
    let journal = Arc::new(Journal::new(
        grid_file(3, &[0.5, -0.25, 0.125], None),
        grid_file(3, &[1.5, -0.75, 0.375], None),
        2,
    ));
    let definition = format!(
        "+proj=pipeline +step +proj=cart +ellps=WGS84 +step +proj=xyzgridshift +grids={XYZ_URL} +step +proj=cart +inv +ellps=WGS84"
    );
    let proj = builder_with(&journal)
        .proj(&definition)
        .expect("the grid is read at creation");
    let (lon, lat) = radians(3.0, 2.0);
    let (x, y, z) = proj
        .convert_xyz(lon, lat, 0.0)
        .expect("(3, 2) is in the grid");
    assert_point(
        (x, y),
        (0.052_359_747_744_978_966, 0.034_906_636_158_660_999),
    );
    assert!((z - 1.470_891_110_599_041).abs() < 1e-9, "z {z}");
    drop(proj);
    let lines = journal.lines();
    assert!(
        lines
            .iter()
            .any(|line| line == "open(https://foo/x.tif, 0, 16384) -> #3 size_read=16384"),
        "the grid is loaded again: {lines:#?}"
    );
}

/// What PROJ 9.9.0 does once the change is read past the head: the open that sees the change, the
/// reload of the head, in PROJ's order (`libproj`, the probe). PROJ also reads the rest of the strip
/// in the same pass and does not read again after the reload, which the port's read sizes do not
/// reproduce (see `docs/PORTING-network.md`), so the check is in order, not exact.
const HGRID_RELOAD: &[&str] = &[
    "open(https://foo/h.tif, 16384, 16384) -> #2 size_read=16384",
    "header #2 Last-Modified = some_date CHANGED!!!!",
    "close #2",
    "open(https://foo/h.tif, 0, 16384) -> #3 size_read=16384",
    "header #3 Last-Modified = some_date CHANGED!!!!",
    "close #3",
];

const GRIDSHIFT_RELOAD: &[&str] = &[
    "open(https://foo/g.tif, 16384, 16384) -> #2 size_read=16384",
    "header #2 Last-Modified = some_date CHANGED!!!!",
    "close #2",
    "open(https://foo/g.tif, 0, 16384) -> #3 size_read=16384",
    "header #3 Last-Modified = some_date CHANGED!!!!",
    "close #3",
];

/// The lines of `expected` appear in `lines`, in this order, with other lines between them.
fn assert_in_order(lines: &[String], expected: &[&str]) {
    let mut next = 0;
    for line in lines {
        if next < expected.len() && line == expected[next] {
            next += 1;
        }
    }
    assert_eq!(next, expected.len(), "PROJ's lines in order, in {lines:#?}");
}

/// Journal of the probe: hgridshift, no change.
const HGRID_UNCHANGED: &[&str] = &[
    "open(https://foo/h.tif, 0, 16384) -> #1 size_read=16384",
    "header #1 Content-Range = bytes 0-16383/33054",
    "header #1 Last-Modified = some_date",
    "header #1 ETag = some_etag",
    "read_range #1 (32768, 16384) -> 286",
    "header #1 Content-Range = bytes 0-16383/33054",
    "header #1 Last-Modified = some_date",
    "header #1 ETag = some_etag",
    "close #1",
    "open(https://foo/h.tif, 16384, 16384) -> #2 size_read=16384",
    "header #2 Content-Range = bytes 0-16383/33054",
    "header #2 Last-Modified = some_date",
    "header #2 ETag = some_etag",
    "close #2",
];

/// Journal of the probe: gridshift, no change.
const GRIDSHIFT_UNCHANGED: &[&str] = &[
    "open(https://foo/g.tif, 0, 16384) -> #1 size_read=16384",
    "header #1 Content-Range = bytes 0-16383/33142",
    "header #1 Last-Modified = some_date",
    "header #1 ETag = some_etag",
    "read_range #1 (32768, 16384) -> 374",
    "header #1 Content-Range = bytes 0-16383/33142",
    "header #1 Last-Modified = some_date",
    "header #1 ETag = some_etag",
    "close #1",
    "open(https://foo/g.tif, 16384, 16384) -> #2 size_read=16384",
    "header #2 Content-Range = bytes 0-16383/33142",
    "header #2 Last-Modified = some_date",
    "header #2 ETag = some_etag",
    "close #2",
];
