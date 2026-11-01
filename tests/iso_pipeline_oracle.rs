//! The operation the ISO 19111 selection picks, written as a PROJ pipeline, against
//! PROJ 9.9.0 values.
//!
//! Each case is `best_pipeline(from, to)`, built with `Operation::create`, run
//! forward on one point. The point is in the facade's order: longitude then
//! latitude for a geographic CRS, easting then northing for a projected one, in
//! the CRS units. The pipeline takes the source CRS axis order, so `EPSG:4326`
//! is fed latitude first. The output is read back in the facade's order too.
//! Tolerance 1e-3 (metres, or feet for a US foot CRS).
//!
//! The expected values are PROJ's and are never edited to match the code. A
//! case with `#[ignore]` names its gap in the reason string.

use proj_rs::iso19111::best_pipeline;
use proj_rs::{Operation, Xyzt};

const TOLERANCE: f64 = 1e-3;

/// One point, `input` and `expected` in the facade's order (see the module doc).
fn assert_matches_proj(from: &str, to: &str, input: (f64, f64), expected: (f64, f64)) {
    let (definition, lat_or_northing_first) =
        best_pipeline(from, to).unwrap_or_else(|error| panic!("{from} -> {to}: {error}"));
    let operation =
        Operation::create(&definition).unwrap_or_else(|error| panic!("{from} -> {to}: {error}"));
    // The pipeline takes the source axis order: latitude first for EPSG:4326.
    let source_input = if from == "EPSG:4326" {
        Xyzt::xy(input.1, input.0)
    } else {
        Xyzt::xy(input.0, input.1)
    };
    let out = operation
        .forward(source_input)
        .unwrap_or_else(|error| panic!("{from} -> {to} at {input:?}: {error}"));
    // A northing-first target writes northing first: read it back as easting, northing.
    let (east, north) = if lat_or_northing_first {
        (out.y, out.x)
    } else {
        (out.x, out.y)
    };
    assert!(
        (east - expected.0).abs() <= TOLERANCE && (north - expected.1).abs() <= TOLERANCE,
        "{from} -> {to} at {input:?}: got ({east}, {north}), PROJ ({}, {}) (definition {definition})",
        expected.0,
        expected.1
    );
}

#[test]
fn wgs84_to_rgf93_lambert93_eiffel() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:2154",
        (2.2945, 48.8582),
        (648_237.102_854_4, 6_862_249.443_404_44),
    );
}

#[test]
fn wgs84_to_rgf93_lambert93_paris_centre() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:2154",
        (2.35, 48.85),
        (652_301.564_830_53, 6_861_302.725_899_58),
    );
}

#[test]
fn wgs84_to_rgf93_lambert93_brittany() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:2154",
        (-1.55, 47.2),
        (355_732.089_656_04, 6_687_666.973_004_26),
    );
}

#[test]
fn wgs84_to_rgf93_lambert93_marseille() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:2154",
        (5.37, 43.3),
        (892_394.777_419_69, 6_247_424.645_813_7),
    );
}

#[test]
fn wgs84_to_conus_albers_atlanta() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:5070",
        (-84.4, 33.7),
        (1_065_090.926_197_4, 1_246_540.246_130_59),
    );
}

#[test]
fn wgs84_to_conus_albers_denver() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:5070",
        (-100.0, 40.0),
        (-338_390.587_550_87, 1_894_100.140_042_56),
    );
}

#[test]
fn wgs84_to_conus_albers_seattle() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:5070",
        (-122.3, 47.6),
        (-1_965_840.775_865_01, 3_007_788.156_319_79),
    );
}

#[test]
fn wgs84_to_nad83_california_zone6_us_feet() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:2230",
        (-115.797_615, 37.264_797_8),
        (6_693_625.672_174_75, 3_497_301.591_802_72),
    );
}

#[test]
fn wgs84_to_etrs89_laea_europe_northing_first() {
    assert_matches_proj(
        "EPSG:4326",
        "EPSG:3035",
        (2.2945, 48.8582),
        (3_756_581.008_814_72, 2_890_102.602_313_5),
    );
}

// The chain is the identity for RGF93 to WGS 84 (EPSG:1671, all parameters zero), so PROJ's
// value is the one the identity gives. The first value the brief gave for this pair was the
// old pipeline output; the CI log has PROJ's value after `vs PROJ`, and it is used here.
#[test]
fn rgf93_lambert93_to_utm31n() {
    assert_matches_proj(
        "EPSG:2154",
        "EPSG:32631",
        (652_000.0, 6_862_000.0),
        (452_013.623_058_75, 5_411_682.073_123_35),
    );
    assert_matches_proj(
        "EPSG:2154",
        "EPSG:32631",
        (700_000.0, 6_600_000.0),
        (500_000.0, 5_149_603.361_656_04),
    );
}

#[test]
fn nad83_california_zone6_us_feet_to_utm46n() {
    assert_matches_proj(
        "EPSG:2230",
        "EPSG:26946",
        (4_760_096.421_921, 3_744_293.729_449),
        (1_450_880.291_060_5, 1_141_263.011_160_5),
    );
}
