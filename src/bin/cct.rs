//! `cct`: transform coordinates with one PROJ operation.
//!
//! Angular input and output are degrees. `-d` sets the printed decimals.
//! `-z` and `-t` fill height and time when the line does not.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "cct writes coordinates to stdout and usage to stderr"
)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use proj_rs::{Operation, Xyzt};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("cct: {message}");
            eprintln!("{}", usage());
            return ExitCode::from(1);
        }
    };
    if options.help {
        eprintln!("{}", usage());
        return ExitCode::SUCCESS;
    }
    let operation = match Operation::create(&options.definition) {
        Ok(operation) => operation,
        Err(error) => {
            eprintln!("cct: {error}");
            return ExitCode::from(1);
        }
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            eprintln!("cct: could not read stdin");
            return ExitCode::from(1);
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(mut coord) = parse_coord(line, options.z, options.t) else {
            let _ = writeln!(stdout, "*");
            continue;
        };
        if operation.angular_input(options.inverse) {
            coord.x = coord.x.to_radians();
            coord.y = coord.y.to_radians();
        }
        let transformed = if options.inverse {
            operation.inverse(coord)
        } else {
            operation.forward(coord)
        };
        let Ok(mut got) = transformed else {
            let _ = writeln!(stdout, "*");
            continue;
        };
        if operation.angular_output(options.inverse) {
            got.x = got.x.to_degrees();
            got.y = got.y.to_degrees();
        }
        let decimals = options.decimals;
        let _ = writeln!(
            stdout,
            "{:.decimals$} {:.decimals$} {:.decimals$} {:.4}",
            got.x, got.y, got.z, got.t
        );
    }
    ExitCode::SUCCESS
}

struct Options {
    help: bool,
    inverse: bool,
    decimals: usize,
    z: Option<f64>,
    t: Option<f64>,
    definition: String,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut inverse = false;
        let mut decimals = 8usize;
        let mut z = None;
        let mut t = None;
        let mut parts = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "-h" | "--help" => help = true,
                "-I" | "--inverse" => inverse = true,
                "-d" | "--decimals" => {
                    index += 1;
                    let text = args
                        .get(index)
                        .ok_or_else(|| "missing -d value".to_owned())?;
                    decimals = text
                        .parse()
                        .map_err(|_| format!("-d {text} is not a decimal count"))?;
                }
                "-z" | "--height" => {
                    index += 1;
                    let text = args
                        .get(index)
                        .ok_or_else(|| "missing -z value".to_owned())?;
                    z = Some(
                        text.parse()
                            .map_err(|_| format!("-z {text} is not a number"))?,
                    );
                }
                "-t" | "--time" => {
                    index += 1;
                    let text = args
                        .get(index)
                        .ok_or_else(|| "missing -t value".to_owned())?;
                    t = Some(
                        text.parse()
                            .map_err(|_| format!("-t {text} is not a number"))?,
                    );
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option {other}"));
                }
                other => parts.push(other.to_owned()),
            }
            index += 1;
        }
        if !help && parts.is_empty() {
            return Err("missing +proj definition".to_owned());
        }
        Ok(Self {
            help,
            inverse,
            decimals,
            z,
            t,
            definition: parts.join(" "),
        })
    }
}

fn parse_coord(line: &str, fixed_z: Option<f64>, fixed_t: Option<f64>) -> Option<Xyzt> {
    let mut numbers = line.split_whitespace();
    let x = numbers.next()?.parse().ok()?;
    let y = numbers.next()?.parse().ok()?;
    let z = match fixed_z {
        Some(value) => value,
        None => numbers
            .next()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0.0),
    };
    let t = match fixed_t {
        Some(value) => value,
        None => numbers
            .next()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0.0),
    };
    Some(Xyzt { x, y, z, t })
}

fn usage() -> &'static str {
    "usage: cct [-I] [-d decimals] [-z height] [-t time] +proj=...\n\
     Angular axes are degrees. One x y [z] [t] point per line."
}
