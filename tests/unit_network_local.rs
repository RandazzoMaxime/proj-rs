//! PROJ `test/unit/test_network.cpp`, the cases that need no reference grid values, against a
//! local server (`tests/net_support`). Nothing here reaches `cdn.proj.org`.
//!
//! Not ported here, with the cause in `docs/AUDIT-network.md`: the callback cases (`custom`,
//! `getfilesize`, `simul_*`), the SQLite grid cache (`cache_*`, `proj_grid_cache_clear`), the
//! explicit download API (`download_whole_files`, `file_api`), and the lazy grid errors of
//! `network_endpoint_api_and_not_reachable_*`.

use proj_rs::ProjBuilder;

#[path = "net_support/mod.rs"]
mod net_support;

use net_support::{LocalServer, Route};

/// `networking.basic`, first part: `proj_context_create` with network off by default makes no
/// request, and the pipeline does not build (`ASSERT_EQ(P, nullptr)`). The grid URL is on the
/// local server, so an answer would show up in `requests()`.
#[test]
fn basic_network_is_off_by_default_and_makes_no_request() {
    let server = LocalServer::spawn(vec![Route {
        path: "/fr_ign_ntf_r93.tif",
        status: 200,
        body: b"unused".to_vec(),
    }]);
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    assert!(!builder.network_enabled());
    let pipeline = format!(
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=hgridshift +grids={}/fr_ign_ntf_r93.tif \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg",
        server.url()
    );
    assert!(builder.proj(&pipeline).is_err());
    assert!(server.requests().is_empty(), "{:?}", server.requests());
}

/// `networking.curl_invalid_resource`: with the network on, a grid that the server does not
/// have gives no operation (`ASSERT_EQ(P, nullptr)`), and exactly one request is made.
#[test]
fn curl_invalid_resource() {
    let server = LocalServer::spawn(Vec::new());
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    let definition = format!(
        "+proj=hgridshift +grids={}/i_do_not.exist/my.tif",
        server.url()
    );
    assert!(builder.proj(&definition).is_err());
    assert_eq!(server.requests(), vec!["/i_do_not.exist/my.tif".to_owned()]);
}
