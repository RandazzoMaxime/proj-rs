//! PROJ `networking.network_endpoint_env_variable`: `PROJ_NETWORK_ENDPOINT` is read when the
//! builder is made, and a grid of the pair is then fetched from that endpoint.
//!
//! One test per file: the variable is process state, and the other test binaries run in other
//! processes.

use std::net::TcpListener;

use proj_rs::{ProjBuilder, ProjError};

#[test]
fn network_endpoint_env_variable() {
    // A port nothing listens on. PROJ's test uses `http://0.0.0.0/`, which is the same case.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    let endpoint = format!("http://127.0.0.1:{port}");
    std::env::set_var("PROJ_NETWORK_ENDPOINT", format!("{endpoint}/"));
    let mut builder = ProjBuilder::new();
    std::env::remove_var("PROJ_NETWORK_ENDPOINT");
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    assert_eq!(
        builder.get_url_endpoint().expect("endpoint"),
        format!("{endpoint}/")
    );
    // NAD83 to NAD83(HARN) in West Virginia, through `wvhpgn.tif`, which is not on disk.
    let proj = builder
        .proj_known_crs("EPSG:4269", "EPSG:4152", None)
        .expect("lazy grid: the operation is created");
    // C++ `c.xyz` is (lat 40, long -80); the Rust known-CRS order is (long, lat).
    let result = proj.project((-80.0_f64, 40.0_f64), false);
    assert!(matches!(result, Err(ProjError::Network)), "{result:?}");
}
