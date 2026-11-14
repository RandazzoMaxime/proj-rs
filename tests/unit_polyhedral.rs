//! PROJ `test/unit/test_polyhedral.cpp`: the five `TEST()`s of the polyhedral
//! projections (`tsea`, `dsea`, `isea`, `ivea`). Tolerances and points are
//! PROJ 9.9.0's; the C++ `{lat, lon}` order of the points is kept.

use proj_rs::{Operation, Xyzt};

fn rad(deg: f64) -> f64 {
    deg.to_radians()
}

fn near(got: f64, expect: f64, tol: f64) {
    assert!((got - expect).abs() <= tol, "got {got}, expected {expect}");
}

const ROUNDTRIP_POINTS: [(f64, f64); 10] = [
    (0.0, 0.0),
    (45.0, 90.0),
    (-45.0, -90.0),
    (89.0, 0.0),
    (-89.0, 170.0),
    (30.0, 120.0),
    (-60.0, -150.0),
    (15.0, 45.0),
    (-30.0, -60.0),
    (70.0, -170.0),
];

#[test]
fn polyhedral_roundtrip() {
    // 1e-13 rad is about 0.6 nm at the equator; the configurations that compose
    // extra rotations or projected offsets get 1e-11 rad, as PROJ does.
    let configs = [
        ("+proj=tsea +R=1", 1e-13),
        ("+proj=tsea +ellps=WGS84", 1e-13),
        ("+proj=tsea +R=1 +orient_lat=90 +orient_lon=0 +azi=0", 1e-13),
        ("+proj=tsea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        (
            "+proj=tsea +R=1 +orient_lat=45 +orient_lon=30 +azi=60 +lat_0=15 +lon_0=-25 +x_0=1000 +y_0=-2000",
            1e-11,
        ),
        ("+proj=dsea +R=1", 1e-13),
        ("+proj=dsea +ellps=WGS84", 1e-13),
        ("+proj=dsea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        ("+proj=isea +R=1", 1e-13),
        ("+proj=isea +ellps=WGS84", 1e-13),
        ("+proj=isea +R=1 +lat_0=10 +lon_0=20", 1e-11),
        ("+proj=ivea +R=1", 1e-13),
        ("+proj=ivea +ellps=WGS84", 1e-13),
        ("+proj=ivea +R=1 +lat_0=10 +lon_0=20", 1e-11),
    ];
    for (definition, tol) in configs {
        let operation =
            Operation::create(definition).unwrap_or_else(|error| panic!("{definition}: {error}"));
        for (lat, lon) in ROUNDTRIP_POINTS {
            let input = Xyzt::xy(rad(lon), rad(lat));
            let flat = operation
                .forward(input)
                .unwrap_or_else(|error| panic!("{definition} {lat},{lon}: {error}"));
            let back = operation
                .inverse(flat)
                .unwrap_or_else(|error| panic!("{definition} inverse {lat},{lon}: {error}"));
            assert!(
                (back.x - input.x).abs() <= tol,
                "{definition}: lon roundtrip for lon={lon} lat={lat}"
            );
            assert!(
                (back.y - input.y).abs() <= tol,
                "{definition}: lat roundtrip for lon={lon} lat={lat}"
            );
        }
    }
}

#[test]
fn polyhedral_lat0_lon0_at_origin() {
    let operation =
        Operation::create("+proj=tsea +R=1 +lat_0=10 +lon_0=20").expect("tsea with origin");
    let got = operation
        .forward(Xyzt::xy(rad(20.0), rad(10.0)))
        .expect("forward");
    near(got.x, 0.0, 1e-13);
    near(got.y, 0.0, 1e-13);
}

#[test]
fn polyhedral_dsea_invalid_net_rejected() {
    assert!(Operation::create("+proj=dsea +R=1 +net=does_not_exist").is_err());
}

#[test]
fn polyhedral_dsea_known_nets_accepted() {
    for net in ["dsea", "a5", "crescent", "flower"] {
        Operation::create(&format!("+proj=dsea +R=1 +net={net}"))
            .unwrap_or_else(|error| panic!("{net}: {error}"));
    }
}

#[test]
fn polyhedral_x0_y0_stacks_on_lat0_lon0() {
    let operation = Operation::create("+proj=tsea +R=1 +lat_0=10 +lon_0=20 +x_0=7 +y_0=-3")
        .expect("tsea with offsets");
    let got = operation
        .forward(Xyzt::xy(rad(20.0), rad(10.0)))
        .expect("forward");
    near(got.x, 7.0, 1e-13);
    near(got.y, -3.0, 1e-13);
}
