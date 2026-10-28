//! `cs2cs` and `cct` against one PROJ string and one EPSG pair.

use std::io::Write;
use std::process::{Command, Stdio};

use proj_rs::{Ellipsoid, Hemisphere, Tmerc, UtmZone};

fn run(bin: &str, args: &[&str], input: &str) -> String {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

fn metres(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-3
}

fn utm_point() -> (f64, f64) {
    let zone = UtmZone::try_new(31).expect("zone");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let typed = utm.forward(3.0, 46.5).expect("forward");
    (typed.east, typed.north)
}

#[test]
fn cs2cs_proj_string_matches_utm() {
    let (east, north) = utm_point();
    let text = run(
        env!("CARGO_BIN_EXE_cs2cs"),
        &[
            "+proj=latlong",
            "+ellps=WGS84",
            "+to",
            "+proj=utm",
            "+zone=31",
            "+ellps=WGS84",
        ],
        "3 46.5\n",
    );
    let mut numbers = text.split_whitespace();
    let got_east: f64 = numbers.next().expect("east").parse().expect("east number");
    let got_north: f64 = numbers
        .next()
        .expect("north")
        .parse()
        .expect("north number");
    assert!(metres(got_east, east), "{got_east} vs {east}");
    assert!(metres(got_north, north), "{got_north} vs {north}");
}

#[test]
fn cs2cs_epsg_pair_matches_utm() {
    let (east, north) = utm_point();
    let text = run(
        env!("CARGO_BIN_EXE_cs2cs"),
        &["EPSG:4326", "EPSG:32631"],
        "3 46.5\n",
    );
    let mut numbers = text.split_whitespace();
    let got_east: f64 = numbers.next().expect("east").parse().expect("east number");
    let got_north: f64 = numbers
        .next()
        .expect("north")
        .parse()
        .expect("north number");
    assert!(metres(got_east, east), "{got_east} vs {east}");
    assert!(metres(got_north, north), "{got_north} vs {north}");
}

#[test]
fn cct_proj_string_matches_utm() {
    let (east, north) = utm_point();
    let text = run(
        env!("CARGO_BIN_EXE_cct"),
        &["+proj=utm", "+zone=31", "+ellps=WGS84"],
        "3 46.5\n",
    );
    let mut numbers = text.split_whitespace();
    let got_east: f64 = numbers.next().expect("east").parse().expect("east number");
    let got_north: f64 = numbers
        .next()
        .expect("north")
        .parse()
        .expect("north number");
    assert!(metres(got_east, east), "{got_east} vs {east}");
    assert!(metres(got_north, north), "{got_north} vs {north}");
}
