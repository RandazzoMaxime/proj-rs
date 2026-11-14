//! The 20 tests of PROJ 9.9.0 `test/unit/gie_self_tests.cpp` (`gie.*`), in Rust.
//!
//! Expected values are PROJ's own, copied from the C++ test. A C assertion that
//! has no counterpart in this crate (a context move, a lazy pipeline text, the
//! `P->` internals) is not written here; `docs/AUDIT-gie-network.md` lists each one
//! with its cause. The PROJ-string constants are generated from the C++ literals.

use proj_rs::errno::PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE;
use proj_rs::{
    list_ellps, list_operations, list_prime_meridians, xy_dist, BatchError, Context, Direction,
    ForceOver, Operation, Proj, ProjError, StepError, Xyzt,
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

/// `cart_selftest` (gie_self_tests.cpp). The context moves of the C test
/// (`proj_context_set`, `P->ctx`) are `Operation::set_context` and `Operation::context`.
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
    // PJ_IDENT (null projection): the coordinate is returned as it is, no call here.
    let a = fwd(&utm, a);
    assert!(xy_dist(a, b) <= 2e-9);
    // proj_errno_reset(P): an operation keeps no error state, errors are returned values.
    let b_argv = fwd(&argv_style, a0);
    assert!(xy_dist(b_argv, b) <= 2e-9);

    let mut cart = Operation::create("+proj=cart +ellps=GRS80").expect("cart");
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

    // Move the operation to a context of its own (proj_context_create, proj_context_set).
    // The C check that the new context is not the default one is a pointer comparison:
    // a Rust `Context` is a value, so it has no counterpart.
    let default = Context::new();
    let ctx = Context::new();
    cart.set_context(ctx.clone());
    assert_eq!(cart.context(), &ctx);
    let b = fwd(&cart, b);

    // Move it back to the default context (proj_context_set(P, NULL)).
    cart.set_context(default.clone());
    assert_eq!(cart.context(), &default);

    // proj_context_destroy(ctx): the operation no longer uses it.
    drop(ctx);
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
    // proj_trans_generic returns n == 2: the slice holds two points, the whole slice is done.
    // proj_trans_array returns 0: `Ok(())`.
    assert_eq!(
        utm32.transform_array(Direction::Forward, &mut arr),
        Ok(()),
        "two points"
    );
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
        assert_eq!(failed.x, f64::INFINITY);
        assert_eq!(failed.y, f64::INFINITY);
        assert_eq!(failed.z, f64::INFINITY);
    }
    assert_eq!(coords[1], b);

    // A constant z: the C test passes one height `h = 27` for both points (stride 0). A Rust
    // point carries its own z, so the constant is the z of each point. The C z buffers
    // (`obs[i].z` kept at 45 and 50, `h` written back) have no counterpart.
    let mut constant = [
        Xyzt {
            x: 12f64.to_radians(),
            y: 55f64.to_radians(),
            z: 27.0,
            t: 0.0,
        },
        Xyzt {
            x: 12f64.to_radians(),
            y: 56f64.to_radians(),
            z: 27.0,
            t: 0.0,
        },
    ];
    utm32
        .transform_array(Direction::Forward, &mut constant)
        .expect("constant z");
    assert_eq!(constant[0].x, a.x);
    assert_eq!(constant[0].y, a.y);
    assert_eq!(constant[1].x, b.x);
    assert_eq!(constant[1].y, b.y);
    assert_eq!(constant[0].z, 27.0);
    assert_ne!(constant[1].z, 50.0);

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

/// `gie.proj_trans_with_a_crs`: `proj_trans` on a CRS object (not a transform) returns
/// `HUGE_VAL` and sets `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`. `HUGE_VAL` is the `Err`.
#[test]
fn trans_with_a_crs_is_an_error() {
    let crs = Proj::new("EPSG:4326").expect("EPSG:4326 is a CRS");
    let error = crs
        .convert_xyzt(0.0, 0.0, 0.0, 0.0)
        .expect_err("a CRS has no image");
    assert!(matches!(error, ProjError::NotATransform), "{error:?}");
    assert_eq!(error.code(), PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE);
}

/// `gie.proj_create_crs_to_crs_from_pj_force_over`, assertion by assertion.
///
/// The C input is latitude first and this crate takes longitude, latitude, so each point
/// is written (longitude, latitude): the C `(lat 0, long 140)` is `(140, 0)`, and the C
/// `(lat 0, long -220)` is `(-220, 0)`. The C `t = HUGE_VAL` is not passed: no time is read
/// here. The context calls and `proj_destroy` are `Drop`. `proj_normalize_for_visualization`
/// is the identity on this facade, whose pair is already in longitude, latitude order, so
/// `Pnormalized` is `P` itself. The count check on the alternatives (`> 1`) is the C
/// comment "more than one coordinate operation candidate", written as an assertion.
#[test]
fn proj_create_crs_to_crs_from_pj_force_over() {
    let epsg27700 = Proj::new("EPSG:27700").expect("EPSG:27700");
    let epsg4326 = Proj::new("EPSG:4326").expect("EPSG:4326");
    let epsg3857 = Proj::new("EPSG:3857").expect("EPSG:3857");

    {
        // FORCE_OVER=YES, EPSG:4326 to EPSG:3857.
        let p = epsg4326
            .create_crs_to_crs_from_pj(&epsg3857, None, Some(vec!["FORCE_OVER=YES"]))
            .expect("P");
        assert_eq!(p.force_over(), ForceOver::Yes); // ASSERT_TRUE(P->over)

        let (x, y, _) = p.convert_xyz(140.0, 0.0, 0.0).expect("output");
        let (x_over, y_over, _) = p.convert_xyz(-220.0, 0.0, 0.0).expect("output_over");
        let p_clone = p.clone();
        let (x_clone_over, _, _) = p_clone.convert_xyz(-220.0, 0.0, 0.0).expect("clone");
        let (lon_inv, lat_inv): (f64, f64) = p.project((x, y), true).expect("input_inv");
        let (lon_over_inv, lat_over_inv): (f64, f64) =
            p.project((x_over, y_over), true).expect("input_over_inv");

        // Web Mercator x's between 0 and 180 longitude come out positive.
        // But when forcing the over flag, the -220 calculation makes it flip.
        assert!(x > 0.0, "{x}"); // EXPECT_GT(output.xyz.x, 0)
        assert!(x_over < 0.0, "{x_over}"); // EXPECT_LT(output_over.xyz.x, 0)

        assert!((x - 15_584_728.711_058_298).abs() <= 1e-8, "{x}");
        assert!((x_over - -24_490_287.974_520_184).abs() <= 1e-8, "{x_over}");

        assert_eq!(x_clone_over, x_over); // EXPECT_EQ(output_clone_over, output_over)

        // The distance from 140 to 180 and -220 to -180 should be pretty much the same.
        let dx_o = (x - 20_037_508.342_789_244).abs();
        let dx_over = (x_over + 20_037_508.342_789_244).abs();
        let dx = (dx_o - dx_over).abs();
        assert!((dx - 0.0).abs() <= 1e-8, "{dx}");

        // The inverse operations get back close to the input: C input.xyz.x is the
        // latitude, input.xyz.y the longitude.
        assert!((0.0 - lat_inv).abs() <= 1e-8, "{lat_inv}");
        assert!((140.0 - lon_inv).abs() <= 1e-8, "{lon_inv}");
        assert!((0.0 - lat_over_inv).abs() <= 1e-8, "{lat_over_inv}");
        assert!((-220.0 - lon_over_inv).abs() <= 1e-8, "{lon_over_inv}");

        let normalized = &p;
        assert_eq!(normalized.force_over(), ForceOver::Yes); // ASSERT_TRUE(Pnormalized->over)
        let (x_over_normalized, _, _) = normalized
            .convert_xyz(-220.0, 0.0, 0.0)
            .expect("output_over_normalized");
        assert!(
            (x_over_normalized - -24_490_287.974_520_184).abs() <= 1e-8,
            "{x_over_normalized}"
        );
    }

    {
        // FORCE_OVER=NO: the flag stays off, the longitudes wrap.
        let p = epsg4326
            .create_crs_to_crs_from_pj(&epsg3857, None, Some(vec!["FORCE_OVER=NO"]))
            .expect("P");
        assert_eq!(p.force_over(), ForceOver::No); // ASSERT_FALSE(P->over)

        let (x, _, _) = p.convert_xyz(140.0, 0.0, 0.0).expect("output");
        let (x_not_over, _, _) = p.convert_xyz(-220.0, 0.0, 0.0).expect("output_notOver");
        assert!(x > 0.0, "{x}"); // EXPECT_GT(output.xyz.x, 0)
        assert!(x_not_over > 0.0, "{x_not_over}"); // EXPECT_GT(output_notOver.xyz.x, 0)
        assert!((x - 15_584_728.711_058_298).abs() <= 1e-8, "{x}");
        assert!(
            (x_not_over - 15_584_728.711_058_298).abs() <= 1e-8,
            "{x_not_over}"
        );
    }

    {
        // No options: the same as FORCE_OVER=NO.
        let p = epsg4326
            .create_crs_to_crs_from_pj(&epsg3857, None, None)
            .expect("P");
        assert_eq!(p.force_over(), ForceOver::No); // ASSERT_FALSE(P->over)

        let (x, _, _) = p.convert_xyz(140.0, 0.0, 0.0).expect("output");
        let (x_not_over, _, _) = p.convert_xyz(-220.0, 0.0, 0.0).expect("output_notOver");
        assert!(x > 0.0, "{x}"); // EXPECT_GT(output.xyz.x, 0)
        assert!(x_not_over > 0.0, "{x_not_over}"); // EXPECT_GT(output_notOver.xyz.x, 0)
        assert!((x - 15_584_728.711_058_298).abs() <= 1e-8, "{x}");
        assert!(
            (x_not_over - 15_584_728.711_058_298).abs() <= 1e-8,
            "{x_not_over}"
        );
    }

    {
        // EPSG:4326 to EPSG:27700 has more than one coordinate operation candidate.
        let p = epsg4326
            .create_crs_to_crs_from_pj(&epsg27700, None, Some(vec!["FORCE_OVER=YES"]))
            .expect("P");
        assert_eq!(p.force_over(), ForceOver::Yes); // ASSERT_TRUE(P->over)

        let (x, y, _) = p.convert_xyz(140.0, 0.0, 0.0).expect("output");
        let (x_over, y_over, _) = p.convert_xyz(-220.0, 0.0, 0.0).expect("output_over");

        // Doesn't actually change the result for this tmerc transformation.
        assert!((x - 4_980_122.749_364_435).abs() <= 1e-8, "{x}");
        assert!((y - 14_467_212.882_603_768).abs() <= 1e-8, "{y}");
        assert!((x_over - 4_980_122.749_364_435).abs() <= 1e-8, "{x_over}");
        assert!((y_over - 14_467_212.882_603_768).abs() <= 1e-8, "{y_over}");

        let normalized = &p;
        assert_eq!(normalized.force_over(), ForceOver::Yes); // ASSERT_TRUE(Pnormalized->over)
        let alternatives = normalized.alternatives_force_over();
        assert!(alternatives.len() > 1, "{}", alternatives.len());
        for over in alternatives {
            assert_eq!(over, ForceOver::Yes); // ASSERT_TRUE(op.pj->over)
        }

        let (x_over_normalized, y_over_normalized, _) = normalized
            .convert_xyz(-220.0, 0.0, 0.0)
            .expect("output_over_normalized");
        assert!(
            (x_over_normalized - 4_980_122.749_364_435).abs() <= 1e-8,
            "{x_over_normalized}"
        );
        assert!(
            (y_over_normalized - 14_467_212.882_603_768).abs() <= 1e-8,
            "{y_over_normalized}"
        );
    }

    {
        // Negative test for 27700: FORCE_OVER=NO.
        let p = epsg4326
            .create_crs_to_crs_from_pj(&epsg27700, None, Some(vec!["FORCE_OVER=NO"]))
            .expect("P");
        assert_eq!(p.force_over(), ForceOver::No); // ASSERT_FALSE(P->over)

        let (x, y, _) = p.convert_xyz(140.0, 0.0, 0.0).expect("output");
        let (x_over, y_over, _) = p.convert_xyz(-220.0, 0.0, 0.0).expect("output_over");
        assert!((x - 4_980_122.749_364_435).abs() <= 1e-8, "{x}");
        assert!((y - 14_467_212.882_603_768).abs() <= 1e-8, "{y}");
        assert!((x_over - 4_980_122.749_364_435).abs() <= 1e-8, "{x_over}");
        assert!((y_over - 14_467_212.882_603_768).abs() <= 1e-8, "{y_over}");
    }
}

/// `gieTest, proj_create_crs_to_crs` (TEST_F). The CRS-name assertions of the C test
/// are not here: this crate has no `proj_get_name` on a transform.
#[test]
fn proj_create_crs_to_crs() {
    let p = Proj::new_known_crs("EPSG:25832", "EPSG:25833", None).expect("25832 to 25833");
    let (x, y, _) = p.convert_xyz(700_000.0, 6_000_000.0, 0.0).expect("convert");
    assert!((x - 307_788.876_117_105_7).abs() <= 1e-8, "{x}");
    assert!((y - 5_999_669.303_603_763).abs() <= 1e-8, "{y}");

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

const TC32_UTM32: &str = " +proj=horner +ellps=intl +range=500000 +fwd_origin=877605.269066,6125810.306769 +inv_origin=877605.760036,6125811.281773 +deg=4 +fwd_v=6.1258112678e+06,9.9999971567e-01,1.5372750011e-10,5.9300860915e-15,2.2609497633e-19,4.3188227445e-05,2.8225130416e-10,7.8740007114e-16,-1.7453997279e-19,1.6877465415e-10,-1.1234649773e-14,-1.7042333358e-18,-7.9303467953e-15,-5.2906832535e-19,3.9984284847e-19 +fwd_u=8.7760574982e+05,9.9999752475e-01,2.8817299305e-10,5.5641310680e-15,-1.5544700949e-18,-4.1357045890e-05,4.2106213519e-11,2.8525551629e-14,-1.9107771273e-18,3.3615590093e-10,2.4380247154e-14,-2.0241230315e-18,1.2429019719e-15,5.3886155968e-19,-1.0167505000e-18 +inv_v=6.1258103208e+06,1.0000002826e+00,-1.5372762184e-10,-5.9304261011e-15,-2.2612705361e-19,-4.3188331419e-05,-2.8225549995e-10,-7.8529116371e-16,1.7476576773e-19,-1.6875687989e-10,1.1236475299e-14,1.7042518057e-18,7.9300735257e-15,5.2881862699e-19,-3.9990736798e-19 +inv_u=8.7760527928e+05,1.0000024735e+00,-2.8817540032e-10,-5.5627059451e-15,1.5543637570e-18,4.1357152105e-05,-4.2114813612e-11,-2.8523713454e-14,1.9109017837e-18,-3.3616407783e-10,-2.4382678126e-14,2.0245020199e-18,-1.2441377565e-15,-5.3885232238e-19,1.0167203661e-18";
const SB_UTM32: &str = " +proj=horner +ellps=intl +range=500000 +tolerance=0.0005 +fwd_origin=4.94690026817276e+05,6.13342113183056e+06 +inv_origin=6.19480258923588e+05,6.13258568148837e+06 +deg=3 +fwd_c=6.13258562111350e+06,6.19480105709997e+05,9.99378966275206e-01,-2.82153291753490e-02,-2.27089979140026e-10,-1.77019590701470e-09,1.08522286274070e-14,2.11430298751604e-15 +inv_c=6.13342118787027e+06,4.94690181709311e+05,9.99824464710368e-01,2.82279070814774e-02,7.66123542220864e-11,1.78425334628927e-09,-1.05584823306400e-14,-3.32554258683744e-15";
const TC32_UTM32_FWD_ONLY: &str = " +proj=horner +ellps=intl +range=10000000 +fwd_origin=877605.269066,6125810.306769 +deg=4 +fwd_v=6.1258112678e+06,9.9999971567e-01,1.5372750011e-10,5.9300860915e-15,2.2609497633e-19,4.3188227445e-05,2.8225130416e-10,7.8740007114e-16,-1.7453997279e-19,1.6877465415e-10,-1.1234649773e-14,-1.7042333358e-18,-7.9303467953e-15,-5.2906832535e-19,3.9984284847e-19 +fwd_u=8.7760574982e+05,9.9999752475e-01,2.8817299305e-10,5.5641310680e-15,-1.5544700949e-18,-4.1357045890e-05,4.2106213519e-11,2.8525551629e-14,-1.9107771273e-18,3.3615590093e-10,2.4380247154e-14,-2.0241230315e-18,1.2429019719e-15,5.3886155968e-19,-1.0167505000e-18";
const SB_UTM32_FWD_ONLY: &str = " +proj=horner +ellps=intl +range=10000000 +fwd_origin=4.94690026817276e+05,6.13342113183056e+06 +deg=3 +fwd_c=6.13258562111350e+06,6.19480105709997e+05,9.99378966275206e-01,-2.82153291753490e-02,-2.27089979140026e-10,-1.77019590701470e-09,1.08522286274070e-14,2.11430298751604e-15";
const HATT_TO_GGRS: &str = " +proj=horner +ellps=bessel +fwd_origin=0.0, 0.0 +deg=2 +range=10000000 +fwd_u=370552.68, 0.9997155, -1.08e-09, 0.0175123, 2.04e-09, 1.63e-09 +fwd_v=4511927.23, 0.9996979, 5.60e-10, -0.0174755, -1.65e-09, -6.50e-10";

/// `horner_selftest`: two horner polynomials, forward and inverse, and a round trip.
#[test]
fn horner_selftest() {
    let tc32 = Operation::create(TC32_UTM32).expect("tc32");
    let a = xy(878354.8539, 6125305.4245);
    let (_, dist) = tc32
        .roundtrip(Direction::Forward, one(), a)
        .expect("roundtrip");
    assert!(dist <= 0.01, "{dist}");

    let sb = Operation::create(SB_UTM32).expect("sb");
    let a = xy(495136.8544, 6130821.2945);
    let c = xy(620000.0, 6130000.0);
    let b = fwd(&sb, a);
    assert!(xy_dist(b, c) <= 0.001, "{}", xy_dist(b, c));
    let b = inv(&sb, c);
    assert!(xy_dist(b, a) <= 0.001, "{}", xy_dist(b, a));
    let (_, dist) = sb
        .roundtrip(Direction::Forward, one(), a)
        .expect("roundtrip");
    assert!(dist <= 0.01, "{dist}");
}

/// `horner_only_fwd_selftest`: the forward-only polynomials of the C test.
#[test]
fn horner_only_fwd_selftest() {
    let tc32 = Operation::create(TC32_UTM32_FWD_ONLY).expect("tc32 fwd only");
    let a = xy(878354.8539, 6125305.4245);
    let (_, dist) = tc32
        .roundtrip(Direction::Forward, one(), a)
        .expect("roundtrip");
    assert!(dist <= 0.01, "{dist}");

    let hatt = Operation::create(HATT_TO_GGRS).expect("hatt");
    let a = xy(-10157.950, -21121.093);
    let c = xy(360028.794, 4490989.862);
    let b = fwd(&hatt, a);
    assert!(xy_dist(b, c) <= 0.001, "{}", xy_dist(b, c));
    let b = inv(&hatt, c);
    assert!(xy_dist(b, a) <= 0.001, "{}", xy_dist(b, a));
    let (_, dist) = hatt
        .roundtrip(Direction::Forward, one(), a)
        .expect("roundtrip");
    assert!(dist <= 0.01, "{dist}");

    let sb = Operation::create(SB_UTM32_FWD_ONLY).expect("sb fwd only");
    let a = xy(495136.8544, 6130821.2945);
    let c = xy(620000.0, 6130000.0);
    let b = fwd(&sb, a);
    assert!(xy_dist(b, c) <= 0.001, "{}", xy_dist(b, c));
    let b = inv(&sb, c);
    assert!(xy_dist(b, a) <= 0.001, "{}", xy_dist(b, a));
    let (_, dist) = sb
        .roundtrip(Direction::Forward, one(), a)
        .expect("roundtrip");
    assert!(dist <= 0.01, "{dist}");
}

fn xy(x: f64, y: f64) -> Xyzt {
    Xyzt::xy(x, y)
}

/// `proj_factors` (gie.proj_factors): Mercator and UTM, as projection strings. The
/// C test's EPSG CRS blocks and its `+units=ft` block are not here (see the audit).
#[test]
fn proj_factors() {
    let merc = Operation::create("+proj=merc +ellps=WGS84").expect("merc");
    let f = merc
        .factors(12f64.to_radians(), 55f64.to_radians())
        .expect("factors");
    assert!(
        f.angular_distortion.abs() <= 1e-7,
        "{}",
        f.angular_distortion
    );
    assert!(
        (f.meridian_parallel_angle - std::f64::consts::FRAC_PI_2).abs() <= 1e-7,
        "{}",
        f.meridian_parallel_angle
    );
    assert_eq!(f.meridian_convergence, 0.0);

    let utm = Operation::create("+proj=utm +zone=32 +ellps=GRS80").expect("utm");
    let f = utm.factors(9f64.to_radians(), 0.0).expect("factors");
    assert!(
        (f.meridional_scale - 0.9996).abs() <= 1e-8,
        "{}",
        f.meridional_scale
    );
    assert!(
        (f.parallel_scale - 0.9996).abs() <= 1e-8,
        "{}",
        f.parallel_scale
    );
    assert!(
        f.angular_distortion.abs() <= 1e-7,
        "{}",
        f.angular_distortion
    );
    assert!(
        (f.meridian_parallel_angle - std::f64::consts::FRAC_PI_2).abs() <= 1e-7,
        "{}",
        f.meridian_parallel_angle
    );
    assert!(
        (f.areal_scale - 0.999_200_16).abs() <= 1e-7,
        "{}",
        f.areal_scale
    );
}
