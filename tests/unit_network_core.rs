//! PROJ `test/unit/test_network.cpp`, the network cases that need no grid read in ranges: the
//! callbacks (`simul_open_error`) and the `network_endpoint_api_and_not_reachable_*` cases.
//! Nothing here reaches `cdn.proj.org`: the endpoint is a local address with no server.
//!
//! The assertions are PROJ's. `HUGE_VAL` with `PROJ_ERR_OTHER_NETWORK_ERROR` is
//! `Err(ProjError::Network)` here, and the description of the last used operation is the
//! C++ `proj_trans_get_last_used_operation`. The C++ input is in EPSG axis order (lat, long);
//! the Rust known-CRS API takes (long, lat) for geographic CRS, so the inputs are swapped.

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
