//! Geographic CRS in grads, through `cs2cs`, against PROJ 9.9.0.
//!
//! PROJ reads the input of a grad CRS as plain numbers in grads, and always prints a
//! geographic result in degrees (`cs2cs.cpp`, `destToRadians * RAD_TO_DEG`), DMS by default.
//! The expected values are the output of the PROJ 9.9.0 `cs2cs` binary with `-d 10`, its data
//! being the reference build only: the NTv2 grid `ntf_r93.gsb` of `data/tests` is not read.

use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], input: &str) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cs2cs"));
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn cs2cs");
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

fn rows(args: &[&str], input: &str) -> Vec<Vec<f64>> {
    let mut full = vec!["-d", "10"];
    full.extend_from_slice(args);
    run(&full, input)
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse::<f64>().expect("number"))
                .collect()
        })
        .collect()
}

/// Each field is compared to PROJ's value: 1e-7 in degrees, 1e-4 in metres.
fn assert_row(args: &[&str], input: &str, expected: &[f64], tolerance: f64) {
    let got = rows(args, input);
    assert_eq!(got.len(), 1, "one output line for {args:?}");
    let got = &got[0];
    assert_eq!(got.len(), expected.len(), "fields for {args:?}");
    for (axis, (got, want)) in got.iter().zip(expected).enumerate() {
        assert!(
            (got - want).abs() <= tolerance,
            "axis {axis} of {args:?}: got {got}, PROJ {want}"
        );
    }
}

const DEG: f64 = 1e-7;
const METRE: f64 = 1e-4;

#[test]
fn grad_source_reads_plain_grads_and_prints_degrees() {
    assert_row(
        &["EPSG:4807", "+to", "EPSG:4326"],
        "54.2869662585 -0.0466709370 0\n",
        &[48.8582000005, 2.2945000049, 0.0],
        DEG,
    );
}

#[test]
fn degree_source_to_grad_target_prints_degrees() {
    assert_row(
        &["EPSG:4326", "+to", "EPSG:4807"],
        "48.8582 2.2945 0\n",
        &[48.8582696327, -0.0420038433, 0.0],
        DEG,
    );
}

#[test]
fn grad_source_to_projected_reads_grads() {
    assert_row(
        &["EPSG:4807", "+to", "EPSG:2154"],
        "43.2999763919 3.0333132353 0\n",
        &[880407.395457241, 5764330.973342745, 0.0],
        METRE,
    );
}

#[test]
fn projected_to_grad_target_prints_degrees() {
    assert_row(
        &["EPSG:2154", "+to", "EPSG:4807"],
        "880453.0554161093 5764316.2342027659 0\n",
        &[38.9698363107, 2.7305003045, 0.0],
        DEG,
    );
}

#[test]
fn grad_source_to_degree_target() {
    assert_row(
        &["EPSG:4807", "+to", "EPSG:4171"],
        "43.2999763919 3.0333132353 0\n",
        &[38.9701211876, 5.0666926700, 0.0],
        DEG,
    );
}

#[test]
fn second_grad_crs_reads_grads_and_prints_degrees() {
    assert_row(
        &["EPSG:4810", "+to", "EPSG:4326"],
        "60.0 3.0 0\n",
        &[54.0002384595, 5.0338411535, 0.0],
        DEG,
    );
}

#[test]
fn grad_identity_prints_degrees() {
    assert_row(
        &["EPSG:4807", "+to", "EPSG:4807"],
        "60.0 3.0 0\n",
        &[54.0, 2.7, 0.0],
        DEG,
    );
}

#[test]
fn grad_target_default_output_is_dms() {
    let got = run(&["EPSG:4326", "+to", "EPSG:4807"], "43.3 5.37\n");
    assert_eq!(got, "43d17'59.915\"N\t3d1'59.928\"E 0.000\n");
}
