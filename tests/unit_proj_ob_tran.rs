//! PROJ 9.9.0 values for `+proj=ob_tran +o_proj=moll` (`src/projections/ob_tran.cpp`).
//!
//! The case has no `+ellps`, so both sides use the default GRS80 ellipsoid.

use proj_rs::{Operation, Xyzt};

/// Tolerance for a projected value: 1 micrometre, or 1e-11 relative for large values.
fn assert_metres(got: f64, want: f64, what: &str) {
    let tol = 1e-6 + 1e-11 * want.abs();
    assert!(
        (got - want).abs() <= tol,
        "{what}: got {got}, want {want}, tolerance {tol}"
    );
}

/// Tolerance for an angle in degrees: 1e-9 degree.
fn assert_degrees(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 1e-9,
        "{what}: got {got}, want {want} degrees"
    );
}

fn forward(definition: &str, lon_deg: f64, lat_deg: f64) -> Xyzt {
    Operation::create(definition)
        .expect("create")
        .forward(Xyzt::xy(lon_deg.to_radians(), lat_deg.to_radians()))
        .expect("forward")
}

fn inverse(definition: &str, x: f64, y: f64) -> Xyzt {
    Operation::create(definition)
        .expect("create")
        .inverse(Xyzt::xy(x, y))
        .expect("inverse")
}

#[test]
fn ob_tran_moll_pole_45_forward_and_inverse_match_proj_9_9_0() {
    let definition = "+proj=ob_tran +o_proj=moll +o_lat_p=45 +o_lon_p=0";
    let forward_points = [
        (
            -24.609888047,
            -14.073195102,
            -3063639.744816203,
            -6153687.813233096,
        ),
        (
            -25.220904335,
            15.819921371,
            -2536380.551110123,
            -3053953.561630882,
        ),
        (
            -1.845846663,
            19.628321634,
            -181109.0926158292,
            -3094954.6280164355,
        ),
        (
            -98.134419529,
            -67.417588377,
            -13079163.280806957,
            -4554864.028907454,
        ),
        (
            -64.190131461,
            61.071033326,
            -2742433.1179664023,
            3412548.9401744087,
        ),
        (
            -51.732603463,
            78.782069121,
            -963450.0563996795,
            4505140.8351069465,
        ),
    ];
    for (lon, lat, x, y) in forward_points {
        let got = forward(definition, lon, lat);
        assert_metres(got.x, x, "x");
        assert_metres(got.y, y, "y");
    }
    let inverse_points = [
        (
            -3063639.744816203,
            -6153687.813233096,
            -24.609888047,
            -14.073195102,
        ),
        (
            -2536380.551110123,
            -3053953.561630882,
            -25.220904335,
            15.819921371,
        ),
        (
            -181109.0926158292,
            -3094954.6280164355,
            -1.845846663,
            19.628321634,
        ),
        (
            -13079163.280806957,
            -4554864.028907454,
            -98.134419529,
            -67.417588377,
        ),
        (
            -2742433.1179664023,
            3412548.9401744087,
            -64.190131461,
            61.071033326,
        ),
        (
            -963450.0563996795,
            4505140.8351069465,
            -51.732603463,
            78.782069121,
        ),
    ];
    for (x, y, lon, lat) in inverse_points {
        let got = inverse(definition, x, y);
        assert_degrees(got.x.to_degrees(), lon, "lon");
        assert_degrees(got.y.to_degrees(), lat, "lat");
    }
}
