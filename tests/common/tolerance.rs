//! The one rule for comparing PROJ 9.9.0's numbers, in doubles, text or `proj -b` bytes.
//! The rule is written in `docs/PARITY.md`, « Tolérance de parité » ; no test keeps its own copy.
//!
//! - metres: 1e-6 m, or 1e-13 of the reference value when that is larger (past 1e7 m one ULP
//!   exceeds 1e-6 m: `cs2cs-inverse-0105:4` is 7e11 m, 5.7e-15 relative);
//! - degrees: 1e-11 degree, radians: 1e-13 rad (the same distance on the ground);
//! - scale factors of `proj -S`: 1e-7 relative, 1e-5 absolute when both values are under 1e-4
//!   (finite differences of step 1e-5 rad in `factors.cpp` amplify a one-ULP gap);
//! - text: the numbers are read back and compared by the rule above. A last printed digit that
//!   flips, one unit apart, is rounding of the same value, not a gap.

#![allow(
    dead_code,
    reason = "each test crate uses a different part of this module"
)]

pub(crate) const METRES: f64 = 1e-6;
pub(crate) const DEGREES: f64 = 1e-11;
pub(crate) const RADIANS: f64 = 1e-13;
pub(crate) const LINEAR_RELATIVE: f64 = 1e-13;
pub(crate) const FACTOR_RELATIVE: f64 = 1e-7;
pub(crate) const FACTOR_FLOOR: f64 = 1e-5;
pub(crate) const FACTOR_NEAR_ZERO: f64 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Quantity {
    Metres,
    Degrees,
    Radians,
    Factor,
}

/// A linear gap within `absolute`, or within [`LINEAR_RELATIVE`] of the reference value.
pub(crate) fn linear_within(gap: f64, reference: f64, absolute: f64) -> bool {
    gap <= absolute.max(LINEAR_RELATIVE * reference.abs())
}

/// Whether `got` and `expected` agree for `quantity`. Equal values, equal infinities and two NaN
/// agree; a finite value never agrees with a non-finite one.
pub(crate) fn close(quantity: Quantity, got: f64, expected: f64) -> bool {
    if got == expected || (got.is_nan() && expected.is_nan()) {
        return true;
    }
    if !got.is_finite() || !expected.is_finite() {
        return false;
    }
    let gap = (got - expected).abs();
    match quantity {
        Quantity::Metres => linear_within(gap, expected, METRES),
        Quantity::Degrees => gap <= DEGREES,
        Quantity::Radians => gap <= RADIANS,
        Quantity::Factor => {
            let scale = got.abs().max(expected.abs());
            if scale < FACTOR_NEAR_ZERO {
                gap <= FACTOR_FLOOR
            } else {
                gap <= FACTOR_RELATIVE * scale
            }
        }
    }
}

/// [`close`] on the 64-bit words of `proj -b`.
pub(crate) fn close_bits(quantity: Quantity, got: u64, expected: u64) -> bool {
    close(quantity, f64::from_bits(got), f64::from_bits(expected))
}

/// The value of one unit of the last printed digit of a decimal token (`12.34` is 0.01,
/// `1.5e-06` is 1e-7), or `None` when the token is not a decimal number.
fn printed_unit(token: &str) -> Option<f64> {
    let (mantissa, exponent) = match token.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (token, 0),
    };
    let decimals = mantissa.split_once('.').map_or(0, |(_, tail)| tail.len());
    Some(10f64.powi(exponent - i32::try_from(decimals).ok()?))
}

fn same_token(quantity: Quantity, got: &str, expected: &str) -> bool {
    if got == expected {
        return true;
    }
    let (Ok(x), Ok(y)) = (got.parse::<f64>(), expected.parse::<f64>()) else {
        return false;
    };
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    if close(quantity, x, y) {
        return true;
    }
    match (printed_unit(got), printed_unit(expected)) {
        (Some(a), Some(b)) => (x - y).abs() <= a.min(b) * (1.0 + 1e-6),
        _ => false,
    }
}

/// The tokens of a text, each with whether it sits between `<` and `>` (the factors of `-S`).
fn tokens(text: &str) -> Vec<(&str, bool)> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let mut start = None;
        for (index, c) in line.char_indices().chain([(line.len(), ' ')]) {
            if c.is_whitespace() || c == '<' || c == '>' {
                if let Some(from) = start.take() {
                    out.push((&line[from..index], inside));
                }
                match c {
                    '<' => inside = true,
                    '>' => inside = false,
                    _ => {}
                }
            } else if start.is_none() {
                start = Some(index);
            }
        }
    }
    out
}

/// Whether a command's text matches PROJ's: the same lines and tokens; the numbers within the rule
/// (`coordinates` for those outside `<...>`, factors inside), or one unit of their last digit.
pub(crate) fn same_text(coordinates: Quantity, got: &str, expected: &str) -> bool {
    if got == expected {
        return true;
    }
    let (left, right) = (tokens(got), tokens(expected));
    left.len() == right.len()
        && got.lines().count() == expected.lines().count()
        && left.iter().zip(&right).all(|(&(a, in_a), &(b, in_b))| {
            let quantity = if in_a { Quantity::Factor } else { coordinates };
            in_a == in_b && same_token(quantity, a, b)
        })
}
