//! `Proj::convert_array` and `project_array` send the slice through the transform at once
//! (`Step::forward_batch`), so the vector kernels of `crate::simd` serve the facade. The contract
//! is the one of the points taken one at a time with `convert`:
//!
//! - the values agree to the tolerance of the vector kernels (`docs/PORTING-simd.md`), and with
//!   the PROJ 9.9.0 `cs2cs`, which is the reference;
//! - a failing point gives the error `convert` gives for it, the first failing point's when
//!   several fail. The slice is then partly converted: whole blocks of 1024 points before the
//!   first failing one are converted, and the rest keeps its input (`docs/PORTING-memory.md`).

use proj_rs::{Proj, ProjError};
use std::io::Write;
use std::process::{Command, Stdio};

/// Agreement with the points one at a time, in the unit of the output: metres or degrees.
const METRES: f64 = 1e-7;
const DEGREES: f64 = 1e-11;

/// `count` points of a box, the same every time.
fn sample(count: usize, lon: (f64, f64), lat: (f64, f64)) -> Vec<(f64, f64)> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut unit = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the top 53 bits of the generator are exact in f64"
        )]
        let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    };
    (0..count)
        .map(|_| {
            (
                lon.0 + unit() * (lon.1 - lon.0),
                lat.0 + unit() * (lat.1 - lat.0),
            )
        })
        .collect()
}

fn france(count: usize) -> Vec<(f64, f64)> {
    sample(count, (-4.0, 8.0), (42.0, 51.0))
}

/// The pairs the facade builds as one operation, and one with the candidates of `proj_trans`.
/// `unit` is the tolerance of the output: metres, or degrees for a geographic target.
const PAIRS: &[(&str, &str, f64)] = &[
    ("EPSG:4326", "EPSG:2154", METRES),
    ("EPSG:4326", "EPSG:32631", METRES),
    ("EPSG:4326", "EPSG:3857", METRES),
    ("EPSG:4326", "EPSG:3035", METRES),
    ("EPSG:2154", "EPSG:4326", DEGREES),
    ("EPSG:2154", "EPSG:32631", METRES),
    ("EPSG:3857", "EPSG:4326", DEGREES),
    ("EPSG:4267", "EPSG:4326", DEGREES),
];

/// The input of a pair: geographic degrees, or metres for a projected source (Lambert-93 for
/// 2154, Web Mercator for 3857).
fn input(from: &str, count: usize) -> Vec<(f64, f64)> {
    let geographic = france(count);
    let to = match from {
        "EPSG:2154" => "EPSG:2154",
        "EPSG:3857" => "EPSG:3857",
        _ => return geographic,
    };
    let to_metres = Proj::new_known_crs("EPSG:4326", to, None).expect("4326 to the source");
    geographic
        .iter()
        .map(|&p| to_metres.convert(p).expect("inside France"))
        .collect()
}

fn one_by_one(proj: &Proj, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    points
        .iter()
        .map(|&p| proj.convert(p).expect("one point"))
        .collect()
}

fn worst(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| (x.0 - y.0).abs().max((x.1 - y.1).abs()))
        .fold(0.0, f64::max)
}

#[test]
fn convert_array_agrees_with_convert_on_every_pair() {
    for &(from, to, tolerance) in PAIRS {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        // 9 001 points: more than two chunks of the facade, and not a multiple of a lane count.
        let points = input(from, 9_001);
        let want = one_by_one(&proj, &points);
        let mut got = points;
        proj.convert_array(&mut got).expect("convert_array");
        let gap = worst(&got, &want);
        assert!(gap < tolerance, "{from} to {to}: gap {gap:e}");
    }
}

#[test]
fn project_array_inverse_agrees_with_project() {
    for &(from, to) in &[("EPSG:4326", "EPSG:2154"), ("EPSG:4326", "EPSG:32631")] {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        let projected = one_by_one(&proj, &france(3_001));
        let want: Vec<(f64, f64)> = projected
            .iter()
            .map(|&p| proj.project(p, true).expect("inverse"))
            .collect();
        let mut got = projected;
        proj.project_array(&mut got, true).expect("project_array");
        let gap = worst(&got, &want);
        assert!(gap < DEGREES, "{from} to {to} inverse: gap {gap:e}");
    }
}

#[test]
fn f32_points_agree_with_convert() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    let points: Vec<(f32, f32)> = france(1_001)
        .into_iter()
        .map(|(lon, lat)| (narrow(lon), narrow(lat)))
        .collect();
    let want: Vec<(f32, f32)> = points
        .iter()
        .map(|&p| proj.convert(p).expect("one point"))
        .collect();
    let mut got = points;
    proj.convert_array(&mut got).expect("convert_array");
    for (g, w) in got.iter().zip(&want) {
        // One f32 step is about 0.06 m at 7e6: the two paths round the same f64 result.
        assert!(
            (g.0 - w.0).abs() <= 0.1 && (g.1 - w.1).abs() <= 0.1,
            "{g:?} vs {w:?}"
        );
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "the test narrows an f64 degree to the f32 it feeds"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[test]
fn a_classic_operation_agrees_with_convert() {
    // `Proj::new` of a PROJ string: angles in radians, no facade axis order.
    for definition in [
        "+proj=utm +zone=31 +ellps=WGS84",
        "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
        "+proj=merc +ellps=WGS84",
        "+proj=tmerc +lat_0=40 +lon_0=3 +k=0.9996 +x_0=500000 +y_0=100000 +ellps=GRS80",
    ] {
        let proj = Proj::new(definition).expect("operation");
        let radians: Vec<(f64, f64)> = france(2_001)
            .into_iter()
            .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
            .collect();
        let want = one_by_one(&proj, &radians);
        let mut got = radians;
        proj.convert_array(&mut got).expect("convert_array");
        let gap = worst(&got, &want);
        assert!(gap < METRES, "{definition}: gap {gap:e}");
    }
}

#[test]
fn a_pipeline_that_keeps_a_stack_agrees_with_convert() {
    // `push` and `pop` keep a coordinate stack: the points go through one at a time.
    let proj = Proj::new(
        "+proj=pipeline +step +proj=push +v_3 +step +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=pop +v_3",
    )
    .expect("pipeline");
    let radians: Vec<(f64, f64)> = france(777)
        .into_iter()
        .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
        .collect();
    let want = one_by_one(&proj, &radians);
    let mut got = radians;
    proj.convert_array(&mut got).expect("convert_array");
    assert!(worst(&got, &want) < METRES);
}

/// Datum shifts: `cart`, `helmert` and `push`/`pop` run over the slice (`docs/PORTING-batch-datum.md`).
/// Each pair is one operation, forward and back, over a box inside its area of use.
#[test]
fn datum_shifts_agree_with_convert_both_ways() {
    type Box2 = ((f64, f64), (f64, f64));
    let pairs: [(&str, Box2); 5] = [
        ("EPSG:2100", ((20.0, 28.0), (35.0, 41.0))),
        ("EPSG:28992", ((3.4, 7.2), (50.8, 53.5))),
        ("EPSG:2056", ((6.0, 10.4), (45.8, 47.8))),
        ("EPSG:2169", ((5.8, 6.5), (49.4, 50.2))),
        ("EPSG:5514", ((12.1, 18.8), (48.6, 51.1))),
    ];
    for (target, (lon, lat)) in pairs {
        let forward = Proj::new_known_crs("EPSG:4326", target, None).expect(target);
        let points = sample(9_001, lon, lat);
        let want = one_by_one(&forward, &points);
        let mut got = points;
        forward.convert_array(&mut got).expect("forward");
        let gap = worst(&got, &want);
        assert!(gap < METRES, "4326 to {target}: gap {gap:e}");

        let back = Proj::new_known_crs(target, "EPSG:4326", None).expect(target);
        let want_back = one_by_one(&back, &want);
        let mut got_back = want;
        back.convert_array(&mut got_back).expect("inverse");
        let gap = worst(&got_back, &want_back);
        assert!(gap < DEGREES, "{target} to 4326: gap {gap:e}");
    }
}

/// A pop that finds no push leaves the coordinate, and a pipeline with several pushes gives each
/// pop the value of its own push, for every point of the slice.
#[test]
fn push_and_pop_over_a_slice_agree_with_convert() {
    for definition in [
        "+proj=pipeline +step +proj=pop +v_1 +step +proj=push +v_1 +step +proj=utm +zone=31 \
         +ellps=WGS84",
        "+proj=pipeline +step +proj=push +v_1 +step +proj=push +v_2 +step +proj=utm +zone=31 \
         +ellps=WGS84 +step +proj=pop +v_2 +step +proj=pop +v_1",
        "+proj=pipeline +step +proj=push +v_1 +v_2 +step +proj=utm +zone=31 +ellps=WGS84 \
         +step +inv +proj=utm +zone=31 +ellps=WGS84 +step +proj=pop +v_2 +v_1",
    ] {
        let proj = Proj::new(definition).expect(definition);
        let radians: Vec<(f64, f64)> = france(1_001)
            .into_iter()
            .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
            .collect();
        let want = one_by_one(&proj, &radians);
        let mut got = radians;
        proj.convert_array(&mut got).expect("convert_array");
        assert!(worst(&got, &want) < METRES, "{definition}");
    }
}

/// A point refused between a `push` and its `pop` is the failing point of the slice, and does
/// not shift the values the other points get back.
#[test]
fn a_failing_point_between_push_and_pop_gives_its_error_and_spares_the_others() {
    let proj = Proj::new(
        "+proj=pipeline +step +proj=push +v_1 +step +proj=utm +zone=31 +ellps=WGS84 \
         +step +proj=pop +v_1",
    )
    .expect("pipeline");
    let mut points: Vec<(f64, f64)> = france(300)
        .into_iter()
        .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
        .collect();
    let refused = (0.05, 2.0);
    points[7] = refused;
    let want = message(&proj.convert(refused).expect_err("refused"));
    let mut got = points.clone();
    let error = proj.convert_array(&mut got).expect_err("refused");
    assert_eq!(message(&error), want);
    let mut clean = points;
    clean.remove(7);
    let again = one_by_one(&proj, &clean);
    let mut batch = clean;
    proj.convert_array(&mut batch)
        .expect("without the refused point");
    assert!(worst(&batch, &again) < METRES);
}

/// `+towgs84` on a classic projection: `cart`, `helmert` and `cart` around it, over the slice,
/// and the axis order of `+axis` after it.
#[test]
fn towgs84_on_a_projection_agrees_with_convert_both_ways() {
    for (definition, lon, lat) in [
        (
            "+proj=utm +zone=34 +ellps=GRS80 +towgs84=199.87,-74.79,-246.62",
            (20.0, 28.0),
            (35.0, 41.0),
        ),
        (
            "+proj=utm +zone=31 +ellps=intl +towgs84=-106.8686,52.2978,-103.7239,0.3366,-0.457,1.8422,-1.2747 +axis=neu",
            (0.0, 6.0),
            (46.0, 51.0),
        ),
    ] {
        let proj = Proj::new(definition).expect(definition);
        let radians: Vec<(f64, f64)> = sample(2_001, lon, lat)
            .into_iter()
            .map(|(lon, lat)| (lon.to_radians(), lat.to_radians()))
            .collect();
        let want = one_by_one(&proj, &radians);
        let mut got = radians;
        proj.convert_array(&mut got).expect("forward");
        assert!(worst(&got, &want) < METRES, "{definition}");
        let want_back: Vec<(f64, f64)> = want
            .iter()
            .map(|&p| proj.project(p, true).expect("inverse"))
            .collect();
        let mut got_back = want;
        proj.project_array(&mut got_back, true).expect("inverse");
        assert!(worst(&got_back, &want_back) < 1e-12, "{definition}: back");
    }
}

/// The bits of every value: a `NaN` input compares equal to itself.
fn bits(points: &[(f64, f64)]) -> Vec<(u64, u64)> {
    points
        .iter()
        .map(|&(x, y)| (x.to_bits(), y.to_bits()))
        .collect()
}

fn message(error: &ProjError) -> String {
    error.to_string()
}

/// Points `convert_array` hands to the transform at a time (`FILL_CHUNK` of `src/proj.rs`).
const BLOCK: usize = 1_024;

#[test]
fn a_failing_point_gives_the_error_of_convert_and_converts_whole_blocks_before_it() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    // `convert` refuses these: latitude beyond the pole, a longitude beyond ten radians, and a
    // coordinate that is infinite. A NaN is not refused: PROJ answers NaN (see below).
    let bad = [(3.0, 91.0), (1000.0, 45.0), (3.0, f64::INFINITY)];
    for (index, &point) in bad.iter().enumerate() {
        let want = message(&proj.convert(point).expect_err("convert refuses it"));
        // At the start, in the middle of a lane group, in a later block, and at the end.
        for position in [0, 5, 1_023, 1_024, 1_027, 4_096, 9_999] {
            let mut points = france(10_000);
            points[position] = point;
            let before = bits(&points);
            let error = proj
                .convert_array(&mut points)
                .expect_err("batch refuses it");
            assert_eq!(message(&error), want, "bad point {index} at {position}");
            let after = bits(&points);
            // The blocks before the failing one are converted, as `convert` converts them.
            let done = position / BLOCK * BLOCK;
            let mut original = france(10_000);
            original.truncate(done);
            let converted = one_by_one(&proj, &original);
            let gap = worst(&points[..done], &converted);
            assert!(gap < METRES, "converted blocks: gap {gap:e}");
            // The failing block and the ones after it keep their input.
            assert_eq!(
                after[done..],
                before[done..],
                "bad point {index} at {position}"
            );
        }
    }
}

#[test]
fn a_nan_point_gives_what_convert_gives() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    let nan = (f64::NAN, 45.0);
    let want = proj.convert(nan).expect("PROJ answers NaN for a NaN input");
    let mut points = france(100);
    points[37] = nan;
    proj.convert_array(&mut points).expect("convert_array");
    assert_eq!(bits(&points[37..38]), bits(&[want]));
}

#[test]
fn the_first_failing_point_gives_the_error() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    let first = (3.0, 91.0);
    let second = (3.0, f64::INFINITY);
    let want = message(&proj.convert(first).expect_err("refused"));
    assert_ne!(
        want,
        message(&proj.convert(second).expect_err("refused")),
        "the two errors differ, so the order shows"
    );
    let mut points = france(100);
    points[10] = first;
    points[60] = second;
    let error = proj.convert_array(&mut points).expect_err("refused");
    assert_eq!(message(&error), want);
}

#[test]
fn an_empty_slice_is_converted_and_a_crs_is_not_a_transform() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    let mut none: Vec<(f64, f64)> = Vec::new();
    assert!(proj.convert_array(&mut none).expect("empty").is_empty());
    let crs = Proj::new("EPSG:4326").expect("a CRS");
    let mut empty: Vec<(f64, f64)> = Vec::new();
    assert!(crs.convert_array(&mut empty).is_ok());
    let mut one = vec![(3.0, 45.0)];
    assert!(matches!(
        crs.convert_array(&mut one),
        Err(ProjError::NotATransform)
    ));
}

#[test]
fn lengths_around_the_chunk_sizes() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None).expect("pair");
    for count in [1, 2, 3, 4, 5, 255, 256, 257, 4_095, 4_096, 4_097] {
        let points = france(count);
        let want = one_by_one(&proj, &points);
        let mut got = points;
        proj.convert_array(&mut got).expect("convert_array");
        assert!(worst(&got, &want) < METRES, "{count} points");
    }
}

/// The PROJ `cs2cs` on `PATH`, if there is one. `PROJ_CS2CS_REQUIRED` turns a missing binary into
/// a failure, as the CI does.
fn proj_cs2cs() -> Option<std::path::PathBuf> {
    // `cargo test` puts `target/` on `PATH`: our own `cs2cs` must not shadow the reference.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = if cfg!(windows) { "cs2cs.exe" } else { "cs2cs" };
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|dir| {
            if dir.starts_with(&manifest) {
                return None;
            }
            let candidate = dir.join(name);
            candidate.is_file().then_some(candidate)
        })
    });
    if found.is_none() {
        assert!(
            std::env::var_os("PROJ_CS2CS_REQUIRED").is_none(),
            "PROJ_CS2CS_REQUIRED is set but no PROJ cs2cs is on PATH"
        );
        drop(writeln!(
            std::io::stderr(),
            "skipped: no PROJ cs2cs on PATH"
        ));
    }
    found
}

fn cs2cs(
    program: &std::path::Path,
    from: &str,
    to: &str,
    points: &[(f64, f64)],
) -> Vec<(f64, f64)> {
    let mut child = Command::new(program)
        .args(["-d", "9", from, "+to", to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cs2cs is installed");
    let text: String = points
        .iter()
        .map(|(x, y)| format!("{x:.15} {y:.15}\n"))
        .collect();
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(text.as_bytes())
        .expect("write");
    let done = child.wait_with_output().expect("wait");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    String::from_utf8(done.stdout)
        .expect("utf8")
        .lines()
        .map(|line| {
            let mut columns = line.split_whitespace();
            let x = columns.next().expect("x").parse::<f64>().expect("number");
            let y = columns.next().expect("y").parse::<f64>().expect("number");
            (x, y)
        })
        .collect()
}

#[test]
fn convert_array_matches_cs2cs() {
    let Some(program) = proj_cs2cs() else {
        return;
    };
    // Longitude first on both sides: `cs2cs` reads the authority order, so `-d` and the
    // `+proj=pipeline` text of the facade order are used instead of the EPSG codes.
    let cases: &[(&str, &str, &str, &str)] = &[
        (
            "EPSG:4326",
            "EPSG:2154",
            "+proj=longlat +ellps=WGS84",
            "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80 +towgs84=0,0,0",
        ),
        (
            "EPSG:4326",
            "EPSG:32631",
            "+proj=longlat +ellps=WGS84",
            "+proj=utm +zone=31 +ellps=WGS84",
        ),
    ];
    for &(from, to, proj_from, proj_to) in cases {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        let points = france(2_003);
        let mut got = points.clone();
        proj.convert_array(&mut got).expect("convert_array");
        let want = cs2cs(&program, proj_from, proj_to, &points);
        let gap = worst(&got, &want);
        assert!(gap < 1e-6, "{from} to {to} against cs2cs: gap {gap:e} m");
    }
}
