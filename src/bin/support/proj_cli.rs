//! `proj` and `invproj`: project coordinates, and with `-S` print scale
//! factors, as PROJ 9.9.0's `apps/proj.cpp` does for the switches this port
//! takes.
//!
//! A `+proj` definition is the operation. An `EPSG:` code is the projected
//! CRS: input is longitude and latitude, and `-S` differentiates that
//! projection. Forward input is read as degrees-minutes-seconds, inverse
//! input as plain numbers. `invproj`, or `-I`, runs the inverse. `-E` echoes
//! the input values, `-f` sets the output format, `-t` the tag that copies a
//! line as it is, and `-w`/`-W` the seconds of the degree-minute output.
//!
//! Not ported: the binary formats (`-b`), the reversal of the input and output
//! (`-r`, `-s`), and the other `proj.cpp` switches; they answer as an invalid
//! option.

use std::f64::consts::FRAC_PI_2;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use proj_rs::{create_from_user_input, Crs, DatabaseContext, Operation, Xyzt};

use super::dms::{
    c_strtod, direction_of, dmstor, format_number, program_name, rtodms, Direction, Dms, Width,
    RAD_TO_DEG,
};

/// Step of the numerical derivatives in `pj_factors`, radians.
const DERIV_STEP: f64 = 1e-5;
/// Tolerance of the latitude check in `pj_factors`, radians.
const EPS: f64 = 1e-12;
/// `MAX_LINE` of `proj.cpp`: `fgets` reads one byte less than this.
const MAX_LINE: usize = 200;

/// Entry point of `proj` and `invproj`: the program name decides the direction.
pub(crate) fn run() -> ExitCode {
    let program = program_name();
    let direction = direction_of(&program);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        let _ = writeln!(
            io::stderr(),
            "usage: {program} [-EfIStTwW [args]] [+opt[=arg] ...] [EPSG:code]"
        );
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(&args, direction) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(io::stderr(), "{program}: {message}");
            return ExitCode::from(1);
        }
    };
    let job = match Job::open(&options) {
        Ok(job) => job,
        Err(message) => {
            let _ = writeln!(io::stderr(), "{program}: {message}");
            return ExitCode::from(3);
        }
    };
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        let _ = writeln!(io::stderr(), "{program}: could not read input");
        return ExitCode::from(1);
    }
    let mut stdout = io::stdout().lock();
    let mut output = Vec::new();
    // PROJ keeps one error state for the whole run: after a failed factor
    // computation, the later `-S` lines report it too.
    let mut error_seen = false;
    let mut pos = 0;
    while let Some(line) = next_line(&input, &mut pos) {
        if line.first() == Some(&options.tag()) {
            output.extend_from_slice(&line);
            continue;
        }
        output.extend_from_slice(&record(&line, &options, &job, &mut error_seen));
    }
    let _ = stdout.write_all(&output);
    ExitCode::SUCCESS
}

/// The switches of `proj.cpp` that this port takes.
struct Options {
    inverse: bool,
    echo: bool,
    factors: bool,
    oform: Option<String>,
    tag: Option<u8>,
    dms: Dms,
    definition: String,
    crs: Option<String>,
}

impl Options {
    fn parse(args: &[String], direction: Direction) -> Result<Self, String> {
        let mut options = Self {
            inverse: direction == Direction::Inverse,
            echo: false,
            factors: false,
            oform: None,
            tag: None,
            dms: Dms::default(),
            definition: String::new(),
            crs: None,
        };
        let mut plus = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            if arg == "-h" || arg == "--help" {
                // The usage is printed by `run`, before parsing.
            } else if let Some(letters) = arg.strip_prefix('-').filter(|rest| !rest.is_empty()) {
                let mut chars = letters.chars();
                while let Some(letter) = chars.next() {
                    match letter {
                        'E' => options.echo = true,
                        'I' => options.inverse = true,
                        'S' => options.factors = true,
                        'f' => {
                            index += 1;
                            let value = args
                                .get(index)
                                .ok_or_else(|| "missing -f value".to_owned())?;
                            if format_number(value, 0.0).is_none() {
                                return Err(format!("invalid format string {value}"));
                            }
                            options.oform = Some(value.clone());
                        }
                        't' => {
                            let tag = chars
                                .next()
                                .ok_or_else(|| "missing -t col. 1 tag".to_owned())?;
                            options.tag = Some(
                                u8::try_from(tag).map_err(|_| "tag must be ASCII".to_owned())?,
                            );
                        }
                        'w' | 'W' => {
                            let digit = chars
                                .next()
                                .and_then(|c| c.to_digit(10))
                                .ok_or_else(|| "-W argument missing or non-digit".to_owned())?;
                            let width = if letter == 'W' {
                                Width::Fixed
                            } else {
                                Width::Trimmed
                            };
                            options.dms.set(usize::try_from(digit).unwrap_or(0), width);
                        }
                        other => return Err(format!("invalid option: -{other}")),
                    }
                }
            } else if arg.starts_with('+') {
                plus.push(arg.to_owned());
            } else if options.crs.is_none() && plus.is_empty() {
                options.crs = Some(arg.to_owned());
            } else {
                return Err(format!("unexpected argument {arg}"));
            }
            index += 1;
        }
        options.definition = plus.join(" ");
        Ok(options)
    }

    fn tag(&self) -> u8 {
        self.tag.unwrap_or(b'#')
    }
}

/// The operation, and what its input and output are.
struct Job {
    operation: Operation,
    /// Output is in degrees, as the output of the forward operation is angular.
    degrees_forward: bool,
    /// Output is in degrees, as the input of the forward operation is angular.
    degrees_inverse: bool,
    north_first: bool,
}

impl Job {
    fn open(options: &Options) -> Result<Self, String> {
        let (definition, north_first) = match &options.crs {
            Some(crs) => projected_definition(crs)?,
            None if options.definition.is_empty() => return Err("missing projection".to_owned()),
            None => (options.definition.clone(), false),
        };
        let operation =
            Operation::create_projection(&definition).map_err(|error| error.to_string())?;
        let degrees_forward = operation.angular_output(false)
            || definition.contains("o_proj=lat")
            || definition.contains("o_proj=long");
        let degrees_inverse = operation.angular_input(false);
        Ok(Self {
            operation,
            degrees_forward,
            degrees_inverse,
            north_first,
        })
    }

    /// Forward projection of a longitude and latitude in radians.
    fn project(&self, lon: f64, lat: f64) -> Result<(f64, f64), ()> {
        let coord = Xyzt {
            x: lon,
            y: lat,
            z: 0.0,
            t: 0.0,
        };
        let got = self.operation.forward(coord).map_err(|_| ())?;
        Ok((got.x, got.y))
    }

    /// Inverse projection of a point, to a longitude and latitude in radians.
    fn unproject(&self, x: f64, y: f64) -> Result<(f64, f64), ()> {
        let coord = Xyzt {
            x,
            y,
            z: 0.0,
            t: 0.0,
        };
        let got = self.operation.inverse(coord).map_err(|_| ())?;
        Ok((got.x, got.y))
    }

    /// Scale factors in the order `-S` prints them: meridional, parallel,
    /// areal, angular distortion in degrees, and the two Tissot semi-axes.
    /// This is `pj_factors` of PROJ's `factors.cpp`, at a longitude and
    /// latitude in radians.
    fn factors(&self, lam: f64, phi: f64) -> Result<[f64; 6], ()> {
        let (a, es) = self.operation.distance_ellipsoid();
        let mut phi = phi;
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

/// The output of one input line: the coordinates, then the factors, then the
/// rest of the line, as `proj.cpp` writes them.
fn record(line: &[u8], options: &Options, job: &Job, error_seen: &mut bool) -> Vec<u8> {
    let mut s = 0;
    let (first, used) = if options.inverse {
        c_strtod(&line[s..])
    } else {
        dmstor(&line[s..])
    };
    s += used;
    let (second, used) = if options.inverse {
        c_strtod(&line[s..])
    } else {
        dmstor(&line[s..])
    };
    s += used;
    let mut out = Vec::new();
    if options.echo {
        out.extend_from_slice(&line[..s]);
        out.push(b'\t');
    }
    let mut text = String::new();
    let mut lp = None;
    if first.is_finite() {
        // Forward input is (lon, lat), inverse input is (x, y).
        let result = if options.inverse {
            job.unproject(first, second)
        } else {
            job.project(first, second)
        };
        lp = result.ok();
    }
    let mut factors_bad = false;
    let mut factors = None;
    if options.factors {
        // `proj.cpp` takes the factors at the values it read, as radians, in both
        // directions: in inverse mode those are x and y, which is why PROJ
        // reports them as failed for projected input.
        if first.is_finite() {
            let found = job.factors(first, second);
            *error_seen |= found.is_err();
            factors_bad = *error_seen;
            factors = found.ok();
        } else {
            factors_bad = true;
        }
    }
    match lp {
        Some((a, b)) => {
            if options.inverse {
                let (lon, lat) = if job.degrees_inverse {
                    (a * RAD_TO_DEG, b * RAD_TO_DEG)
                } else {
                    (a, b)
                };
                if let Some(format) = &options.oform {
                    text.push_str(&format_number(format, lon).unwrap_or_default());
                    text.push('\t');
                    text.push_str(&format_number(format, lat).unwrap_or_default());
                } else {
                    text.push_str(&rtodms(a, options.dms, b'E', b'W'));
                    text.push('\t');
                    text.push_str(&rtodms(b, options.dms, b'N', b'S'));
                }
            } else {
                let (east, north) = if job.degrees_forward {
                    (a * RAD_TO_DEG, b * RAD_TO_DEG)
                } else {
                    (a, b)
                };
                let format = options.oform.as_deref().unwrap_or("%.2f");
                let (first_out, second_out) = if job.north_first {
                    (north, east)
                } else {
                    (east, north)
                };
                text.push_str(&format_number(format, first_out).unwrap_or_default());
                text.push('\t');
                text.push_str(&format_number(format, second_out).unwrap_or_default());
            }
        }
        None => text.push_str("*\t*"),
    }
    if options.factors {
        match factors {
            Some(values) if !factors_bad => {
                let parts: Vec<String> = values
                    .iter()
                    .map(|value| format_number("%g", *value).unwrap_or_default())
                    .collect();
                text.push_str(&format!("\t<{}>", parts.join(" ")));
            }
            _ => text.push_str("\t<* * * * * *>"),
        }
    }
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(&line[s..]);
    out
}

/// `fgets` with `MAX_LINE`: a line is read up to its newline or `MAX_LINE - 1`
/// bytes. A longer line is cut there and given a newline, and its rest is skipped.
fn next_line(input: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let rest = input.get(*pos..)?;
    if rest.is_empty() {
        return None;
    }
    let limit = rest.len().min(MAX_LINE - 1);
    let (mut line, cut) = match rest[..limit].iter().position(|&c| c == b'\n') {
        Some(index) => (rest[..=index].to_vec(), false),
        None => (rest[..limit].to_vec(), true),
    };
    *pos += line.len();
    if cut {
        line.push(b'\n');
        while let Some(&c) = input.get(*pos) {
            *pos += 1;
            if c == b'\n' {
                break;
            }
        }
    }
    Some(line)
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
