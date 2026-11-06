//! `cs2cs` between a longitude-first CRS (`OGC:CRS84`) and latitude-first ones.
//!
//! The expected values are the ones `cs2cs` of PROJ 9.9.0 prints for the same point (six decimals,
//! see `docs/PORTING-lonlat-source-axis.md` for the commands). The candidates of such a pair come
//! from the datum pivot through WGS 84: the first leg is the axis order change of the longitude-first
//! CRS, and it must stay in the pipeline (`coordinateoperationfactory.cpp:3480-3486`).

mod common;

use proj_rs::iso19111::{
    select_alternatives, BestOperation, BestPolicy, CandidateFilter, Selection,
};
use proj_rs::{Operation, Xyzt};

fn point(x: f64, y: f64) -> Xyzt {
    Xyzt {
        x,
        y,
        z: 0.0,
        t: 0.0,
    }
}

/// The point through the engine `cs2cs` builds for the pair: the candidates of `proj_trans`, or
/// the single pipeline.
fn forward(source: &str, target: &str, x: f64, y: f64) -> Xyzt {
    common::use_fixture_search_path();
    let selection = select_alternatives(source, target, BestPolicy::Warn, CandidateFilter::NONE)
        .unwrap_or_else(|error| panic!("{source} to {target}: {error}"));
    match selection {
        Selection::Several(alternatives) => alternatives
            .forward(point(x, y))
            .unwrap_or_else(|error| panic!("{source} to {target} at {x} {y}: {error:?}")),
        Selection::Single(BestOperation::Pipeline { definition, .. }) => {
            Operation::create(&definition)
                .expect("pipeline")
                .forward(point(x, y))
                .unwrap_or_else(|error| panic!("{source} to {target} at {x} {y}: {error:?}"))
        }
        Selection::Single(BestOperation::MissingGrid { message }) => {
            panic!("{source} to {target}: {message}")
        }
    }
}

fn assert_near(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: got {actual}, PROJ gives {expected}"
    );
}

/// `cs2cs OGC:CRS84 EPSG:4269`, `-122 45`: `45.000000 -122.000000`. The target is latitude first,
/// so the pipeline output is latitude then longitude.
#[test]
fn crs84_to_nad83_gives_latitude_first_output() {
    let coord = forward("OGC:CRS84", "EPSG:4269", -122.0, 45.0);
    assert_near(coord.x, 45.0, 1e-6, "latitude");
    assert_near(coord.y, -122.0, 1e-6, "longitude");
}

/// `cs2cs OGC:CRS84 EPSG:4267`, `-122 45`: `45.000155 -121.998809`, through the NAD27 grids PROJ
/// has, here the ballpark and the NAD27 to WGS 84 candidates.
#[test]
fn crs84_to_nad27_gives_latitude_first_output() {
    let coord = forward("OGC:CRS84", "EPSG:4267", -122.0, 45.0);
    assert_near(coord.x, 45.000155, 1e-6, "latitude");
    assert_near(coord.y, -121.998809, 1e-6, "longitude");
}

/// `cs2cs OGC:CRS84 EPSG:26710`, `-122 45`: `578911.275196 4983237.990370`. The projected target
/// is UTM 10N on NAD27, whose base is latitude first.
#[test]
fn crs84_to_nad27_utm10n_gives_the_projected_point() {
    let coord = forward("OGC:CRS84", "EPSG:26710", -122.0, 45.0);
    assert_near(coord.x, 578_911.275_196, 1e-5, "easting");
    assert_near(coord.y, 4_983_237.990_370, 1e-5, "northing");
}

/// `cs2cs OGC:CRS84 EPSG:2154`, `2.35 48.85`: `652301.564831 6861302.725900`.
#[test]
fn crs84_to_lambert93_gives_the_projected_point() {
    let coord = forward("OGC:CRS84", "EPSG:2154", 2.35, 48.85);
    assert_near(coord.x, 652_301.564_831, 1e-5, "easting");
    assert_near(coord.y, 6_861_302.725_900, 1e-5, "northing");
}

/// `cs2cs OGC:CRS84 EPSG:2154`, `-122 45`: `-5521727.411403 12731733.988284`.
#[test]
fn crs84_to_lambert93_far_from_france() {
    let coord = forward("OGC:CRS84", "EPSG:2154", -122.0, 45.0);
    assert_near(coord.x, -5_521_727.411_403, 1e-5, "easting");
    assert_near(coord.y, 12_731_733.988_284, 1e-5, "northing");
}

/// `cs2cs OGC:CRS84 EPSG:32631`, `-122 45`: `-3723038.118971 13321749.056464`. A WGS 84 target
/// needs no datum leg, so this pair already worked.
#[test]
fn crs84_to_utm31n_gives_the_projected_point() {
    let coord = forward("OGC:CRS84", "EPSG:32631", -122.0, 45.0);
    assert_near(coord.x, -3_723_038.118_971, 1e-5, "easting");
    assert_near(coord.y, 13_321_749.056_464, 1e-5, "northing");
}

/// `cs2cs EPSG:4269 OGC:CRS84`, `45 -122`: `-122.000000 45.000000`. The longitude-first target
/// takes the leg back to WGS 84.
#[test]
fn nad83_to_crs84_gives_longitude_first_output() {
    let coord = forward("EPSG:4269", "OGC:CRS84", 45.0, -122.0);
    assert_near(coord.x, -122.0, 1e-6, "longitude");
    assert_near(coord.y, 45.0, 1e-6, "latitude");
}

/// `cs2cs EPSG:26710 OGC:CRS84`, `578911.275196 4983237.990370`: `-122.000000 45.000000`.
#[test]
fn nad27_utm10n_to_crs84_gives_longitude_first_output() {
    let coord = forward(
        "EPSG:26710",
        "OGC:CRS84",
        578_911.275_196,
        4_983_237.990_370,
    );
    assert_near(coord.x, -122.0, 1e-6, "longitude");
    assert_near(coord.y, 45.0, 1e-6, "latitude");
}

/// `cs2cs EPSG:2154 OGC:CRS84`, `652301.564831 6861302.725900`: `2.350000 48.850000`.
#[test]
fn lambert93_to_crs84_gives_longitude_first_output() {
    let coord = forward("EPSG:2154", "OGC:CRS84", 652_301.564_831, 6_861_302.725_900);
    assert_near(coord.x, 2.35, 1e-6, "longitude");
    assert_near(coord.y, 48.85, 1e-6, "latitude");
}

/// `cs2cs EPSG:4326 EPSG:4269`, `45 -122`: the two are latitude first, so no axis change. Kept as
/// a guard for the pairs that already worked.
#[test]
fn latitude_first_pair_is_unchanged() {
    let coord = forward("EPSG:4326", "EPSG:4269", 45.0, -122.0);
    assert_near(coord.x, 45.0, 1e-6, "latitude");
    assert_near(coord.y, -122.0, 1e-6, "longitude");
}

/// `cs2cs EPSG:4326 EPSG:26710`, `45 -122`: `578911.275196 4983237.990370`, as before this lot.
#[test]
fn latitude_first_source_to_utm10n_is_unchanged() {
    let coord = forward("EPSG:4326", "EPSG:26710", 45.0, -122.0);
    assert_near(coord.x, 578_911.275_196, 1e-5, "easting");
    assert_near(coord.y, 4_983_237.990_370, 1e-5, "northing");
}
