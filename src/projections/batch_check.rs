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

/// With AVX2, some values must differ from the one-by-one ones at the last bit.
pub(crate) fn assert_vector_taken(label: &str, differing: usize) {
    if crate::simd::batch_path() == "avx2" {
        assert!(
            differing > 0,
            "{label}: no value differs bit for bit, the vector path was not taken"
        );
    }
}
