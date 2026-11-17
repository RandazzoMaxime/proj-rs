//! A grid `proj.db` knows, missing from the disk, is available when the network opens it.
//!
//! PROJ 9.9.0 `DatabaseContext::lookForGridInfo` (factory.cpp:3398-3540) resolves the file with
//! `open_resource_file`, which reads it from the network when the network is on; a grid it opens
//! there is available. Measured with `projinfo` 9.9.0 (TIFF and curl build), network on:
//! `INVERSE(EPSG):12219` from `EPSG:9288` to `EPSG:11036` uses `nl_nsgi_nlgeo2018.tif` and is
//! listed without « at least one grid missing ». The port marked it missing, so `--only-best`
//! fell back to a ballpark without the grid (72 invocations of the network corpus).
//!
//! The grid is served by a local server set as the endpoint: nothing reaches `cdn.proj.org`.
#![cfg(feature = "network")]

use std::env;
use std::fs;
use std::path::PathBuf;

use proj_rs::{GridAvailabilityUse, GridCacheMode, ProjBuilder};

#[path = "net_support/mod.rs"]
mod net_support;
use net_support::{LocalServer, Route};

const NAME: &str = "nl_nsgi_nlgeo2018.tif";

#[derive(Clone, Copy)]
enum Network {
    On,
    Off,
}

fn builder(network: Network, endpoint: &str, dir: &PathBuf) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder
        .set_grid_cache_enabled(GridCacheMode::Disabled)
        .expect("grid cache off");
    builder
        .set_user_writable_directory(dir)
        .expect("user-writable directory");
    builder.set_url_endpoint(endpoint).expect("endpoint");
    builder
        .enable_network(matches!(network, Network::On))
        .expect("network switch");
    builder
}

#[test]
fn a_known_grid_on_the_network_is_available() {
    let server = LocalServer::spawn(vec![Route {
        path: "/nl_nsgi_nlgeo2018.tif",
        status: 200,
        body: vec![0x49; 40_000],
    }]);
    let dir = env::temp_dir().join(format!("proj-rs-grid-available-{}", std::process::id()));
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let on = builder(Network::On, server.url(), &dir)
        .look_for_grid_info(NAME, GridAvailabilityUse::UseForSorting)
        .expect("lookup with the network on");
    assert!(on.in_database, "proj.db has a row for {NAME}");
    assert!(on.info.available, "the network opens {NAME}");
    assert!(
        server.requests().iter().any(|line| line.contains(NAME)),
        "the grid was asked for: {:?}",
        server.requests()
    );

    let off = builder(Network::Off, server.url(), &dir)
        .look_for_grid_info(NAME, GridAvailabilityUse::UseForSorting)
        .expect("lookup with the network off");
    assert!(off.in_database);
    assert!(
        !off.info.available,
        "without the network the grid is missing"
    );

    let _gone = fs::remove_dir_all(&dir);
}
