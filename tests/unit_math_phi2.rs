//! PROJ `test/unit/pj_phi2_test.cpp` (`PjPhi2Test.Basic`), through the public
//! `proj_rs::math::phi2`. Expected values are PROJ 9.9.0's. Comparisons use
//! gtest's `EXPECT_DOUBLE_EQ` (4 ULPs) and `EXPECT_EQ` (exact).

use std::f64::consts::{FRAC_PI_2, PI};

use proj_rs::math::phi2;

/// gtest's `AlmostEquals`: within 4 units in the last place, same sign.
fn double_eq(got: f64, want: f64) -> bool {
    if got.is_nan() || want.is_nan() {
        return false;
    }
    if got == want {
        return true;
    }
    if got.is_sign_negative() != want.is_sign_negative() {
        return false;
    }
    let diff = i128::from(got.to_bits()) - i128::from(want.to_bits());
    diff.abs() <= 4
}

fn phi2_ok(ts: f64, e: f64) -> f64 {
    phi2(ts, e).expect("phi2 converges for the PROJ test inputs")
}

#[test]
fn pj_phi2_basic() {
    let inf = f64::INFINITY;
    let nan = f64::NAN;

    // Exact equality, as the C++ test demands.
    assert_eq!(FRAC_PI_2, phi2_ok(0.0, 0.0));
    assert_eq!(0.0, phi2_ok(1.0, 0.0));
    assert_eq!(-FRAC_PI_2, phi2_ok(inf, 0.0));
    // ts = +0.0 and ts = -0.0 return different results.
    assert_eq!(-FRAC_PI_2, phi2_ok(-0.0, 0.0));
    assert_eq!(0.0, phi2_ok(-1.0, 0.0));
    assert_eq!(FRAC_PI_2, phi2_ok(-inf, 0.0));

    let e = 0.2;
    assert_eq!(FRAC_PI_2, phi2_ok(0.0, e));
    assert_eq!(0.0, phi2_ok(1.0, e));
    assert_eq!(-FRAC_PI_2, phi2_ok(inf, e));
    assert_eq!(-FRAC_PI_2, phi2_ok(-0.0, e));
    assert_eq!(0.0, phi2_ok(-1.0, e));
    assert_eq!(FRAC_PI_2, phi2_ok(-inf, e));

    // NaN in either argument gives NaN.
    for (ts, e) in [
        (nan, 0.0),
        (nan, e),
        (0.0, nan),
        (1.0, nan),
        (inf, nan),
        (-0.0, nan),
        (-1.0, nan),
        (-inf, nan),
        (nan, nan),
    ] {
        assert!(phi2(ts, e).is_ok_and(f64::is_nan), "ts={ts} e={e}");
    }

    // Spherical inverse: ts = 1 / (sqrt(3) + 2) is phi = pi/3.
    let s3 = 3.0_f64.sqrt();
    let s2 = 2.0_f64.sqrt();
    assert!(double_eq(PI / 3.0, phi2_ok(1.0 / (s3 + 2.0), 0.0)));
    assert!(double_eq(PI / 4.0, phi2_ok(1.0 / (s2 + 1.0), 0.0)));
    assert!(double_eq(PI / 6.0, phi2_ok(1.0 / s3, 0.0)));
    assert!(double_eq(-PI / 3.0, phi2_ok(s3 + 2.0, 0.0)));
    assert!(double_eq(-PI / 4.0, phi2_ok(s2 + 1.0, 0.0)));
    assert!(double_eq(-PI / 6.0, phi2_ok(s3, 0.0)));

    // Ellipsoidal: generated in PROJ's test with
    // exp(e * atanh(e * sin(phi))) / (tan(phi) + sec(phi)).
    assert!(double_eq(PI / 3.0, phi2_ok(0.27749174377027025, e)));
    assert!(double_eq(PI / 4.0, phi2_ok(0.42617788119104194, e)));
    assert!(double_eq(PI / 6.0, phi2_ok(0.5890530244862673, e)));
    assert!(double_eq(-PI / 3.0, phi2_ok(3.603710821853783, e)));
    assert!(double_eq(-PI / 4.0, phi2_ok(2.3464380582241713, e)));
    assert!(double_eq(-PI / 6.0, phi2_ok(1.6976400399134413, e)));
}
