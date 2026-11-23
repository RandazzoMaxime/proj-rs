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

fn rss_report(tag: &str) {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let get = |k: &str| s.lines().find(|l| l.starts_with(k)).unwrap_or("").split_whitespace().nth(1).unwrap_or("0").to_string();
    let roll = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap_or_default();
    let roll_get = |k: &str| roll.lines().find(|l| l.starts_with(k)).unwrap_or("").split_whitespace().nth(1).unwrap_or("0").to_string();
    println!("{tag:>10}  VmHWM {:>6} kB  RssAnon {:>6} kB  RssFile {:>6} kB  (rollup anon {} file {})", get("VmHWM"), get("RssAnon"), get("RssFile"), roll_get("Anonymous"), roll_get("Private_Clean"));
}

fn main() {
    if std::env::args().nth(3).as_deref() == Some("mem") {
        rss_report("start");
        let args: Vec<String> = std::env::args().collect();
        let pair = args.get(1).unwrap();
        let (from, to) = pair.split_once('|').unwrap();
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        rss_report("created");
        let sm = std::fs::read_to_string("/proc/self/smaps").unwrap_or_default();
        let mut name = String::new();
        let mut rows = Vec::new();
        for line in sm.lines() {
            if line.chars().next().map_or(false, |c| c.is_ascii_hexdigit()) && line.contains('-') && line.split_whitespace().count() >= 5 {
                name = line.split_whitespace().last().unwrap_or("").to_string();
                if line.split_whitespace().count() == 5 { name = "[anon]".into(); }
            } else if line.starts_with("Rss:") {
                let kb: u64 = line.split_whitespace().nth(1).unwrap().parse().unwrap();
                if kb >= 100 { rows.push((kb, name.clone())); }
            }
        }
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        for (kb, n) in rows.iter().take(12) { println!("   {kb:>6} kB  {n}"); }
        let (x, y) = proj.convert((2.35f64, 48.85f64)).expect("c");
        rss_report("convert");
        println!("{x} {y}");
        return;
    }
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
