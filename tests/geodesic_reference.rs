//! The geodesics against what the `invgeod` and `geod` of PROJ 9.9.0 print
//! (`tools/gen-geodesic-fixtures.py`, `tests/geodesic/*.txt`): 690 inverse and 600 direct
//! problems on six ellipsoids, with the hard cases (antipodal and nearly so, meridians, the
//! equator, the poles, coincident and very close points, long lines).
//!
//! `geod` prints the back azimuth at the second point, the direction of the first one seen from
//! there; the library gives the forward azimuth at arrival, half a turn away.
//!
//! The expected values are PROJ's. The tolerance is the parity rule of
//! `docs/PARITY.md` (`common::tolerance`): the text PROJ prints is the limit, not the port.

mod common;

use common::tolerance::{DEGREES, METRES};
use proj_rs::geod::Geod;

/// Semi-major axis and flattening of the ellipsoids of the fixtures, as `geod` reads them.
fn ellipsoid(name: &str) -> (f64, f64) {
    match name {
        "wgs84" => (6_378_137.0, 1.0 / 298.257_223_563),
        "grs80" => (6_378_137.0, 1.0 / 298.257_222_101),
        "bessel" => (6_377_397.155, 1.0 / 299.152_812_8),
        "sphere" => (6_371_000.0, 0.0),
        "flat20" => (6_378_137.0, 0.05),
        "flat300" => (6_378_137.0, 0.003_352_810_664_747_480_5),
        other => panic!("unknown ellipsoid {other}"),
    }
}

/// The gap of two angles in degrees, modulo a turn: `-180` and `180` are the same direction.
fn angle_gap(x: f64, y: f64) -> f64 {
    ((x - y + 180.0).rem_euclid(360.0) - 180.0).abs()
}

/// An angle as the tools of PROJ hand it to the geodesic: read in degrees, held in radians
/// (`DEG_TO_RAD`), given back in degrees. The round trip moves the last bit, which a line of a
/// few metres turns into 5e-10 degree of azimuth.
fn as_geod_reads(degrees: f64) -> f64 {
    const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;
    (degrees * DEG_TO_RAD) / DEG_TO_RAD
}

/// The tolerance on an azimuth of a line of `distance` metres. The references come from PROJ on
/// glibc; another C math library (Windows, macOS) differs in the last bit of `sin`, `cos`,
/// `atan2`, a few `1e-9` m of position at the scale of the Earth, and a short line turns that
/// into an angle: `4e-9 m / distance`, in degrees, doubled. For a line of 100 m this is 5e-9
/// degree, for a line of 1 000 km 5e-16 and the parity rule alone applies.
fn azimuth_tolerance(distance: f64, a: f64, f: f64) -> f64 {
    // Near the antipode the azimuth is ill-conditioned whatever the length: Karney's hard cases,
    // 7e-10 degree measured on Windows and 1.4e-9 on macOS against the glibc text. The distance
    // stays exact to the parity rule.
    let near_antipodal = distance > 0.985 * std::f64::consts::PI * a * (1.0 - f);
    let antipodal = if near_antipodal { 1e-7 } else { 0.0 };
    DEGREES + 5e-7 / distance.max(1.0) + antipodal
}

fn numbers(line: &str) -> (&str, Vec<f64>) {
    let mut words = line.split_whitespace();
    let name = words.next().expect("an ellipsoid name");
    (name, words.map(|w| w.parse().expect("a number")).collect())
}

#[test]
fn inverse_matches_proj() {
    let mut worst: f64 = 0.0;
    let mut failures = Vec::new();
    for line in include_str!("geodesic/inverse.txt").lines() {
        let (name, v) = numbers(line);
        let (a, f) = ellipsoid(name);
        let [lat1, lon1, lat2, lon2] = [v[0], v[1], v[2], v[3]].map(as_geod_reads);
        let got = Geod::new(a, f).inverse(lat1, lon1, lat2, lon2);
        let (azimuth1, azimuth2, distance) = (v[4], v[5], v[6]);
        // A distance of 20 000 km is printed to the millimetre.
        if (got.distance - distance).abs() > METRES {
            failures.push(format!(
                "{line}: distance {} gap {:e}",
                got.distance,
                got.distance - distance
            ));
        }
        for (got, want, what) in [
            (got.azimuth1, azimuth1, "azi1"),
            (got.azimuth2 + 180.0, azimuth2, "azi2"),
        ] {
            let gap = angle_gap(got, want);
            if gap > azimuth_tolerance(distance, a, f) {
                failures.push(format!("{line}: {what} {got} against {want}, gap {gap:e}"));
            }
            worst = worst.max(gap);
        }
    }
    eprintln!("inverse: largest azimuth gap {worst:e}");
    // Every failure at once: a platform whose math library differs shows all its cases in one run.
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn direct_matches_proj() {
    let mut worst: f64 = 0.0;
    for line in include_str!("geodesic/direct.txt").lines() {
        let (name, v) = numbers(line);
        let (a, f) = ellipsoid(name);
        let [lat1, lon1, azimuth1] = [v[0], v[1], v[2]].map(as_geod_reads);
        let (lat, lon, azimuth) = Geod::new(a, f).line(lat1, lon1, azimuth1).position(v[3]);
        let (lat2, lon2, azimuth2) = (v[4], v[5], v[6]);
        for (got, want, what) in [
            (lat, lat2, "lat2"),
            (lon, lon2, "lon2"),
            (azimuth + 180.0, azimuth2, "azi2"),
        ] {
            assert!(
                angle_gap(got, want) <= DEGREES,
                "{line}: {what} {got} against {want}"
            );
            worst = worst.max(angle_gap(got, want));
        }
    }
    eprintln!("direct: largest gap {worst:e}");
}

#[test]
fn the_round_trip_closes() {
    // Going out by `direct` and coming back by `inverse` gives the distance and the azimuth.
    for line in include_str!("geodesic/direct.txt").lines() {
        let (name, v) = numbers(line);
        let (a, f) = ellipsoid(name);
        let earth = Geod::new(a, f);
        let (lat2, lon2, _) = earth.line(v[0], v[1], v[2]).position(v[3]);
        let back = earth.inverse(v[0], v[1], lat2, lon2);
        // Half the equator is the longest a geodesic stays the shortest path.
        if v[3] < 0.45 * 2.0 * std::f64::consts::PI * a {
            assert!(
                (back.distance - v[3]).abs() <= 1e-5,
                "{line}: back {} for {}",
                back.distance,
                v[3]
            );
        }
    }
}
