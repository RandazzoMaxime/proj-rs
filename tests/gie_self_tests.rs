//! The 20 tests of PROJ 9.9.0 `test/unit/gie_self_tests.cpp` (`gie.*`), in Rust.
//!
//! Expected values are PROJ's own, copied from the C++ test. A C assertion that
//! has no counterpart in this crate (a context move, a lazy pipeline text, the
//! `P->` internals) is not written here; `docs/AUDIT-gie-network.md` lists each one
//! with its cause. The PROJ-string constants are generated from the C++ literals.

use proj_rs::{
    list_ellps, list_operations, list_prime_meridians, xy_dist, BatchError, Direction, Operation,
    Proj, StepError, Xyzt,
};
use std::num::NonZeroUsize;

fn one() -> NonZeroUsize {
    NonZeroUsize::MIN
}

fn steps(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("n >= 1")
}

fn fwd(op: &Operation, c: Xyzt) -> Xyzt {
    op.forward(c).expect("forward")
}

fn inv(op: &Operation, c: Xyzt) -> Xyzt {
    op.inverse(c).expect("inverse")
}

fn xy(x: f64, y: f64) -> Xyzt {
    Xyzt::xy(x, y)
}

/// `cart_selftest` (gie_self_tests.cpp). The context moves of the C test
/// (`proj_context_set`) have no counterpart: a Rust object holds no context.
#[test]
fn cart_selftest() {
    let utm = Operation::create("+proj=utm; +zone=32; +ellps=GRS80").expect("utm with ;");
    let argv_style = Operation::create("+proj=utm +zone=32 +ellps=GRS80").expect("utm");
    // (lon, lat) = (12, 55) in degrees, as radians.
    let a0 = Xyzt {
        x: 12f64.to_radians(),
        y: 55f64.to_radians(),
        z: 0.0,
        t: 0.0,
    };
    let b = fwd(&utm, a0);
    let a = inv(&utm, b);
    let a = fwd(&utm, a);
    assert!(xy_dist(a, b) <= 2e-9);
    let b_argv = fwd(&argv_style, a0);
    assert!(xy_dist(b_argv, b) <= 2e-9);

    let cart = Operation::create("+proj=cart +ellps=GRS80").expect("cart");
    let a = Xyzt {
        x: 12f64.to_radians(),
        y: 55f64.to_radians(),
        z: 100.0,
        t: 0.0,
    };
    let b = fwd(&cart, a);
    let (_, d_fwd) = cart
        .roundtrip(Direction::Forward, steps(10_000), a)
        .expect("roundtrip");
    let (_, d_inv) = cart
        .roundtrip(Direction::Inverse, steps(10_000), b)
        .expect("roundtrip");
    assert!(d_fwd + d_inv <= 4e-9, "{d_fwd} {d_inv}");

    // North Pole.
    let north = Xyzt {
        x: 0.0,
        y: 90f64.to_radians(),
        z: 100.0,
        t: 0.0,
    };
    let (_, d) = cart
        .roundtrip(Direction::Forward, one(), north)
        .expect("roundtrip");
    assert!(d <= 1e-9, "{d}");

    // South Pole.
    let south = Xyzt {
        x: 0.0,
        y: (-90f64).to_radians(),
        z: 100.0,
        t: 0.0,
    };
    let (_, d) = cart
        .roundtrip(Direction::Forward, one(), south)
        .expect("roundtrip");
    assert!(d <= 4e-9, "{d}");
    let b = inv(&cart, south);
    let b = fwd(&cart, b);
    let _ = inv(&cart, b);

    // proj_trans_generic with three coordinates, as points.
    let utm32 = Operation::create("+proj=utm +zone=32 +ellps=GRS80").expect("utm");
    let obs0 = Xyzt {
        x: 12f64.to_radians(),
        y: 55f64.to_radians(),
        z: 45.0,
        t: 0.0,
    };
    let obs1 = Xyzt {
        x: 12f64.to_radians(),
        y: 56f64.to_radians(),
        z: 50.0,
        t: 0.0,
    };
    let a = fwd(&utm32, obs0);
    let b = fwd(&utm32, obs1);
    let mut arr = [obs0, obs1];
    utm32
        .transform_array(Direction::Forward, &mut arr)
        .expect("two points");
    assert_eq!(arr[0], a);
    assert_eq!(arr[1], b);
    assert_eq!(arr[0].z, 45.0);
    assert_eq!(arr[1].z, 50.0);

    // proj_trans_array with two failed points for the same reason.
    let mut coords = [
        Xyzt {
            x: 12f64.to_radians(),
            y: 95f64.to_radians(),
            z: 45.0,
            t: 0.0,
        },
        obs1,
        Xyzt {
            x: 12f64.to_radians(),
            y: 95f64.to_radians(),
            z: 45.0,
            t: 0.0,
        },
    ];
    let err = utm32.transform_array(Direction::Forward, &mut coords);
    assert_eq!(
        err,
        Err(BatchError::Uniform(StepError::InvalidCoord)),
        "two invalid latitudes"
    );
    for failed in [coords[0], coords[2]] {
        assert!(failed.x.is_infinite() && failed.y.is_infinite() && failed.z.is_infinite());
    }
    assert_eq!(coords[1], b);

    // Two failed points for different reasons: the generic code.
    let mut coords = [
        Xyzt::xy(12f64.to_radians(), 95f64.to_radians()),
        Xyzt::xy(105f64.to_radians(), 0.0),
    ];
    assert_eq!(
        utm32.transform_array(Direction::Forward, &mut coords),
        Err(BatchError::Mixed)
    );
}

/// `io_predicates`: `proj_angular_input` and `proj_angular_output`. The C test sets
/// `P->inverted`; here the same direction is read as `inverse = true`.
#[test]
fn io_predicates() {
    let cart = Operation::create("+proj=cart +ellps=GRS80").expect("cart");
    assert!(cart.angular_input(false));
    assert!(!cart.angular_input(true));
    assert!(!cart.angular_output(false));
    assert!(cart.angular_output(true));

    let molodensky = Operation::create(
        "+proj=molodensky +a=6378160 +rf=298.25 +da=-23 +df=-8.120449e-8 +dx=-134 +dy=-48 +dz=149 +abridged",
    )
    .expect("molodensky");
    for inverse in [false, true] {
        assert!(molodensky.angular_input(inverse));
        assert!(molodensky.angular_output(inverse));
    }

    let helmert = Operation::create(
        " +proj=helmert +x=0.0127 +y=0.0065 +z=-0.0209 +s=0.00195 +rx=-0.00039 +ry=0.00080 \
         +rz=-0.00114 +dx=-0.0029 +dy=-0.0002 +dz=-0.0006 +ds=0.00001 +drx=-0.00011 \
         +dry=-0.00019 +drz=0.00007 +t_epoch=1988.0 +convention=coordinate_frame",
    )
    .expect("helmert");
    for inverse in [false, true] {
        assert!(!helmert.angular_input(inverse));
        assert!(!helmert.angular_output(inverse));
    }
    // pj_init_ctx defaults to GRS80: a = 6378137, f = 1/298.257222101.
    let (a, es) = helmert.distance_ellipsoid();
    let f = 1.0 / 298.257_222_101;
    assert_eq!(a, 6_378_137.0);
    assert!((es - f * (2.0 - f)).abs() < 1e-15, "{es}");
}

/// `proj_trans` of NaN input: the C test asserts NaN output. A Rust operation
/// returns an error for a non-finite input, so it returns no coordinate.
#[test]
fn nan_input_gives_no_coordinate() {
    let merc = Operation::create("+proj=merc +ellps=WGS84").expect("merc");
    let nan = Xyzt {
        x: f64::NAN,
        y: f64::NAN,
        z: f64::NAN,
        t: f64::NAN,
    };
    assert!(merc.forward(nan).is_err());
    assert!(merc.inverse(nan).is_err());
}

/// `unitconvert_selftest`: time conversions through `t`.
#[test]
fn unitconvert_selftest() {
    fn test_time(args: &str, tol: f64, t_in: f64, t_exp: f64) {
        let op = Operation::create(args).expect("unitconvert");
        let out = op
            .forward(Xyzt {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                t: t_in,
            })
            .expect("forward");
        assert!((out.t - t_exp).abs() <= tol, "{args}: {} vs {t_exp}", out.t);
        let back = op.inverse(out).expect("inverse");
        assert!(
            (back.t - t_in).abs() <= tol,
            "{args} inverse: {} vs {t_in}",
            back.t
        );
    }
    test_time(
        "+proj=unitconvert +t_in=decimalyear +t_out=decimalyear",
        1e-6,
        2004.25,
        2004.25,
    );
    test_time(
        "+proj=unitconvert +t_in=gps_week +t_out=gps_week",
        1e-6,
        1782.0,
        1782.0,
    );
    test_time(
        "+proj=unitconvert +t_in=mjd +t_out=mjd",
        1e-6,
        57390.0,
        57390.0,
    );
    test_time(
        "+proj=unitconvert +t_in=gps_week +t_out=decimalyear",
        1e-6,
        1877.71428,
        2016.0,
    );
    test_time(
        "+proj=unitconvert +t_in=yyyymmdd +t_out=yyyymmdd",
        1e-6,
        20170131.0,
        20170131.0,
    );
}

/// `unitconvert_selftest_date`: decimal years and `yyyymmdd` both ways.
#[test]
fn unitconvert_selftest_date() {
    fn test_date(args: &str, t_in: f64, t_exp: f64) {
        let op = Operation::create(args).expect("unitconvert");
        let out = op
            .forward(Xyzt {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                t: t_in,
            })
            .expect("forward");
        assert!(
            (out.t - t_exp).abs() <= 1e-6,
            "{args} {t_in}: {} vs {t_exp}",
            out.t
        );
    }
    let dec_to_date = "+proj=unitconvert +t_in=decimalyear +t_out=yyyymmdd";
    test_date(dec_to_date, 2022.0027, 20220102.0);
    test_date(dec_to_date, 1990.0, 19900101.0);
    test_date(dec_to_date, 2004.1612, 20040229.0);
    test_date(dec_to_date, 1899.999, 19000101.0);
    let date_to_dec = "+proj=unitconvert +t_in=yyyymmdd +t_out=decimalyear";
    test_date(date_to_dec, 20220102.0, 2022.0027397);
    test_date(date_to_dec, 19900101.0, 1990.0);
    test_date(date_to_dec, 20040229.0, 2004.1612022);
    test_date(date_to_dec, 18991231.0, 1899.9972603);
}

/// `proj_trans` on a CRS object (not a transform) fails: `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`.
#[test]
fn trans_with_a_crs_is_an_error() {
    let crs = Proj::new("EPSG:4326").expect("EPSG:4326 is a CRS");
    assert!(crs.convert_xyzt(0.0, 0.0, 0.0, 0.0).is_err());
}

/// `gieTest, proj_create_crs_to_crs` (TEST_F). The CRS-name assertions of the C test
/// are not here: this crate has no `proj_get_name` on a transform.
#[test]
fn proj_create_crs_to_crs() {
    let p = Proj::new_known_crs("EPSG:25832", "EPSG:25833", None).expect("25832 to 25833");
    let (x, y, _) = p.convert_xyz(700_000.0, 6_000_000.0, 0.0).expect("convert");
    assert!((x - 307_788.876_117_105_7).abs() <= 1e-8, "{x}");
    assert!((y - 5_999_669.303_603_762_8).abs() <= 1e-8, "{y}");

    // PROJ strings are accepted as CRS sides.
    assert!(Proj::new_known_crs(
        "proj=utm +zone=32 +datum=WGS84",
        "proj=utm +zone=33 +datum=WGS84",
        None
    )
    .is_ok());
    assert!(Proj::new_known_crs("invalid", "EPSG:25833", None).is_err());
    assert!(Proj::new_known_crs("EPSG:25832", "invalid", None).is_err());
}

/// `gieTest, proj_create_crs_to_crs_EPSG_4326`. The C input is latitude first; this
/// crate takes longitude, latitude, so the point is (3, 0).
#[test]
fn proj_create_crs_to_crs_epsg_4326() {
    let p = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None).expect("4326 to 32631");
    let (x, y, _) = p.convert_xyz(3.0, 0.0, 0.0).expect("convert");
    assert!((x - 500_000.0).abs() <= 1e-9, "{x}");
    assert!(y.abs() <= 1e-9, "{y}");
}

/// `gieTest, proj_create_crs_to_crs_proj_longlat`: a PROJ string source.
#[test]
fn proj_create_crs_to_crs_proj_longlat() {
    let p = Proj::new_known_crs("+proj=longlat +datum=WGS84", "EPSG:32631", None)
        .expect("longlat to 32631");
    let (x, y, _) = p.convert_xyz(3.0, 0.0, 0.0).expect("convert");
    assert!((x - 500_000.0).abs() <= 1e-9, "{x}");
    assert!(y.abs() <= 1e-9, "{y}");
}

/// `proj_create_crs_to_crs_PULKOVO42_ETRS89`: values and the pipeline text after the
/// first transform. The CRS names and the "unavailable until proj_trans" text are C
/// internals (lazy construction) and are not checked.
#[test]
fn proj_create_crs_to_crs_pulkovo42_etrs89() {
    let p = Proj::new_known_crs("EPSG:4179", "EPSG:4258", None).expect("4179 to 4258");
    // Longitude, latitude order here: the C test gives latitude 45, longitude 25.
    let (lon, lat, _) = p.convert_xyz(25.0, 45.0, 0.0).expect("convert");
    assert!((lat - 44.999_701_238).abs() <= 1e-9, "{lat}");
    assert!((lon - 24.998_474_948).abs() <= 1e-9, "{lon}");
    let (back_lon, back_lat): (f64, f64) = p.project((lon, lat), true).expect("inverse");
    assert!((back_lat - 45.0).abs() <= 1e-8, "{back_lat}");
    assert!((back_lon - 25.0).abs() <= 1e-8, "{back_lon}");
}

/// `proj_create_crs_to_crs_with_area_large`: the area of use is given.
#[test]
fn proj_create_crs_to_crs_with_area_large() {
    let area = proj_rs::Area::new(-14.1324, 49.5614, 3.76488, 62.1463);
    let p = Proj::new_known_crs("EPSG:4277", "EPSG:4326", Some(area)).expect("4277 to 4326");
    // Longitude, latitude: the C test gives latitude 50, longitude -2.
    let (lon, lat, _) = p.convert_xyz(-2.0, 50.0, 0.0).expect("convert");
    assert!((lon - -2.001_339_89).abs() <= 1e-8, "{lon}");
    assert!((lat - 50.000_656_28).abs() <= 1e-8, "{lat}");
}

/// `proj_create_crs_to_crs_with_longitude_outside_minus_180_180`.
#[test]
fn proj_create_crs_to_crs_with_longitude_outside_minus_180_180() {
    let p = Proj::new_known_crs("EPSG:4277", "EPSG:4326", None).expect("4277 to 4326");
    for longitude in [-2.0 + 360.0, -2.0 - 360.0] {
        let (lon, lat, _) = p.convert_xyz(longitude, 50.0, 0.0).expect("convert");
        assert!((lon - -2.001_339_89).abs() <= 1e-8, "{lon}");
        assert!((lat - 50.000_656_28).abs() <= 1e-8, "{lat}");
    }
    for longitude in [-2.001_339_89 + 360.0, -2.001_339_89 - 360.0] {
        let (lon, lat): (f64, f64) = p
            .project((longitude, 50.000_656_28), true)
            .expect("inverse");
        assert!((lon - -2.0).abs() <= 1e-8, "{lon}");
        assert!((lat - 50.0).abs() <= 1e-8, "{lat}");
    }
}

/// `proj_create_crs_to_crs_outside_area_of_use`: EPSG:4807 is in grads. The C test
/// gives latitude 58 and longitude 5 in degrees, and reads the result in grads.
#[test]
fn proj_create_crs_to_crs_outside_area_of_use() {
    let p = Proj::new_known_crs("EPSG:4275", "EPSG:4807", None).expect("4275 to 4807");
    let (lon, lat, _) = p.convert_xyz(5.0, 58.0, 0.0).expect("convert");
    assert!((lat - 64.444_444_444_444_44).abs() <= 1e-9, "{lat}");
    assert!((lon - 2.958_634_259_259_258).abs() <= 1e-9, "{lon}");
}

/// `proj_trans_generic`: GDA2020 to WGS 84 (G1762). With no time the helmert step
/// does nothing, so the point (latitude -60, longitude 120) comes back unchanged.
#[test]
fn proj_trans_generic() {
    let op = Operation::create(
        "+proj=pipeline +step +proj=axisswap +order=2,1 \
         +step +proj=unitconvert +xy_in=deg +xy_out=rad \
         +step +proj=cart +ellps=GRS80 \
         +step +proj=helmert +x=0 +y=0 +z=0 +rx=0 +ry=0 +rz=0 +s=0 +dx=0 \
         +dy=0 +dz=0 +drx=-0.00150379 +dry=-0.00118346 +drz=-0.00120716 \
         +ds=0 +t_epoch=2020 +convention=coordinate_frame \
         +step +inv +proj=cart +ellps=WGS84 \
         +step +proj=unitconvert +xy_in=rad +xy_out=deg \
         +step +proj=axisswap +order=2,1",
    )
    .expect("GDA2020 to WGS 84");
    let mut points = [Xyzt {
        x: -60.0,
        y: 120.0,
        z: 0.0,
        t: f64::INFINITY,
    }];
    op.transform_array(Direction::Forward, &mut points)
        .expect("forward");
    assert!((points[0].x - -60.0).abs() <= 1e-9, "{}", points[0].x);
    assert!((points[0].y - 120.0).abs() <= 1e-9, "{}", points[0].y);
}

/// `list_functions`: `proj_list_operations`, `proj_list_ellps`, and the 14 prime meridians.
#[test]
fn list_functions() {
    assert_ne!(list_operations().len(), 0);
    assert_ne!(list_ellps().len(), 0);
    assert_eq!(list_prime_meridians().len(), 14);
}
