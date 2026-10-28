//! `proj`: project coordinates and, with `-S`, print scale factors.
//!
//! A `+proj` definition is the operation. An `EPSG:` code is the projected CRS:
//! input is longitude and latitude in degrees, and `-S` differentiates that
//! projection. `-E` echoes the input line. `-f` sets the coordinate format.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "proj writes coordinates to stdout and usage to stderr"
)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use proj_rs::{CrsPipeline, Operation, Xyzt};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("proj: {message}");
            return ExitCode::from(1);
        }
    };
    if options.help {
        eprintln!("usage: proj [-ESf] [+opt[=arg] ...] [EPSG:code]");
        return ExitCode::SUCCESS;
    }
    let job = match Job::open(&options) {
        Ok(job) => job,
        Err(message) => {
            eprintln!("proj: {message}");
            return ExitCode::from(1);
        }
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            eprintln!("proj: could not read input");
            return ExitCode::from(1);
        };
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let Some((lon, lat)) = two_numbers(&line) else {
            let _ = writeln!(stdout, "*\t*");
            continue;
        };
        let projected = job.project(lon, lat);
        let Ok((east, north)) = projected else {
            let _ = writeln!(stdout, "*\t*");
            continue;
        };
        let mut text = String::new();
        if options.echo {
            text.push_str(line.trim());
            text.push('\t');
        }
        text.push_str(&format_fixed(east, options.precision));
        text.push('\t');
        text.push_str(&format_fixed(north, options.precision));
        if options.factors {
            if let Some(factors) = job.factors(lon, lat) {
                text.push('\t');
                text.push('<');
                text.push_str(
                    &factors
                        .iter()
                        .map(|value| format_g(*value))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                text.push('>');
            }
        }
        let _ = writeln!(stdout, "{text}");
    }
    ExitCode::SUCCESS
}

struct Options {
    help: bool,
    echo: bool,
    factors: bool,
    precision: usize,
    definition: String,
    crs: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut echo = false;
        let mut factors = false;
        let mut precision = 2usize;
        let mut plus = Vec::new();
        let mut crs = None;
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            if arg == "-h" || arg == "--help" {
                help = true;
            } else if arg == "-E" {
                echo = true;
            } else if arg == "-S" {
                factors = true;
            } else if arg == "-f" {
                index += 1;
                let text = args.get(index).ok_or_else(|| "missing -f value".to_owned())?;
                precision = precision_of(text)?;
            } else if arg.starts_with('+') {
                plus.push(arg.to_owned());
            } else if arg.starts_with('-') {
                return Err(format!("unknown option {arg}"));
            } else if crs.is_none() && plus.is_empty() {
                crs = Some(arg.to_owned());
            } else {
                return Err(format!("unexpected argument {arg}"));
            }
            index += 1;
        }
        Ok(Self {
            help,
            echo,
            factors,
            precision,
            definition: plus.join(" "),
            crs,
        })
    }
}

fn precision_of(text: &str) -> Result<usize, String> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'%') {
        return Err(format!("invalid format string {text}"));
    }
    let mut index = 1;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if bytes.get(index) != Some(&b'.') {
        return Err(format!("invalid format string {text}"));
    }
    index += 1;
    let mut value = 0usize;
    let mut any = false;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        any = true;
        value = value
            .saturating_mul(10)
            .saturating_add(usize::from(bytes[index] - b'0'));
        index += 1;
    }
    if !any || index + 1 != bytes.len() || !matches!(bytes[index], b'f' | b'F') {
        return Err(format!("invalid format string {text}"));
    }
    Ok(value)
}

enum Job {
    Native { operation: Operation, degrees_out: bool },
    Epsg(CrsPipeline),
}

impl Job {
    fn open(options: &Options) -> Result<Self, String> {
        if let Some(crs) = &options.crs {
            let pipe = CrsPipeline::create("EPSG:4326", crs).map_err(|error| error.to_string())?;
            return Ok(Self::Epsg(pipe));
        }
        if options.definition.is_empty() {
            return Err("missing projection".to_owned());
        }
        let operation = Operation::create(&options.definition).map_err(|error| error.to_string())?;
        let degrees_out = operation.angular_output(false)
            || options.definition.contains("o_proj=lat")
            || options.definition.contains("o_proj=long");
        Ok(Self::Native {
            operation,
            degrees_out,
        })
    }

    fn project(&self, lon: f64, lat: f64) -> Result<(f64, f64), ()> {
        match self {
            Self::Epsg(pipe) => pipe
                .operation
                .forward(Xyzt {
                    x: lat,
                    y: lon,
                    z: 0.0,
                    t: 0.0,
                })
                .map(|coord| (coord.x, coord.y))
                .map_err(|_| ()),
            Self::Native {
                operation,
                degrees_out,
            } => {
                let coord = Xyzt {
                    x: lon.to_radians(),
                    y: lat.to_radians(),
                    z: 0.0,
                    t: 0.0,
                };
                let got = operation.forward(coord).map_err(|_| ())?;
                if *degrees_out {
                    Ok((got.x.to_degrees(), got.y.to_degrees()))
                } else {
                    Ok((got.x, got.y))
                }
            }
        }
    }

    fn factors(&self, lon: f64, lat: f64) -> Option<[f64; 6]> {
        let (a, es) = match self {
            Self::Epsg(pipe) => pipe.operation.distance_ellipsoid(),
            Self::Native { operation, .. } => operation.distance_ellipsoid(),
        };
        if a == 0.0 {
            return None;
        }
        let sample = |dlat: f64, dlon: f64| -> Option<(f64, f64)> {
            self.project(lon + dlon.to_degrees(), lat + dlat.to_degrees())
                .ok()
        };
        let h = 1e-5;
        let east = sample(0.0, h)?;
        let west = sample(0.0, -h)?;
        let north = sample(h, 0.0)?;
        let south = sample(-h, 0.0)?;
        let der_x_l = (east.0 - west.0) / (2.0 * h) / a;
        let der_y_l = (east.1 - west.1) / (2.0 * h) / a;
        let der_x_p = (north.0 - south.0) / (2.0 * h) / a;
        let der_y_p = (north.1 - south.1) / (2.0 * h) / a;
        let phi = lat.to_radians();
        let cosphi = phi.cos();
        if cosphi.abs() < 1e-12 {
            return None;
        }
        let mut meridional = der_x_p.hypot(der_y_p);
        let mut parallel = der_x_l.hypot(der_y_l) / cosphi;
        let one_es = 1.0 - es;
        let radius_factor = if es != 0.0 {
            let t = 1.0 - es * phi.sin().powi(2);
            let n = t.sqrt();
            meridional *= t * n / one_es;
            parallel *= n;
            t * t / one_es
        } else {
            1.0
        };
        let areal = (der_y_p * der_x_l - der_x_p * der_y_l) * radius_factor / cosphi;
        let mut major = (parallel * parallel + meridional * meridional + 2.0 * areal).sqrt();
        let mut minor_sq = parallel * parallel + meridional * meridional - 2.0 * areal;
        if minor_sq < 0.0 {
            minor_sq = 0.0;
        }
        let minor_span = minor_sq.sqrt();
        let semi_minor = 0.5 * (major - minor_span);
        major = 0.5 * (major + minor_span);
        let omega = if major + semi_minor == 0.0 {
            0.0
        } else {
            2.0 * ((major - semi_minor) / (major + semi_minor)).asin()
        };
        Some([
            meridional,
            parallel,
            areal,
            omega.to_degrees(),
            major,
            semi_minor,
        ])
    }
}

fn two_numbers(line: &str) -> Option<(f64, f64)> {
    let mut parts = line.split_whitespace();
    let lon = parts.next()?.parse().ok()?;
    let lat = parts.next()?.parse().ok()?;
    Some((lon, lat))
}

fn format_fixed(value: f64, precision: usize) -> String {
    format!("{value:.precision$}")
}

fn format_g(value: f64) -> String {
    if value == 0.0 || value.abs() < 5e-7 {
        return "0".to_owned();
    }
    let exp = decade(value.abs());
    if (-4..6).contains(&exp) {
        let precision = usize::try_from(6 - exp - 1).unwrap_or(6);
        let mut text = format!("{value:.precision$}");
        if text.contains('.') {
            while text.ends_with('0') {
                text.pop();
            }
            if text.ends_with('.') {
                text.pop();
            }
        }
        text
    } else {
        format!("{value:.6e}")
    }
}

fn decade(value: f64) -> i32 {
    let mut exp = 0i32;
    let mut scaled = value;
    if scaled >= 10.0 {
        while scaled >= 10.0 && exp < 30 {
            scaled /= 10.0;
            exp += 1;
        }
    } else if scaled > 0.0 && scaled < 1.0 {
        while scaled < 1.0 && exp > -30 {
            scaled *= 10.0;
            exp -= 1;
        }
    }
    exp
}
