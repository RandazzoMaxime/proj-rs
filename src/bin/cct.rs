//! `cct`: transform coordinates with one PROJ operation.
//!
//! Angular input is degrees and becomes radians before the operation. A degree
//! output (`+xy_out=deg`) is already degrees, so it is not converted again.
//! `-d` sets the decimals of x, y, and z. Time stays at 4 decimals. A missing
//! height or time is infinity. `-o` writes those same lines to a file.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "cct writes coordinates to stdout and usage to stderr"
)]

use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::ExitCode;

use proj_rs::{Operation, Xyzt};

const DEFAULT_ANGLE_DECIMALS: usize = 10;
const DEFAULT_DISTANCE_DECIMALS: usize = 4;

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
    match options.output.as_deref() {
        Some(path) => match File::create(path) {
            Ok(file) => transcribe(&options, &operation, BufWriter::new(file)),
            Err(_) => {
                eprintln!("cct: Cannot open '{path}' for output");
                ExitCode::from(1)
            }
        },
        None => transcribe(&options, &operation, io::stdout()),
    }
}

struct Options {
    help: bool,
    inverse: bool,
    /// `None` keeps PROJ's split default: 10 decimals for angles, 4 for distances.
    decimals: Option<usize>,
    z: Option<f64>,
    t: Option<f64>,
    output: Option<String>,
    definition: String,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut inverse = false;
        let mut decimals = None;
        let mut z = None;
        let mut t = None;
        let mut output = None;
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
                    decimals = Some(
                        text.parse()
                            .map_err(|_| format!("-d {text} is not a decimal count"))?,
                    );
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
                "-o" | "--output" => {
                    index += 1;
                    let text = args
                        .get(index)
                        .ok_or_else(|| "missing -o value".to_owned())?;
                    output = Some(text.clone());
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
            output,
            definition: parts.join(" "),
        })
    }
}

fn transcribe(options: &Options, operation: &Operation, mut output: impl Write) -> ExitCode {
    let decimals_angles = options.decimals.unwrap_or(DEFAULT_ANGLE_DECIMALS);
    let decimals_distances = options.decimals.unwrap_or(DEFAULT_DISTANCE_DECIMALS);
    // `%14` / `%12` when the output is radians or degrees; `%13` for metres.
    let columns =
        if operation.angular_output(options.inverse) || operation.degree_output(options.inverse) {
            Columns::Angle
        } else {
            Columns::Distance
        };
    let stdin = io::stdin();
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
            if emit(&mut output, "*").is_err() {
                return ExitCode::from(1);
            }
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
            if emit(&mut output, "*").is_err() {
                return ExitCode::from(1);
            }
            continue;
        };
        if operation.angular_output(options.inverse) {
            got.x = got.x.to_degrees();
            got.y = got.y.to_degrees();
        }
        let text = format_point(got, columns, decimals_angles, decimals_distances);
        if emit(&mut output, &text).is_err() {
            return ExitCode::from(1);
        }
    }
    if output.flush().is_err() {
        eprintln!("cct: write error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn emit(output: &mut impl Write, line: &str) -> io::Result<()> {
    writeln!(output, "{line}").inspect_err(|_| eprintln!("cct: write error"))
}

/// Columns stay 1, 2, 3, 4. `-z` and `-t` fill those axes; they do not shift
/// the next token into them. A missing ordinate is `HUGE_VAL`, printed as `inf`.
fn parse_coord(line: &str, fixed_z: Option<f64>, fixed_t: Option<f64>) -> Option<Xyzt> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let x = fields.first()?.parse().ok()?;
    let y = fields.get(1)?.parse().ok()?;
    let z = if let Some(value) = fixed_z {
        value
    } else {
        match fields.get(2) {
            Some(text) => text.parse().ok()?,
            None => f64::INFINITY,
        }
    };
    let t = if let Some(value) = fixed_t {
        value
    } else {
        match fields.get(3) {
            Some(text) => text.parse().ok()?,
            None => f64::INFINITY,
        }
    };
    Some(Xyzt { x, y, z, t })
}

/// Angle columns are wider than distance columns. Not a `bool`: the call site
/// must name which layout it wants.
#[derive(Clone, Copy)]
enum Columns {
    Angle,
    Distance,
}

/// `cct.cpp` prints `%14.*f  %14.*f  %12.*f  %12.4f` for angles and
/// `%13.*f  %13.*f  %12.*f  %12.4f` for distances. Time is always 4 decimals.
fn format_point(
    point: Xyzt,
    columns: Columns,
    decimals_angles: usize,
    decimals_distances: usize,
) -> String {
    let (xy_width, xy_prec) = match columns {
        Columns::Angle => (14, decimals_angles),
        Columns::Distance => (13, decimals_distances),
    };
    let z_prec = decimals_distances;
    let z_width = 12;
    let t_width = 12;
    let t_prec = 4;
    let Xyzt { x, y, z, t } = point;
    format!(
        "{x:>xy_width$.xy_prec$}  {y:>xy_width$.xy_prec$}  {z:>z_width$.z_prec$}  {t:>t_width$.t_prec$}"
    )
}

fn usage() -> &'static str {
    "usage: cct [-I] [-d decimals] [-z height] [-t time] [-o file] +proj=...\n\
     Angular axes are degrees. One x y [z] [t] point per line.\n\
     -o writes the coordinate lines to a file instead of stdout."
}
