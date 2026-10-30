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
            "-d",
            "8",
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
        &["-d", "8", "EPSG:4326", "EPSG:32631"],
        "46.5 3\n",
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
fn cs2cs_d_changes_printed_precision() {
    let args = [
        "+proj=latlong",
        "+ellps=WGS84",
        "+to",
        "+proj=utm",
        "+zone=31",
        "+ellps=WGS84",
    ];
    let coarse = run(env!("CARGO_BIN_EXE_cs2cs"), &args, "3 46.5\n");
    let mut fine_args = vec!["-d", "8"];
    fine_args.extend_from_slice(&args);
    let fine = run(env!("CARGO_BIN_EXE_cs2cs"), &fine_args, "3 46.5\n");
    let places = |text: &str, index: usize| {
        text.split_whitespace()
            .nth(index)
            .expect("number")
            .split('.')
            .nth(1)
            .expect("fraction")
            .len()
    };
    assert_eq!(places(&coarse, 0), 2, "{coarse}");
    assert_eq!(places(&coarse, 1), 2, "{coarse}");
    assert_eq!(places(&coarse, 2), 2, "{coarse}");
    assert_eq!(places(&fine, 0), 8, "{fine}");
    assert_eq!(places(&fine, 1), 8, "{fine}");
    assert_eq!(places(&fine, 2), 8, "{fine}");
    assert_ne!(coarse, fine);
}

#[test]
fn cs2cs_dms_roundtrip_and_constant_width() {
    let pair = [
        "+proj=latlong",
        "+ellps=WGS84",
        "+to",
        "+proj=latlong",
        "+ellps=WGS84",
    ];
    let text = run(
        env!("CARGO_BIN_EXE_cs2cs"),
        &pair,
        "79d58'00.000\"W 37d02'00.000\"N 0\n",
    );
    assert_eq!(text, "79d58'W\t37d2'N 0.000\n");
    let mut wide = vec!["-W0"];
    wide.extend_from_slice(&pair);
    let text = run(env!("CARGO_BIN_EXE_cs2cs"), &wide, "0 0\n");
    assert_eq!(text, "0d00'00\"E\t0d00'00\"N 0.000\n");
    let mut eight = vec!["-d", "8"];
    eight.extend_from_slice(&pair);
    let text = run(env!("CARGO_BIN_EXE_cs2cs"), &eight, "1 2 3\n");
    assert_eq!(text, "1.00000000\t2.00000000 3.00000000\n");
}

#[test]
fn cs2cs_bad_w_exits_with_proj_message() {
    let output = Command::new(env!("CARGO_BIN_EXE_cs2cs"))
        .args(["-W9"])
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(1));
    let err = String::from_utf8(output.stderr).expect("utf-8");
    assert!(
        err.contains("-W argument missing or not in range [0,8]"),
        "{err}"
    );
    assert!(err.contains("program abnormally terminated"), "{err}");
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

#[test]
fn cct_yaml_decimals_height_time_pipeline_and_output_file() {
    let cases = [
        (
            &["-d", "8", "+proj=merc", "+R=1"][..],
            "90 45 0\n",
            "   1.57079633     0.88137359    0.00000000           inf\n",
        ),
        (
            &[
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=deg",
                "+xy_out=rad",
            ][..],
            "0.5 2\n",
            "  0.5000000000    2.0000000000        0.0000        0.0000\n",
        ),
        (
            &[
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=deg",
                "+xy_out=deg",
            ][..],
            "0.5 2\n",
            "  0.5000000000    2.0000000000        0.0000        0.0000\n",
        ),
        (
            &[
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=m",
                "+xy_out=km",
            ][..],
            "0.5 2\n",
            "       0.0005         0.0020        0.0000        0.0000\n",
        ),
        (
            &[
                "-d",
                "6",
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=deg",
                "+xy_out=rad",
            ][..],
            "0.5 2\n",
            "      0.500000        2.000000      0.000000        0.0000\n",
        ),
        (
            &[
                "-d",
                "6",
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=deg",
                "+xy_out=deg",
            ][..],
            "0.5 2\n",
            "      0.500000        2.000000      0.000000        0.0000\n",
        ),
        (
            &[
                "-d",
                "6",
                "-z",
                "0",
                "-t",
                "0",
                "+proj=pipeline",
                "+step",
                "+proj=unitconvert",
                "+xy_in=m",
                "+xy_out=km",
            ][..],
            "0.5 2\n",
            "     0.000500       0.002000      0.000000        0.0000\n",
        ),
    ];
    for (args, input, expected) in cases {
        let text = run(env!("CARGO_BIN_EXE_cct"), args, input);
        assert_eq!(text, expected, "{args:?}");
    }

    let path = std::env::temp_dir().join(format!("proj-rs-cct-{}.txt", std::process::id()));
    let path_text = path.to_str().expect("temp path is utf-8");
    let stdout = run(
        env!("CARGO_BIN_EXE_cct"),
        &[
            "-o",
            path_text,
            "-z",
            "0",
            "-t",
            "0",
            "+proj=pipeline",
            "+step",
            "+proj=unitconvert",
            "+xy_in=m",
            "+xy_out=km",
        ],
        "0.5 2\n",
    );
    assert_eq!(stdout, "");
    let file = std::fs::read_to_string(&path).expect("output file");
    assert_eq!(
        file, "       0.0005         0.0020        0.0000        0.0000\n",
        "cct -o file"
    );
    std::fs::remove_file(&path).expect("remove output file");
}
