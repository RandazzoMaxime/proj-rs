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

use std::f64::consts::FRAC_PI_2;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use proj_rs::{create_from_user_input, Crs, DatabaseContext, Operation, Xyzt};

/// Step of the numerical derivatives in `pj_factors`, radians.
const DERIV_STEP: f64 = 1e-5;
/// Tolerance of the latitude check in `pj_factors`, radians.
const EPS: f64 = 1e-12;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("proj: {message}");
            return ExitCode::from(1);
        }
    };
    if options.help || args.is_empty() {
        eprintln!("usage: proj [-ESf] [+opt[=arg] ...] [EPSG:code]");
        return ExitCode::SUCCESS;
    }
    let job = match Job::open(&options) {
        Ok(job) => job,
        Err(message) => {
            eprintln!("proj: {message}");
            return ExitCode::from(3);
        }
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    // PROJ keeps one error state for the whole run. Once a projection or a
    // factor computation has failed, later `-S` lines report it too.
    let mut error_seen = false;
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
        let mut factors_bad = false;
        let factors = if options.factors {
            let found = job.factors(lon, lat);
            error_seen |= found.is_err();
            factors_bad = error_seen;
            found.ok()
        } else {
            None
        };
        let projected = job.project(lon, lat);
        error_seen |= projected.is_err();
        let mut text = String::new();
        if options.echo {
            text.push_str(line.trim());
            text.push('\t');
        }
        match projected {
            Ok((east, north)) => {
                let (first, second) = if job.north_first {
                    (north, east)
                } else {
                    (east, north)
                };
                text.push_str(&format_fixed(first, options.precision));
                text.push('\t');
                text.push_str(&format_fixed(second, options.precision));
            }
            Err(()) => text.push_str("*\t*"),
        }
        if options.factors {
            match factors {
                Some(factors) if !factors_bad => {
                    text.push_str("\t<");
                    text.push_str(
                        &factors
                            .iter()
                            .map(|value| format_g(*value))
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    text.push('>');
                }
                _ => text.push_str("\t<* * * * * *>"),
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
                let text = args
                    .get(index)
                    .ok_or_else(|| "missing -f value".to_owned())?;
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

struct Job {
    operation: Operation,
    degrees_out: bool,
    north_first: bool,
}

impl Job {
    fn open(options: &Options) -> Result<Self, String> {
        let (definition, north_first) = match &options.crs {
            Some(crs) => projected_definition(crs)?,
            None if options.definition.is_empty() => return Err("missing projection".to_owned()),
            None => (options.definition.clone(), false),
        };
        let operation = Operation::create(&definition).map_err(|error| error.to_string())?;
        let degrees_out = operation.angular_output(false)
            || definition.contains("o_proj=lat")
            || definition.contains("o_proj=long");
        Ok(Self {
            operation,
            degrees_out,
            north_first,
        })
    }

    fn project(&self, lon: f64, lat: f64) -> Result<(f64, f64), ()> {
        let coord = Xyzt {
            x: lon.to_radians(),
            y: lat.to_radians(),
            z: 0.0,
            t: 0.0,
        };
        let got = self.operation.forward(coord).map_err(|_| ())?;
        if self.degrees_out {
            Ok((got.x.to_degrees(), got.y.to_degrees()))
        } else {
            Ok((got.x, got.y))
        }
    }

    /// Scale factors in the order `-S` prints them: meridional, parallel,
    /// areal, angular distortion in degrees, and the two Tissot semi-axes.
    /// This is `pj_factors` of PROJ's `factors.cpp`.
    fn factors(&self, lon: f64, lat: f64) -> Result<[f64; 6], ()> {
        let (a, es) = self.operation.distance_ellipsoid();
        let lam = lon.to_radians();
        let mut phi = lat.to_radians();
        if a == 0.0 || phi.abs() - FRAC_PI_2 > EPS || lam.abs() > 10.0 {
            return Err(());
        }
        let h = DERIV_STEP;
        if phi.abs() > FRAC_PI_2 - h {
            phi = phi.signum() * (FRAC_PI_2 - h);
        }
        let [x_l, y_p, x_p, y_l] = self.derivatives(lam, phi, h).ok_or(())?;
        let scale = 1.0 / a;
        let (x_l, y_p, x_p, y_l) = (x_l * scale, y_p * scale, x_p * scale, y_l * scale);
        let cosphi = phi.cos();
        let mut meridional = x_p.hypot(y_p);
        let mut parallel = x_l.hypot(y_l) / cosphi;
        let r = if es != 0.0 {
            let t = 1.0 - es * phi.sin() * phi.sin();
            let n = t.sqrt();
            meridional *= t * n / (1.0 - es);
            parallel *= n;
            t * t / (1.0 - es)
        } else {
            1.0
        };
        let areal = (y_p * x_l - x_p * y_l) * r / cosphi;
        let t = parallel * parallel + meridional * meridional;
        let major = (t + 2.0 * areal).sqrt();
        let t = t - 2.0 * areal;
        let t = if t > 0.0 { t.sqrt() } else { 0.0 };
        let minor = 0.5 * (major - t);
        let major = 0.5 * (major + t);
        let omega = 2.0 * aasin((major - minor) / (major + minor));
        let factors = [
            meridional,
            parallel,
            areal,
            omega.to_degrees(),
            major,
            minor,
        ];
        if factors.iter().all(|value| value.is_finite()) {
            Ok(factors)
        } else {
            Err(())
        }
    }

    /// Central differences of the forward operation, `pj_deriv` of PROJ's
    /// `deriv.cpp`. Returns `[x_lam, y_phi, x_phi, y_lam]`, in metres per radian.
    fn derivatives(&self, lam: f64, phi: f64, h: f64) -> Option<[f64; 4]> {
        let forward = |lam: f64, phi: f64| -> Option<(f64, f64)> {
            let got = self
                .operation
                .forward(Xyzt {
                    x: lam,
                    y: phi,
                    z: 0.0,
                    t: 0.0,
                })
                .ok()?;
            Some((got.x, got.y))
        };
        let (lam, phi) = (lam + h, phi + h);
        if phi.abs() > FRAC_PI_2 {
            return None;
        }
        let step = 2.0 * h;
        let (x, y) = forward(lam, phi)?;
        let (mut x_l, mut y_p, mut x_p, mut y_l) = (x, y, x, y);
        let phi = phi - step;
        if phi.abs() > FRAC_PI_2 {
            return None;
        }
        let (x, y) = forward(lam, phi)?;
        x_l += x;
        y_p -= y;
        x_p -= x;
        y_l += y;
        let lam = lam - step;
        let (x, y) = forward(lam, phi)?;
        x_l -= x;
        y_p -= y;
        x_p -= x;
        y_l -= y;
        let phi = phi + step;
        let (x, y) = forward(lam, phi)?;
        x_l -= x;
        y_p += y;
        x_p += x;
        y_l -= y;
        let divisor = 2.0 * step;
        Some([x_l / divisor, y_p / divisor, x_p / divisor, y_l / divisor])
    }
}

/// PROJ string of the projected CRS `text` names, and whether its first axis
/// is north. A compound CRS stands for its horizontal part. The datum is not
/// changed: input longitude is Greenwich, and the prime meridian of the CRS is
/// in the string.
fn projected_definition(text: &str) -> Result<(String, bool), String> {
    let db = DatabaseContext::create().map_err(|error| error.to_string())?;
    let crs = create_from_user_input(text, &db).map_err(|_| "CRS is not parseable".to_owned())?;
    let projected = match crs {
        Crs::Projected(projected) => projected,
        Crs::Compound(compound) => match compound.components().first() {
            Some(Crs::Projected(projected)) => projected.clone(),
            _ => return Err("CRS must be projected".to_owned()),
        },
        _ => return Err("CRS must be projected".to_owned()),
    };
    let north_first = projected
        .coordinate_system()
        .axes()
        .first()
        .is_some_and(|axis| axis.direction() == "north");
    let definition = projected
        .export_to_proj_string()
        .map_err(|error| error.to_string())?;
    Ok((definition, north_first))
}

/// `aasin` of PROJ: an argument beyond ±1 is clamped, without an error.
fn aasin(value: f64) -> f64 {
    if value.abs() >= 1.0 {
        if value < 0.0 {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        }
    } else {
        value.asin()
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

/// C's `%g`: six significant digits, trailing zeros removed, and an exponent
/// of at least two digits.
fn format_g(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let scientific = format!("{value:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    if (-4..6).contains(&exponent) {
        let precision = usize::try_from(5 - exponent).unwrap_or(0);
        trim_zeros(&format!("{value:.precision$}"))
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!(
            "{}e{sign}{:02}",
            trim_zeros(mantissa),
            exponent.unsigned_abs()
        )
    }
}

fn trim_zeros(text: &str) -> String {
    if !text.contains('.') {
        return text.to_owned();
    }
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::{format_g, trim_zeros};

    #[test]
    fn format_g_matches_c_printf() {
        assert_eq!(format_g(0.999_877), "0.999877");
        assert_eq!(format_g(1.0), "1");
        assert_eq!(format_g(0.0), "0");
        assert_eq!(format_g(500_000.0), "500000");
        assert_eq!(format_g(1_234_567.0), "1.23457e+06");
        assert_eq!(format_g(8.538_78e-7), "8.53878e-07");
        assert_eq!(format_g(1.198_788e-6), "1.19879e-06");
        assert_eq!(format_g(-1.005_59), "-1.00559");
        assert_eq!(format_g(0.000_1), "0.0001");
    }

    #[test]
    fn trim_zeros_keeps_integers() {
        assert_eq!(trim_zeros("100"), "100");
        assert_eq!(trim_zeros("1.2000"), "1.2");
    }
}
