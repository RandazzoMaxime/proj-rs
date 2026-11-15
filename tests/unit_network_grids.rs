//! PROJ 9.9.0 `test/unit/test_network.cpp`, `networking.curl_hgridshift` (line 1109),
//! `networking.curl_vgridshift` (line 1144) and `networking.curl_vgridshift_vertcon` (line 1180),
//! whole, against the real grids of `https://cdn.proj.org`.
//!
//! Each case returns at once when the CDN does not answer (`networkAccessOK`, the probe of
//! `initial_check`, shared here as `tests/net_cdn`). The grids are the C++ ones:
//! `fr_ign_gr3df97a.tif` (NTF to ETRS89), `us_nga_egm08_25.tif` (EGM2008 height; the C++ comment
//! says `egm08_25.tif`, but the name PROJ asks the CDN for is `us_nga_egm08_25.tif`) and
//! `us_noaa_vertcone.tif` (NGVD29 to NAVD88; the C++ comment says `vertcone.tif`). The
//! `libproj` 9.9.0 log (`PROJ_DEBUG=3`) shows these names as the ones downloaded.
//!
//! The C++ assertions are kept with their tolerances. The values checked on top of them are
//! PROJ 9.9.0's, measured with `libproj` 9.9.0 (`%.17g`, network on, `PROJ_DATA` of the reference
//! build, `HTTPS_PROXY` for the CDN), and checked to 1e-9:
//!
//! - `curl_hgridshift`: lat 48.999932260028217, lon 1.9992776848010316, height 0;
//! - `curl_vgridshift`: lat -29.999999999999996, lon 150, height -31.889799118042117;
//! - `curl_vgridshift_vertcon`: lat 40, lon -80, height -0.15375280380249004.
//!
//! The C++ cases call `proj_assign_context` on the transform and destroy the context; the port
//! has no such call (see `docs/PORTING-network.md`). The grid cache is off, as in the C++ cases,
//! and the user-writable directory is a temporary folder, not the user's home.

use std::fs;
use std::path::{Path, PathBuf};

use proj_rs::{GridCacheMode, ProjBuilder};

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::cdn_reachable;

/// Tolerance of the values measured with `libproj` 9.9.0.
const MEASURED_TOLERANCE: f64 = 1e-9;

/// `networking.curl_hgridshift`: NTF to ETRS89 (`EPSG:4275` to `EPSG:4171`), with
/// `fr_ign_gr3df97a.tif`. The C++ point is `(lat, long) = (49, 2)`; this API takes the GIS order
/// `(lon, lat)`, so the input is `(2, 49)`.
#[test]
fn curl_hgridshift() {
    if !cdn_reachable() {
        eprintln!("curl_hgridshift: the CDN does not answer, the case returns as PROJ does");
        return;
    }
    let dir = user_dir("curl_hgridshift");
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let proj = builder(&dir)
        .proj_known_crs("EPSG:4275", "EPSG:4171", None)
        .expect("ASSERT_NE(P, nullptr): NTF to ETRS89 with the grid");
    let (lon, lat, h) = proj.convert_xyz(2.0, 49.0, 0.0).expect("proj_trans");

    // EXPECT_NEAR(c.xyz.x, 48.9999322600, 1e-8): lat.
    assert!((lat - 48.999_932_26).abs() < 1e-8, "lat {lat}");
    // EXPECT_NEAR(c.xyz.y, 1.9992776848, 1e-8): long.
    assert!((lon - 1.999_277_684_8).abs() < 1e-8, "lon {lon}");
    // EXPECT_NEAR(c.xyz.z, 0, 1e-2).
    assert!(h.abs() < 1e-2, "height {h}");

    // The values of libproj 9.9.0 on the same grid.
    assert!(
        (lat - 48.999_932_260_028).abs() < MEASURED_TOLERANCE,
        "lat {lat} against PROJ"
    );
    assert!(
        (lon - 1.999_277_684_801).abs() < MEASURED_TOLERANCE,
        "lon {lon} against PROJ"
    );
    assert!(h.abs() < MEASURED_TOLERANCE, "height {h} against PROJ");

    let _gone = fs::remove_dir_all(&dir);
}

/// `networking.curl_vgridshift`: WGS 84 (3D) to WGS 84 with the EGM2008 height
/// (`EPSG:4979` to `EPSG:4326+3855`), with `us_nga_egm08_25.tif`. The C++ point is
/// `(lat, long, h) = (-30, 150, 0)`; the input here is `(lon, lat, h)`.
#[test]
fn curl_vgridshift() {
    if !cdn_reachable() {
        eprintln!("curl_vgridshift: the CDN does not answer, the case returns as PROJ does");
        return;
    }
    let dir = user_dir("curl_vgridshift");
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let proj = builder(&dir)
        .proj_known_crs("EPSG:4979", "EPSG:4326+3855", None)
        .expect("ASSERT_NE(P, nullptr): WGS 84 to EGM2008 height with the grid");
    let (lon, lat, h) = proj.convert_xyz(150.0, -30.0, 0.0).expect("proj_trans");

    // EXPECT_NEAR(c.xyz.x, -30, 1e-8): lat.
    assert!((lat + 30.0).abs() < 1e-8, "lat {lat}");
    // EXPECT_NEAR(c.xyz.y, 150, 1e-8): long.
    assert!((lon - 150.0).abs() < 1e-8, "lon {lon}");
    // EXPECT_NEAR(c.xyz.z, -31.89, 1e-2).
    assert!((h + 31.89).abs() < 1e-2, "height {h}");

    // The values of libproj 9.9.0 on the same grid.
    assert!(
        (lat + 30.0).abs() < MEASURED_TOLERANCE,
        "lat {lat} against PROJ"
    );
    assert!(
        (lon - 150.0).abs() < MEASURED_TOLERANCE,
        "lon {lon} against PROJ"
    );
    assert!(
        (h + 31.889_799_118_042).abs() < MEASURED_TOLERANCE,
        "height {h} against PROJ"
    );

    let _gone = fs::remove_dir_all(&dir);
}

/// `networking.curl_vgridshift_vertcon`: NGVD29 to NAVD88 height (`EPSG:4269+7968` to
/// `EPSG:4269+5703`), with `us_noaa_vertcone.tif`. The C++ point is `(lat, long, h) = (40, -80, 0)`;
/// the input here is `(lon, lat, h)`.
#[test]
fn curl_vgridshift_vertcon() {
    if !cdn_reachable() {
        eprintln!(
            "curl_vgridshift_vertcon: the CDN does not answer, the case returns as PROJ does"
        );
        return;
    }
    let dir = user_dir("curl_vgridshift_vertcon");
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let proj = builder(&dir)
        .proj_known_crs("EPSG:4269+7968", "EPSG:4269+5703", None)
        .expect("ASSERT_NE(P, nullptr): NGVD29 to NAVD88 height with the grid");
    let (lon, lat, h) = proj.convert_xyz(-80.0, 40.0, 0.0).expect("proj_trans");

    // EXPECT_NEAR(c.xyz.x, 40, 1e-8): lat.
    assert!((lat - 40.0).abs() < 1e-8, "lat {lat}");
    // EXPECT_NEAR(c.xyz.y, -80, 1e-8): long.
    assert!((lon + 80.0).abs() < 1e-8, "lon {lon}");
    // EXPECT_NEAR(c.xyz.z, -0.15, 1e-2).
    assert!((h + 0.15).abs() < 1e-2, "height {h}");

    // The values of libproj 9.9.0 on the same grid.
    assert!(
        (lat - 40.0).abs() < MEASURED_TOLERANCE,
        "lat {lat} against PROJ"
    );
    assert!(
        (lon + 80.0).abs() < MEASURED_TOLERANCE,
        "lon {lon} against PROJ"
    );
    assert!(
        (h + 0.153_752_803_802_49).abs() < MEASURED_TOLERANCE,
        "height {h} against PROJ"
    );

    let _gone = fs::remove_dir_all(&dir);
}

/// A builder as the C++ cases make their contexts: the grid cache off, the network on, and the
/// user-writable directory in `dir`.
fn builder(dir: &Path) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder
        .set_grid_cache_enabled(GridCacheMode::Disabled)
        .expect("grid cache off");
    builder.enable_network(true).expect("network on");
    builder
        .set_user_writable_directory(dir)
        .expect("user-writable directory");
    builder
}

/// A temporary folder for one case of this process, never inside the repository.
fn user_dir(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!("proj-rs-{case}-{}", std::process::id()))
}
