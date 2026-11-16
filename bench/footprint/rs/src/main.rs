//! One conversion program, run by bench/footprint.sh in one mode per process.
//! `bench/footprint/c/convert.c` does the same work through libproj.
//!
//! Modes: `convert` (one point, the program whose size is measured), `empty`, `create`,
//! `points`, `array`, `multi`. Stdout is `ok=<n> fail=<n> sum=<x>`, stderr `vmhwm_kb=<n>`.

use proj_rs::Proj;
use std::hint::black_box;

const N: usize = 1_000_000;

/// Same generator as `bench/footprint/c/convert.c`: `N` lon/lat points in a box over France.
struct Points(u64);

impl Points {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 11) as f64 / 9_007_199_254_740_992.0
    }

    fn next(&mut self) -> (f64, f64) {
        let lon = -4.5 + self.unit() * 12.5;
        let lat = 41.5 + self.unit() * 9.5;
        (lon, lat)
    }
}

#[derive(Default)]
struct Tally {
    ok: u64,
    fail: u64,
    sum: f64,
}

impl Tally {
    fn add(&mut self, result: Option<(f64, f64)>) {
        match result {
            Some((x, y)) if x.is_finite() && y.is_finite() => {
                self.ok += 1;
                self.sum += x + y;
            }
            _ => self.fail += 1,
        }
    }
}

fn new_proj(from: &str, to: &str) -> Proj {
    Proj::new_known_crs(from, to, None).expect("transform")
}

fn one_by_one(proj: &Proj) -> Tally {
    let mut points = Points(6);
    let mut tally = Tally::default();
    for _ in 0..N {
        tally.add(proj.convert(black_box(points.next())).ok());
    }
    tally
}

fn array(proj: &Proj) -> Tally {
    let mut points = Points(6);
    let mut buffer: Vec<(f64, f64)> = (0..N).map(|_| points.next()).collect();
    let mut tally = Tally::default();
    match proj.convert_array(&mut buffer) {
        Ok(done) => done.iter().for_each(|&p| tally.add(Some(p))),
        Err(_) => tally.fail = N as u64,
    }
    tally
}

/// Peak resident set of this process so far, in KiB (`VmHWM` of /proc/self/status).
fn peak_rss_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
        .unwrap_or(0)
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "convert".into());
    let mut tally = Tally::default();
    match mode.as_str() {
        "empty" => {}
        "convert" => {
            let proj = new_proj("EPSG:4326", "EPSG:2154");
            tally.add(proj.convert((2.35, 48.85)).ok());
        }
        "create" => {
            black_box(new_proj("EPSG:4326", "EPSG:2154"));
        }
        "points" => tally = one_by_one(&new_proj("EPSG:4326", "EPSG:2154")),
        "array" => tally = array(&new_proj("EPSG:4326", "EPSG:2154")),
        "multi" => tally = one_by_one(&new_proj("EPSG:4267", "EPSG:4326")),
        other => {
            eprintln!("unknown mode {other}");
            std::process::exit(2);
        }
    }
    eprintln!("vmhwm_kb={}", peak_rss_kb());
    println!("ok={} fail={} sum={:.6}", tally.ok, tally.fail, tally.sum);
}
