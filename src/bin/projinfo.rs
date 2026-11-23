//! `projinfo`: the command line of `proj_rs::projinfo`. It prints the report of
//! `proj_rs::projinfo::run`: the warnings, then the errors, on stderr; the text on stdout.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the projinfo binary writes its report to stdout and stderr"
)]

use std::io::Write;
use std::process::ExitCode;

use proj_rs::projinfo::{run, Status};

pub(crate) fn main() -> ExitCode {
    run_with(std::env::args().collect())
}

/// Runs the command with `argv` as the process arguments, the program name first.
pub(crate) fn run_with(argv: Vec<String>) -> ExitCode {
    let args: Vec<String> = argv.into_iter().skip(1).collect();
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    let report = run(&arguments);
    {
        let mut stderr = std::io::stderr().lock();
        for warning in &report.warning_bytes {
            let _ = stderr.write_all(warning);
            let _ = stderr.write_all(b"\n");
        }
    }
    if !report.output.is_empty() {
        println!("{}", report.output);
    }
    for error in &report.errors {
        eprintln!("{error}");
    }
    match report.status {
        Status::Success => ExitCode::SUCCESS,
        Status::Failure => ExitCode::from(1),
    }
}
