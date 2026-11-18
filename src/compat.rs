//! Stable-API equivalents of standard-library methods newer than the minimum Rust version.
//!
//! The crate declares the oldest Rust it was measured to build with (`rust-version`), and the
//! release is built with a newer one. These are the few places where the newer standard library
//! has a method the code wants: each shim does what that method does, with the same name where
//! it can, so the call can go back to the std method the day the minimum catches up.

/// The least `f64` greater than `x` (`f64::next_up`, Rust 1.86). `NaN` and `+inf` come back as
/// they are; both zeros give the smallest positive subnormal.
pub(crate) fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let bits = x.to_bits();
    f64::from_bits(if x > 0.0 { bits + 1 } else { bits - 1 })
}

/// The greatest `f64` less than `x` (`f64::next_down`, Rust 1.86).
pub(crate) fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

/// Whether `n` is a multiple of `of` (`usize::is_multiple_of`, Rust 1.91). `of` is not zero at any
/// call: the standard method answers `n == 0` for a zero, this one would panic.
// Clippy suggests the standard method only above the minimum Rust version, which this crate
// does not declare: no lint to expect.
pub(crate) fn is_multiple_of(n: usize, of: usize) -> bool {
    n % of == 0
}

/// The groups of four of a slice whose length is a multiple of four (`<[f64]>::as_chunks`,
/// Rust 1.88).
#[cfg(target_arch = "x86_64")]
pub(crate) fn quads(slice: &[f64]) -> impl Iterator<Item = &[f64; 4]> {
    slice
        .chunks_exact(4)
        .map(|chunk| <&[f64; 4]>::try_from(chunk).expect("a chunk of exactly four"))
}

/// [`quads`] for a mutable slice (`<[f64]>::as_chunks_mut`).
#[cfg(target_arch = "x86_64")]
pub(crate) fn quads_mut(slice: &mut [f64]) -> impl Iterator<Item = &mut [f64; 4]> {
    slice
        .chunks_exact_mut(4)
        .map(|chunk| <&mut [f64; 4]>::try_from(chunk).expect("a chunk of exactly four"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_up_and_down_step_one_representable_value() {
        assert_eq!(next_up(1.0), 1.0 + f64::EPSILON);
        assert_eq!(next_down(1.0), 1.0 - f64::EPSILON / 2.0);
        assert_eq!(next_up(-1.0), -(1.0 - f64::EPSILON / 2.0));
        assert_eq!(next_down(-1.0), -(1.0 + f64::EPSILON));
        assert_eq!(next_up(0.0), f64::from_bits(1));
        assert_eq!(next_up(-0.0), f64::from_bits(1));
        assert_eq!(next_down(0.0), -f64::from_bits(1));
        assert_eq!(next_up(-f64::from_bits(1)).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(next_up(f64::MAX), f64::INFINITY);
        assert_eq!(next_up(f64::INFINITY), f64::INFINITY);
        assert_eq!(next_down(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(next_up(f64::NAN).is_nan());
    }

    #[test]
    fn a_multiple_of_is_the_remainder_test() {
        assert!(is_multiple_of(0, 3));
        assert!(is_multiple_of(12, 4));
        assert!(!is_multiple_of(13, 4));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn quads_group_by_four() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let groups: Vec<[f64; 4]> = quads(&data).copied().collect();
        assert_eq!(groups, vec![[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]]);
        let mut out = [0.0; 8];
        for (group, source) in quads_mut(&mut out).zip(quads(&data)) {
            *group = *source;
        }
        assert_eq!(out, data);
    }
}
