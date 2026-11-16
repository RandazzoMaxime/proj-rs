//! GeoTIFF grids compressed with LZW or PackBits, read from a local file (no callbacks): the
//! whole-file path, as `proj=vgridshift grids=<file>` reads them.
//!
//! The fixtures are written by `tools/make-lzw-grid.py` (see its docstring). The values are
//! libproj 9.9.0 linked to libtiff (`proj-reference-net`): `cct` with `+proj=vgridshift
//! +grids=<file> +multiplier=1` and `-d 17`. The LZW grids have the samples of the Deflate grid
//! `planar_deflate_predictor3*.tif`, so their values are that grid's.
//!
//! libtiff applies the floating point predictor to LZW and Deflate chunks only: with PackBits the
//! predictor tag is read and ignored, so the decoded bytes are the samples (measured: a grid with
//! the predictor gives 0.5760 at (21.3, 45.7), the same bytes with PackBits give 4.76e-35).

mod common;

use proj_rs::ProjBuilder;

/// The vertical offset of `grid` at `(lon, lat)` in degrees, as `vgridshift` reads it.
fn vertical_offset(grid: &str, lon: f64, lat: f64) -> f64 {
    common::use_fixture_search_path();
    let builder = ProjBuilder::new();
    let proj = builder
        .proj(&format!("proj=vgridshift grids={grid} multiplier=1"))
        .expect("the grid is opened");
    let (_, _, z) = proj
        .convert_xyz(lon.to_radians(), lat.to_radians(), 0.0)
        .expect("the point is in the grid");
    z
}

#[track_caller]
fn assert_value(actual: f64, expected: f64, at: &str) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "{at}: {actual}, libproj 9.9.0 gives {expected}"
    );
}

#[test]
fn an_lzw_strip_grid_is_read_whole_from_a_local_file() {
    let grid = "lzw_predictor3_planar.tif";
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        0.866_195_723_414_421_1,
        "(20, 50)",
    );
    assert_value(
        vertical_offset(grid, 41.5, 37.2),
        0.699_234_727_025_032_3,
        "(41.5, 37.2)",
    );
}

#[test]
fn an_lzw_tiled_grid_is_read_whole_from_a_local_file() {
    let grid = "lzw_predictor3_planar_tiled.tif";
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        0.866_195_723_414_421_1,
        "(20, 50)",
    );
    assert_value(
        vertical_offset(grid, 34.625, 42.875),
        0.880_941_569_805_145_5,
        "(34.625, 42.875)",
    );
}

#[test]
fn a_big_endian_lzw_strip_grid_is_read_whole_from_a_local_file() {
    let grid = "lzw_be_predictor3_planar.tif";
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        0.866_195_723_414_421_1,
        "(20, 50)",
    );
    assert_value(
        vertical_offset(grid, 41.5, 37.2),
        0.699_234_727_025_032_3,
        "(41.5, 37.2)",
    );
}

#[test]
fn a_big_endian_lzw_tiled_grid_is_read_whole_from_a_local_file() {
    let grid = "lzw_be_predictor3_planar_tiled.tif";
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        0.866_195_723_414_421_1,
        "(20, 50)",
    );
    assert_value(
        vertical_offset(grid, 34.625, 42.875),
        0.880_941_569_805_145_5,
        "(34.625, 42.875)",
    );
}

#[test]
fn a_packbits_grid_without_predictor_is_read_whole_from_a_local_file() {
    let grid = "packbits_planar_tiled.tif";
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        0.866_195_723_414_421_1,
        "(20, 50)",
    );
    assert_value(
        vertical_offset(grid, 21.3, 45.7),
        0.576_007_464_528_083_8,
        "(21.3, 45.7)",
    );
}

/// The predictor tag is ignored with PackBits, as libtiff ignores it: the samples are the decoded
/// bytes, which are the predictor's planes.
#[test]
fn a_packbits_grid_ignores_its_predictor_as_libtiff_does() {
    let grid = "packbits_predictor3_planar_tiled.tif";
    assert_value(
        vertical_offset(grid, 21.3, 45.7),
        4.759_953_529_565_562e-35,
        "(21.3, 45.7)",
    );
    assert_value(
        vertical_offset(grid, 20.0, 50.0),
        6.136_462_936_853_599e-36,
        "(20, 50)",
    );
}

/// libtiff refuses to decode JPEG (`Compression` 7) and CCITT (2) samples, but PROJ opens the grid
/// and its values are `inf`, with no error: `cct` prints `inf` and exits 0. The fixtures are the
/// Deflate grid with its `Compression` tag patched (`tools/patch-compression-tag.py`).
#[test]
fn a_jpeg_or_ccitt_grid_opens_and_its_values_are_infinite() {
    for grid in [
        "planar_deflate_predictor3_jpeg.tif",
        "planar_deflate_predictor3_ccitt.tif",
    ] {
        assert_eq!(vertical_offset(grid, 20.0, 50.0), f64::INFINITY, "{grid}");
        assert_eq!(vertical_offset(grid, 41.5, 37.2), f64::INFINITY, "{grid}");
    }
}
