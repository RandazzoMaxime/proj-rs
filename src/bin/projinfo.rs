//! `projinfo`: the command line of `proj_rs::projinfo`. It prints the report of
//! `proj_rs::projinfo::run`: the warnings, then the errors, on stderr; the text on stdout.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the projinfo binary writes its report to stdout and stderr"
)]

use std::process::ExitCode;

use proj_rs::projinfo::{run, Status};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    let report = run(&arguments);
    for warning in &report.warnings {
        eprintln!("{warning}");
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
