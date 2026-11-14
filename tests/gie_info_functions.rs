//! `gie.info_functions` of PROJ 9.9.0 (`test/unit/gie_self_tests.cpp`, lines
//! 351-482), in Rust, assertion for assertion.
//!
//! Expected values are PROJ's own, copied from the C++ literals or read from
//! PROJ 9.9.0 on the same input (`proj_init_info` on the fixture below). The C
//! test runs its grid block under `#ifdef TIFF_ENABLED`. The reference build in
//! `target/proj-reference` has TIFF off; the crate always reads GeoTIFF grids,
//! so the block runs here, with the C literals as its oracle.
//!
//! Three differences of form, none of value:
//! - `proj_info().release` is `Rel. 9.9.0 (hash), date` in PROJ and `9.9.0`
//!   here. The C test only checks that it is not empty, and so does this one.
//! - The C buffer of `proj_rtodms2(buf, size, ...)` holds `size - 1` characters
//!   and a terminating nul. The Rust `rtodms` takes that count directly as
//!   `max_len`, so each C size `n` is `max_len = n - 1` here.
//! - `proj_pj_info` returns C strings that may be null; `Proj::proj_info` returns
//!   `Option<String>`, with the same text.

use std::f64::consts::PI;
use std::path::Path;
use std::sync::Once;

use proj_rs::dms::{dmstor, rtodms, DmsFormat, DmsSign};
use proj_rs::info::{grid_info, init_info, library_info};
use proj_rs::Proj;

/// `PROJ_DATA` for this test process, as PROJ's tests set it (`for_tests`).
/// The vendored `data` directory holds `proj.db` and `tests/test_hgrid.tif`;
/// `tests/gie` holds the init fixture; the `proj` directory gives the search
/// path the `/proj` check looks for on any checkout, whatever its name.
fn set_proj_data() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dirs = [
            root.join("data"),
            root.join("tests").join("gie"),
            std::env::temp_dir().join("proj"),
        ];
        let value = std::env::join_paths(dirs).expect("the paths hold no separator");
        std::env::set_var("PROJ_DATA", value);
    });
}

fn assert_near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} is not within {tolerance} of {expected}"
    );
}

#[test]
fn info_functions() {
    set_proj_data();

    // proj_info(): the version is xx.yy.zz when it is set, the release is named,
    // and the search path holds a `proj` directory.
    let info = library_info();
    if !info.version.is_empty() {
        assert_eq!(
            info.version,
            format!("{}.{}.{}", info.major, info.minor, info.patch)
        );
    }
    assert!(!info.release.is_empty());
    if std::env::var_os("HOME").is_some()
        || std::env::var_os("PROJ_LIB").is_some()
        || std::env::var_os("PROJ_DATA").is_some()
    {
        assert!(!info.searchpath.is_empty());
    }
    let proj_directory = format!("{}proj", std::path::MAIN_SEPARATOR);
    assert!(
        info.searchpath.contains(&proj_directory),
        "search path {}",
        info.searchpath
    );

    // proj_pj_info(): august has no inverse.
    let august = Proj::new("+proj=august").expect("august");
    assert!(!august.proj_info().has_inverse);

    // proj_pj_info(): the definition is the input shrunk by pj_shrink, which
    // drops the `+` and `;` and keeps one space between items.
    let utm = Proj::new("+proj=utm; +zone=32; +ellps=GRS80").expect("utm");
    let pj_info = utm.proj_info();
    assert!(pj_info.has_inverse);
    assert_eq!(
        pj_info.definition.as_deref(),
        Some("proj=utm zone=32 ellps=GRS80")
    );
    assert_eq!(pj_info.id.as_deref(), Some("utm"));

    // proj_grid_info(): a GeoTIFF grid of 4 by 4 nodes, one degree of pi / 180 cell
    // sizes, bounds in radians.
    let grid = grid_info("tests/test_hgrid.tif").expect("test_hgrid.tif is a grid");
    assert!(grid.filename.is_some());
    assert_eq!(grid.gridname, "tests/test_hgrid.tif");
    assert_eq!(grid.format, "gtiff");
    assert_eq!(grid.n_lon, 4);
    assert_eq!(grid.n_lat, 4);
    assert_near(grid.cs_lon, 0.017_453_292_519_943_295, 1e-15);
    assert_near(grid.cs_lat, 0.017_453_292_519_943_295, 1e-15);
    assert_near(grid.lower_left.lon, 0.069_813_170_079_773_18, 1e-15);
    assert_near(grid.lower_left.lat, 0.907_571_211_037_051_4, 1e-15);
    assert_near(grid.upper_right.lon, 0.122_173_047_639_603_07, 1e-15);
    assert_near(grid.upper_right.lat, 0.959_931_088_596_881_3, 1e-15);

    // A name that is not on the search path is not a grid.
    assert!(grid_info("nonexistinggrid").is_none());
    // The file exists, but it is not a grid.
    assert!(grid_info("proj.db").is_none());

    // proj_init_info(): an unknown init file has no metadata.
    assert!(init_info("unknowninit").is_none());
    // `epsg` is not a file here: its metadata comes from the database, and
    // the origin is EPSG (or Unknown, for a file without metadata).
    let epsg = init_info("epsg").expect("epsg metadata from the database");
    assert!(
        epsg.origin == "EPSG" || epsg.origin == "Unknown",
        "origin {}",
        epsg.origin
    );
    assert_eq!(epsg.name, "epsg");

    // proj_rtodms2() and proj_dmstor(). The C buffer is 40 bytes: max_len 39.
    let hemisphere = DmsSign::Hemisphere {
        positive: 'N',
        negative: 'S',
    };
    let format = DmsFormat::default();
    let buf = rtodms(PI, hemisphere, format, 39);
    assert_eq!(buf, "180dN");
    assert_eq!(dmstor(buf.as_bytes()), PI);

    assert_eq!(rtodms(-2.0, hemisphere, format, 39), "114d35'29.612\"S");

    // Buffer of just one byte: only the terminating nul fits.
    assert_eq!(rtodms(-2.0, hemisphere, format, 0), "");

    // Last character truncated: a 15-byte buffer holds 14 characters.
    assert_eq!(rtodms(-2.0, hemisphere, format, 14), "114d35'29.612\"");

    // Just enough for the string and its terminating nul: a 16-byte buffer.
    assert_eq!(rtodms(-2.0, hemisphere, format, 15), "114d35'29.612\"S");

    // Buffer of just one byte, with a minus sign instead of letters.
    assert_eq!(rtodms(-2.0, DmsSign::Minus, format, 0), "");

    // Last character truncated.
    assert_eq!(rtodms(-2.0, DmsSign::Minus, format, 14), "-114d35'29.612");

    // Just enough bytes for the string and its terminating nul.
    assert_eq!(rtodms(-2.0, DmsSign::Minus, format, 15), "-114d35'29.612\"");

    // The buffer is now "-114d35'29.612\"": it reads back as -2 within 1e-7.
    let buf = rtodms(-2.0, DmsSign::Minus, format, 15);
    assert_near(dmstor(buf.as_bytes()), -2.0, 1e-7);

    // UTF-8 degree sign on DMS input.
    assert_near(dmstor(b"19\xC2\xB046'27\"E"), 0.345_124_32, 1e-7);

    // ISO 8859-1 (cp1252) degree sign on DMS input: a raw byte 0xB0.
    assert_near(dmstor(b"19\xB046'27\"E"), 0.345_124_32, 1e-7);
}

#[test]
fn init_info_reads_the_metadata_section_of_a_file() {
    set_proj_data();

    // Values from PROJ 9.9.0 `proj_init_info("test_init_metadata")` on the
    // fixture tests/gie/test_init_metadata.
    let init = init_info("test_init_metadata").expect("fixture init file");
    assert!(init
        .filename
        .as_ref()
        .is_some_and(|path| path.ends_with("test_init_metadata")));
    assert_eq!(init.name, "test_init_metadata");
    assert_eq!(init.origin, "Test");
    assert_eq!(init.version, "1.2");
    assert_eq!(init.lastupdate, "2024-05-17");
}
