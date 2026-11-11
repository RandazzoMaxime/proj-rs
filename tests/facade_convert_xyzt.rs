//! Four-dimensional forward conversion on the facade, `Proj::convert_xyzt`, and the `t = 0`
//! of `Proj::convert_xyz`.
//!
//! The operation and the expected values are PROJ 9.9.0's own: the helmert with `t_epoch=1988.0`
//! of `test/gie/more_builtins.gie` (lines 405-416, "Test coordinates from GNSStrans"), tolerance
//! 0.1 mm. They are not computed by this crate.

use proj_rs::Proj;

/// `test/gie/more_builtins.gie`, lines 405-410.
const GNSS_HELMERT: &str = "+proj=helmert +convention=position_vector \
    +x=0.01270 +dx=-0.0029 +rx=-0.00039 +drx=-0.00011 \
    +y=0.00650 +dy=-0.0002 +ry=0.00080 +dry=-0.00019 \
    +z=-0.0209 +dz=-0.0006 +rz=-0.00114 +drz=0.00007 \
    +s=0.00195 +ds=0.00001 \
    +t_epoch=1988.0";

/// `test/gie/more_builtins.gie`, line 413: the input, geocentric metres.
const INPUT: (f64, f64, f64) = (3_370_658.378, 711_877.314, 5_349_787.086);

/// Tolerance of the gie block: 0.1 mm.
const TOL: f64 = 1e-4;

fn near(got: f64, expect: f64) {
    assert!((got - expect).abs() <= TOL, "{got} vs {expect} (tol {TOL})");
}

/// `test/gie/more_builtins.gie`, line 414, at `t = 2017.0`.
#[test]
fn helmert_with_epoch_takes_the_time_of_the_point_2017() {
    let proj = Proj::new(GNSS_HELMERT).expect("helmert");
    let (x, y, z, _t) = proj
        .convert_xyzt(INPUT.0, INPUT.1, INPUT.2, 2017.0)
        .expect("2017");
    near(x, 3_370_658.188_90);
    near(y, 711_877.423_70);
    near(z, 5_349_787.124_30);
}

/// `test/gie/more_builtins.gie`, line 416, at `t = 2018.0`: the same input moves with the rates.
#[test]
fn helmert_with_epoch_takes_the_time_of_the_point_2018() {
    let proj = Proj::new(GNSS_HELMERT).expect("helmert");
    let (x, y, z, _t) = proj
        .convert_xyzt(INPUT.0, INPUT.1, INPUT.2, 2018.0)
        .expect("2018");
    near(x, 3_370_658.180_87);
    near(y, 711_877.427_50);
    near(z, 5_349_787.126_48);
}

/// `convert_xyz` passes `t = 0`: it is `convert_xyzt` at time zero. The helmert has rates and
/// `t_epoch = 1988`, so time zero (about 2000 years before the epoch) is far from the result
/// at 2017. The distance bound is a sanity check, not a PROJ value.
#[test]
fn convert_xyz_is_time_zero_and_not_the_epoch_of_the_point() {
    let proj = Proj::new(GNSS_HELMERT).expect("helmert");
    let (x0, y0, z0) = proj
        .convert_xyz(INPUT.0, INPUT.1, INPUT.2)
        .expect("time zero");
    let (xt, yt, zt, _t) = proj
        .convert_xyzt(INPUT.0, INPUT.1, INPUT.2, 0.0)
        .expect("time zero, four-dimensional");
    assert_eq!((x0, y0, z0), (xt, yt, zt));

    let distance = ((x0 - 3_370_658.188_90).powi(2)
        + (y0 - 711_877.423_70).powi(2)
        + (z0 - 5_349_787.124_30).powi(2))
    .sqrt();
    assert!(
        distance > 1.0,
        "time zero is {distance} m from the 2017 value"
    );
}
