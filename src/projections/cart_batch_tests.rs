//! `cart` over a slice against `cart` point by point (`docs/PORTING-batch-datum.md`).

use super::Coord;
use crate::engine::batch_check::{
    assert_agrees, assert_vector_taken, unit_source, Way, METRES, RADIANS,
};
use crate::engine::Operation;
use std::f64::consts::{FRAC_PI_2, PI};

/// The frame offsets, a sphere, units, `+over`, a flat and a large ellipsoid.
const DEFINITIONS: [&str; 7] = [
    "+proj=cart +ellps=GRS80",
    "+proj=cart +ellps=WGS84 +lon_0=-170 +pm=paris",
    "+proj=cart +R=6371000",
    "+proj=cart +ellps=intl +units=km",
    "+proj=cart +ellps=GRS80 +over",
    "+proj=cart +a=6378137 +rf=50",
    "+proj=cart +ellps=bessel +geoc",
];

fn at(x: f64, y: f64, z: f64) -> Coord {
    Coord { x, y, z, t: 2020.5 }
}

/// Radians, with the poles, the edges of the longitude, refused latitudes and non-finite values.
fn geographic() -> Vec<Coord> {
    let (mut unit, mut height) = (unit_source(11), unit_source(12));
    let mut points: Vec<Coord> = (0..3_001)
        .map(|_| {
            at(
                (unit() - 0.5) * 2.0 * PI,
                (unit() - 0.5) * PI,
                (height() - 0.2) * 1e5,
            )
        })
        .collect();
    for lat in [
        FRAC_PI_2,
        -FRAC_PI_2,
        FRAC_PI_2 - 1e-9,
        FRAC_PI_2 + 1e-11,
        FRAC_PI_2 + 1e-5,
        -FRAC_PI_2 - 1e-5,
        0.0,
        1e300,
        f64::NAN,
        f64::INFINITY,
    ] {
        for lon in [
            0.0,
            PI,
            -PI,
            PI + 1e-9,
            4.0,
            9.9,
            10.5,
            f64::NAN,
            f64::NEG_INFINITY,
        ] {
            points.push(at(lon, lat, 0.0));
        }
    }
    points.extend([
        at(0.3, 0.5, f64::NAN),
        at(0.3, 0.5, f64::INFINITY),
        at(0.3, 0.5, -6.5e6),
        at(0.3, 0.5, 1e10),
    ]);
    points
}

/// Metres: the surface, depth, altitude, the axis, the centre, the equator, non-finite values.
fn geocentric() -> Vec<Coord> {
    let (mut unit, mut height) = (unit_source(21), unit_source(22));
    let mut points: Vec<Coord> = (0..3_001)
        .map(|_| {
            let (lon, lat) = ((unit() - 0.5) * 2.0 * PI, (unit() - 0.5) * PI);
            let radius = 6_378_137.0 + (height() - 0.5) * 2e5;
            at(
                radius * lat.cos() * lon.cos(),
                radius * lat.cos() * lon.sin(),
                radius * lat.sin(),
            )
        })
        .collect();
    points.extend([
        at(0.0, 0.0, 6.356e6),
        at(0.0, 0.0, -6.356e6),
        at(1e-3, 0.0, 6.356e6),
        at(1.0, 1.0, 6.3e6),
        at(0.0, 0.0, 0.0),
        at(6.378e6, 0.0, 0.0),
        at(-6.378e6, 0.0, 0.0),
        at(0.0, 6.378e6, 0.0),
        at(1e-9, 0.0, 1e-9),
        at(1e3, 2e3, 3e3),
        at(1e12, 1e12, 1e12),
        at(f64::NAN, 0.0, 6e6),
        at(6e6, f64::INFINITY, 1.0),
        at(6e6, 1.0, f64::NEG_INFINITY),
    ]);
    points
}

#[test]
fn forward_batch_agrees_with_one_by_one() {
    let points = geographic();
    for definition in DEFINITIONS {
        let op = Operation::create(definition).expect(definition);
        let differing = assert_agrees(&op, &points, Way::Forward, [METRES; 3]);
        assert_vector_taken(definition, differing);
    }
}

#[test]
fn inverse_batch_agrees_with_one_by_one() {
    let points = geocentric();
    for definition in DEFINITIONS {
        let op = Operation::create(definition).expect(definition);
        let differing = assert_agrees(&op, &points, Way::Inverse, [RADIANS, RADIANS, METRES]);
        assert_vector_taken(definition, differing);
    }
}

/// A failed slot is not handed to the vector code: its neighbours keep their vector values.
#[test]
fn a_failed_point_does_not_change_its_group() {
    let op = Operation::create(DEFINITIONS[0]).expect("cart");
    let mut points: Vec<Coord> = geographic().into_iter().take(16).collect();
    let want = points.clone();
    points[5] = at(0.3, 2.0, 0.0);
    let mut failed = vec![None; points.len()];
    op.forward_batch(&mut points, &mut failed);
    assert!(failed[5].is_some());
    for (slot, point) in want.iter().enumerate().filter(|(slot, _)| *slot != 5) {
        let one = op.forward(*point).expect("in domain");
        assert!((points[slot].x - one.x).abs() < 1e-6, "slot {slot}");
    }
}
