//! `somerc` over a slice against `somerc` point by point (`docs/PORTING-batch-oblique.md`).

use crate::engine::batch_check::{
    assert_agrees, assert_vector_taken, geographic_points, projected_points, Way, METRES, RADIANS,
};
use crate::engine::Operation;

/// EPSG:2056 and 21781 (Bessel), a sphere, and an offset origin with units.
const DEFINITIONS: [&str; 4] = [
    "+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1 +x_0=2600000 \
     +y_0=1200000 +ellps=bessel",
    "+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1 +x_0=600000 \
     +y_0=200000 +ellps=bessel",
    "+proj=somerc +lat_0=47 +lon_0=8 +k_0=0.9996 +R=6371000",
    "+proj=somerc +lat_0=-20 +lon_0=170 +k_0=0.9 +x_0=10 +y_0=-20 +ellps=GRS80 +units=km",
];

/// The area of use, then the world: far from the origin the `asin` arguments and the Newton loop
/// leave the vector domain, and the group goes to the scalar.
const BOXES: [((f64, f64), (f64, f64)); 3] = [
    ((5.9, 10.5), (45.8, 47.9)),
    ((-180.0, 180.0), (-89.9, 89.9)),
    ((165.0, 175.0), (-25.0, -15.0)),
];

#[test]
fn both_directions_agree_with_one_by_one() {
    for definition in DEFINITIONS {
        let op = Operation::create(definition).expect(definition);
        let mut differing = 0;
        for (seed, (lon, lat)) in (1..).zip(BOXES) {
            let geographic = geographic_points(3_001, lon, lat, seed);
            differing += assert_agrees(&op, &geographic, Way::Forward, [METRES; 3]);
            let projected = projected_points(&op, &geographic, 5e6);
            differing += assert_agrees(&op, &projected, Way::Inverse, [RADIANS; 3]);
        }
        assert_vector_taken(definition, differing);
    }
}
