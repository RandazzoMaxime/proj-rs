//! The `gie` binary against the cases of PROJ 9.9.0 `test/cli/test_gie.yaml`,
//! and against the options of `apps/gie.cpp` the yaml does not reach.
//!
//! The yaml runner in `tests/cli_yaml.rs` does not run `gie`, so the four
//! cases of `test_gie.yaml` are written out here. Expected text is PROJ's.

use std::path::PathBuf;
use std::process::{Command, Output};

fn gie(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gie"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run gie")
}

/// The output as text. The program prints its own file name, which ends in `.exe` on Windows;
/// PROJ's yaml runner substitutes the bare program name, so the extension is dropped here.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("gie.exe", "gie")
}

#[test]
fn invalid_argument_is_reported_on_stderr() {
    // test_gie.yaml: "Test invalid argument"
    let out = gie(&["--invalid"]);
    assert_eq!(text(&out.stderr).trim_end(), "Invalid option \"invalid\"");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn version_argument_prints_the_release() {
    // test_gie.yaml: "Test version argument". The yaml substitutes the program name.
    let out = gie(&["--version"]);
    assert!(
        text(&out.stdout).starts_with("gie: Rel"),
        "{}",
        text(&out.stdout)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn help_argument_prints_the_usage() {
    // test_gie.yaml: "Test help argument"
    let out = gie(&["-h"]);
    assert!(text(&out.stdout).contains("Usage: gie [-options]... infile..."));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn missing_input_file_is_reported_with_the_delimiter() {
    // test_gie.yaml: "Test gie with non existing input file"
    let out = gie(&["i_do_not_exist.txt"]);
    let delim = "-".repeat(79);
    assert_eq!(
        text(&out.stderr),
        format!("{delim}\nCannot open specified input file 'i_do_not_exist.txt' - bye!\n")
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn no_arguments_prints_the_usage_and_succeeds() {
    // gie.cpp: `argc == 1` prints usage.
    let out = Command::new(env!("CARGO_BIN_EXE_gie"))
        .output()
        .expect("run gie");
    assert!(text(&out.stdout).starts_with(&"-".repeat(80)));
    assert!(text(&out.stdout).contains("Usage: gie [-options]... infile..."));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn quiet_without_files_returns_minus_one() {
    // gie.cpp: `-q` with no input file returns -1, which the shell shows as 255.
    let out = gie(&["-q"]);
    assert_eq!(out.status.code(), Some(255));
    assert!(out.stdout.is_empty());
}

#[test]
fn no_files_without_quiet_says_nothing_to_do() {
    // gie.cpp: `Nothing to do` when no file is given and the output is not quiet.
    let out = gie(&["-v"]);
    assert_eq!(text(&out.stdout), "Nothing to do\n");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn list_prints_the_error_codes_of_the_lookup_table() {
    // gie.cpp `list_err_codes`: 17 names, right-aligned to 25 columns, with the code.
    let out = gie(&["-l"]);
    let lines: Vec<String> = text(&out.stdout).lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 17);
    assert_eq!(
        lines[0],
        "               invalid_op  (1024):  Unspecified error related to coordinate operation initialization"
    );
    assert!(lines[16].starts_with("            network_error  (4099):  "));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn passing_file_reports_the_summary_and_exits_zero() {
    // gie.cpp default verbosity: "Reading file", then "total:" between delimiters.
    let out = gie(&["tests/gie/GDA.gie"]);
    let delim = "-".repeat(79);
    let expected = format!(
        "{delim}\nReading file 'tests/gie/GDA.gie'\n{delim}\ntotal:  3 tests succeeded,  0 tests skipped,  0 tests failed.\n{delim}\n"
    );
    assert_eq!(text(&out.stdout), expected);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn output_option_writes_the_report_to_the_named_file() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gie_cli_output");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("report.txt");
    let out = gie(&[
        "-o",
        file.to_str().expect("utf-8 path"),
        "tests/gie/GDA.gie",
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    let report = std::fs::read_to_string(&file).expect("report written");
    assert!(report.contains("total:  3 tests succeeded"));
    std::fs::remove_file(&file).expect("remove report");
}
