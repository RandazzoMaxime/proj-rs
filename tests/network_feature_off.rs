//! Without the `network` feature, proj-rs behaves as PROJ 9.9.0 built without libcurl
//! (`src/networkfilemanager.cpp`, `proj_context_set_enable_network` and `no_op_network_open`):
//! the flag can be set, the call reports that access is not possible, and a remote grid does not
//! open. Callbacks from the application still work. See `docs/PORTING-network-feature.md`.
#![cfg(not(feature = "network"))]

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use proj_rs::{NetworkCallbacks, NetworkError, NetworkHandle, OpenReply, ProjBuilder};

#[path = "net_support/mod.rs"]
mod net_support;

use net_support::{LocalServer, Route};

fn pipeline(server: &LocalServer) -> String {
    format!("+proj=hgridshift +grids={}/missing.tif", server.url())
}

fn server() -> LocalServer {
    LocalServer::spawn(vec![Route {
        path: "/missing.tif",
        status: 200,
        body: b"unused".to_vec(),
    }])
}

#[derive(Default)]
struct CountingOpen(AtomicUsize);

impl NetworkCallbacks for CountingOpen {
    fn open(&self, _: &str, _: u64, _: usize) -> Result<OpenReply, NetworkError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Err(NetworkError::Open("refused".to_owned()))
    }

    fn read_range(&self, _: NetworkHandle, _: u64, _: usize) -> Result<Vec<u8>, NetworkError> {
        Err(NetworkError::ReadRange("refused".to_owned()))
    }

    fn close(&self, _: NetworkHandle) {}
}

/// `proj_context_set_enable_network` returns `enabled && open != no_op_network_open`, and
/// `proj_context_is_network_enabled` returns the flag.
#[test]
fn enabling_is_refused_without_callbacks_and_accepted_with_them() {
    let mut builder = ProjBuilder::new();
    assert_eq!(builder.enable_network(true).expect("call"), 0);
    assert!(builder.network_enabled());
    assert_eq!(builder.enable_network(false).expect("call"), 0);
    assert!(!builder.network_enabled());
    builder.set_network_callbacks(Arc::new(CountingOpen::default()));
    assert_eq!(builder.enable_network(true).expect("call"), 1);
    assert!(builder.network_enabled());
}

/// The grid does not open, no request leaves the process, and the error is the one of a build
/// with the network off.
#[test]
fn remote_grid_fails_like_network_off_and_sends_nothing() {
    let server = server();
    let mut off = ProjBuilder::new();
    off.grid_cache_enable(false).expect("cache off");
    let off_error = off.proj(&pipeline(&server)).expect_err("grid is remote");
    let mut on = ProjBuilder::new();
    on.grid_cache_enable(false).expect("cache off");
    on.enable_network(true).expect("call");
    let on_error = on.proj(&pipeline(&server)).expect_err("no client");
    assert_eq!(on_error.to_string(), off_error.to_string());
    assert!(server.requests().is_empty(), "{:?}", server.requests());
}

/// `PROJ_NETWORK=ON` turns the flag on and nothing more, as in PROJ without libcurl: the tool
/// fails on the grid, like a run with the network off, and no request is sent.
#[test]
fn env_switch_sends_nothing() {
    let server = server();
    let run = |network: &str| {
        Command::new(env!("CARGO_BIN_EXE_cct"))
            .env("PROJ_NETWORK", network)
            .env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES")
            .arg("+proj=hgridshift")
            .arg(format!("+grids={}/missing.tif", server.url()))
            .stdin(Stdio::null())
            .output()
            .expect("run cct")
    };
    let off = run("OFF");
    let on = run("ON");
    assert!(!on.status.success());
    assert_eq!(on.stderr, off.stderr);
    assert!(server.requests().is_empty(), "{:?}", server.requests());
}

/// The application's callbacks are the only client: they are called, with the grid URL.
#[test]
fn callbacks_are_the_only_client() {
    let server = server();
    let callbacks = Arc::new(CountingOpen::default());
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("call");
    let shared: Arc<dyn NetworkCallbacks> = Arc::<CountingOpen>::clone(&callbacks);
    builder.set_network_callbacks(shared);
    assert!(builder.proj(&pipeline(&server)).is_err());
    assert_eq!(callbacks.0.load(Ordering::Relaxed), 1);
    assert!(server.requests().is_empty());
}
