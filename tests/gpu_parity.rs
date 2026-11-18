//! The GPU back end against PROJ 9.9.0 and against the CPU path.
//!
//! The expected values of `proj_values_*` are what the reference `cs2cs` 9.9.0 wrote
//! (`tools/build-reference.sh`, `-f %.9f`). The other tests compare the GPU with `convert_array`,
//! which `cli_yaml` and the parity tests check against PROJ.
//!
//! Without an adapter (no GPU, no Vulkan driver) every test returns early, as the CI machines
//! have none. `PROJ_GPU_REQUIRED=1` turns that into a failure; `lavapipe` (Mesa's software
//! Vulkan) is enough to run them.
#![cfg(feature = "gpu")]

mod common;

use common::tolerance::{DEGREES, METRES};
use proj_rs::gpu::{Fallback, GpuContext, GpuError, GpuOptions, Precision};
use proj_rs::{Direction, Proj};

// The tolerance is the parity rule of `docs/PARITY.md`, 1e-6 m and 1e-11 degree: the GPU meets
// it as the CPU does. Measured against PROJ 9.9.0 on lavapipe: at most 1e-7 m in double-float
// and 4e-9 m in `f64`.

/// One context per precision the machine offers, with small blocks so a call crosses several.
fn contexts() -> Vec<GpuContext> {
    let mut found = Vec::new();
    for precision in [Precision::DoubleFloat, Precision::Float64] {
        let options = GpuOptions::default()
            .with_precision(precision)
            .with_min_points(0)
            .with_chunk_points(1000);
        match GpuContext::with_options(options) {
            Ok(context) => found.push(context),
            Err(error) => eprintln!("no {precision:?} context: {error}"),
        }
    }
    assert!(
        !found.is_empty() || std::env::var_os("PROJ_GPU_REQUIRED").is_none(),
        "PROJ_GPU_REQUIRED is set and no GPU context could be made"
    );
    found
}

fn proj(from: &str, to: &str) -> Proj {
    Proj::new_known_crs(from, to, None).expect("pair")
}

/// `points` through the GPU, all of them on the GPU.
fn on_gpu(proj: &Proj, gpu: &GpuContext, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = points.to_vec();
    let (_, report) = proj.convert_array_gpu(&mut out, gpu).expect("gpu");
    assert_eq!(report.gpu_points, points.len(), "{report:?} on {gpu:?}");
    assert_eq!(report.cpu_points, 0, "{report:?}");
    assert_eq!(report.fallback, None);
    out
}

fn assert_close(got: (f64, f64), want: (f64, f64), tolerance: f64, what: &str) {
    let gap = (got.0 - want.0).abs().max((got.1 - want.1).abs());
    assert!(
        gap <= tolerance,
        "{what}: {got:?} against {want:?}, gap {gap:e}"
    );
}

/// An input and what PROJ 9.9.0 gives for it.
type Case = ((f64, f64), (f64, f64));

// Longitude, latitude in; PROJ 9.9.0 `cs2cs -f %.9f` out.
const TO_WEB: [Case; 5] = [
    ((2.3522, 48.8566), (261845.706243938, 6250564.349543125)),
    (
        (151.2093, -33.8688),
        (16832542.27920734, -4011198.647307572),
    ),
    (
        (-122.4194, 37.7749),
        (-13627665.271218073, 4547675.354340558),
    ),
    ((179.9, -80.0), (20026376.393709917, -15538711.09630922)),
    ((0.0, 0.0), (0.0, 0.0)),
];
const TO_UTM31: [Case; 5] = [
    ((2.3522, 48.8566), (452482.532702628, 5411717.1768689)),
    ((5.9, 43.1), (735997.362704527, 4776002.354463808)),
    ((0.5, 49.2), (317885.342807504, 5452697.744582471)),
    ((3.0, -30.0), (500000.000000001, -3318785.352581206)),
    ((4.0, 80.0), (519384.803295973, 8881752.41480744)),
];
const TO_LAMBERT93: [Case; 5] = [
    ((2.3522, 48.8566), (652469.022709136, 6862035.259420079)),
    ((-1.55, 47.2), (355732.089656044, 6687666.973004261)),
    ((7.75, 48.58), (1050163.943556587, 6841622.71548826)),
    ((9.0, 42.0), (1197656.702093824, 6118854.941944701)),
    ((2.5, 51.0), (664824.361988445, 7100575.954104623)),
];
// Easting, northing in; longitude, latitude out (`-f %.13f`).
const FROM_WEB: [Case; 3] = [
    ((261844.4, 6250566.0), (2.3521882658111, 48.8566097549059)),
    (
        (16832000.0, -4009000.0),
        (151.2044286229979, -33.8523990346663),
    ),
    (
        (-13627000.0, 4547000.0),
        (-122.4134237669672, 37.7701044946726),
    ),
];
const FROM_UTM31: [Case; 3] = [
    ((452000.0, 5411000.0), (2.3457062096593, 48.8501116624614)),
    ((700000.0, 4770000.0), (5.4559296968185, 43.0563523847278)),
    ((330000.0, 5450000.0), (0.6672963482537, 49.1792319875852)),
];
const FROM_LAMBERT93: [Case; 3] = [
    ((652000.0, 6862000.0), (2.3458119586807, 48.8562481409627)),
    ((350000.0, 6700000.0), (-1.6351429130654, 47.3078415175247)),
    ((1000000.0, 6300000.0), (6.7244517408986, 43.7353376381924)),
];

fn check_cases(pair: (&str, &str), cases: &[Case], tolerance: f64) {
    let proj = proj(pair.0, pair.1);
    for gpu in contexts() {
        let input: Vec<_> = cases.iter().map(|case| case.0).collect();
        let out = on_gpu(&proj, &gpu, &input);
        let mut worst = 0.0_f64;
        for ((got, case), _) in out.iter().zip(cases).zip(&input) {
            let gap = (got.0 - case.1 .0).abs().max((got.1 - case.1 .1).abs());
            worst = worst.max(gap);
            assert_close(*got, case.1, tolerance, &format!("{pair:?} {:?}", case.0));
        }
        eprintln!("{pair:?} {:?}: worst gap {worst:e}", gpu.precision());
    }
}

#[test]
fn proj_values_web_mercator() {
    check_cases(("EPSG:4326", "EPSG:3857"), &TO_WEB, METRES);
    check_cases(("EPSG:3857", "EPSG:4326"), &FROM_WEB, DEGREES);
}

#[test]
fn proj_values_utm() {
    check_cases(("EPSG:4326", "EPSG:32631"), &TO_UTM31, METRES);
    check_cases(("EPSG:32631", "EPSG:4326"), &FROM_UTM31, DEGREES);
}

#[test]
fn proj_values_lambert93() {
    check_cases(("EPSG:4326", "EPSG:2154"), &TO_LAMBERT93, METRES);
    check_cases(("EPSG:2154", "EPSG:4326"), &FROM_LAMBERT93, DEGREES);
}

// ---- The GPU against the CPU path, on many points ----

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    f64::from(u32::try_from(*state >> 32).expect("32 bits")) / 4_294_967_296.0
}

/// `count` points in the box (lon0, lon1, lat0, lat1), degrees.
fn scatter(count: usize, area: (f64, f64, f64, f64), seed: u64) -> Vec<(f64, f64)> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            let lon = area.0 + (area.1 - area.0) * lcg(&mut state);
            let lat = area.2 + (area.3 - area.2) * lcg(&mut state);
            (lon, lat)
        })
        .collect()
}

/// The CPU path of the same call: the reference.
fn on_cpu(proj: &Proj, points: &[(f64, f64)], way: Direction) -> Vec<(f64, f64)> {
    let mut out = points.to_vec();
    proj.project_array(&mut out, way == Direction::Inverse)
        .expect("cpu");
    out
}

fn same_as_cpu(proj: &Proj, label: &str, points: &[(f64, f64)], way: Direction, tolerance: f64) {
    let cpu = on_cpu(proj, points, way);
    for gpu in contexts() {
        let mut out = points.to_vec();
        let (_, report) = proj.convert_array_gpu_in(&mut out, way, &gpu).expect("gpu");
        assert_eq!(report.gpu_points, points.len(), "{label}: {report:?}");
        assert_eq!(report.fallback, None, "{label}: {report:?}");
        let mut worst = 0.0_f64;
        for (index, (got, want)) in out.iter().zip(&cpu).enumerate() {
            let gap = (got.0 - want.0).abs().max((got.1 - want.1).abs());
            worst = worst.max(gap);
            assert!(
                gap <= tolerance,
                "{label} point {index} {:?}: gpu {got:?}, cpu {want:?}, gap {gap:e} on {gpu:?}",
                points[index]
            );
        }
        eprintln!("{label} {way:?} {:?}: worst gap {worst:e}", gpu.precision());
    }
}

/// Both ways of a pair over a box of geographic points: forward from the points, inverse from
/// what the forward made.
fn round_trip_pair(from: &str, to: &str, area: (f64, f64, f64, f64)) {
    let proj = proj(from, to);
    let points = scatter(5000, area, 0x5eed);
    same_as_cpu(
        &proj,
        &format!("{from} -> {to}"),
        &points,
        Direction::Forward,
        METRES,
    );
    let projected = on_cpu(&proj, &points, Direction::Forward);
    same_as_cpu(
        &proj,
        &format!("{to} -> {from}"),
        &projected,
        Direction::Inverse,
        DEGREES,
    );
}

#[test]
fn web_mercator_over_the_world() {
    round_trip_pair("EPSG:4326", "EPSG:3857", (-180.0, 180.0, -85.0, 85.0));
}

#[test]
fn mercator_near_the_pole_stays_within_the_rule() {
    // Double-float cannot hold 1 micrometre there (the projection multiplies its 48-bit input by
    // R / cos(lat)): the GPU hands those blocks to the CPU. `f64` keeps them.
    let proj = proj("EPSG:4326", "EPSG:3857");
    let points = scatter(4000, (-180.0, 180.0, 85.0, 89.5), 41);
    let cpu = on_cpu(&proj, &points, Direction::Forward);
    for gpu in contexts() {
        let mut out = points.clone();
        let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("pole");
        if gpu.precision() == Precision::DoubleFloat {
            assert!(report.cpu_points > 0, "{report:?}");
        } else {
            assert_eq!(report.cpu_points, 0, "{report:?}");
        }
        for (index, (got, want)) in out.iter().zip(&cpu).enumerate() {
            let gap = (got.0 - want.0).abs().max((got.1 - want.1).abs());
            assert!(
                gap <= METRES,
                "point {index} {:?}: gap {gap:e} {report:?}",
                points[index]
            );
        }
    }
}

#[test]
fn utm_over_its_zone() {
    round_trip_pair("EPSG:4326", "EPSG:32631", (0.0, 6.0, -80.0, 84.0));
}

#[test]
fn utm_south_hemisphere_and_another_zone() {
    round_trip_pair("EPSG:4326", "EPSG:32756", (150.0, 156.0, -80.0, 0.0));
}

#[test]
fn lambert93_over_france() {
    round_trip_pair("EPSG:4326", "EPSG:2154", (-5.5, 10.0, 41.0, 51.5));
}

#[test]
fn two_projections_in_one_pipeline() {
    // The inverse of one and the forward of the other, in a single program.
    let proj = proj("EPSG:2154", "EPSG:32631");
    let lambert = on_cpu(
        &self::proj("EPSG:4326", "EPSG:2154"),
        &scatter(5000, (0.0, 6.0, 42.0, 51.0), 7),
        Direction::Forward,
    );
    same_as_cpu(&proj, "2154 -> 32631", &lambert, Direction::Forward, METRES);
}

#[test]
fn native_proj_strings_in_radians() {
    let cases = [
        ("+proj=utm +zone=31 +ellps=WGS84", (0.0, 6.0, -80.0, 84.0)),
        ("+proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=500000 +ellps=intl", (6.0, 12.0, -60.0, 80.0)),
        (
            "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
            (-5.0, 10.0, 41.0, 51.5),
        ),
        // A cone that opens the other way: n < 0.
        (
            "+proj=lcc +lat_1=-36 +lat_2=-38 +lat_0=-37 +lon_0=145 +x_0=2500000 +y_0=2500000 +ellps=GRS80",
            (140.0, 150.0, -39.5, -33.0),
        ),
        ("+proj=lcc +lat_1=33 +lat_2=45 +lat_0=39 +lon_0=-96 +R=6371000", (-110.0, -80.0, 25.0, 50.0)),
        ("+proj=webmerc +datum=WGS84", (-180.0, 180.0, -80.0, 80.0)),
        ("+proj=merc +a=6371000 +b=6371000 +lon_0=10 +k_0=0.99", (-170.0, 170.0, -80.0, 80.0)),
    ];
    for (definition, area) in cases {
        let proj = Proj::new(definition).expect(definition);
        let radians: Vec<(f64, f64)> = scatter(3000, area, 11)
            .into_iter()
            .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
            .collect();
        same_as_cpu(&proj, definition, &radians, Direction::Forward, METRES);
        let metres = on_cpu(&proj, &radians, Direction::Forward);
        // The inverse gives radians: 1e-13 of a radian is the parity rule of `tolerance.rs`.
        same_as_cpu(&proj, definition, &metres, Direction::Inverse, 1e-13);
    }
}

// ---- What the GPU does not decide ----

fn definite(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    points.to_vec()
}

#[test]
fn a_transform_without_a_kernel_runs_on_the_cpu() {
    // NAD27 to WGS84: the operation is chosen per point, from grids.
    let proj = proj("EPSG:4267", "EPSG:4326");
    let points = scatter(3000, (-100.0, -90.0, 30.0, 40.0), 3);
    let cpu = on_cpu(&proj, &points, Direction::Forward);
    for gpu in contexts() {
        let mut out = definite(&points);
        let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("fallback");
        assert_eq!(report.gpu_points, 0);
        assert_eq!(report.cpu_points, points.len());
        assert_eq!(report.fallback, Some(Fallback::Unsupported));
        assert_eq!(out, cpu, "the CPU path gives the CPU answer bit for bit");
    }
}

#[test]
fn projections_without_a_kernel_are_unsupported() {
    for definition in [
        // The ellipsoidal Mercator inverts by iteration, which the GPU does not run.
        "+proj=merc +ellps=WGS84",
        "+proj=utm +zone=31 +ellps=WGS84 +approx",
        "+proj=stere +lat_0=90 +ellps=WGS84",
        "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +ellps=GRS80 +over",
    ] {
        let proj = Proj::new(definition).expect(definition);
        let radians: Vec<(f64, f64)> = scatter(2000, (1.0, 5.0, 45.0, 48.0), 5)
            .into_iter()
            .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
            .collect();
        let cpu = on_cpu(&proj, &radians, Direction::Forward);
        for gpu in contexts() {
            let mut out = radians.clone();
            let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect(definition);
            assert_eq!(report.fallback, Some(Fallback::Unsupported), "{definition}");
            assert_eq!(out, cpu, "{definition}");
        }
    }
}

#[test]
fn a_small_call_stays_on_the_cpu() {
    let proj = proj("EPSG:4326", "EPSG:32631");
    let points = scatter(100, (0.0, 6.0, 40.0, 50.0), 9);
    let cpu = on_cpu(&proj, &points, Direction::Forward);
    let Ok(gpu) = GpuContext::new() else {
        return;
    };
    assert_eq!(gpu.options().min_points, 1 << 18);
    let mut out = points;
    let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("small");
    assert_eq!(report.fallback, Some(Fallback::TooFewPoints));
    assert_eq!(report.cpu_points, 100);
    assert_eq!(out, cpu);
}

#[test]
fn a_block_with_a_point_the_gpu_does_not_decide_goes_to_the_cpu() {
    let proj = proj("EPSG:4326", "EPSG:32631");
    // Five blocks of 1000. The third holds a NaN and a latitude past the pole by less than the
    // tolerance PROJ clamps; the others are clean.
    let mut points = scatter(5000, (1.0, 5.0, 40.0, 50.0), 21);
    points[2500] = (3.0, f64::NAN);
    let cpu = on_cpu(&proj, &points, Direction::Forward);
    for gpu in contexts() {
        let mut out = points.clone();
        let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("flagged");
        assert_eq!(report.fallback, Some(Fallback::Flagged), "{report:?}");
        assert_eq!(report.gpu_points, 4000);
        assert_eq!(report.cpu_points, 1000);
        for (index, (got, want)) in out.iter().zip(&cpu).enumerate() {
            if index / 1000 == 2 {
                // The CPU block: the CPU answer, NaN included.
                assert!(
                    (got.0 == want.0 || (got.0.is_nan() && want.0.is_nan()))
                        && (got.1 == want.1 || (got.1.is_nan() && want.1.is_nan())),
                    "point {index}: {got:?} against {want:?}"
                );
            } else {
                let gap = (got.0 - want.0).abs().max((got.1 - want.1).abs());
                assert!(gap <= METRES, "point {index}: gap {gap:e}");
            }
        }
    }
}

#[test]
fn an_error_is_the_cpu_error() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    for bad in [(2.0, 95.0), (700.0, 45.0), (f64::INFINITY, 45.0)] {
        let mut points = scatter(3000, (0.0, 5.0, 44.0, 50.0), 2);
        points[1200] = bad;
        let mut reference = points.clone();
        let expected = proj.convert_array(&mut reference).map(|_| ());
        for gpu in contexts() {
            let mut out = points.clone();
            let got = proj.convert_array_gpu(&mut out, &gpu).map(|_| ());
            assert_eq!(format!("{got:?}"), format!("{expected:?}"), "{bad:?}");
            assert!(expected.is_err(), "{bad:?} must fail on the CPU");
            // The first block converted on the GPU, the failing one and the rest untouched:
            // the same contract as `convert_array`, with a block of the size of the context.
            assert_eq!(out[1000..], points[1000..], "{bad:?}");
        }
    }
}

#[test]
fn a_longitude_wraps_as_on_the_cpu() {
    let proj = proj("EPSG:4326", "EPSG:3857");
    // Past the antimeridian, at it, and far out of range but below the 10 radians PROJ refuses.
    let mut points = scatter(2000, (-180.0, 180.0, -80.0, 80.0), 31);
    points[10] = (180.0, 10.0);
    points[11] = (-180.0, 10.0);
    points[12] = (190.0, 10.0);
    points[13] = (-250.0, 10.0);
    points[14] = (540.0, 10.0);
    let cpu = on_cpu(&proj, &points, Direction::Forward);
    for gpu in contexts() {
        let mut out = points.clone();
        let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("wrap");
        for (index, (got, want)) in out.iter().zip(&cpu).enumerate() {
            let gap = (got.0 - want.0).abs().max((got.1 - want.1).abs());
            // A point on the seam may go either way on the CPU; the block then ran there.
            assert!(
                gap <= METRES || report.fallback == Some(Fallback::Flagged),
                "point {index} {:?}: gap {gap:e} {report:?}",
                points[index]
            );
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "the test builds single-precision input on purpose"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[test]
fn single_precision_slices() {
    let proj = proj("EPSG:4326", "EPSG:32631");
    let points: Vec<(f32, f32)> = scatter(3000, (0.0, 6.0, 30.0, 70.0), 13)
        .into_iter()
        .map(|(lon, lat)| (narrow(lon), narrow(lat)))
        .collect();
    let mut cpu = points.clone();
    proj.convert_array(&mut cpu).expect("cpu");
    for gpu in contexts() {
        let mut out = points.clone();
        let (_, report) = proj.convert_array_gpu(&mut out, &gpu).expect("gpu");
        assert_eq!(report.gpu_points, points.len());
        for (got, want) in out.iter().zip(&cpu) {
            // The same f64 rounded to f32: at most the neighbouring float.
            assert!(
                (got.0 - want.0).abs() <= f32::EPSILON * want.0.abs(),
                "{got:?} {want:?}"
            );
            assert!(
                (got.1 - want.1).abs() <= f32::EPSILON * want.1.abs(),
                "{got:?} {want:?}"
            );
        }
    }
}

#[test]
fn the_float64_request_is_honest() {
    match GpuContext::with_options(GpuOptions::default().with_precision(Precision::Float64)) {
        Ok(gpu) => assert_eq!(gpu.precision(), Precision::Float64),
        Err(GpuError::Float64Unavailable | GpuError::NoAdapter(_)) => {}
        Err(other) => panic!("unexpected {other}"),
    }
}

#[test]
fn the_default_picks_the_best_precision() {
    let Ok(gpu) = GpuContext::new() else {
        return;
    };
    eprintln!("{gpu:?}");
    assert!(matches!(
        gpu.precision(),
        Precision::Float64 | Precision::DoubleFloat
    ));
    assert!(gpu.chunk_points() >= 64);
}
