//! Test support: a step over a slice must agree with the same step point by point.

use crate::engine::{Coord, Operation};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Way {
    Forward,
    Inverse,
}

/// The tolerance of `docs/PARITY.md` for one axis: `floor` absolute, `relative` of the value.
#[derive(Clone, Copy)]
pub(crate) struct Tolerance {
    pub(crate) floor: f64,
    pub(crate) relative: f64,
}

pub(crate) const METRES: Tolerance = Tolerance {
    floor: 1e-6,
    relative: 1e-13,
};
pub(crate) const RADIANS: Tolerance = Tolerance {
    floor: 1.75e-13,
    relative: 0.0,
};

impl Tolerance {
    fn holds(self, got: f64, want: f64) -> bool {
        if got.is_nan() || want.is_nan() {
            return got.is_nan() && want.is_nan();
        }
        if got.is_infinite() || want.is_infinite() {
            return got == want;
        }
        (got - want).abs() <= self.floor.max(self.relative * want.abs())
    }
}

/// A generator of the same numbers every run.
pub(crate) fn unit_source(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the top 53 bits of the generator are exact in f64"
        )]
        let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    }
}

/// `batch` and `one by one` must agree on the three axes (per axis tolerance) and keep the
/// time; a point the step refuses must be the same slot, with the same error. Returns how many
/// outputs differ bit for bit from the one-by-one value: zero means the vector path never ran.
pub(crate) fn assert_agrees(
    op: &Operation,
    points: &[Coord],
    way: Way,
    tolerance: [Tolerance; 3],
) -> usize {
    let mut batch = points.to_vec();
    let mut failed = vec![None; points.len()];
    match way {
        Way::Forward => op.forward_batch(&mut batch, &mut failed),
        Way::Inverse => op.inverse_batch(&mut batch, &mut failed),
    }
    let mut differing = 0;
    for (slot, point) in points.iter().enumerate() {
        let one = match way {
            Way::Forward => op.forward(*point),
            Way::Inverse => op.inverse(*point),
        };
        match one {
            Ok(want) => {
                assert_eq!(
                    failed[slot], None,
                    "slot {slot} {point:?}: batch fails alone"
                );
                let got = batch[slot];
                let agree = tolerance[0].holds(got.x, want.x)
                    && tolerance[1].holds(got.y, want.y)
                    && tolerance[2].holds(got.z, want.z);
                assert!(
                    agree,
                    "slot {slot} {point:?}: batch {got:?}, one by one {want:?}"
                );
                assert_eq!(got.t.to_bits(), want.t.to_bits(), "slot {slot}: time");
                if [got.x, got.y, got.z]
                    .iter()
                    .zip([want.x, want.y, want.z])
                    .any(|(g, w)| g.to_bits() != w.to_bits())
                {
                    differing += 1;
                }
            }
            Err(error) => {
                assert_eq!(failed[slot], Some(error), "slot {slot} {point:?}");
                assert_eq!(
                    batch[slot].x.to_bits(),
                    point.x.to_bits(),
                    "slot {slot}: kept"
                );
            }
        }
    }
    differing
}

/// With a vector path, some values must differ from the one-by-one ones at the last bit.
pub(crate) fn assert_vector_taken(label: &str, differing: usize) {
    if crate::simd::has_lanes() {
        assert!(
            differing > 0,
            "{label}: no value differs bit for bit, the vector path was not taken"
        );
    }
}

/// Geographic points (radians) in a box in degrees, plus the box corners, the poles, the
/// antimeridian, refused latitudes and non-finite values.
pub(crate) fn geographic_points(
    count: usize,
    lon: (f64, f64),
    lat: (f64, f64),
    seed: u64,
) -> Vec<Coord> {
    let mut unit = unit_source(seed);
    let at = |lon: f64, lat: f64| Coord {
        x: lon,
        y: lat,
        z: 0.0,
        t: 0.0,
    };
    let mut points: Vec<Coord> = (0..count)
        .map(|_| {
            at(
                (lon.0 + unit() * (lon.1 - lon.0)).to_radians(),
                (lat.0 + unit() * (lat.1 - lat.0)).to_radians(),
            )
        })
        .collect();
    let half_pi = std::f64::consts::FRAC_PI_2;
    for (x, y) in [
        (lon.0.to_radians(), lat.0.to_radians()),
        (lon.1.to_radians(), lat.1.to_radians()),
        (0.0, 0.0),
        (std::f64::consts::PI, 0.3),
        (-std::f64::consts::PI, 0.3),
        (0.3, half_pi),
        (0.3, -half_pi),
        (0.3, half_pi + 1e-5),
        (0.3, half_pi + 1e-11),
        (f64::NAN, 0.3),
        (0.3, f64::NAN),
        (f64::INFINITY, 0.3),
        (0.3, f64::NEG_INFINITY),
    ] {
        points.push(at(x, y));
    }
    points
}

/// The projected image of `points` by `op`, the points it refuses left out, then the origin and
/// values far outside, so that the inverse sees the whole range.
pub(crate) fn projected_points(op: &Operation, points: &[Coord], far: f64) -> Vec<Coord> {
    let at = |x: f64, y: f64| Coord {
        x,
        y,
        z: 0.0,
        t: 0.0,
    };
    let mut images: Vec<Coord> = points.iter().filter_map(|p| op.forward(*p).ok()).collect();
    images.extend([
        at(0.0, 0.0),
        at(far, far),
        at(-far, far),
        at(far, 0.0),
        at(1e-9, -1e-9),
        at(f64::NAN, 1.0),
        at(1.0, f64::INFINITY),
        at(1e300, 1e300),
    ]);
    images
}

#[cfg(test)]
#[path = "kernel_bits_tests.rs"]
mod kernel_bits;
