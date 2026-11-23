//! `sterea` over a slice against `sterea` point by point (`docs/PORTING-batch-oblique.md`).

use crate::engine::batch_check::{
    assert_agrees, assert_vector_taken, geographic_points, projected_points, Tolerance, Way,
    METRES, RADIANS,
};
use crate::engine::Operation;

/// EPSG:28992 (Bessel), a sphere, an offset origin with units, and a southern origin.
const DEFINITIONS: [&str; 4] = [
    "+proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 +k=0.9999079 +x_0=155000 \
     +y_0=463000 +ellps=bessel",
    "+proj=sterea +lat_0=46 +lon_0=25 +k=0.99975 +x_0=500000 +y_0=500000 +R=6371000",
    "+proj=sterea +lat_0=60 +lon_0=-100 +k=1 +x_0=10 +y_0=-20 +ellps=GRS80 +units=km \
     +lon_wrap=180",
    "+proj=sterea +lat_0=-33.9 +lon_0=151.2 +k=0.9996 +ellps=WGS84",
];

/// The area of use, then the world: far from the origin the Newton loop and the `asin` argument
/// leave the vector domain, and the group goes to the scalar.
const BOXES: [((f64, f64), (f64, f64)); 3] = [
    ((3.0, 8.0), (50.0, 55.0)),
    ((-180.0, 180.0), (-89.9, 89.9)),
    ((140.0, 160.0), (-45.0, -25.0)),
];

/// Thousands of kilometres out the problem is ill-conditioned: the `sin`/`atan` of the platform C
/// library move the scalar result by about 1e-11 of its size (measured on Apple silicon).
const WORLD_METRES: Tolerance = Tolerance {
    floor: 1e-6,
    relative: 2e-11,
};

#[test]
fn both_directions_agree_with_one_by_one() {
    for definition in DEFINITIONS {
        let op = Operation::create(definition).expect(definition);
        let mut differing = 0;
        for (seed, (lon, lat)) in (1..).zip(BOXES) {
            let geographic = geographic_points(3_001, lon, lat, seed);
            let metres = if lon.1 - lon.0 > 100.0 {
                WORLD_METRES
            } else {
                METRES
            };
            differing += assert_agrees(&op, &geographic, Way::Forward, [metres; 3]);
            let projected = projected_points(&op, &geographic, 5e6);
            differing += assert_agrees(&op, &projected, Way::Inverse, [RADIANS; 3]);
        }
        assert_vector_taken(definition, differing);
    }
}
