//! The grid of `BoxIndex` must give, for every point, the choice of the full scan.

use super::{Alternatives, Bounds, Instantiability};
use crate::iso19111::{select_alternatives, BestPolicy, CandidateFilter, Selection};
use crate::proj::Direction;

const POINTS: usize = if cfg!(debug_assertions) {
    4_000
} else {
    500_000
};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f64 {
        f64::from(u32::try_from(self.next() >> 32).unwrap_or(0)) / 4_294_967_296.0
    }

    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }

    fn pick<T: Copy>(&mut self, values: &[T]) -> T {
        values[usize::try_from(self.next()).unwrap_or(0) % values.len()]
    }
}

fn alternatives(source: &str, target: &str, policy: BestPolicy) -> Alternatives {
    match select_alternatives(source, target, policy, CandidateFilter::NONE).expect("selection") {
        Selection::Several(alternatives) => alternatives,
        Selection::Single(..) => panic!("{source} to {target} has one candidate"),
    }
}

fn edges(bounds: &Bounds) -> Vec<f64> {
    [bounds.min_x, bounds.max_x, bounds.min_y, bounds.max_y]
        .into_iter()
        .flat_map(|value| [value, value.next_up(), value.next_down()])
        .collect()
}

fn check(alternatives: &Alternatives, seed: u64, label: &str, points: usize) {
    let mut rng = Rng(seed);
    let mut specials = vec![
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        180.0,
        -180.0,
        360.0,
        -360.0,
        90.0,
        -90.0,
        179.999_999,
        -179.999_999,
        180.000_001,
        -180.000_001,
        540.0,
        -540.0,
        1e300,
        -1e300,
    ];
    for item in &alternatives.items {
        specials.extend(edges(&item.bounds));
        specials.extend(edges(&item.target_bounds));
    }
    let (mut low_x, mut high_x) = (f64::MAX, f64::MIN);
    for item in &alternatives.items {
        for bounds in [&item.bounds, &item.target_bounds] {
            if bounds.min_x.is_finite() && bounds.max_x.is_finite() {
                low_x = low_x.min(bounds.min_x);
                high_x = high_x.max(bounds.max_x);
            }
        }
    }
    let excluded_sets = [[None, None]];
    for round in 0..points {
        let (x, y) = match round % 4 {
            0 => (
                rng.between(low_x - 20.0, high_x + 20.0),
                rng.between(-100.0, 100.0),
            ),
            1 => (rng.pick(&specials), rng.pick(&specials)),
            2 => (rng.between(-400.0, 400.0), rng.between(-400.0, 400.0)),
            _ => (rng.pick(&specials), rng.between(-100.0, 100.0)),
        };
        let point = if round % 97 == 0 { None } else { Some((x, y)) };
        let excluded = if round % 5 == 0 {
            [
                Some(usize::try_from(rng.next()).unwrap_or(0) % alternatives.len()),
                None,
            ]
        } else {
            excluded_sets[0]
        };
        let instantiability = if round % 3 == 0 {
            Instantiability::InstantiableOnly
        } else {
            Instantiability::Any
        };
        let way = if round % 2 == 0 {
            Direction::Forward
        } else {
            Direction::Inverse
        };
        assert_eq!(
            alternatives.suggest(point, &excluded, instantiability, way),
            alternatives.suggest_full(point, &excluded, instantiability, way),
            "{label}: {point:?} {excluded:?} {way:?}"
        );
    }
}

fn run(points: usize) {
    for (source, target, policy, seed) in [
        ("EPSG:4267", "EPSG:4326", BestPolicy::Warn, 1),
        ("EPSG:4267", "EPSG:4326", BestPolicy::Discard, 2),
        ("EPSG:4326", "EPSG:4267", BestPolicy::Warn, 3),
        ("EPSG:4230", "EPSG:4326", BestPolicy::Warn, 4),
        ("EPSG:4326", "EPSG:4230", BestPolicy::Warn, 5),
        ("EPSG:4269", "EPSG:4326", BestPolicy::Warn, 6),
        ("EPSG:4152", "EPSG:4267", BestPolicy::Warn, 7),
        ("EPSG:4267", "EPSG:4979", BestPolicy::Warn, 8),
        ("EPSG:4362", "EPSG:4269", BestPolicy::Warn, 9),
        ("EPSG:4269", "EPSG:4362", BestPolicy::Warn, 10),
        ("EPSG:4984", "EPSG:4269", BestPolicy::Warn, 11),
        ("EPSG:4892", "EPSG:4269", BestPolicy::Warn, 12),
        ("EPSG:4978", "EPSG:4269", BestPolicy::Warn, 13),
        ("EPSG:4385", "EPSG:4326", BestPolicy::Warn, 14),
        ("OGC:CRS84", "EPSG:4267", BestPolicy::Warn, 15),
        ("EPSG:4267", "OGC:CRS84", BestPolicy::Warn, 16),
        (
            "+proj=longlat +datum=NAD27",
            "+proj=longlat +datum=WGS84",
            BestPolicy::Warn,
            17,
        ),
    ] {
        let alternatives = alternatives(source, target, policy);
        assert!(
            alternatives.len() >= 6,
            "{source} to {target}: {}",
            alternatives.len()
        );
        check(
            &alternatives,
            seed,
            &format!("{source} to {target} {policy:?}"),
            points,
        );
    }
}

#[test]
fn grid_choice_equals_full_scan() {
    run(POINTS);
}

#[test]
#[ignore = "heavy: 8 million points"]
fn grid_choice_equals_full_scan_heavy() {
    run(1_000_000);
}
