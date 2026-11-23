use proj_rs::Proj;
use std::time::Instant;

#[inline(never)]
fn run_point(proj: &Proj, n: usize) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..n {
        let f = i as f64 / n as f64;
        let p = (700_000.0 + f * 100_000.0, 6_600_000.0 + f * 50_000.0);
        let (x, y) = proj.convert(p).expect("convert");
        sum += x + y;
    }
    sum
}

#[inline(never)]
fn run_one_by_one_array(proj: &Proj, n: usize) -> f64 {
    let mut sum = 0.0f64;
    let mut buf = [(0.0f64, 0.0f64); 1];
    for i in 0..n {
        let f = i as f64 / n as f64;
        buf[0] = (700_000.0 + f * 100_000.0, 6_600_000.0 + f * 50_000.0);
        proj.convert_array(&mut buf).expect("array");
        sum += buf[0].0 + buf[0].1;
    }
    sum
}

#[inline(never)]
fn run_array(proj: &Proj, n: usize) -> f64 {
    let mut pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let f = i as f64 / n as f64;
            (700_000.0 + f * 100_000.0, 6_600_000.0 + f * 50_000.0)
        })
        .collect();
    proj.convert_array(&mut pts).expect("array");
    pts.iter().map(|p| p.0 + p.1).sum()
}

#[inline(never)]
fn run_four(proj: &Proj, n: usize) -> f64 {
    let mut sum = 0.0f64;
    let mut buf = [(0.0f64, 0.0f64); 4];
    for i in 0..n {
        let f = i as f64 / n as f64;
        let p = (700_000.0 + f * 100_000.0, 6_600_000.0 + f * 50_000.0);
        buf = [p; 4];
        proj.convert_array(&mut buf).expect("array");
        sum += buf[0].0 + buf[0].1;
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pair = args.get(1).map(String::as_str).unwrap_or("EPSG:2154|EPSG:32631");
    let (from, to) = pair.split_once('|').unwrap();
    let proj = Proj::new_known_crs(from, to, None).expect("pair");
    let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2_000_000);
    let mode = args.get(3).map(String::as_str).unwrap_or("all");
    let mut run = |name: &str, f: &dyn Fn() -> f64| {
        f();
        let t = Instant::now();
        let s = f();
        println!("{name:>18} {:>8.1} ns/point  (checksum {s:.3})", t.elapsed().as_nanos() as f64 / n as f64);
    };
    if mode == "all" || mode == "point" { run("convert", &|| run_point(&proj, n)); }
    if mode == "all" || mode == "one" { run("array of one", &|| run_one_by_one_array(&proj, n)); }
    if mode == "prof" {
        let mut pts: Vec<(f64, f64)> = (0..n).map(|i| {
            let f = i as f64 / n as f64;
            (700_000.0 + f * 100_000.0, 6_600_000.0 + f * 50_000.0)
        }).collect();
        proj.convert_array(&mut pts).expect("array");
        println!("{}", pts[0].0);
        return;
    }
    if mode == "all" || mode == "four" { run("four identical", &|| run_four(&proj, n)); }
    if mode == "all" || mode == "array" { run("convert_array", &|| run_array(&proj, n)); }
}
