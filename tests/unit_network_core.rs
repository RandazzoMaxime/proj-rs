//! PROJ `test/unit/test_network.cpp`, the network cases that need no grid read in ranges: the
//! callbacks (`custom`, `getfilesize`, `simul_open_error`, `simul_read_range_error`), the
//! `network_endpoint_api_and_not_reachable_*` cases, and the operation factory cases
//! `pyproj_issue_1192` and `do_not_attempt_network_access_known_available_network_on`, and
//! `proj_coordoperation_get_grid_used`. Nothing here reaches `cdn.proj.org` over the network: the
//! endpoint is a local address with no server, and the one CDN URL that `grid_used` names is
//! answered by the callbacks.
//!
//! The assertions are PROJ's. `HUGE_VAL` with `PROJ_ERR_OTHER_NETWORK_ERROR` is
//! `Err(ProjError::Network)` here, and the description of the last used operation is the
//! C++ `proj_trans_get_last_used_operation`. The C++ input is in EPSG axis order (lat, long);
//! the Rust known-CRS API takes (long, lat) for geographic CRS, so the inputs are swapped.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use std::net::TcpListener;

use proj_rs::{
    create_from_user_input, CoordinateOperation, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, NetworkCallbacks,
    NetworkError, NetworkHandle, OpenReply, ProjBuilder, ProjError, ProjLines, SpatialCriterion,
};

/// A `http://` address that nothing listens on: the port of a listener that is then dropped.
/// PROJ's test uses `http://0.0.0.0` for the same case.
fn unreachable_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// A builder with the network on, the cache off, and an endpoint that does not answer.
fn network_builder() -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    builder
        .set_url_endpoint(&unreachable_endpoint())
        .expect("endpoint");
    builder
}

/// Records each `open` and answers with the error `"Cannot open file"`, as the C++ `open_cbk`
/// does for `errorMsg`.
#[derive(Default)]
struct FailingOpen {
    opens: Mutex<Vec<(String, u64, usize)>>,
}

impl NetworkCallbacks for FailingOpen {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        if let Ok(mut opens) = self.opens.lock() {
            opens.push((url.to_owned(), offset, size_to_read));
        }
        Err(NetworkError::Open("Cannot open file".to_owned()))
    }

    fn read_range(
        &self,
        _handle: NetworkHandle,
        _offset: u64,
        _size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        Err(NetworkError::ReadRange("no file is open".to_owned()))
    }

    fn close(&self, _handle: NetworkHandle) {}
}

/// `networking.simul_open_error`: the first `open` of the grid is `(url, 0, 16384)`, it fails,
/// and the explicit operation string does not build. No second call is made.
#[test]
fn simul_open_error_fails_creation_after_one_open() {
    let callbacks = Arc::new(FailingOpen::default());
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let observer = Arc::clone(&callbacks);
    builder.set_network_callbacks(callbacks);
    let url = "https://foo/open_error.tif";
    let definition = format!("+proj=vgridshift +grids={url} +multiplier=1");
    assert!(builder.proj(&definition).is_err());
    let opens = observer.opens.lock().expect("lock").clone();
    assert_eq!(opens, vec![(url.to_owned(), 0, 16384)]);
}

/// One call the callbacks receive, in PROJ's order. `file` numbers the files as the C++ test
/// does: the handle of `file` is the one its `open` gave.
enum Call {
    Open {
        file: usize,
        url: &'static str,
        offset: u64,
        size: usize,
        response: Vec<u8>,
    },
    Header {
        file: usize,
        key: &'static str,
        value: &'static str,
    },
    ReadRange {
        file: usize,
        offset: u64,
        size: usize,
        response: Vec<u8>,
    },
    /// A `read_range` that the callbacks refuse with `message`, as PROJ's `errorMsg`.
    ReadRangeFails {
        file: usize,
        offset: u64,
        size: usize,
        message: &'static str,
    },
    Close {
        file: usize,
    },
}

#[derive(Default)]
struct ExchangeState {
    /// Index of the next call expected.
    next: usize,
    /// The handle each file was opened with.
    handles: HashMap<usize, NetworkHandle>,
    next_handle: u64,
    /// Every call that did not match the expected one, in the order it happened.
    errors: Vec<String>,
}

/// PROJ's `ExchangeWithCallback` (`test/unit/test_network.cpp`): the callbacks must be called
/// in the order given, with the same arguments and on the handle of the same file. A call that
/// does not match is an error, and the call fails.
struct Exchange {
    expected: Vec<Call>,
    state: Mutex<ExchangeState>,
}

impl Exchange {
    fn new(expected: Vec<Call>) -> Self {
        Self {
            expected,
            state: Mutex::new(ExchangeState::default()),
        }
    }

    /// Every call up to `count` was made as expected, and none was unexpected.
    fn reached(&self, count: usize) {
        let state = self.state.lock().expect("lock");
        assert!(state.errors.is_empty(), "{:?}", state.errors);
        assert_eq!(state.next, count, "calls made as expected");
    }

    /// Checks the call against the next expected one. `matches` gives the answer when the call
    /// is the expected one, and `None` (after recording the error) when it is not.
    fn expect<T>(
        &self,
        what: &str,
        matches: impl FnOnce(&Call, &mut ExchangeState) -> Option<T>,
    ) -> Option<T> {
        let mut state = self.state.lock().expect("lock");
        let Some(call) = self.expected.get(state.next) else {
            state.errors.push(format!("unexpected {what}"));
            return None;
        };
        let answer = matches(call, &mut state);
        if answer.is_some() {
            state.next += 1;
        } else {
            let index = state.next;
            state.errors.push(format!("{what} is not call {index}"));
        }
        answer
    }
}

impl NetworkCallbacks for Exchange {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        let answer = self.expect(
            &format!("open({url}, {offset}, {size_to_read})"),
            |call, state| {
                let Call::Open {
                    file,
                    url: expected_url,
                    offset: expected_offset,
                    size,
                    response,
                } = call
                else {
                    return None;
                };
                if *expected_url != url || *expected_offset != offset || *size != size_to_read {
                    return None;
                }
                state.next_handle += 1;
                let handle = NetworkHandle(state.next_handle);
                state.handles.insert(*file, handle);
                Some(OpenReply {
                    handle,
                    data: response.clone(),
                })
            },
        );
        answer.ok_or_else(|| NetworkError::Open("the exchange did not expect this open".to_owned()))
    }

    fn header(&self, handle: NetworkHandle, name: &str) -> Option<String> {
        self.expect(&format!("header({name})"), |call, state| {
            let Call::Header { file, key, value } = call else {
                return None;
            };
            if *key != name || state.handles.get(file) != Some(&handle) {
                return None;
            }
            Some((*value).to_owned())
        })
    }

    fn read_range(
        &self,
        handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        let answer = self.expect(
            &format!("read_range({offset}, {size_to_read})"),
            |call, state| {
                let (file, expected_offset, size, reply) = match call {
                    Call::ReadRange {
                        file,
                        offset,
                        size,
                        response,
                    } => (file, offset, size, Ok(response.clone())),
                    Call::ReadRangeFails {
                        file,
                        offset,
                        size,
                        message,
                    } => (
                        file,
                        offset,
                        size,
                        Err(NetworkError::ReadRange((*message).to_owned())),
                    ),
                    _ => return None,
                };
                if *expected_offset != offset
                    || *size != size_to_read
                    || state.handles.get(file) != Some(&handle)
                {
                    return None;
                }
                Some(reply)
            },
        );
        answer.unwrap_or_else(|| {
            Err(NetworkError::ReadRange(
                "the exchange did not expect this read".to_owned(),
            ))
        })
    }

    fn close(&self, handle: NetworkHandle) {
        self.expect("close()", |call, state| {
            let Call::Close { file } = call else {
                return None;
            };
            (state.handles.get(file) == Some(&handle)).then_some(())
        });
    }
}

/// `networking.custom`: the grid is an uncompressed GeoTIFF of `egm96_15`, read by tile. Its
/// first 956 bytes are the file, and every tile after the header is served by the callbacks.
///
/// Creation opens `(url, 0, 16384)`, reads its three headers and closes. The first transform
/// (lat 49) opens the tile at 525244 as the aligned cover `(524288, 278528)`, and the second
/// (lat -49) reads the tile at 3670972 as `read_range(3670016, 278528)` on the same handle. The
/// handle is closed by `proj_destroy`. A second creation on the same builder makes no call: the
/// tile read by the first is still there, and it gives the same value.
///
/// The values are PROJ 9.9.0's: 1.25 and 2.25 are the constant tiles the C++ test serves.
#[test]
fn custom() {
    let url = "https://foo/my.tif";
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/tests/egm96_15_uncompressed_truncated.tif"
    ))
    .expect("the truncated egm96 fixture");
    assert_eq!(fixture.len(), 956);
    let mut head = fixture;
    head.resize(16384, 0);
    let tile = |value: f32| -> Vec<u8> {
        std::iter::repeat_n(value.to_le_bytes(), 278528 / 4)
            .flatten()
            .collect()
    };
    let headers = |file: usize| {
        [
            Call::Header {
                file,
                key: "Content-Range",
                value: "bytes=0-16383/10000000",
            },
            Call::Header {
                file,
                key: "Last-Modified",
                value: "some_date",
            },
            Call::Header {
                file,
                key: "ETag",
                value: "some_etag",
            },
        ]
    };
    let mut calls = vec![Call::Open {
        file: 1,
        url,
        offset: 0,
        size: 16384,
        response: head,
    }];
    calls.extend(headers(1));
    calls.push(Call::Close { file: 1 });
    let after_creation = calls.len();
    calls.push(Call::Open {
        file: 2,
        url,
        offset: 524288,
        size: 278528,
        response: tile(1.25),
    });
    calls.extend(headers(2));
    let after_first_transform = calls.len();
    calls.push(Call::ReadRange {
        file: 2,
        offset: 3670016,
        size: 278528,
        response: tile(2.25),
    });
    calls.extend(headers(2));
    let after_second_transform = calls.len();
    calls.push(Call::Close { file: 2 });
    let after_destroy = calls.len();

    let exchange = Arc::new(Exchange::new(calls));
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let callbacks: Arc<dyn NetworkCallbacks> = Arc::<Exchange>::clone(&exchange);
    builder.set_network_callbacks(callbacks);
    let definition = format!("+proj=vgridshift +grids={url} +multiplier=1");

    // A clone of the builder is the same context: the callbacks and what they served stay.
    let proj = builder
        .clone()
        .proj(&definition)
        .expect("the grid is read at creation");
    exchange.reached(after_creation);

    // Radians: the operation has no unit conversion, as the C++ `PJ_COORD` input.
    let longitude = 2.0 / 180.0 * PI;
    let (_, _, z) = proj
        .convert_xyz(longitude, 49.0 / 180.0 * PI, 0.0)
        .expect("lat 49 is in the grid");
    assert_eq!(z, 1.25);
    exchange.reached(after_first_transform);

    let (_, _, z) = proj
        .convert_xyz(longitude, -49.0 / 180.0 * PI, 0.0)
        .expect("lat -49 is in the grid");
    assert_eq!(z, 2.25);
    exchange.reached(after_second_transform);

    drop(proj);
    exchange.reached(after_destroy);

    // Once again, no network access.
    let proj = builder.proj(&definition).expect("second creation");
    let (_, _, z) = proj
        .convert_xyz(longitude, 49.0 / 180.0 * PI, 0.0)
        .expect("lat 49 is in the grid");
    assert_eq!(z, 1.25);
    exchange.reached(after_destroy);
}

/// `networking.getfilesize`: the grid is a single-strip GeoTIFF of which the callbacks serve
/// only the first 550 bytes, while `Content-Range` gives the file as 4153510 bytes.
///
/// Creation opens `(url, 0, 16384)`, reads its three headers and closes, and reads no strip: the
/// strip is not in the reply. The second creation on the same builder makes no call, the
/// first 16384-byte chunk being kept. No transform is run here, so the size from
/// `Content-Range` is not used by a read.
#[test]
fn getfilesize() {
    let url = "https://foo/getfilesize.tif";
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/tests/test_vgrid_single_strip_truncated.tif"
    ))
    .expect("the single-strip fixture");
    assert!(fixture.len() >= 550, "the reply is the first 550 bytes");
    let head = fixture[..550].to_vec();
    let calls = vec![
        Call::Open {
            file: 1,
            url,
            offset: 0,
            size: 16384,
            response: head,
        },
        Call::Header {
            file: 1,
            key: "Content-Range",
            value: "bytes 0-16383/4153510",
        },
        Call::Header {
            file: 1,
            key: "Last-Modified",
            value: "some_date",
        },
        Call::Header {
            file: 1,
            key: "ETag",
            value: "some_etag",
        },
        Call::Close { file: 1 },
    ];
    let after_creation = calls.len();

    let exchange = Arc::new(Exchange::new(calls));
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let callbacks: Arc<dyn NetworkCallbacks> = Arc::<Exchange>::clone(&exchange);
    builder.set_network_callbacks(callbacks);
    let definition = format!("+proj=vgridshift +grids={url} +multiplier=1");

    let proj = builder
        .clone()
        .proj(&definition)
        .expect("the grid is created from its first chunk");
    exchange.reached(after_creation);
    drop(proj);
    exchange.reached(after_creation);

    let proj = builder.proj(&definition).expect("second creation");
    exchange.reached(after_creation);
    drop(proj);
    exchange.reached(after_creation);
}

/// `networking.simul_read_range_error`: the grid is the uncompressed `egm96_15` of `custom`, and
/// the `read_range` of its second tile is refused with "read range error".
///
/// Creation and the first transform (lat 49, 1.25) are those of `custom`. The second transform
/// (lat -49) calls `read_range(3670016, 278528)` on the handle of the first tile, and nothing
/// else: no header is read after the refused read. The handle is closed by `proj_destroy`.
///
/// Measured against PROJ 9.9.0 (`libproj` of `target/proj-reference`, a C harness with the same
/// callbacks): creation `open(url, 0, 16384)`, three headers, `close`; first transform
/// `open(url, 524288, 278528)`, three headers, `z` 1.25; second transform
/// `read_range(3670016, 278528)`, then no call, `z` `HUGE_VAL` and `proj_errno` 4099
/// (`PROJ_ERR_OTHER_NETWORK_ERROR`); `close` at `proj_destroy`.
#[test]
fn simul_read_range_error() {
    let url = "https://foo/read_range_error.tif";
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/tests/egm96_15_uncompressed_truncated.tif"
    ))
    .expect("the truncated egm96 fixture");
    assert_eq!(fixture.len(), 956);
    let mut head = fixture;
    head.resize(16384, 0);
    let tile: Vec<u8> = std::iter::repeat_n(1.25_f32.to_le_bytes(), 278528 / 4)
        .flatten()
        .collect();
    let headers = |file: usize| {
        [
            Call::Header {
                file,
                key: "Content-Range",
                value: "bytes=0-16383/10000000",
            },
            Call::Header {
                file,
                key: "Last-Modified",
                value: "some_date",
            },
            Call::Header {
                file,
                key: "ETag",
                value: "some_etag",
            },
        ]
    };
    let mut calls = vec![Call::Open {
        file: 1,
        url,
        offset: 0,
        size: 16384,
        response: head,
    }];
    calls.extend(headers(1));
    calls.push(Call::Close { file: 1 });
    let after_creation = calls.len();
    calls.push(Call::Open {
        file: 2,
        url,
        offset: 524288,
        size: 278528,
        response: tile,
    });
    calls.extend(headers(2));
    let after_first_transform = calls.len();
    calls.push(Call::ReadRangeFails {
        file: 2,
        offset: 3670016,
        size: 278528,
        message: "read range error",
    });
    let after_second_transform = calls.len();
    calls.push(Call::Close { file: 2 });
    let after_destroy = calls.len();

    let exchange = Arc::new(Exchange::new(calls));
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let callbacks: Arc<dyn NetworkCallbacks> = Arc::<Exchange>::clone(&exchange);
    builder.set_network_callbacks(callbacks);
    let definition = format!("+proj=vgridshift +grids={url} +multiplier=1");

    let proj = builder
        .proj(&definition)
        .expect("the grid is read at creation");
    exchange.reached(after_creation);

    // Radians, as in `custom`: the operation has no unit conversion.
    let longitude = 2.0 / 180.0 * PI;
    let (_, _, z) = proj
        .convert_xyz(longitude, 49.0 / 180.0 * PI, 0.0)
        .expect("lat 49 is in the grid");
    assert_eq!(z, 1.25);
    exchange.reached(after_first_transform);

    let result = proj.convert_xyz(longitude, -49.0 / 180.0 * PI, 0.0);
    assert!(
        matches!(result, Err(ProjError::Network)),
        "lat -49 is a network error, not {result:?}"
    );
    exchange.reached(after_second_transform);

    drop(proj);
    exchange.reached(after_destroy);
}

/// `networking.simul_file_change_while_opened`: the grid is the `egm96_15` GeoTIFF of `custom`, and
/// its file changes while it is open. The first tile read (lat 49) reads the tile as the aligned
/// cover `(524288, 278528)`, and that reply has a `Last-Modified` other than the one of the
/// creation. PROJ then drops the grid (its file closes), loads it again from `(url, 0, 16384)`, and
/// runs the point again: the tile is in the chunk cache, so the second run makes no call. The
/// creation's file is closed before the transform, the reloaded one stays open until the operation
/// is destroyed.
///
/// The value is PROJ's 1.25, from the tile of the changed file.
#[test]
fn simul_file_change_while_opened() {
    let url = "https://foo/file_change_while_opened.tif";
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/tests/egm96_15_uncompressed_truncated.tif"
    ))
    .expect("the truncated egm96 fixture");
    assert_eq!(fixture.len(), 956);
    let mut head = fixture;
    head.resize(16384, 0);
    let tile = |value: f32| -> Vec<u8> {
        std::iter::repeat_n(value.to_le_bytes(), 278528 / 4)
            .flatten()
            .collect()
    };
    let headers = |file: usize, last_modified: &'static str| {
        [
            Call::Header {
                file,
                key: "Content-Range",
                value: "bytes=0-16383/10000000",
            },
            Call::Header {
                file,
                key: "Last-Modified",
                value: last_modified,
            },
            Call::Header {
                file,
                key: "ETag",
                value: "some_etag",
            },
        ]
    };
    let mut calls = vec![Call::Open {
        file: 1,
        url,
        offset: 0,
        size: 16384,
        response: head.clone(),
    }];
    calls.extend(headers(1, "some_date"));
    calls.push(Call::Close { file: 1 });
    let after_creation = calls.len();
    calls.push(Call::Open {
        file: 2,
        url,
        offset: 524288,
        size: 278528,
        response: tile(1.25),
    });
    calls.extend(headers(2, "some_date CHANGED!!!!"));
    calls.push(Call::Close { file: 2 });
    calls.push(Call::Open {
        file: 3,
        url,
        offset: 0,
        size: 16384,
        response: head,
    });
    calls.extend(headers(3, "some_date CHANGED!!!!"));
    let after_transform = calls.len();
    calls.push(Call::Close { file: 3 });
    let after_destroy = calls.len();

    let exchange = Arc::new(Exchange::new(calls));
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let callbacks: Arc<dyn NetworkCallbacks> = Arc::<Exchange>::clone(&exchange);
    builder.set_network_callbacks(callbacks);
    let definition = format!("+proj=vgridshift +grids={url} +multiplier=1");

    let proj = builder
        .proj(&definition)
        .expect("the grid is read at creation");
    exchange.reached(after_creation);

    // Radians: the operation has no unit conversion, as the C++ `PJ_COORD` input.
    let longitude = 2.0 / 180.0 * PI;
    let (_, _, z) = proj
        .convert_xyz(longitude, 49.0 / 180.0 * PI, 0.0)
        .expect("lat 49 is in the grid");
    assert_eq!(z, 1.25);
    exchange.reached(after_transform);

    drop(proj);
    exchange.reached(after_destroy);
}

/// The operation of a CRS pair whose grid is unreachable is created. Forward, forward, then
/// inverse, each transform gives a network error, and the last used operation is the candidate
/// PROJ names in `description`.
fn assert_not_reachable(from: &str, to: &str, input: (f64, f64), description: &str) {
    let proj = network_builder()
        .proj_known_crs(from, to, None)
        .expect("lazy grid: the operation is created");
    for inverse in [false, false, true] {
        let result = proj.project(input, inverse);
        assert!(
            matches!(result, Err(ProjError::Network)),
            "{from} to {to}, inverse={inverse}: {result:?}"
        );
        let last = proj.last_used_operation().expect("last used operation");
        assert_eq!(last.proj_info().description.as_deref(), Some(description));
    }
}

/// `networking.network_endpoint_api_and_not_reachable_gridshift`: NAD83 to NAD83(HARN) through
/// `us_noaa_nadcon5_nad83_1986_nad83_harn_conus.tif`, not on disk.
#[test]
fn endpoint_api_not_reachable_gridshift() {
    assert_not_reachable(
        "EPSG:4269",
        "EPSG:4152",
        (-80.0, 40.0),
        "NAD83 to NAD83(HARN) (47)",
    );
}

/// `networking.network_endpoint_api_and_not_reachable_hgridshift`: MGI to ETRS89 (8).
#[test]
fn endpoint_api_not_reachable_hgridshift() {
    assert_not_reachable("EPSG:4312", "EPSG:4258", (15.0, 48.0), "MGI to ETRS89 (8)");
}

/// `networking.network_endpoint_api_and_not_reachable_xyzgridshift`: NTF to ETRS89-FRA.
#[test]
fn endpoint_api_not_reachable_xyzgridshift() {
    assert_not_reachable(
        "EPSG:4275",
        "EPSG:4171",
        (2.0, 49.0),
        "NTF to ETRS89-FRA [RGF93 v1] (1)",
    );
}

/// `networking.network_endpoint_api_and_not_reachable_vgridshift`: POSGAR 2007 to SRVN16 height.
#[test]
fn endpoint_api_not_reachable_vgridshift() {
    assert_not_reachable(
        "EPSG:5342",
        "EPSG:9521",
        (-60.0, -40.0),
        "POSGAR 2007 to SRVN16 height (1)",
    );
}

/// The operations of `networking.pyproj_issue_1192` (EPSG:4326 to EPSG:2964), in the order of
/// `proj_list_get`. Measured on PROJ 9.9.0 (`libproj`, TIFF and curl build) with a C probe.
const ISSUE_1192_NAMES: [&str; 10] = [
    "Inverse of NAD27 to WGS 84 (85) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (7) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (33) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (3) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (13) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (14) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (10) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (21) + Alaska Albers",
    "Inverse of NAD27 to WGS 84 (22) + Alaska Albers",
    "Ballpark geographic offset from WGS 84 to NAD27 + Alaska Albers",
];

/// `proj_coordoperation_is_instantiable` with the network off, PROJ 9.9.0: operations 00 and 02
/// name grids that are not on disk.
const ISSUE_1192_INSTANTIABLE_NETWORK_OFF: [bool; 10] =
    [false, true, false, true, true, true, true, true, true, true];

/// The same with the network on, PROJ 9.9.0: every operation is instantiable, the grids that the
/// database knows counting as available.
const ISSUE_1192_INSTANTIABLE_NETWORK_ON: [bool; 10] = [true; 10];

/// `networking.pyproj_issue_1192`: the factory context of the C++ test (grid availability
/// IGNORED, spatial criterion PARTIAL_INTERSECTION, all authorities), run once with the network
/// on and once off. Each instantiable operation has a `proj_pj_info` id.
///
/// `proj_coordoperation_is_instantiable(ctx, P)` reads the network flag of the context. Here the
/// flag is the grid availability: network on is `KnownAvailable` (a grid the database knows
/// counts as available, `considerKnownGridsAsAvailable`), network off is `UseForSorting` (only
/// the grids on disk). No download happens in either half: PROJ's answer with the network on is
/// the same with cdn.proj.org refused, which is the case here.
///
/// The id is PROJ's `proj_pj_info` id of a pipeline, `pipeline`. Only the operations whose grids
/// are on disk can be built here (`ProjBuilder::proj` reads the grid at creation). The network-on
/// operations that need a grid not on disk are checked by their PROJ string only: PROJ builds
/// them lazily, which the Rust API does not offer for a single operation.
#[test]
fn pyproj_issue_1192() {
    let db = DatabaseContext::create().expect("embedded proj.db");
    let from = create_from_user_input("EPSG:4326", &db).expect("EPSG:4326");
    let to = create_from_user_input("EPSG:2964", &db).expect("EPSG:2964");

    let halves = [
        (
            false,
            GridAvailabilityUse::UseForSorting,
            ISSUE_1192_INSTANTIABLE_NETWORK_OFF,
        ),
        (
            true,
            GridAvailabilityUse::KnownAvailable,
            ISSUE_1192_INSTANTIABLE_NETWORK_ON,
        ),
    ];
    for (network_on, known, expected) in halves {
        let mut context = CoordinateOperationContext::create(&db, "");
        context
            .set_grid_availability_use(GridAvailabilityUse::IgnoreGridAvailability)
            .set_spatial_criterion(SpatialCriterion::PartialIntersection);
        let operations = CoordinateOperationFactory::create()
            .create_operations(&from, &to, &context)
            .expect("operations");

        let names: Vec<&str> = operations.iter().map(CoordinateOperation::name).collect();
        assert_eq!(names, ISSUE_1192_NAMES, "network on: {network_on}");

        let flags: Vec<bool> = operations
            .iter()
            .map(|operation| operation.is_proj_instantiable(&db, known))
            .collect();
        assert_eq!(flags, expected, "network on: {network_on}");

        for (operation, instantiable) in operations.iter().zip(flags) {
            if !instantiable {
                continue;
            }
            let definition = operation
                .export_to_proj_string_with_database(&db, ProjLines::Single)
                .expect("the operation has a PROJ string");
            assert!(
                definition.starts_with("+proj=pipeline"),
                "{}: {definition}",
                operation.name()
            );
            if !network_on {
                let proj = ProjBuilder::new()
                    .proj(&definition)
                    .expect("no grid is needed to build it");
                let info = proj.proj_info();
                assert_eq!(info.id.as_deref(), Some("pipeline"), "{}", operation.name());
            }
        }
    }
}

/// Counts every callback the builder makes. Every method counts once, `header` included, so the
/// count is the number of network calls PROJ would make.
struct CountingCallbacks {
    calls: Arc<AtomicUsize>,
}

impl NetworkCallbacks for CountingCallbacks {
    fn open(
        &self,
        _url: &str,
        _offset: u64,
        _size_to_read: usize,
    ) -> Result<OpenReply, NetworkError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(NetworkError::Open("Cannot open file".to_owned()))
    }

    fn header(&self, _handle: NetworkHandle, _name: &str) -> Option<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        None
    }

    fn read_range(
        &self,
        _handle: NetworkHandle,
        _offset: u64,
        _size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(NetworkError::ReadRange("no file is open".to_owned()))
    }

    fn close(&self, _handle: NetworkHandle) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

/// The 79 operations of PROJ 9.9.0 from EPSG:4326 to EPSG:4267 under `KNOWN_AVAILABLE` and
/// `PARTIAL_INTERSECTION`, in PROJ's order (`proj_list_get` from 0 to 78). The same list with
/// network off, measured with `libproj` 9.9.0.
const DO_NOT_ATTEMPT_NAMES: [&str; 79] = [
    "Inverse of NAD27 to WGS 84 (33)",
    "Inverse of NAD27 to WGS 84 (3)",
    "Inverse of NAD27 to WGS 84 (79)",
    "Inverse of NAD27 to WGS 84 (4)",
    "Inverse of NAD27 to WGS 84 (13)",
    "Inverse of NAD27 to WGS 84 (85)",
    "Inverse of NAD27 to WGS 84 (6)",
    "Inverse of NAD27 to WGS 84 (5)",
    "Inverse of NAD27 to WGS 84 (18)",
    "Inverse of NAD27 to WGS 84 (78)",
    "Inverse of NAD27 to WGS 84 (11)",
    "Inverse of NAD27 to WGS 84 (12)",
    "Inverse of NAD27 to WGS 84 (7)",
    "Inverse of NAD27 to WGS 84 (1)",
    "Inverse of NAD27 to WGS 84 (34)",
    "Inverse of NAD27 to WGS 84 (10)",
    "Inverse of NAD27 to WGS 84 (2)",
    "Inverse of NAD27 to WGS 84 (84)",
    "Inverse of NAD27 to WGS 84 (88)",
    "Inverse of NAD27 to WGS 84 (30)",
    "Inverse of NAD27 to WGS 84 (14)",
    "Inverse of NAD27 to WGS 84 (36)",
    "Inverse of NAD27 to WGS 84 (32)",
    "Inverse of NAD27 to WGS 84 (71)",
    "Inverse of NAD27 to WGS 84 (72)",
    "Inverse of NAD27 to WGS 84 (43)",
    "Inverse of NAD27 to WGS 84 (58)",
    "Inverse of NAD27 to WGS 84 (68)",
    "Inverse of NAD27 to WGS 84 (54)",
    "Inverse of NAD27 to WGS 84 (8)",
    "Inverse of NAD27 to WGS 84 (16)",
    "Inverse of NAD27 to WGS 84 (80)",
    "Inverse of NAD27 to WGS 84 (40)",
    "Inverse of NAD27 to WGS 84 (61)",
    "Inverse of NAD27 to WGS 84 (81)",
    "Inverse of NAD27 to WGS 84 (55)",
    "Inverse of NAD27 to WGS 84 (63)",
    "Inverse of NAD27 to WGS 84 (38)",
    "Inverse of NAD27 to WGS 84 (64)",
    "Inverse of NAD27 to WGS 84 (41)",
    "Inverse of NAD27 to WGS 84 (57)",
    "Inverse of NAD27 to WGS 84 (67)",
    "Inverse of NAD27 to WGS 84 (76)",
    "Inverse of NAD27 to WGS 84 (21)",
    "Inverse of NAD27 to WGS 84 (42)",
    "Inverse of NAD27 to WGS 84 (59)",
    "Inverse of NAD27 to WGS 84 (77)",
    "Inverse of NAD27 to WGS 84 (60)",
    "Inverse of NAD27 to WGS 84 (73)",
    "Inverse of NAD27 to WGS 84 (74)",
    "Inverse of NAD27 to WGS 84 (45)",
    "Inverse of NAD27 to WGS 84 (44)",
    "Inverse of NAD27 to WGS 84 (69)",
    "Inverse of NAD27 to WGS 84 (50)",
    "Inverse of NAD27 to WGS 84 (48)",
    "Inverse of NAD27 to WGS 84 (49)",
    "Inverse of NAD27 to WGS 84 (65)",
    "Inverse of NAD27 to WGS 84 (47)",
    "Inverse of NAD27 to WGS 84 (56)",
    "Inverse of NAD27 to WGS 84 (37)",
    "Inverse of NAD27 to WGS 84 (89)",
    "Inverse of NAD27 to WGS 84 (87)",
    "Inverse of NAD27 to WGS 84 (39)",
    "Inverse of NAD27 to WGS 84 (75)",
    "Inverse of NAD27 to WGS 84 (51)",
    "Inverse of NAD27 to WGS 84 (66)",
    "Inverse of NAD27 to WGS 84 (53)",
    "Inverse of NAD27 to WGS 84 (70)",
    "Inverse of NAD27 to WGS 84 (86)",
    "Inverse of NAD27 to WGS 84 (46)",
    "Inverse of NAD27 to WGS 84 (17)",
    "Inverse of NAD27 to WGS 84 (22)",
    "Inverse of NAD27 to WGS 84 (52)",
    "Inverse of NAD27 to WGS 84 (82)",
    "Inverse of NAD27 to WGS 84 (62)",
    "Inverse of NAD27 to WGS 84 (83)",
    "Inverse of NAD27 to WGS 84 (15)",
    "Inverse of NAD27 to WGS 84 (9)",
    "Ballpark geographic offset from WGS 84 to NAD27",
];

/// `networking.do_not_attempt_network_access_known_available_network_on`: with network on and
/// callbacks registered on the builder, the operation factory in `KNOWN_AVAILABLE` mode creates
/// the operations of PROJ 9.9.0 and makes no callback call. The control below shows the counter
/// is wired: an absent grid named by an explicit string reaches `open` once.
#[test]
fn do_not_attempt_network_access_known_available_network_on() {
    let db = DatabaseContext::create().expect("embedded proj.db");
    let from = create_from_user_input("EPSG:4326", &db).expect("EPSG:4326");
    let to = create_from_user_input("EPSG:4267", &db).expect("EPSG:4267");

    let calls = Arc::new(AtomicUsize::new(0));
    let mut builder = ProjBuilder::new();
    builder.enable_network(true).expect("network on");
    builder
        .set_url_endpoint(&unreachable_endpoint())
        .expect("endpoint");
    builder.set_network_callbacks(Arc::new(CountingCallbacks {
        calls: Arc::clone(&calls),
    }));
    // `ASSERT_TRUE(proj_context_set_network_callbacks(...))`: the Rust registration returns
    // nothing and cannot fail, so its outcome is the network state the builder reports here.
    assert!(builder.network_enabled());

    let mut context = CoordinateOperationContext::create(&db, "");
    context
        .set_grid_availability_use(GridAvailabilityUse::KnownAvailable)
        .set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let operations = builder
        .create_operations(&from, &to, &context)
        .expect("operations");

    // PROJ: `EXPECT_GE(num_operations, 10)`. The exact list is checked below.
    assert!(operations.len() >= 10, "{} operations", operations.len());
    let names: Vec<&str> = operations.iter().map(CoordinateOperation::name).collect();
    assert_eq!(names, DO_NOT_ATTEMPT_NAMES);

    // PROJ: `EXPECT_FALSE(networkActivity)`.
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "callback calls by the factory"
    );

    // Control: the counter sees a fetch. PROJ makes one `open` for this explicit string and
    // `proj_create` gives `NULL`.
    let control = builder
        .clone()
        .proj("+proj=hgridshift +grids=proj_rs_no_such_grid.tif");
    assert!(control.is_err(), "the absent grid must not build");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "callback calls of the control"
    );
}

/// Serves the bytes of one file under one URL, and refuses every other URL. Each `open` and
/// `read_range` gives the bytes of the file at the offset asked; `Content-Range` gives its size.
struct ServeOneFile {
    url: &'static str,
    bytes: Vec<u8>,
    /// Every `open` the callbacks received, as `(url, offset, size_to_read)`.
    opens: Mutex<Vec<(String, u64, usize)>>,
}

impl ServeOneFile {
    /// Up to `size` bytes of the file from `offset`, fewer at its end.
    fn slice(&self, offset: u64, size: usize) -> Vec<u8> {
        let len = self.bytes.len();
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(len);
        let end = start.saturating_add(size).min(len);
        self.bytes[start..end].to_vec()
    }
}

impl NetworkCallbacks for ServeOneFile {
    fn open(&self, url: &str, offset: u64, size_to_read: usize) -> Result<OpenReply, NetworkError> {
        if let Ok(mut opens) = self.opens.lock() {
            opens.push((url.to_owned(), offset, size_to_read));
        }
        if url != self.url {
            return Err(NetworkError::Open("Cannot open file".to_owned()));
        }
        Ok(OpenReply {
            handle: NetworkHandle(1),
            data: self.slice(offset, size_to_read),
        })
    }

    fn header(&self, _handle: NetworkHandle, name: &str) -> Option<String> {
        (name == "Content-Range").then(|| {
            let total = self.bytes.len();
            format!("bytes 0-{}/{total}", total.saturating_sub(1))
        })
    }

    fn read_range(
        &self,
        _handle: NetworkHandle,
        offset: u64,
        size_to_read: usize,
    ) -> Result<Vec<u8>, NetworkError> {
        Ok(self.slice(offset, size_to_read))
    }

    fn close(&self, _handle: NetworkHandle) {}
}

/// `networking.proj_coordoperation_get_grid_used` (`test/unit/test_network.cpp`): the grid of
/// `proj=vgridshift grids=nz_linz_nzgd2000-c120100904-grid01.tif` is in neither `proj.db` nor on
/// disk. With the network on, the grid is the file at the CDN URL: its short name is the name it
/// is asked by, its full name is empty, and its URL is `https://cdn.proj.org/` and the name. Two
/// creations give the same answer.
///
/// The CDN file is not in the repository and the sandbox has no route to the CDN, so the
/// callbacks answer the one URL with the first 956 bytes of the truncated egm96 fixture, and
/// refuse every other URL. The three assertions of the C++ test do not depend on the bytes: they
/// name the file, not its content. `proj.db` has no row for this name, so the URL is the one PROJ
/// gives when the open of the CDN URL succeeds, as `FileManager::open_resource_file` does.
///
/// The `available` flag and the open are not asserted by the C++ test. They are the values PROJ
/// 9.9.0 gives for the same callbacks (`libproj` probe, `TIFF`+`curl` build): `available` 1,
/// `directDownload` 0, `openLicense` 0, one `open(url, 0, 16384)` for the two creations, with no
/// call from the `grids_needed` queries.
#[test]
fn proj_coordoperation_get_grid_used() {
    const NAME: &str = "nz_linz_nzgd2000-c120100904-grid01.tif";
    const URL: &str = "https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif";
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/tests/egm96_15_uncompressed_truncated.tif"
    ))
    .expect("the truncated egm96 fixture");
    assert_eq!(bytes.len(), 956);

    let served = Arc::new(ServeOneFile {
        url: URL,
        bytes,
        opens: Mutex::new(Vec::new()),
    });
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let shared: Arc<dyn NetworkCallbacks> = Arc::<ServeOneFile>::clone(&served);
    builder.set_network_callbacks(shared);

    for creation in 0..2 {
        let proj = builder
            .clone()
            .proj(&format!("proj=vgridshift grids={NAME}"))
            .expect("the vgridshift of the CDN grid");
        let grids = proj.grids_needed().expect("the grids the operation needs");
        assert_eq!(grids.len(), 1, "creation {creation}: one grid");
        let grid = &grids[0];
        // PROJ: `EXPECT_EQ(std::string(shortName), ...)`.
        assert_eq!(grid.short_name, NAME, "creation {creation}: short name");
        // PROJ: `EXPECT_EQ(std::string(fullName), "")`.
        assert_eq!(grid.info.full_name, "", "creation {creation}: full name");
        // PROJ: `EXPECT_EQ(std::string(url), "https://cdn.proj.org/...")`.
        assert_eq!(grid.info.url, URL, "creation {creation}: url");
        // Measured with PROJ 9.9.0, not asserted by the C++ test.
        assert!(grid.info.available, "creation {creation}: available");
        assert!(!grid.info.direct_download, "creation {creation}: direct");
        assert!(!grid.info.open_license, "creation {creation}: open license");
    }

    // Measured with PROJ 9.9.0: one open for the two creations, with the first chunk.
    let opens = served.opens.lock().expect("lock").clone();
    assert_eq!(opens, vec![(URL.to_owned(), 0, 16384)], "callback opens");
}
