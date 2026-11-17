//! PROJ 9.9.0 `test/unit/test_network.cpp`, `networking.proj_coordoperation_get_grid_used`
//! (line 2191), against the real grid `nz_linz_nzgd2000-c120100904-grid01.tif` of
//! `https://cdn.proj.org`.
//!
//! The C++ case creates `proj=vgridshift grids=nz_linz_nzgd2000-c120100904-grid01.tif` twice, with
//! the network on and the grid cache off, and checks the short name, the empty full name and the
//! CDN URL of the grid that `proj.db` does not know. It has no callbacks: PROJ reads the CDN
//! itself, so the port does too. `HTTPS_PROXY` is followed as curl follows it.
//!
//! The case is guarded by the `initial_check` probe (`tests/net_cdn`): when the CDN does not
//! answer `206`, the case returns, as PROJ's `if (!networkAccessOK) return;` does.
//!
//! The values beyond the C++ assertions are PROJ 9.9.0's, measured with `libproj` 9.9.0 (TIFF and
//! curl build, `PROJ_DATA` of the reference build) against the same CDN file: one grid, package
//! empty, `directDownload` 0, `openLicense` 0, `available` 1, in both creations. `proj_create`
//! gives a non-`NULL` operation in both creations.
//!
//! The user-writable directory is a temporary folder, not the user's home, and the grid cache is
//! off, so nothing is written next to the test.
#![cfg(feature = "network")]

use std::env;
use std::fs;
use std::path::PathBuf;

use proj_rs::{GridCacheMode, ProjBuilder};

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::cdn_reachable;

const NAME: &str = "nz_linz_nzgd2000-c120100904-grid01.tif";
const URL: &str = "https://cdn.proj.org/nz_linz_nzgd2000-c120100904-grid01.tif";

#[test]
fn proj_coordoperation_get_grid_used() {
    if !cdn_reachable() {
        eprintln!("proj_coordoperation_get_grid_used: the CDN does not answer, the case returns");
        return;
    }
    let dir = user_dir();
    let _gone = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temporary user-writable directory");

    let mut builder = ProjBuilder::new();
    builder
        .set_grid_cache_enabled(GridCacheMode::Disabled)
        .expect("grid cache off");
    builder
        .set_user_writable_directory(&dir)
        .expect("user-writable directory");
    builder.enable_network(true).expect("network on");

    // Bugfix for https://github.com/OSGeo/PROJ/issues/3444#issuecomment-1309499342: the second
    // creation gives the same answer as the first.
    for creation in 0..2 {
        let proj = builder
            .clone()
            .proj(&format!("proj=vgridshift grids={NAME}"))
            .unwrap_or_else(|error| panic!("creation {creation}: proj_create gives NULL: {error}"));
        let grids = proj.grids_needed().expect("the grids the operation needs");
        assert_eq!(grids.len(), 1, "creation {creation}: one grid");
        let grid = &grids[0];
        // PROJ: `EXPECT_EQ(std::string(shortName), "nz_linz_nzgd2000-c120100904-grid01.tif")`.
        assert_eq!(grid.short_name, NAME, "creation {creation}: short name");
        // PROJ: `EXPECT_EQ(std::string(fullName), "")`.
        assert_eq!(grid.info.full_name, "", "creation {creation}: full name");
        // PROJ: `EXPECT_EQ(std::string(url), "https://cdn.proj.org/...")`.
        assert_eq!(grid.info.url, URL, "creation {creation}: url");
        // Measured with PROJ 9.9.0 against the CDN file.
        assert_eq!(grid.info.package_name, "", "creation {creation}: package");
        assert!(grid.info.available, "creation {creation}: available");
        assert!(!grid.info.direct_download, "creation {creation}: direct");
        assert!(!grid.info.open_license, "creation {creation}: open license");
    }

    let _gone = fs::remove_dir_all(&dir);
}

/// A temporary folder for this process, never inside the repository.
fn user_dir() -> PathBuf {
    env::temp_dir().join(format!("proj-rs-grid-used-{}", std::process::id()))
}
