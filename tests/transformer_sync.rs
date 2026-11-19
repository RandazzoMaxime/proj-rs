//! `Transformer` is `Send + Sync`: one transformer serves several threads at once.
//!
//! The expected values come from PROJ 9.9.0, never from this crate: `cs2cs` 9.9.0 for the few
//! points of [`CS2CS_POINTS`], and a transformer used on one thread for the rest (the same points
//! converted one after the other, on a fresh transformer). Those are compared bit for bit.

use proj_rs::{Context, Crs, Error, Point, Transformer};
use std::thread;

/// Compile-time proof: this does not build while `Transformer` is not `Sync`.
fn assert_send_sync<T: Send + Sync>() {}

const THREADS: usize = 8;
const POINTS: usize = 10_000;

/// `cs2cs` 9.9.0 (`target/proj-reference/build/bin/cs2cs -f %.9f EPSG:4326 EPSG:32631`, with
/// `PROJ_DATA=data`), latitude then longitude in, easting then northing out.
const CS2CS_UTM31: [((f64, f64), (f64, f64)); 2] = [
    ((48.85, 2.35), (452_314.891_233_666, 5_410_984.887_604_439)),
    ((45.76, 4.84), (643_089.580_627_053, 5_069_028.739_955_27)),
];

/// `cs2cs` 9.9.0 (`-f %.9f EPSG:4267 EPSG:4269`), latitude then longitude in and out.
const CS2CS_NAD27_NAD83: [((f64, f64), (f64, f64)); 3] = [
    ((38.0, -95.0), (38.000_039_034, -95.000_248_512)),
    ((40.7, -74.0), (40.700_000_397, -73.999_569_167)),
    ((47.6, -122.3), (47.599_793_895, -122.301_219_728)),
];

/// Points of a box, from a fixed-seed generator (no random crate): reproducible on every run.
fn points(count: usize, seed: u64, lat: (f64, f64), lon: (f64, f64)) -> Vec<Point> {
    let mut state = seed;
    let mut unit = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // The mantissa of 1.0 plus 52 random bits, minus 1: a uniform value in [0, 1).
        f64::from_bits(0x3FF0_0000_0000_0000 | (state >> 12)) - 1.0
    };
    (0..count)
        .map(|_| {
            let la = lat.0 + (lat.1 - lat.0) * unit();
            let lo = lon.0 + (lon.1 - lon.0) * unit();
            Point::new(la, lo)
        })
        .collect()
}

/// The bits of a result, so that two results compare exactly, error text included.
type Key = Result<[u64; 4], String>;

/// The results of one thread: per point, and per slice (`Err` when the slice call fails).
type ThreadResults = (Vec<Key>, Result<Vec<Key>, String>);

fn key(result: &Result<Point, Error>) -> Key {
    match result {
        Ok(p) => Ok([p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), p.t.to_bits()]),
        Err(error) => Err(format!("{error:?}")),
    }
}

fn transformer(source: &str, target: &str) -> Transformer {
    let ctx = Context::default();
    let src = Crs::from_user_input(&ctx, source).expect("source CRS");
    let dst = Crs::from_user_input(&ctx, target).expect("target CRS");
    Transformer::builder(&src, &dst)
        .build()
        .expect("transformer")
}

/// Every point through `forward`, one after the other, on the calling thread.
fn forward_each(t: &Transformer, input: &[Point]) -> Vec<Key> {
    input.iter().map(|p| key(&t.forward(*p))).collect()
}

/// The same points through `forward_slice`, in one slice. `None` when the slice call fails.
fn forward_slice_keys(t: &Transformer, input: &[Point]) -> Result<Vec<Key>, String> {
    let mut slice = input.to_vec();
    t.forward_slice(&mut slice)
        .map_err(|error| format!("{error:?}"))?;
    Ok(slice.iter().map(|p| key(&Ok(*p))).collect())
}

/// Checks one pair: `THREADS` threads share `&Transformer` and each converts the whole input,
/// per point and per slice; every result must equal the single-thread one, bit for bit.
fn check_shared_pair(source: &str, target: &str, input: &[Point]) {
    let reference_each = forward_each(&transformer(source, target), input);
    let reference_slice = forward_slice_keys(&transformer(source, target), input);
    let shared = transformer(source, target);
    let shared_ref = &shared;

    let results: Vec<ThreadResults> = thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(move || {
                    (
                        forward_each(shared_ref, input),
                        forward_slice_keys(shared_ref, input),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a conversion thread panicked"))
            .collect()
    });

    assert_eq!(results.len(), THREADS);
    for (each, slice) in &results {
        assert_eq!(each.len(), reference_each.len());
        let mismatches = each
            .iter()
            .zip(&reference_each)
            .filter(|(got, want)| got != want)
            .count();
        assert_eq!(
            mismatches, 0,
            "{source} to {target}: per-point results differ"
        );
        assert_eq!(
            slice, &reference_slice,
            "{source} to {target}: slice results differ"
        );
    }
}

#[test]
fn transformer_is_send_and_sync() {
    assert_send_sync::<Transformer>();
}

#[test]
fn shared_transformer_matches_one_thread_bit_for_bit_single_candidate() {
    // EPSG:4326 to EPSG:32631: one operation, no alternatives.
    let input = points(POINTS, 0x5eed_0001, (35.0, 70.0), (-10.0, 20.0));
    check_shared_pair("EPSG:4326", "EPSG:32631", &input);
}

#[test]
fn shared_transformer_matches_one_thread_bit_for_bit_several_candidates() {
    // EPSG:4267 to EPSG:4269: several candidates, with failures that change the choice.
    let input = points(POINTS, 0x5eed_0002, (25.0, 49.0), (-125.0, -66.0));
    check_shared_pair("EPSG:4267", "EPSG:4269", &input);
}

#[test]
fn shared_transformer_agrees_with_cs2cs_on_a_few_points() {
    let utm = transformer("EPSG:4326", "EPSG:32631");
    for ((lat, lon), (e, n)) in CS2CS_UTM31 {
        let p = utm.forward(Point::new(lat, lon)).expect("forward");
        assert!((p.x - e).abs() < 1e-6, "easting {} vs cs2cs {e}", p.x);
        assert!((p.y - n).abs() < 1e-6, "northing {} vs cs2cs {n}", p.y);
    }
    let nad = transformer("EPSG:4267", "EPSG:4269");
    for ((lat, lon), (olat, olon)) in CS2CS_NAD27_NAD83 {
        let p = nad.forward(Point::new(lat, lon)).expect("forward");
        assert!(
            (p.x - olat).abs() < 1e-8,
            "latitude {} vs cs2cs {olat}",
            p.x
        );
        assert!(
            (p.y - olon).abs() < 1e-8,
            "longitude {} vs cs2cs {olon}",
            p.y
        );
    }
}

#[test]
fn last_used_operation_reads_while_points_convert() {
    let input = points(POINTS, 0x5eed_0003, (25.0, 49.0), (-125.0, -66.0));
    // What a reader may see. A single thread reads only the operation each point settles on. A
    // reader on another thread can also see a candidate that a point in flight tried first:
    // `current` is set before the candidate runs, and a failure moves on to the next one. Those
    // tried candidates are candidates of the pair, the ones `operations()` lists. So the oracle
    // is the names one thread reports for these points, plus the candidates of the pair.
    let single = transformer("EPSG:4267", "EPSG:4269");
    let mut names = std::collections::BTreeSet::new();
    for p in &input {
        let _ = single.forward(*p);
        if let Some(op) = single.last_used_operation() {
            names.insert(op.name().expect("a named operation").to_owned());
        }
    }
    assert!(
        names.len() > 1,
        "the points must use several operations: {names:?}"
    );
    for op in single.operations().expect("candidates") {
        names.insert(op.name().expect("a named candidate").to_owned());
    }

    let t = transformer("EPSG:4267", "EPSG:4269");
    let t_ref = &t;
    let names_ref = &names;
    let input_ref = &input;
    let seen = thread::scope(|scope| {
        let writers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(move || {
                    for p in input_ref {
                        let _ = t_ref.forward(*p);
                    }
                })
            })
            .collect();
        let readers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(move || {
                    let mut seen = 0_usize;
                    for _ in 0..POINTS {
                        if let Some(op) = t_ref.last_used_operation() {
                            let name = op.name().expect("a named operation");
                            assert!(
                                names_ref.contains(name),
                                "{name} is neither an operation of a single thread nor a candidate"
                            );
                            seen += 1;
                        }
                    }
                    seen
                })
            })
            .collect();
        for w in writers {
            w.join().expect("a conversion thread panicked");
        }
        readers
            .into_iter()
            .map(|r| r.join().expect("a reader thread panicked"))
            .sum::<usize>()
    });
    assert!(seen > 0, "the readers saw no operation");
    // Once the writers are done, the operation read is the last one written: a candidate.
    let last = t
        .last_used_operation()
        .expect("an operation after conversions");
    assert!(names.contains(last.name().expect("a named operation")));
}
