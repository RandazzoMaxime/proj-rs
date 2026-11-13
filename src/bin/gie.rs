//! `gie`: run the `.gie` test files of PROJ 9.9.0 (`apps/gie.cpp`).
//!
//! The runner is `proj_rs::gie`. This binary keeps the option set, the
//! messages and the summary lines of `gie.cpp`. Not ported: `-vv` counters for
//! roundtrips and failures, which the library does not split (the summary has
//! the per-file totals only), and the `<gie>`-block structure errors, which the
//! library does not report. `-o` writes the report to the named file; `gie.cpp`
//! opens it for reading, which is a bug there.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "gie writes its report to stdout and its usage and file errors to stderr"
)]

use proj_rs::gie::{config_from_env, run_text, Counts};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";

/// `delim` of `gie.cpp`: 79 dashes and a newline.
fn delim() -> String {
    format!("{}\n", "-".repeat(79))
}

/// The usage text of `gie.cpp`, with the program name as given.
/// The usage text of `gie.cpp`, with the program name as given.
fn usage(program: &str) -> String {
    let rule = "-".repeat(80);
    let lines = [
        rule.clone(),
        format!("Usage: {program} [-options]... infile..."),
        rule.clone(),
        "Options:".to_owned(),
        rule.clone(),
        "    -h                Help: print this usage information".to_owned(),
        "    -o /path/to/file  Specify output file name".to_owned(),
        "    -v                Verbose: Provide non-essential informational output.".to_owned(),
        "                      Repeat -v for more verbosity (e.g. -vv)".to_owned(),
        "    -q                Quiet: Opposite of verbose. In quiet mode not even errors"
            .to_owned(),
        "                      are reported. Only interaction is through the return code"
            .to_owned(),
        "                      (0 on success, non-zero indicates number of FAILED tests)"
            .to_owned(),
        "    -l                List the PROJ internal system error codes".to_owned(),
        rule.clone(),
        "Long Options:".to_owned(),
        rule.clone(),
        "    --output          Alias for -o".to_owned(),
        "    --verbose         Alias for -v".to_owned(),
        "    --help            Alias for -h".to_owned(),
        "    --list            Alias for -l".to_owned(),
        "    --version         Print version number".to_owned(),
        rule.clone(),
        "Examples:".to_owned(),
        rule.clone(),
        "1. Run all tests in file \"corner-cases.gie\", providing much extra information"
            .to_owned(),
        "       gie -vvvv corner-cases.gie".to_owned(),
        "2. Run all tests in files \"foo\" and \"bar\", providing info on failures only".to_owned(),
        "       gie foo bar".to_owned(),
        rule,
    ];
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// The error codes of `gie.cpp`'s `lookup` table, with the PROJ message of each.
const ERROR_CODES: &[(&str, i32)] = &[
    ("invalid_op", 1024),
    ("invalid_op_wrong_syntax", 1025),
    ("invalid_op_missing_arg", 1026),
    ("invalid_op_illegal_arg_value", 1027),
    ("invalid_op_mutually_exclusive_args", 1028),
    ("invalid_op_file_not_found_or_invalid", 1029),
    ("coord_transfm", 2048),
    ("coord_transfm_invalid_coord", 2049),
    ("coord_transfm_outside_projection_domain", 2050),
    ("coord_transfm_no_operation", 2051),
    ("coord_transfm_outside_grid", 2052),
    ("coord_transfm_grid_at_nodata", 2053),
    ("coord_transfm_missing_time", 2055),
    ("other", 4096),
    ("api_misuse", 4097),
    ("no_inverse_op", 4098),
    ("network_error", 4099),
];

#[derive(Default)]
struct Options {
    help: bool,
    version: bool,
    list: bool,
    verbose: usize,
    quiet: bool,
    output: Option<String>,
    files: Vec<String>,
}

/// `opt_parse` of PROJ's `getopt`-like parser: short flags `-hlvq` with `-o`
/// taking a value, the long names of `gie.cpp`, and `--` ending the options.
fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = args.iter();
    let mut only_files = false;
    while let Some(arg) = iter.next() {
        if only_files || arg == "-" || !arg.starts_with('-') {
            options.files.push(arg.clone());
        } else if arg == "--" {
            only_files = true;
        } else if let Some(long) = arg.strip_prefix("--") {
            let (name, value) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (long, None),
            };
            match name {
                "help" => options.help = true,
                "list" => options.list = true,
                "version" => options.version = true,
                "verbose" => options.verbose += 1,
                "quiet" => options.quiet = true,
                "output" => {
                    let value = match value {
                        Some(value) => value,
                        None => iter
                            .next()
                            .cloned()
                            .ok_or_else(|| format!("Invalid option \"{name}\""))?,
                    };
                    options.output = Some(value);
                }
                _ => return Err(format!("Invalid option \"{name}\"")),
            }
        } else {
            let mut chars = arg[1..].chars();
            while let Some(flag) = chars.next() {
                match flag {
                    'h' => options.help = true,
                    'l' => options.list = true,
                    'v' => options.verbose += 1,
                    'q' => options.quiet = true,
                    'o' => {
                        let rest: String = chars.by_ref().collect();
                        let value = if rest.is_empty() {
                            iter.next()
                                .cloned()
                                .ok_or_else(|| "Invalid option \"o\"".to_owned())?
                        } else {
                            rest
                        };
                        options.output = Some(value);
                        break;
                    }
                    other => return Err(format!("Invalid option \"{other}\"")),
                }
            }
        }
    }
    Ok(options)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let program = args
        .first()
        .and_then(|argv0| std::path::Path::new(argv0).file_name())
        .map_or_else(
            || "gie".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
    if args.len() == 1 {
        print!("{}", usage(&program));
        return ExitCode::SUCCESS;
    }
    let options = match parse(&args[1..]) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    };
    if options.help {
        print!("{}", usage(&program));
        return ExitCode::SUCCESS;
    }
    if options.version {
        println!("{program}: {RELEASE}");
        return ExitCode::SUCCESS;
    }
    // `T.verbosity`: -1 for quiet, else 1 plus the count of -v.
    let verbosity: i64 = if options.quiet {
        -1
    } else {
        i64::try_from(options.verbose)
            .unwrap_or(i64::MAX)
            .saturating_add(1)
    };
    let mut out: Box<dyn Write> = match &options.output {
        Some(path) => match File::create(PathBuf::from(path)) {
            Ok(file) => Box::new(file),
            Err(_) => {
                eprintln!("{program}: Cannot open '{path}' for output");
                return ExitCode::from(1);
            }
        },
        None => Box::new(io::stdout()),
    };
    if options.list {
        for (name, code) in ERROR_CODES {
            let message = proj_rs::errno::error_string(*code).unwrap_or_default();
            let _ = writeln!(out, "{name:>25}  ({code:2}):  {message}");
        }
        return ExitCode::SUCCESS;
    }
    if options.files.is_empty() {
        if verbosity == -1 {
            return ExitCode::from(255);
        }
        let _ = writeln!(out, "Nothing to do");
        return ExitCode::SUCCESS;
    }
    for file in &options.files {
        if File::open(file).is_err() {
            eprint!(
                "{}Cannot open specified input file '{file}' - bye!\n",
                delim()
            );
            return ExitCode::from(1);
        }
    }
    let config = config_from_env();
    let mut grand = Counts::default();
    for file in &options.files {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(_) => {
                eprint!(
                    "{}Cannot open specified input file '{file}' - bye!\n",
                    delim()
                );
                return ExitCode::from(1);
            }
        };
        let report = run_text(&text, &config);
        if verbosity > 0 {
            let _ = write!(out, "{}Reading file '{file}'\n", delim());
        }
        if verbosity >= 0 {
            for note in &report.notes {
                let _ = writeln!(out, "{note}");
            }
        }
        let counts = report.counts;
        grand.passed += counts.passed;
        grand.failed += counts.failed;
        grand.skipped += counts.skipped;
        if verbosity > 0 {
            let _ = write!(
                out,
                "{}total: {:2} tests succeeded, {:2} tests skipped, {:2} tests {}\n",
                delim(),
                counts.passed,
                counts.skipped,
                counts.failed,
                if counts.failed > 0 {
                    "FAILED!"
                } else {
                    "failed."
                }
            );
        }
    }
    if verbosity > 0 {
        if options.files.len() > 1 {
            let _ = write!(
                out,
                "{}Grand total: {}. Success: {}, Skipped: {}, Failure: {}\n",
                delim(),
                grand.passed + grand.failed + grand.skipped,
                grand.passed,
                grand.skipped,
                grand.failed
            );
        }
        let _ = write!(out, "{}", delim());
    } else if grand.failed > 0 && verbosity >= 0 {
        let _ = write!(out, "Failures: {}", grand.failed);
    }
    let _ = out.flush();
    ExitCode::from(u8::try_from(grand.failed % 256).unwrap_or(0))
}
