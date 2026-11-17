//! The parity tolerance of `docs/PARITY.md` is not empty: it accepts what it must and refuses a
//! gap of 1e-5 m, 1e-9 degree, 1e-12 rad or 1e-6 relative on a scale factor.

mod common;
use common::tolerance::{close, close_bits, same_text, Quantity};

#[test]
fn metres_accept_a_micrometre_and_refuse_ten() {
    assert!(close(
        Quantity::Metres,
        563_629.522_465_540_7,
        563_629.522_465_542_2
    ));
    assert!(close(Quantity::Metres, 1000.0, 1000.0 + 5e-7));
    assert!(!close(Quantity::Metres, 1000.0, 1000.0 + 1e-5));
    assert!(!close(Quantity::Metres, -1000.0, -1000.0 - 1e-5));
}

#[test]
fn angles_accept_the_ground_equivalent_and_refuse_more() {
    assert!(close(Quantity::Degrees, 3.0, 3.0 + 5e-12));
    assert!(!close(Quantity::Degrees, 3.0, 3.0 + 1e-9));
    assert!(close(Quantity::Radians, 0.05, 0.05 + 5e-14));
    assert!(!close(Quantity::Radians, 0.05, 0.05 + 1e-12));
}

#[test]
fn factors_are_relative_with_a_floor_near_zero() {
    assert!(close(Quantity::Factor, 1.000_000_000, 1.0 + 5e-10));
    assert!(!close(Quantity::Factor, 1.0, 1.0 + 1e-6));
    assert!(close(Quantity::Factor, 0.0, 1.2e-6));
    assert!(!close(Quantity::Factor, 0.0, 2e-5));
    assert!(close(Quantity::Factor, 2.51241e-5, 2.50931e-5));
}

#[test]
fn non_finite_values_agree_only_with_themselves() {
    for quantity in [
        Quantity::Metres,
        Quantity::Degrees,
        Quantity::Radians,
        Quantity::Factor,
    ] {
        assert!(close(quantity, f64::NAN, f64::NAN));
        assert!(close(quantity, f64::INFINITY, f64::INFINITY));
        assert!(!close(quantity, f64::INFINITY, f64::NEG_INFINITY));
        assert!(!close(quantity, f64::NAN, 0.0));
        assert!(!close(quantity, f64::INFINITY, f64::MAX));
    }
    assert!(close_bits(
        Quantity::Metres,
        1.0_f64.to_bits(),
        1.0_f64.to_bits()
    ));
}

#[test]
fn text_reads_the_numbers_back() {
    assert!(same_text(
        Quantity::Metres,
        "1.0000004 2\n",
        "1.0000000 2\n"
    ));
    assert!(!same_text(
        Quantity::Metres,
        "1.0000100 2\n",
        "1.0000000 2\n"
    ));
    assert!(!same_text(Quantity::Metres, "1 2\n", "1 2 3\n"));
    assert!(!same_text(Quantity::Metres, "1 2\n3 4\n", "1 2 3 4\n"));
    assert!(!same_text(Quantity::Metres, "*\t*\n", "inf\tinf\n"));
}

#[test]
fn a_last_digit_that_flips_is_rounding_and_two_are_not() {
    assert!(same_text(Quantity::Metres, "12345.68\n", "12345.67\n"));
    assert!(same_text(Quantity::Metres, "<5.74006>", "<5.74005>"));
    assert!(same_text(Quantity::Metres, "<1.2e-06>", "<1.3e-06>"));
    assert!(!same_text(Quantity::Metres, "12345.69\n", "12345.67\n"));
    assert!(!same_text(Quantity::Metres, "<5.74007>", "<5.74005>"));
}

#[test]
fn factors_inside_brackets_follow_the_factor_rule() {
    assert!(same_text(
        Quantity::Metres,
        "<1 1 1 1.2e-06 1 1>",
        "<1 1 1 0 1 1>"
    ));
    assert!(!same_text(
        Quantity::Metres,
        "<1 1 1 2e-05 1 1>",
        "<1 1 1 0 1 1>"
    ));
    assert!(!same_text(
        Quantity::Metres,
        "(1 1 1 0 1 1>",
        "<1 1 1 0 1 1>"
    ));
}
