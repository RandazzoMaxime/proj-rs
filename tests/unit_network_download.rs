//! PROJ 9.9.0 `test/unit/test_network.cpp`, `networking.download_whole_files` (line 1859).
//!
//! The C++ case downloads `dk_sdfe_dvr90.tif` from `https://cdn.proj.org`, into the user-writable
//! directory, and it returns at once when the CDN does not answer (`networkAccessOK`, the probe of
//! `initial_check`). The port is the same: `cdn_reachable` is `initial_check`'s request, a `GET` of
//! `fr_ign_ntf_r93.tif` with `Range: bytes=0-1`, and the case returns when it does not get `206`.
//! The probe makes its own request, so a broken download cannot pass for an unreachable CDN.
//!
//! Every value below is PROJ 9.9.0's, measured with `libproj` 9.9.0 (TIFF and curl) on the same
//! CDN file: `z` 36.590999603271491, and the three progress values of a 331904-byte file read in
//! 100000-byte pieces. `PROJ_FULL_FILE_CHUNK_SIZE=100000` is the knob the C++ case sets.
//!
//! Where the port differs from the C++ case, the reason is in `docs/PORTING-network.md`: the
//! context is a `ProjBuilder` (the C++ case toggles the default context and a second one), and
//! the `ctx` argument of the progress callback is the `&ProjBuilder` that downloads.

use std::fs::{self, File};
use std::path::Path;
use std::ptr;

use proj_rs::{GridAvailabilityUse, ProjBuilder, TtlSetting};
use rusqlite::Connection;

#[path = "net_cdn/mod.rs"]
mod net_cdn;
use net_cdn::cdn_reachable;

const GRID: &str = "dk_sdfe_dvr90.tif";
const PIPELINE: &str = "+proj=pipeline \
    +step +proj=unitconvert +xy_in=deg +xy_out=rad \
    +step +proj=vgridshift +grids=dk_sdfe_dvr90.tif +multiplier=1 \
    +step +proj=unitconvert +xy_in=rad +xy_out=deg";

/// `z` of PROJ 9.9.0 at lon 12, lat 56 (`EXPECT_NEAR` with 1e-10).
const EXPECTED_Z: f64 = 36.5909996032715;

/// The progress values of PROJ 9.9.0: 200000, 300000 and 331904 bytes of a 331904-byte file, as
/// `double(totalDownloaded) / props.size` gives them (0.60258387967605087 and
/// 0.90387581951407636 at 17 digits; the shortest form that reads back to the same double is
/// written here).
const EXPECTED_PROGRESS: [f64; 3] = [0.602_583_879_676_050_9, 0.903_875_819_514_076_4, 1.0];

/// The whole-file test of PROJ, with its 37 assertions as the C++ file orders them.
#[test]
fn download_whole_files() {
    if !cdn_reachable() {
        eprintln!("download_whole_files: the CDN does not answer, the case returns as PROJ does");
        return;
    }
    // Only this test of the binary sets it: PROJ reads it at each download.
    std::env::set_var("PROJ_FULL_FILE_CHUNK_SIZE", "100000");
    let dir = std::env::temp_dir().join(format!("proj-rs-download-whole-{}", std::process::id()));
    let _gone = fs::remove_dir_all(&dir);

    // proj_context_set_enable_network(nullptr, true) on the default context, then proj_grid_info.
    let default_like = builder(&dir, Network::On);
    let grid_info = default_like
        .grid_info(GRID)
        .expect("grid info with the network on");
    assert!(grid_info.filename.is_none(), "filename is empty");
    assert_eq!(grid_info.gridname, GRID);
    assert_eq!(grid_info.format, "gtiff");

    // proj_context_create(): the network is off, and no grid is on disk.
    let mut ctx = builder(&dir, Network::Off);
    let before = ctx
        .look_for_grid_info(GRID, GridAvailabilityUse::UseForSorting)
        .expect("lookForGridInfo");
    assert!(!before.in_database, "the grid has no row in proj.db");
    assert!(
        !before.info.available,
        "gridAvailable is false before the download"
    );

    ctx.enable_network(true).expect("network on");
    assert!(ctx
        .is_download_needed(GRID, TtlSetting::Respect)
        .expect("needed"));
    assert!(
        ctx.find_local_file(GRID).is_none(),
        "pj_find_file(disable_network) is false"
    );

    ctx.download_file(GRID, TtlSetting::Respect, None)
        .expect("proj_download_file");
    let local = ctx
        .find_local_file(GRID)
        .expect("pj_find_file after the download");
    assert!(!local.as_os_str().is_empty(), "the path is not empty");

    let after = ctx
        .look_for_grid_info(GRID, GridAvailabilityUse::UseForSorting)
        .expect("lookForGridInfo");
    assert!(!after.in_database, "still no row in proj.db");
    assert!(
        after.info.available,
        "gridAvailable is true once the file is on disk"
    );
    assert!(File::open(dir.join(GRID)).is_ok(), "the file opens");

    // proj_context_set_enable_network(ctx, false), then proj_create and proj_trans_generic.
    ctx.enable_network(false).expect("network off");
    let proj = ctx
        .clone()
        .proj(PIPELINE)
        .expect("proj_create of the vgridshift pipeline");
    let (_lon, _lat, z) = proj.convert_xyz(12.0, 56.0, 0.0).expect("transform");
    assert!((z - EXPECTED_Z).abs() <= 1e-10, "z = {z}");
    drop(proj);

    ctx.enable_network(true).expect("network on");
    assert!(!ctx
        .is_download_needed(GRID, TtlSetting::Respect)
        .expect("needed"));

    let db = ctx.grid_cache_path();
    let conn = Connection::open(&db).expect("open cache.db");
    conn.execute("UPDATE downloaded_file_properties SET lastChecked = 0", [])
        .expect("force lastChecked to the epoch");
    drop(conn);

    // With the TTL ignored, no request is sent: lastChecked stays 0.
    assert!(!ctx
        .is_download_needed(GRID, TtlSetting::Ignore)
        .expect("needed"));
    let conn = Connection::open(&db).expect("open cache.db");
    let last_checked: i64 = conn
        .query_row(
            "SELECT lastChecked FROM downloaded_file_properties",
            [],
            |row| row.get(0),
        )
        .expect("row");
    assert_eq!(last_checked, 0, "lastChecked is still 0");
    drop(conn);

    // The TTL has run out: the CDN is asked again, nothing is downloaded, lastChecked moves on.
    assert!(!ctx
        .is_download_needed(GRID, TtlSetting::Respect)
        .expect("needed"));
    let conn = Connection::open(&db).expect("open cache.db");
    let last_checked: i64 = conn
        .query_row(
            "SELECT lastChecked FROM downloaded_file_properties",
            [],
            |row| row.get(0),
        )
        .expect("row");
    assert_ne!(last_checked, 0, "lastChecked has been updated");
    // An invalid lastModified forces a new download.
    conn.execute(
        "UPDATE downloaded_file_properties SET lastChecked = 0, lastModified = 'foo'",
        [],
    )
    .expect("invalidate lastModified");
    drop(conn);
    assert!(ctx
        .is_download_needed(GRID, TtlSetting::Respect)
        .expect("needed"));

    // Download again with a progress callback. The callback gets the builder that downloads.
    fs::remove_file(dir.join(GRID)).expect("remove the file");
    let mut calls: Vec<(bool, f64)> = Vec::new();
    let mut progress = |downloader: &ProjBuilder, pct: f64| {
        calls.push((ptr::eq(downloader, &ctx), pct));
        true
    };
    ctx.download_file(GRID, TtlSetting::Respect, Some(&mut progress))
        .expect("proj_download_file with a callback");

    assert_eq!(calls.len(), 3, "three progress callbacks");
    assert!(
        calls.iter().all(|(same, _)| *same),
        "the callback gets the downloading builder"
    );
    let pcts: Vec<f64> = calls.iter().map(|(_, pct)| *pct).collect();
    for (got, want) in pcts.iter().zip(EXPECTED_PROGRESS) {
        assert!(
            (got - want).abs() < 1e-15,
            "progress {got} against PROJ {want}"
        );
    }
    assert_eq!(pcts.last().copied(), Some(1.0), "the last progress is 1.0");

    remove_dir(&dir);
}

/// The network state of a builder in these tests.
#[derive(Clone, Copy)]
enum Network {
    On,
    Off,
}

/// A builder with the given network state, writing its grids and `cache.db` to `dir`.
fn builder(dir: &Path, network: Network) -> ProjBuilder {
    let mut builder = ProjBuilder::new();
    builder
        .enable_network(matches!(network, Network::On))
        .expect("network state");
    builder
        .set_user_writable_directory(dir)
        .expect("user-writable directory");
    builder
}

/// The user-writable directory is left as the case left it: removed.
fn remove_dir(dir: &Path) {
    let _gone = fs::remove_dir_all(dir);
}
