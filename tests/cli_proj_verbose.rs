//! `proj -v` (the parameter list) and `proj -V` (the verbose monitor), against the output of PROJ
//! 9.9.0's `apps/proj.cpp` and `pr_list.cpp`. The expected text is in `tests/cli_proj_verbose/`,
//! captured from the reference binaries: `proj` and `invproj` of `target/proj-reference`.

use std::io::Write;
use std::process::{Command, Stdio};

mod common;

struct Case<'a> {
    bin: &'static str,
    args: &'a [&'a str],
    input: &'static str,
    expected: &'static str,
    exit: i32,
}

/// The data path of the reference runs: the vendored data, then its fixtures (`nad83` lives there).
const PROJ_DATA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data:",
    env!("CARGO_MANIFEST_DIR"),
    "/data/tests"
);

fn run(bin: &str, args: &[&str], input: &str) -> (i32, String, String) {
    let mut child = Command::new(bin)
        .args(args)
        .env("PROJ_DATA", PROJ_DATA)
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
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).expect("utf-8"),
        String::from_utf8(output.stderr).expect("utf-8"),
    )
}

fn binary(name: &str) -> &'static str {
    match name {
        "proj" => env!("CARGO_BIN_EXE_proj"),
        "invproj" => env!("CARGO_BIN_EXE_invproj"),
        other => panic!("unknown binary {other}"),
    }
}

fn check(case: &Case<'_>) {
    let (code, stdout, stderr) = run(binary(case.bin), case.args, case.input);
    assert_eq!(code, case.exit, "exit code, stderr:\n{stderr}");
    assert!(
        common::same_cli_text(&stdout, case.expected),
        "stdout:\n{stdout}\nexpected:\n{}\nstderr:\n{stderr}",
        case.expected
    );
}

/// Relative tolerance of the scale factor `h`, `k` and `s` at a point where the derivatives of
/// `pj_factors` are taken at the origin or at a pole. The values differ from PROJ 9.9.0 in the
/// last digits, from the order of the finite differences (`Operation::factors`), not from the
/// coordinates: `docs/PORTING-cli-rest.md`, « Écarts ouverts ».
const FACTOR_TOLERANCE: f64 = 1e-9;

/// The `h`, `k` or `s` of a factor line, and its error in per cent, when `line` is one.
fn factor_values(line: &str) -> Option<(f64, f64)> {
    let label = [
        "Meridian scale (h) :",
        "Parallel scale (k) :",
        "Areal scale (s):",
    ]
    .into_iter()
    .find(|label| line.starts_with(label))?;
    let rest = line[label.len()..].trim_start();
    let value: f64 = rest.split_whitespace().next()?.parse().ok()?;
    let error = rest.split("( ").nth(1)?.split(' ').next()?.parse().ok()?;
    Some((value, error))
}

/// `check`, except that the factor lines may differ by [`FACTOR_TOLERANCE`]. Every other line must
/// match exactly.
fn check_factors_within_tolerance(case: &Case<'_>) {
    let (code, stdout, stderr) = run(binary(case.bin), case.args, case.input);
    assert_eq!(code, case.exit, "exit code, stderr:\n{stderr}");
    let got: Vec<&str> = stdout.lines().collect();
    let want: Vec<&str> = case.expected.lines().collect();
    assert_eq!(got.len(), want.len(), "line count, stderr:\n{stderr}");
    for (got, want) in got.iter().zip(&want) {
        match (factor_values(got), factor_values(want)) {
            (Some((value, error)), Some((expected, expected_error))) => {
                let relative = (value - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
                assert!(
                    relative <= FACTOR_TOLERANCE,
                    "factor {value} against {expected}\n{got}\n{want}"
                );
                assert!(
                    (error - expected_error).abs() <= 1e-6,
                    "error {error} against {expected_error}\n{got}\n{want}"
                );
            }
            _ => assert!(common::same_cli_text(got, want), "{got}\n{want}"),
        }
    }
}

const MERC_GRS80: &[&str] = &["+proj=merc", "+ellps=GRS80"];

#[test]
fn v_header_without_input() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=merc", "+ellps=GRS80"],
        input: "",
        expected: include_str!("cli_proj_verbose/header_no_input.out"),
        exit: 0,
    });
}

#[test]
fn v_forward_point_prints_list_then_point() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=merc", "+ellps=GRS80"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_forward.out"),
        exit: 0,
    });
}

#[test]
fn v_unused_parameters_are_listed_after_the_used_ones() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=merc", "+ellps=GRS80", "+foo=bar", "+lat_1=10"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_unused.out"),
        exit: 0,
    });
}

#[test]
fn v_utm_datum_expands_ellipsoid_and_towgs84() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=utm", "+zone=31", "+datum=WGS84"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_utm_datum.out"),
        exit: 0,
    });
}

#[test]
fn v_lcc_with_towgs84_is_used() {
    check(&Case {
        bin: "proj",
        args: &[
            "-v",
            "+proj=lcc",
            "+lat_1=44",
            "+lat_2=49",
            "+lat_0=46.5",
            "+lon_0=3",
            "+x_0=700000",
            "+y_0=6600000",
            "+ellps=GRS80",
            "+towgs84=0,0,0",
        ],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_lcc.out"),
        exit: 0,
    });
}

#[test]
fn v_radius_makes_the_ellipsoid_unused() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=tmerc", "+R=6371000"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_tmerc_radius.out"),
        exit: 0,
    });
}

#[test]
fn v_inverse_utm_lists_the_inverse_input() {
    check(&Case {
        bin: "proj",
        args: &["-v", "-I", "+proj=utm", "+zone=31", "+datum=WGS84"],
        input: "200000 300000\n",
        expected: include_str!("cli_proj_verbose/v_inverse_utm.out"),
        exit: 0,
    });
}

#[test]
fn v_datum_and_ellps_conflict_leaves_the_datum_ellps_unused() {
    check(&Case {
        bin: "proj",
        args: &[
            "-v",
            "+proj=utm",
            "+zone=31",
            "+datum=WGS84",
            "+ellps=GRS80",
        ],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_datum_conflict.out"),
        exit: 0,
    });
}

#[test]
fn v_default_ellps_is_appended_and_used() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=merc"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_default_ellps.out"),
        exit: 0,
    });
}

#[test]
fn v_duplicate_key_keeps_the_first_as_used() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+proj=merc", "+ellps=GRS80", "+ellps=WGS84"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_duplicate.out"),
        exit: 0,
    });
}

#[test]
fn v_init_file_section_is_expanded_in_place() {
    check(&Case {
        bin: "proj",
        args: &["-v", "+init=nad83:5002"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/v_init_file.out"),
        exit: 0,
    });
}

#[test]
fn monitor_prints_the_figure_and_the_factors_of_each_point() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    check(&Case {
        bin: "proj",
        args: &args,
        input: "2 49\n3 50\n-3 40\n",
        expected: include_str!("cli_proj_verbose/V_points.out"),
        exit: 0,
    });
}

#[test]
fn monitor_inverse_reads_plain_numbers_with_three_decimals() {
    check(&Case {
        bin: "proj",
        args: &["-V", "-I", "+proj=utm", "+zone=31", "+datum=WGS84"],
        input: "200000 300000\n",
        expected: include_str!("cli_proj_verbose/V_inverse_utm.out"),
        exit: 0,
    });
}

#[test]
fn monitor_f_sets_the_number_format_of_the_figure_too() {
    let mut args = vec!["-V", "-f", "%.6f"];
    args.extend_from_slice(MERC_GRS80);
    check(&Case {
        bin: "proj",
        args: &args,
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/V_format.out"),
        exit: 0,
    });
}

/// The bare `I` line reads the origin, where the factors differ from PROJ in the last digits.
#[test]
fn monitor_copies_tag_lines_and_echoes_text_after_the_numbers() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    check_factors_within_tolerance(&Case {
        bin: "proj",
        args: &args,
        input: "# tag line\n2 49 tagA\nI\n3 50\n",
        expected: include_str!("cli_proj_verbose/V_tag_text.out"),
        exit: 0,
    });
}

#[test]
fn monitor_refuses_binary_io_after_printing_the_header() {
    let mut args = vec!["-V", "-b"];
    args.extend_from_slice(MERC_GRS80);
    let (code, stdout, stderr) = run(env!("CARGO_BIN_EXE_proj"), &args, "2 49\n");
    assert_eq!(code, 1);
    assert_eq!(
        stdout,
        include_str!("cli_proj_verbose/V_binary_refused.out")
    );
    assert!(
        stderr.contains("binary I/O not available in -V option"),
        "{stderr}"
    );
}

#[test]
fn monitor_on_a_sphere_prints_the_radius() {
    check(&Case {
        bin: "proj",
        args: &["-V", "+proj=tmerc", "+R=6371000"],
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/V_sphere.out"),
        exit: 0,
    });
}

#[test]
fn monitor_applies_m_as_the_output_factor_of_the_forward_projection() {
    let mut args = vec!["-V", "-m", "2"];
    args.extend_from_slice(MERC_GRS80);
    check(&Case {
        bin: "proj",
        args: &args,
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/V_postscale.out"),
        exit: 0,
    });
}

#[test]
fn monitor_applies_units_to_the_printed_coordinates() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    args.push("+units=ft");
    check(&Case {
        bin: "proj",
        args: &args,
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/V_feet.out"),
        exit: 0,
    });
}

/// An unreadable line reads as the origin, see [`FACTOR_TOLERANCE`].
#[test]
fn monitor_echoes_an_unreadable_line_as_zero_then_its_text() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    check_factors_within_tolerance(&Case {
        bin: "proj",
        args: &args,
        input: "abc def\n2 49\n",
        expected: include_str!("cli_proj_verbose/V_bad_line.out"),
        exit: 0,
    });
}

#[test]
fn monitor_d_sets_the_decimals_of_the_coordinates() {
    let mut args = vec!["-V", "-d", "4"];
    args.extend_from_slice(MERC_GRS80);
    check(&Case {
        bin: "proj",
        args: &args,
        input: "2 49\n",
        expected: include_str!("cli_proj_verbose/V_digits.out"),
        exit: 0,
    });
}

/// Mercator at the pole: `pj_fwd` gives finite values, and the factors see [`FACTOR_TOLERANCE`].
#[test]
fn monitor_pole_of_mercator_keeps_the_finite_values_of_pj_fwd() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    check_factors_within_tolerance(&Case {
        bin: "proj",
        args: &args,
        input: "2 90\n",
        expected: include_str!("cli_proj_verbose/V_pole.out"),
        exit: 0,
    });
}

#[test]
fn monitor_invproj_inverts_and_prints_the_figure() {
    let mut args = vec!["-V"];
    args.extend_from_slice(MERC_GRS80);
    check(&Case {
        bin: "invproj",
        args: &args,
        input: "222638.98 6242596\n",
        expected: include_str!("cli_proj_verbose/V_invproj.out"),
        exit: 0,
    });
}
