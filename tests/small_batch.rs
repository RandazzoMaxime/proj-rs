//! Under eight points `convert_array` and `convert_iter` go through the transform one point at a
//! time (the vector path costs more than it saves there); from eight they use the vector path.
//! Either way the result is `convert`'s: bit for bit below the threshold, to the tolerance of the
//! vector kernels from it, and an error is the one `convert` gives for the first failing point.

use proj_rs::{Proj, ProjError};

/// Agreement of the vector path with `convert`, in metres, as `convert_iter`'s tests.
const METRES: f64 = 1e-7;
/// The threshold of `SMALL_BATCH` in `src/proj.rs`.
const SMALL: usize = 8;

fn proj(from: &str, to: &str) -> Proj {
    Proj::new_known_crs(from, to, None).expect("pair")
}

/// `count` points in France, the same every time.
fn france(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|i| {
            let step = f64::from(u32::try_from(i).expect("small"));
            (-4.0 + 0.7 * step, 42.5 + 0.55 * step)
        })
        .collect()
}

fn gap(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

fn one_by_one(proj: &Proj, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    points
        .iter()
        .map(|&point| proj.convert(point).expect("convert"))
        .collect()
}

const PAIRS: [(&str, &str, f64); 3] = [
    ("EPSG:4326", "EPSG:2154", METRES),
    ("EPSG:4326", "EPSG:32631", METRES),
    ("EPSG:4326", "EPSG:3857", METRES),
];

#[test]
fn a_slice_under_the_threshold_is_convert_bit_for_bit() {
    for (from, to, _) in PAIRS {
        let proj = proj(from, to);
        for count in 1..SMALL {
            let points = france(count);
            let mut slice = points.clone();
            proj.convert_array(&mut slice).expect("slice");
            assert_eq!(
                slice,
                one_by_one(&proj, &points),
                "{from} -> {to}, {count} points"
            );
        }
    }
}

#[test]
fn from_the_threshold_the_slice_agrees_to_the_vector_tolerance() {
    for (from, to, tolerance) in PAIRS {
        let proj = proj(from, to);
        for count in SMALL..=SMALL + 12 {
            let points = france(count);
            let mut slice = points.clone();
            proj.convert_array(&mut slice).expect("slice");
            for (got, want) in slice.iter().zip(one_by_one(&proj, &points)) {
                assert!(
                    gap(*got, want) <= tolerance,
                    "{from} -> {to}, {count} points"
                );
            }
        }
    }
}

#[test]
fn a_native_operation_in_radians_too() {
    let proj = Proj::new("+proj=utm +zone=31 +ellps=WGS84").expect("utm");
    for count in [1, 3, 7, 8, 20] {
        let radians: Vec<(f64, f64)> = france(count)
            .iter()
            .map(|&(lon, lat)| ((lon + 4.0).to_radians(), lat.to_radians()))
            .collect();
        let mut slice = radians.clone();
        proj.convert_array(&mut slice).expect("slice");
        let each: Vec<(f64, f64)> = radians
            .iter()
            .map(|&point| proj.convert(point).expect("convert"))
            .collect();
        if count < SMALL {
            assert_eq!(slice, each);
        } else {
            assert!(slice.iter().zip(&each).all(|(g, w)| gap(*g, *w) <= METRES));
        }
    }
}

#[test]
fn a_failing_point_gives_convert_error_and_leaves_the_slice() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    for count in [1, 2, 5, 7, 8, 12] {
        for bad in 0..count {
            let mut points = france(count);
            points[bad] = (2.0, 95.0);
            let expected = proj
                .convert((2.0, 95.0))
                .expect_err("a latitude past the pole");
            let mut slice = points.clone();
            let got = proj.convert_array(&mut slice).map(|_| ());
            assert_eq!(
                format!("{got:?}"),
                format!("{:?}", Err::<(), ProjError>(expected)),
                "{count} points, bad at {bad}"
            );
            assert_eq!(slice, points, "one block: nothing written, {count} points");
        }
    }
}

#[test]
fn the_first_failing_point_is_the_one_reported() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    let mut points = france(5);
    points[1] = (2.0, 95.0);
    points[3] = (2.0, -97.0);
    let first = proj.convert(points[1]).expect_err("first");
    let got = proj.convert_array(&mut points).expect_err("slice");
    assert_eq!(format!("{got:?}"), format!("{first:?}"));
}

#[test]
fn an_empty_slice_and_a_single_point() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    let mut none: Vec<(f64, f64)> = Vec::new();
    assert!(proj.convert_array(&mut none).expect("empty").is_empty());
    let mut one = [(2.35, 48.85)];
    proj.convert_array(&mut one).expect("one");
    assert_eq!(one[0], proj.convert((2.35, 48.85)).expect("convert"));
}

#[test]
fn single_precision_slices_under_the_threshold() {
    let proj = proj("EPSG:4326", "EPSG:32631");
    let mut slice: Vec<(f32, f32)> = vec![(2.35, 48.85), (3.1, 47.0), (0.5, 49.0)];
    let want: Vec<(f32, f32)> = slice
        .iter()
        .map(|&point| proj.convert(point).expect("convert"))
        .collect();
    proj.convert_array(&mut slice).expect("slice");
    assert_eq!(slice, want);
}

#[test]
fn the_iterator_gives_convert_items_for_every_length() {
    for (from, to, tolerance) in PAIRS {
        let proj = proj(from, to);
        for count in 0..=SMALL + 12 {
            let points = france(count);
            let items: Vec<(f64, f64)> = proj
                .convert_iter(points.iter().copied())
                .collect::<Result<_, _>>()
                .expect("items");
            let each = one_by_one(&proj, &points);
            assert_eq!(items.len(), count);
            for (got, want) in items.iter().zip(&each) {
                if count < SMALL {
                    assert_eq!(got, want, "{from} -> {to}, {count} points");
                } else {
                    assert!(
                        gap(*got, *want) <= tolerance,
                        "{from} -> {to}, {count} points"
                    );
                }
            }
        }
    }
}

#[test]
fn the_iterator_goes_on_after_an_error_in_a_short_input() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    for count in [3, 7, 8, 30] {
        let mut points = france(count);
        points[1] = (2.0, 95.0);
        let items: Vec<_> = proj.convert_iter(points.iter().copied()).collect();
        assert_eq!(items.len(), count);
        for (index, (item, &point)) in items.iter().zip(&points).enumerate() {
            let want = proj.convert(point);
            match (item, &want) {
                (Ok(got), Ok(want)) => assert!(gap(*got, *want) <= METRES, "item {index}"),
                (Err(got), Err(want)) => {
                    assert_eq!(format!("{got:?}"), format!("{want:?}"), "item {index}");
                }
                _ => panic!("item {index} ({count} points): {item:?} against {want:?}"),
            }
        }
    }
}

#[test]
fn a_short_iterator_is_read_no_further_than_it_goes() {
    let proj = proj("EPSG:4326", "EPSG:2154");
    let taken = std::cell::Cell::new(0_usize);
    let source = france(5)
        .into_iter()
        .inspect(|_| taken.set(taken.get() + 1));
    let items: Vec<_> = proj.convert_iter(source).collect();
    assert_eq!(items.len(), 5);
    assert_eq!(taken.get(), 5);
}
