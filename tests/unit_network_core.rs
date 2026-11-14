//! PROJ `test/unit/test_network.cpp`, the network cases that need no grid read in ranges: the
//! callbacks (`custom`, `simul_open_error`) and the `network_endpoint_api_and_not_reachable_*`
//! cases. Nothing here reaches `cdn.proj.org`: the endpoint is a local address with no server.
//!
//! The assertions are PROJ's. `HUGE_VAL` with `PROJ_ERR_OTHER_NETWORK_ERROR` is
//! `Err(ProjError::Network)` here, and the description of the last used operation is the
//! C++ `proj_trans_get_last_used_operation`. The C++ input is in EPSG axis order (lat, long);
//! the Rust known-CRS API takes (long, lat) for geographic CRS, so the inputs are swapped.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use std::net::TcpListener;

use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder, ProjError};

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
                let Call::ReadRange {
                    file,
                    offset: expected_offset,
                    size,
                    response,
                } = call
                else {
                    return None;
                };
                if *expected_offset != offset
                    || *size != size_to_read
                    || state.handles.get(file) != Some(&handle)
                {
                    return None;
                }
                Some(response.clone())
            },
        );
        answer.ok_or_else(|| {
            NetworkError::ReadRange("the exchange did not expect this read".to_owned())
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
