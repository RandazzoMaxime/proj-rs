//! `laea` over a slice against `laea` point by point (`docs/PORTING-batch-oblique.md`).

use crate::engine::batch_check::{
    assert_agrees, assert_vector_taken, geographic_points, projected_points, Way, METRES, RADIANS,
};
use crate::engine::Operation;

/// EPSG:3035, the equatorial mode, an origin in the south with units and an offset.
const VECTOR: [&str; 3] = [
    "+proj=laea +lat_0=52 +lon_0=10 +x_0=4321000 +y_0=3210000 +ellps=GRS80",
    "+proj=laea +lat_0=0 +lon_0=20 +ellps=WGS84 +x_0=100 +y_0=-200",
    "+proj=laea +lat_0=-30 +lon_0=-60 +ellps=bessel +units=km",
];

/// The polar modes, a sphere and a flat ellipsoid keep the scalar loop.
const SCALAR: [&str; 4] = [
    "+proj=laea +lat_0=90 +lon_0=0 +ellps=WGS84",
    "+proj=laea +lat_0=-90 +lon_0=0 +ellps=WGS84",
    "+proj=laea +lat_0=40 +lon_0=0 +R=6371000",
    "+proj=laea +lat_0=40 +lon_0=0 +a=6378137 +rf=50",
];

/// The area of use, then the hemisphere around the origin, then the world.
const BOXES: [((f64, f64), (f64, f64)); 3] = [
    ((-10.0, 35.0), (35.0, 72.0)),
    ((-100.0, 100.0), (-80.0, 80.0)),
    ((-180.0, 180.0), (-89.9, 89.9)),
];

fn check(definition: &str) -> usize {
    let op = Operation::create(definition).expect(definition);
    let mut differing = 0;
    for (seed, (lon, lat)) in (1..).zip(BOXES) {
        let geographic = geographic_points(3_001, lon, lat, seed);
        differing += assert_agrees(&op, &geographic, Way::Forward, [METRES; 3]);
        let projected = projected_points(&op, &geographic, 5e6);
        differing += assert_agrees(&op, &projected, Way::Inverse, [RADIANS; 3]);
    }
    differing
}

#[test]
fn the_oblique_and_equatorial_modes_agree_with_one_by_one() {
    for definition in VECTOR {
        assert_vector_taken(definition, check(definition));
    }
}

#[test]
fn the_other_modes_are_the_scalar_loop() {
    for definition in SCALAR {
        assert_eq!(check(definition), 0, "{definition}: bit for bit");
    }
}
