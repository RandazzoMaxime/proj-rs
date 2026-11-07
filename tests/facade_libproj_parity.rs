//! The georust facade against libproj: `proj_create_crs_to_crs`, then
//! `proj_normalize_for_visualization`, then `proj_trans`, as georust `proj` 0.31 calls them.
//!
//! The expected values are PROJ 9.9.0's. They were read from a C program that makes those three
//! calls against the PROJ 9.9.0 shared library (`scratchpad/facade-parity/oracle/georust_oracle.c`),
//! with the `proj.db` of that build and no grid. The facade points are longitude then latitude,
//! or easting then northing, in the CRS units, as georust gives them.

use proj_rs::Proj;

fn metres(got: f64, want: f64) -> bool {
    (got - want).abs() < 1e-6
}

#[test]
fn polar_stereographic_3413_keeps_easting_first() {
    // EPSG:3413 writes easting first on axes that point south. libproj's
    // `mustAxisOrderBeSwitchedForVisualization` switches a polar CRS only along the meridians it
    // names (`crs.cpp:869-947`), so the output keeps the pipeline's order: easting, then northing.
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:3413", None).expect("pair");
    let (east, north, _) = proj.convert_xyz(-45.0, 75.0, 0.0).expect("forward");
    assert!(metres(east, 0.0), "{east}");
    assert!(metres(north, -1_633_879.497_470_785), "{north}");
}

/// Without a grid, `EPSG:4326` to `EPSG:4267` has several candidates, and the facade chooses one
/// per point as `proj_trans` does. The values are PROJ 9.9.0's (georust path, no grid).
#[test]
fn wgs84_to_nad27_without_grid_chooses_per_point() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:4267", None).expect("pair");
    let cases: [((f64, f64), (f64, f64)); 3] = [
        ((-74.0, 40.0), (-74.000_426_385_734, 39.999_991_517_645)),
        ((-100.0, 40.0), (-99.999_584_426_821, 40.000_003_116_098)),
        ((-122.3, 47.6), (-122.298_780_307_473, 47.600_206_092_212)),
    ];
    for (input, want) in cases {
        let (east, north) = proj.convert(input).expect("forward");
        assert!((east - want.0).abs() < 1e-9, "{input:?}: {east}");
        assert!((north - want.1).abs() < 1e-9, "{input:?}: {north}");
    }
}

/// The inverse of the same pair, per point: PROJ 9.9.0's values, which return the input to about
/// 1e-9 degrees.
#[test]
fn nad27_to_wgs84_inverse_without_grid_chooses_per_point() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:4267", None).expect("pair");
    let cases: [((f64, f64), (f64, f64)); 3] = [
        (
            (-74.000_426_385_734, 39.999_991_517_645),
            (-73.999_999_997_658, 40.000_000_000_047),
        ),
        (
            (-99.999_584_426_821, 40.000_003_116_098),
            (-100.000_000_002_284, 39.999_999_999_983),
        ),
        (
            (-122.298_780_307_473, 47.600_206_092_212),
            (-122.300_000_003_561, 47.599_999_999_397),
        ),
    ];
    for (input, want) in cases {
        let (lon, lat) = proj.project(input, true).expect("inverse");
        assert!((lon - want.0).abs() < 1e-9, "{input:?}: {lon}");
        assert!((lat - want.1).abs() < 1e-9, "{input:?}: {lat}");
    }
}

/// A pair whose only candidate is a ballpark offset is still a transformation: libproj returns
/// that operation at once, and `EPSG:4326` to `EPSG:4610` (Xian 1980) is the zero offset.
#[test]
fn a_ballpark_only_pair_is_a_transformation() {
    let xian = Proj::new_known_crs("EPSG:4326", "EPSG:4610", None).expect("ballpark only");
    let (lon, lat) = xian.convert((116.4_f64, 39.9_f64)).expect("forward");
    assert!((lon - 116.4).abs() < 1e-9, "{lon}");
    assert!((lat - 39.9).abs() < 1e-9, "{lat}");
    // EPSG:2393 (KKJ / Finland): the ballpark is a geocentric translation and a Transverse Mercator.
    let kkj = Proj::new_known_crs("EPSG:4326", "EPSG:2393", None).expect("ballpark only");
    let (east, north) = kkj.convert((24.0_f64, 65.0_f64)).expect("forward");
    assert!((east - 3_358_686.366_799_975).abs() < 1e-6, "{east}");
    assert!((north - 7_214_830.355_803_429).abs() < 1e-6, "{north}");
}

/// A vertical grid named with `@` is optional: libproj builds the pipeline without it, and a
/// missing optional grid is an identity step (`isPROJInstantiable` skips it,
/// `singleoperation.cpp:347-354`). The CRS is the one `cs2cs` reads as `+geoidgrids=@...`.
#[test]
fn an_optional_missing_geoid_grid_is_an_identity_step() {
    let source = "+proj=longlat +datum=WGS84 +units=m +geoidgrids=@i_dont_exist.tif +vunits=m +no_defs +type=crs";
    let proj = Proj::new_known_crs(source, "EPSG:4979", None).expect("optional grid");
    let (lon, lat, height) = proj.convert_xyz(2.0, 49.0, 0.0).expect("forward");
    assert!(metres(lon, 2.0) && metres(lat, 49.0), "{lon} {lat}");
    assert!(metres(height, 0.0), "{height}");
}

#[test]
fn ups_north_32661_is_switched_for_visualization() {
    // EPSG:32661 writes north first along the 180 and 90 meridians, so it is switched.
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32661", None).expect("pair");
    let (east, north, _) = proj.convert_xyz(-45.0, 75.0, 0.0).expect("forward");
    assert!(metres(east, 815_914.202_587_687), "{east}");
    assert!(metres(north, 815_914.202_587_687), "{north}");
}
