//! PROJ `test/unit/test_network.cpp`, `curl_hgridshift`, `curl_vgridshift` and
//! `curl_vgridshift_vertcon`, against the local server (`tests/net_support`) instead of
//! `cdn.proj.org`.
//!
//! The C++ tests read the CDN grids (`fr_ign_gr3df97a.tif`, `egm08_25.tif`, `vertcone.tif`),
//! which are not in the tree. The grids here are small GeoTIFF fixtures written by
//! `net_support/grids.rs`, served under the names PROJ asks for. The expected values are
//! PROJ 9.9.0 itself: the network-enabled reference build (`cs2cs -f %.10f`, `PROJ_NETWORK=ON`,
//! `PROJ_NETWORK_ENDPOINT` on a local server) run on the same bytes. They are not the CDN values
//! of the C++ test, so the values differ from its `-31.89` and `-0.15`.

use proj_rs::ProjBuilder;

#[path = "net_support/grids.rs"]
#[expect(dead_code, reason = "this file serves the three CDN grids only")]
mod grids;
#[path = "net_support/mod.rs"]
mod net_support;

use net_support::{LocalServer, Route};

/// The three grids of the C++ cases, served under the names the reference asks for.
fn grid_server() -> LocalServer {
    LocalServer::spawn(vec![
        Route {
            path: "/fr_ign_gr3df97a.tif",
            status: 200,
            body: grids::france_xyz_grid(),
        },
        Route {
            path: "/us_nga_egm08_25.tif",
            status: 200,
            body: grids::global_geoid_grid(),
        },
        Route {
            path: "/us_noaa_vertcone.tif",
            status: 200,
            body: grids::conus_vertical_grid(),
        },
    ])
}

/// A builder with the network on, the endpoint on `server`, and the grid cache off
/// (`proj_grid_cache_set_enable(ctx, false)`).
fn builder_for(server: &LocalServer) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder.grid_cache_enable(false).expect("cache off");
    builder.enable_network(true).expect("network on");
    builder.set_url_endpoint(server.url()).expect("endpoint");
    builder
}

/// `networking.curl_hgridshift`: NTF to ETRS89 (`EPSG:4275` to `EPSG:4171`), the grid is
/// `fr_ign_gr3df97a.tif`. The C++ test passes `(lat, lon) = (49, 2)`; this API takes the GIS
/// order `(lon, lat)`, so the input is `(2, 49)`. Reference: PROJ 9.9.0
/// `cs2cs -f %.10f EPSG:4275 EPSG:4171` with `49 2 0` gives `48.9968893575 1.9999925459 0`.
#[test]
fn curl_hgridshift() {
    let server = grid_server();
    let proj = builder_for(&server)
        .proj_known_crs("EPSG:4275", "EPSG:4171", None)
        .expect("NTF to ETRS89 with the grid");
    let (lon, lat, h) = proj.convert_xyz(2.0, 49.0, 0.0).expect("point");
    assert!((lat - 48.996_889_357_5).abs() < 1e-8, "lat {lat}");
    assert!((lon - 1.999_992_545_9).abs() < 1e-8, "lon {lon}");
    assert!(h.abs() < 1e-8, "h {h}");
    assert!(
        server
            .requests()
            .contains(&"/fr_ign_gr3df97a.tif".to_owned()),
        "{:?}",
        server.requests()
    );
}

/// `networking.curl_vgridshift`: WGS 84 (3D) to WGS 84 with the EGM2008 height
/// (`EPSG:4979` to `EPSG:4326+3855`), the grid is `us_nga_egm08_25.tif`. Input `(lon, lat)` is
/// `(150, -30)`. Reference: PROJ 9.9.0 `cs2cs -f %.10f EPSG:4979 EPSG:4326+3855` with
/// `-30 150 0` gives `-30 150 14.6249995232`.
#[test]
fn curl_vgridshift() {
    let server = grid_server();
    let proj = builder_for(&server)
        .proj_known_crs("EPSG:4979", "EPSG:4326+3855", None)
        .expect("WGS 84 to EGM2008 height with the grid");
    let (lon, lat, h) = proj.convert_xyz(150.0, -30.0, 0.0).expect("point");
    assert!((lat + 30.0).abs() < 1e-8, "lat {lat}");
    assert!((lon - 150.0).abs() < 1e-8, "lon {lon}");
    assert!((h - 14.624_999_523_2).abs() < 1e-8, "h {h}");
    assert!(
        server
            .requests()
            .contains(&"/us_nga_egm08_25.tif".to_owned()),
        "{:?}",
        server.requests()
    );
}

/// `networking.curl_vgridshift_vertcon`: NGVD29 to NAVD88 height (`EPSG:4269+7968` to
/// `EPSG:4269+5703`), the grid is `us_noaa_vertcone.tif`. Input `(lon, lat)` is `(-80, 40)`.
/// Reference: PROJ 9.9.0 `cs2cs -f %.10f EPSG:4269+7968 EPSG:4269+5703` with `40 -80 0` gives
/// `40 -80 -0.1295000091`.
#[test]
fn curl_vgridshift_vertcon() {
    let server = grid_server();
    let proj = builder_for(&server)
        .proj_known_crs("EPSG:4269+7968", "EPSG:4269+5703", None)
        .expect("NGVD29 to NAVD88 with the grid");
    let (lon, lat, h) = proj.convert_xyz(-80.0, 40.0, 0.0).expect("point");
    assert!((lat - 40.0).abs() < 1e-8, "lat {lat}");
    assert!((lon + 80.0).abs() < 1e-8, "lon {lon}");
    assert!((h + 0.129_500_009_1).abs() < 1e-8, "h {h}");
    assert!(
        server
            .requests()
            .contains(&"/us_noaa_vertcone.tif".to_owned()),
        "{:?}",
        server.requests()
    );
}
