//! `Proj::convert_iter`: the items are those of `points.map(|p| convert(p))`. A value agrees to
//! the tolerance of the vector kernels (`convert_array`), an error is the one `convert` gives,
//! and the iterator goes on after an error. It takes its input a block at a time, not all at once.

use proj_rs::{Proj, ProjError};
use std::cell::Cell;

/// Agreement with `convert`, in the unit of the output: metres or degrees (`convert_array`'s).
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

/// Items of `convert_iter` against `convert`, point by point.
fn assert_same_items(proj: &Proj, points: &[(f64, f64)], tolerance: f64) {
    let one: Vec<Result<(f64, f64), ProjError>> = points.iter().map(|&p| proj.convert(p)).collect();
    let many: Vec<Result<(f64, f64), ProjError>> =
        proj.convert_iter(points.iter().copied()).collect();
    assert_eq!(one.len(), many.len(), "one item per point");
    for (index, (a, b)) in one.iter().zip(&many).enumerate() {
        match (a, b) {
            (Ok(a), Ok(b)) => assert!(
                (a.0 - b.0).abs() <= tolerance && (a.1 - b.1).abs() <= tolerance,
                "point {index}: convert {a:?}, convert_iter {b:?}"
            ),
            (Err(a), Err(b)) => assert_eq!(
                format!("{a:?}"),
                format!("{b:?}"),
                "point {index}: the error differs"
            ),
            _ => panic!("point {index}: convert {a:?}, convert_iter {b:?}"),
        }
    }
}

#[test]
fn items_match_convert_for_a_known_crs_pair() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    assert_same_items(&proj, &sample(3_000, (-4.5, 8.0), (41.5, 51.0)), METRES);
}

#[test]
fn items_match_convert_for_several_candidates() {
    let proj = Proj::new_known_crs("EPSG:4267", "EPSG:4326", None).expect("4267 to 4326");
    assert_same_items(
        &proj,
        &sample(2_500, (-100.0, -60.0), (25.0, 50.0)),
        DEGREES,
    );
}

#[test]
fn items_match_convert_for_a_classic_projection() {
    let proj = Proj::new("+proj=merc +ellps=WGS84 +units=m").expect("merc");
    let radians = sample(2_500, (-3.0, 3.0), (-1.4, 1.4));
    assert_same_items(&proj, &radians, METRES);
}

#[test]
fn a_failing_point_gives_its_own_error_and_the_stream_goes_on() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    let mut points = sample(3_000, (-4.5, 8.0), (41.5, 51.0));
    points[700] = (3.0, 95.0);
    points[1_200] = (3.0, f64::INFINITY);
    assert_same_items(&proj, &points, METRES);
    let items: Vec<_> = proj.convert_iter(points.iter().copied()).collect();
    let failing: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.is_err().then_some(index))
        .collect();
    assert_eq!(
        failing,
        vec![700, 1_200],
        "the errors are at their own points"
    );
}

#[test]
fn an_empty_input_gives_no_item() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    assert!(proj
        .convert_iter(std::iter::empty::<(f64, f64)>())
        .next()
        .is_none());
}

#[test]
fn the_input_is_taken_a_block_at_a_time() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    let pulled = Cell::new(0_usize);
    let endless = (0..).map(|index: u64| {
        pulled.set(pulled.get() + 1);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the index is small and exact in f64"
        )]
        let x = index as f64;
        (2.0 + x * 1e-6, 46.0)
    });
    let mut items = proj.convert_iter(endless);
    assert!(items.next().expect("an item").is_ok());
    assert!(
        pulled.get() <= 1_024,
        "{} points taken for one item",
        pulled.get()
    );
}

#[test]
fn single_precision_items_match_convert() {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the test rounds the degrees to single precision on purpose"
    )]
    let points: Vec<(f32, f32)> = sample(2_000, (-4.5, 8.0), (41.5, 51.0))
        .into_iter()
        .map(|(x, y)| (x as f32, y as f32))
        .collect();
    let one: Vec<_> = points
        .iter()
        .map(|&p| proj.convert(p).expect("point"))
        .collect();
    let many: Vec<_> = proj
        .convert_iter(points.iter().copied())
        .map(|item| item.expect("item"))
        .collect();
    for (index, (a, b)) in one.iter().zip(&many).enumerate() {
        assert!(
            (a.0 - b.0).abs() <= 0.5 && (a.1 - b.1).abs() <= 0.5,
            "point {index}: convert {a:?}, convert_iter {b:?}"
        );
    }
}
