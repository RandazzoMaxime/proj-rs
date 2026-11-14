//! PROJ 9.9.0 `test/unit/test_network.cpp`, `networking.basic` (line 121), whole, against the real
//! grid `fr_ign_ntf_r93.tif` of `https://cdn.proj.org`.
//!
//! The C++ case builds one pipeline in four states, and checks the value of the last one:
//!
//! 1. network off by default: `proj_create` gives `NULL` (`ASSERT_EQ(P, nullptr)`);
//! 2. `PROJ_NETWORK=ON` in the environment: `proj_create` is not `NULL`, only when the CDN answers
//!    (`networkAccessOK`, the probe of `initial_check`, shared here as `tests/net_cdn`);
//! 3. the variable cleared again: `NULL`;
//! 4. `proj_context_set_enable_network(ctx, true)`: when the CDN answers, `proj_create` is not
//!    `NULL` and the forward transform of (2, 49) gives (1.9992776848, 48.99993226) to 1e-10;
//!    otherwise `NULL`, and the case returns.
//!
//! The values are PROJ 9.9.0's, measured with `libproj` 9.9.0 (TIFF and curl, `PROJ_DATA` of the
//! reference build, `HTTPS_PROXY` for the CDN): lon 1.9992776847752205, lat 48.999932259999646.
//!
//! The C++ case toggles the default context and the process environment. The port uses
//! `ProjBuilder`s, one per state, and sets `PROJ_NETWORK` the way `putenv` does (an empty value is
//! absent). The case is the only test of this binary, so the environment is not shared with another
//! test. The user-writable directory is a temporary folder, not the user's home.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use proj_rs::{GridCacheMode, ProjBuilder};

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::cdn_reachable;

const PIPELINE: &str = "+proj=pipeline \
    +step +proj=unitconvert +xy_in=deg +xy_out=rad \
    +step +proj=hgridshift +grids=https://cdn.proj.org/fr_ign_ntf_r93.tif \
    +step +proj=unitconvert +xy_in=rad +xy_out=deg";

/// `EXPECT_NEAR(longitude, 1.9992776848, 1e-10)`, `EXPECT_NEAR(lat, 48.9999322600, 1e-10)`.
const EXPECTED_LON: f64 = 1.9992776848;
const EXPECTED_LAT: f64 = 48.9999322600;
const TOLERANCE: f64 = 1e-10;

#[test]
fn basic() {
    // `initial_check`: the case keeps going without the CDN, and only the states that need it
    // are skipped, as PROJ's `if (networkAccessOK)` does.
    let network_access_ok = cdn_reachable();
    if !network_access_ok {
        eprintln!("basic: the CDN does not answer, the network states are not checked");
    }
    let dir = user_dir();
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    // Network disabled by default. The variable is set empty, which is absent for PROJ, so a
    // `PROJ_NETWORK` set in the caller's environment does not change the first state.
    env::set_var("PROJ_NETWORK", "");
    let off = context_builder(&dir);
    assert!(!off.network_enabled(), "network off by default");
    assert!(
        off.proj(PIPELINE).is_err(),
        "network off: proj_create gives NULL"
    );

    // Enabled through the environment: `ProjBuilder::new` reads `PROJ_NETWORK`.
    env::set_var("PROJ_NETWORK", "ON");
    let mut from_env = ProjBuilder::new();
    from_env
        .set_user_writable_directory(&dir)
        .expect("user-writable directory");
    let built = from_env.proj(PIPELINE);
    if network_access_ok {
        assert!(
            built.is_ok(),
            "PROJ_NETWORK=ON: proj_create is not NULL: {:?}",
            built.err()
        );
    }
    drop(built);
    env::set_var("PROJ_NETWORK", "");

    // Still disabled once the variable is cleared.
    assert!(
        context_builder(&dir).proj(PIPELINE).is_err(),
        "PROJ_NETWORK cleared: proj_create gives NULL"
    );

    // Enabled through the API, on a builder that starts with the network off.
    let mut api = context_builder(&dir);
    api.enable_network(true).expect("network on");
    let result = api.proj(PIPELINE);
    if network_access_ok {
        let proj = result.expect("network on: proj_create is not NULL");
        let (lon, lat) = proj
            .convert((2.0_f64, 49.0_f64))
            .expect("forward transform");
        assert!(
            (lon - EXPECTED_LON).abs() <= TOLERANCE,
            "longitude {lon} against PROJ {EXPECTED_LON}"
        );
        assert!(
            (lat - EXPECTED_LAT).abs() <= TOLERANCE,
            "latitude {lat} against PROJ {EXPECTED_LAT}"
        );
    } else {
        assert!(
            result.is_err(),
            "network on, CDN not answering: proj_create gives NULL"
        );
    }

    let _gone = fs::remove_dir_all(&dir);
}

/// A builder as the C++ case makes its contexts: the grid cache off, the user-writable directory
/// in `dir`, and the network state of the environment (`ProjBuilder::new`).
fn context_builder(dir: &Path) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder
        .set_grid_cache_enabled(GridCacheMode::Disabled)
        .expect("grid cache off");
    builder
        .set_user_writable_directory(dir)
        .expect("user-writable directory");
    builder
}

/// A temporary folder for this process, never inside the repository.
fn user_dir() -> PathBuf {
    env::temp_dir().join(format!("proj-rs-basic-{}", std::process::id()))
}
