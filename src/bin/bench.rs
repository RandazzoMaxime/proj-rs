//! Chrono du lot direct, même grilles que le comparatif libproj.
//!
//! Métrique : pire grille, nanosecondes par point. Chaque grille est la
//! médiane de 3 essais, chaque essai la médiane de 11 passes d'un million
//! de points. Épingler le processus (`taskset -c 0`) avant de comparer.

use proj_rs::{Ellipsoid, Hemisphere, Lcc, Tmerc, UtmZone};
use std::hint::black_box;
use std::time::Instant;

const N: usize = 1_000_000;
const REPEATS: usize = 11;
const TRIALS: usize = 3;

struct Fix {
    name: &'static str,
    seed: u64,
    lon0: f64,
    lon1: f64,
    lat0: f64,
    lat1: f64,
    kind: Kind,
}

enum Kind {
    Utm31,
    Lambert93,
}

const FIXTURES: [Fix; 3] = [
    Fix {
        name: "utm31_center",
        seed: 1,
        lon0: 2.0,
        lon1: 4.0,
        lat0: 43.0,
        lat1: 50.0,
        kind: Kind::Utm31,
    },
    Fix {
        name: "utm31_edge",
        seed: 2,
        lon0: 0.05,
        lon1: 0.55,
        lat0: 43.0,
        lat1: 50.0,
        kind: Kind::Utm31,
    },
    Fix {
        name: "lambert93",
        seed: 3,
        lon0: -4.5,
        lon1: 8.0,
        lat0: 41.5,
        lat1: 51.0,
        kind: Kind::Lambert93,
    },
];

fn next_unit(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    let bits = *state >> 11;
    // 53 bits de mantisse. La valeur tient dans la plage exacte d'un f64.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the top 53 bits of the LCG are an exact integer in f64"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "no infallible From<u64> for f64; the value is at most 2^53"
    )]
    {
        (bits as f64) * (1.0 / 9_007_199_254_740_992.0)
    }
}

fn fill(fix: &Fix) -> (Vec<f64>, Vec<f64>) {
    let mut state = fix.seed;
    let mut lon = vec![0.0; N];
    let mut lat = vec![0.0; N];
    let dl = fix.lon1 - fix.lon0;
    let dp = fix.lat1 - fix.lat0;
    for i in 0..N {
        lon[i] = fix.lon0 + next_unit(&mut state) * dl;
        lat[i] = fix.lat0 + next_unit(&mut state) * dp;
    }
    (lon, lat)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

#[expect(
    clippy::cast_precision_loss,
    reason = "N is 1_000_000, which is exactly representable as f64"
)]
#[expect(
    clippy::as_conversions,
    reason = "no infallible From<usize> for f64; N fits in the mantissa"
)]
fn time_stable(mut run: impl FnMut()) -> f64 {
    let mut trials = [0.0; TRIALS];
    for trial in &mut trials {
        run();
        let mut samples = [0.0; REPEATS];
        for sample in &mut samples {
            let start = Instant::now();
            run();
            *sample = start.elapsed().as_secs_f64() * 1e9 / N as f64;
        }
        *trial = median(&mut samples);
    }
    median(&mut trials)
}

#[expect(
    clippy::print_stdout,
    reason = "the benchmark's product is this JSON record on stdout"
)]
fn main() {
    let zone = UtmZone::try_new(31).expect("zone 31");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let l93 = Lcc::lambert93();
    let mut east = vec![0.0; N];
    let mut north = vec![0.0; N];
    let mut worst = 0.0_f64;
    let mut rows = String::new();

    for (index, fix) in FIXTURES.iter().enumerate() {
        let (lon, lat) = fill(fix);
        let ns = match fix.kind {
            Kind::Utm31 => time_stable(|| {
                utm.forward_batch(
                    black_box(&lon),
                    black_box(&lat),
                    black_box(&mut east),
                    black_box(&mut north),
                );
            }),
            Kind::Lambert93 => time_stable(|| {
                l93.forward_batch(
                    black_box(&lon),
                    black_box(&lat),
                    black_box(&mut east),
                    black_box(&mut north),
                );
            }),
        };
        worst = worst.max(ns);
        if index > 0 {
            rows.push(',');
        }
        rows.push_str(&format!(
            "{{\"name\":\"{}\",\"ns_per_point\":{ns:.6}}}",
            fix.name
        ));
    }

    println!(
        "{{\"impl\":\"proj-rs\",\"n\":{N},\"repeats\":{REPEATS},\"trials\":{TRIALS},\"worst_ns_per_point\":{worst:.6},\"fixtures\":[{rows}]}}"
    );
}
