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
//! Also: `-d` (decimals), `-e` (text of an unprojectable point), `-m` (scale), `-r`/`-s`
//! (reversed input and output), and `-i`/`-o`/`-b` (native doubles, two per point).
//!
//! Not ported: the lists `-l` (see `geod`/`proj` lists in `docs/PORTING-cli-rest.md`) and the
//! monitor output of `-v`/`-V`, which prints PROJ's internal parameter dump; they answer as an
//! invalid option.

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
    if options.bin_in {
        // `process`: each record is two native doubles, `PJ_UV`; no tag, no echo.
        for record_bytes in input.chunks_exact(16) {
            let u = f64::from_ne_bytes(to_array(&record_bytes[..8]));
            let v = f64::from_ne_bytes(to_array(&record_bytes[8..]));
            let measure = measure(u, v, &options, &job, &mut error_seen);
            if options.bin_out {
                output.extend_from_slice(&binary_out(&measure));
            } else {
                output.extend_from_slice(text_out(&measure, &options, &job).as_bytes());
                output.push(b'\n');
            }
        }
    } else {
        let mut pos = 0;
        while let Some(line) = next_line(&input, &mut pos) {
            if line.first() == Some(&options.tag()) {
                if !options.bin_out {
                    output.extend_from_slice(&line);
                }
                continue;
            }
            output.extend_from_slice(&record(&line, &options, &job, &mut error_seen));
        }
    }
    let _ = stdout.write_all(&output);
    ExitCode::SUCCESS
}

/// The eight bytes of a native double, from a slice of at least eight.
fn to_array(bytes: &[u8]) -> [u8; 8] {
    let mut buf = [0_u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    buf
}

/// `atoi` of PROJ's `-d` argument: optional sign and leading digits, 0 otherwise.
fn atoi(text: &str) -> i64 {
    let trimmed = text.trim_start();
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let end = digits
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse::<i64>().map_or(0, |value| sign * value)
}

/// The factor of `-m`: `1/x` or `1:x` gives `1/x`, otherwise the number itself. `atof` gives 0
/// for text that is no number, and PROJ refuses 0.
fn scale_argument(text: &str) -> Result<f64, String> {
    let value = match text.get(..2) {
        Some("1/" | "1:") => 1.0 / c_strtod(text[2..].as_bytes()).0,
        _ => c_strtod(text.as_bytes()).0,
    };
    if value == 0.0 || !value.is_finite() {
        return Err("invalid scale argument".to_owned());
    }
    Ok(value)
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
    /// `-r`: the input pair is read as (second, first).
    reverse_in: bool,
    /// `-s`: the output pair is written second, first.
    reverse_out: bool,
    /// `-i` and `-b`: the input is native doubles, two per point.
    bin_in: bool,
    /// `-o` and `-b`: the output is native doubles, two per point.
    bin_out: bool,
    /// `-e`: the text written for a point that cannot be projected.
    error_text: String,
    /// `-m` with `-I`: the factor applied to the input before the inverse.
    prescale: Option<f64>,
    /// `-m` without `-I`: the factor applied to the output of the forward projection.
    postscale: Option<f64>,
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
            reverse_in: false,
            reverse_out: false,
            bin_in: false,
            bin_out: false,
            error_text: "*\t*".to_owned(),
            prescale: None,
            postscale: None,
        };
        let mut scale = None;
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
                        'b' => {
                            options.bin_in = true;
                            options.bin_out = true;
                        }
                        'i' => options.bin_in = true,
                        'o' => options.bin_out = true,
                        'r' => options.reverse_in = true,
                        's' => options.reverse_out = true,
                        'e' => {
                            index += 1;
                            options.error_text = args
                                .get(index)
                                .ok_or_else(|| "missing argument for -e".to_owned())?
                                .clone();
                        }
                        'm' => {
                            index += 1;
                            let value = args
                                .get(index)
                                .ok_or_else(|| "missing argument for -m".to_owned())?;
                            scale = Some(scale_argument(value)?);
                        }
                        'd' => {
                            index += 1;
                            let digits = args
                                .get(index)
                                .ok_or_else(|| "missing argument for -d".to_owned())?;
                            options.oform = Some(format!("%.{}f", atoi(digits)));
                            // `proj.cpp` ends the switches of this argument here.
                            break;
                        }
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
        // `proj.cpp`: an inverse scales its input by the reciprocal, which is the
        // factor of the output of the forward projection.
        if let Some(factor) = scale {
            if options.inverse {
                options.prescale = Some(1.0 / factor);
            } else {
                options.postscale = Some(factor);
            }
        }
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

    /// Forward projection of a longitude and latitude in radians. The range checks of
    /// `fwd_prepare` (`pj_fwd`, `fwd.cpp`) come first: a latitude within 1e-12 of the pole is the
    /// pole, a latitude beyond it or a negative one past the pole is refused, and a longitude
    /// beyond 10 radians is refused.
    fn project(&self, lon: f64, lat: f64) -> Result<(f64, f64), ()> {
        let mut lat = lat;
        if lat.abs() > FRAC_PI_2 {
            if lat > 0.0 && lat - FRAC_PI_2 <= 1e-12 {
                lat = FRAC_PI_2;
            } else {
                return Err(());
            }
        }
        if lon.abs() > 10.0 {
            return Err(());
        }
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
        let (x_l, y_p, x_p, y_l) = (x_l / a, y_p / a, x_p / a, y_l / a);
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
    // `-r`: the first number read is the second coordinate.
    let (u, v) = if options.reverse_in {
        (second, first)
    } else {
        (first, second)
    };
    let measure = measure(u, v, options, job, error_seen);
    if options.bin_out {
        return binary_out(&measure).to_vec();
    }
    let mut out = Vec::new();
    if options.echo {
        out.extend_from_slice(&line[..s]);
        out.push(b'\t');
    }
    out.extend_from_slice(text_out(&measure, options, job).as_bytes());
    out.extend_from_slice(&line[s..]);
    out
}

/// One point after `proj.cpp`'s transformation: its coordinates, or `None` for a point that
/// cannot be projected, and the text of its factors when `-S` is on.
struct Measure {
    value: Option<(f64, f64)>,
    factors: Option<String>,
}

/// The transformation of one point, with the factors of `-S` (`process`, `proj.cpp`). In
/// inverse mode the input is scaled by `-m` first; in forward mode the output is.
fn measure(u: f64, v: f64, options: &Options, job: &Job, error_seen: &mut bool) -> Measure {
    let (u, v) = match options.prescale {
        Some(factor) => (u * factor, v * factor),
        None => (u, v),
    };
    let finite = u.is_finite() && v.is_finite();
    let mut factors = None;
    if options.factors {
        // `proj.cpp` takes the factors at the values it read, as radians, in both
        // directions: in inverse mode those are x and y, which is why PROJ
        // reports them as failed for projected input.
        let text = if finite {
            let found = job.factors(u, v);
            *error_seen |= found.is_err();
            match found {
                Ok(values) if !*error_seen => {
                    let parts: Vec<String> = values
                        .iter()
                        .map(|value| format_number("%g", *value).unwrap_or_default())
                        .collect();
                    format!("\t<{}>", parts.join(" "))
                }
                _ => "\t<* * * * * *>".to_owned(),
            }
        } else {
            "\t<* * * * * *>".to_owned()
        };
        factors = Some(text);
    }
    // Forward input is (lon, lat), inverse input is (x, y).
    let value = if finite {
        let result = if options.inverse {
            job.unproject(u, v)
        } else {
            job.project(u, v)
        };
        result.ok().map(|(a, b)| match options.postscale {
            Some(factor) => (a * factor, b * factor),
            None => (a, b),
        })
    } else {
        None
    };
    Measure { value, factors }
}

/// The text of a measured point: its two coordinates (or `-e` for an error), then its factors.
fn text_out(measure: &Measure, options: &Options, job: &Job) -> String {
    let mut text = match measure.value {
        Some((a, b)) => coordinates_text(a, b, options, job),
        None => options.error_text.clone(),
    };
    if let Some(factors) = &measure.factors {
        text.push_str(factors);
    }
    text
}

/// The two coordinates of a point, in the order `-s` and the CRS axis order ask for.
fn coordinates_text(a: f64, b: f64, options: &Options, job: &Job) -> String {
    if options.inverse {
        let (lon, lat) = if job.degrees_inverse {
            (a * RAD_TO_DEG, b * RAD_TO_DEG)
        } else {
            (a, b)
        };
        if let Some(format) = &options.oform {
            let first = format_number(format, lon).unwrap_or_default();
            let second = format_number(format, lat).unwrap_or_default();
            joined(first, second, options.reverse_out)
        } else {
            let first = rtodms(a, options.dms, b'E', b'W');
            let second = rtodms(b, options.dms, b'N', b'S');
            joined(first, second, options.reverse_out)
        }
    } else {
        let (east, north) = if job.degrees_forward {
            (a * RAD_TO_DEG, b * RAD_TO_DEG)
        } else {
            (a, b)
        };
        let format = options.oform.as_deref().unwrap_or("%.2f");
        let first = format_number(format, east).unwrap_or_default();
        let second = format_number(format, north).unwrap_or_default();
        joined(first, second, job.north_first != options.reverse_out)
    }
}

fn joined(first: String, second: String, swap: bool) -> String {
    if swap {
        format!("{second}\t{first}")
    } else {
        format!("{first}\t{second}")
    }
}

/// The binary output of a measured point: its two native doubles, or the error marker of
/// `HUGE_VAL` for a point that cannot be projected (`process`, binary branch).
fn binary_out(measure: &Measure) -> [u8; 16] {
    let (a, b) = measure.value.unwrap_or((f64::INFINITY, f64::INFINITY));
    let mut out = [0_u8; 16];
    out[..8].copy_from_slice(&a.to_ne_bytes());
    out[8..].copy_from_slice(&b.to_ne_bytes());
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
