//! `geod` and `invgeod`: geodesics from lines of text, as PROJ 9.9.0's
//! `apps/geod.cpp` and `geod_set.cpp` compute them.
//!
//! Forward mode reads `lat lon azimuth distance` and writes the end point and
//! its back azimuth. Inverse mode reads `lat1 lon1 lat2 lon2` and writes the
//! two azimuths and the distance. The program name `invgeod`, or `-I`, selects
//! inverse mode. `+lat_1 +lon_1 ...` arguments select the arc and geodesic
//! modes, which take no input file.
//!
//! Not ported: `-l` (lists of ellipsoids and units) and `+sunits` (a unit
//! other than metres), which need PROJ's unit and ellipsoid tables.

use std::f64::consts::PI;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use proj_rs::geod::{Geod, Line};
use proj_rs::Operation;

use super::dms::{
    c_strtod, direction_of, dmstor, format_number, program_name, rtodms, Direction, Dms, Width,
    DEG_TO_RAD, RAD_TO_DEG,
};

/// `MAXLINE` of `geod.cpp`: `fgets` reads one byte less than this.
const MAX_LINE: usize = 200;
/// `MAX_PARGS` of `geod.cpp`: the most `+` arguments.
const MAX_PARGS: usize = 50;
/// `adjlon` of PROJ: a longitude in radians brought into -π..π, with a small
/// overshoot kept so that the date line does not flip the sign.
fn adjlon(longitude: f64) -> f64 {
    if longitude.abs() < PI + 1e-12 {
        return longitude;
    }
    let two_pi = 2.0 * PI;
    let shifted = longitude + PI;
    let shifted = shifted - two_pi * (shifted / two_pi).floor();
    shifted - PI
}

/// Keys of `pj_ell_set`: the arguments that describe the ellipsoid.
const ELLIPSOID_KEYS: &[&str] = &[
    "ellps", "a", "b", "rf", "f", "es", "e", "R", "R_A", "R_V", "R_a", "R_g", "R_h", "R_lat_a",
    "R_lat_g",
];

/// Entry point of `geod` and `invgeod`: the program name decides the direction.
pub(crate) fn run() -> ExitCode {
    let program = program_name();
    let direction = direction_of(&program);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        let _ = writeln!(
            io::stderr(),
            "usage: {program} [-afFIlptwW [args]] [+opt[=arg] ...] [file ...]"
        );
        return ExitCode::SUCCESS;
    }
    let mut options = Options {
        inverse: direction == Direction::Inverse,
        ..Options::default()
    };
    let mut pargs = Vec::new();
    let mut files = Vec::new();
    if let Err(message) = parse_args(&args, &mut options, &mut pargs, &mut files) {
        return fail(&program, &message);
    }
    let (mut calc, mode) = match setup(&pargs) {
        Ok(setup) => setup,
        Err(message) => return fail(&program, &message),
    };
    let mut stdout = io::stdout().lock();
    if let Some(mode) = mode {
        if !files.is_empty() {
            return fail(&program, "files specified for arc/geodesic mode");
        }
        let text = run_mode(&mut calc, &options, &mode);
        let _ = stdout.write_all(text.as_bytes());
        return ExitCode::SUCCESS;
    }
    if files.is_empty() {
        files.push("-".to_owned());
    }
    for name in &files {
        let input = if name == "-" {
            let mut bytes = Vec::new();
            if io::stdin().read_to_end(&mut bytes).is_err() {
                return fail(&program, "could not read <stdin>");
            }
            bytes
        } else {
            match std::fs::read(name) {
                Ok(bytes) => bytes,
                Err(_) => {
                    // `emess(-2, ...)`: reported, and the other files still run.
                    let _ = writeln!(io::stderr(), "<{program}>: input file: {name}");
                    continue;
                }
            }
        };
        let output = process(&input, &options, &mut calc);
        let _ = stdout.write_all(&output);
    }
    ExitCode::SUCCESS
}

/// Message and status of `emess(1, ...)`.
fn fail(program: &str, message: &str) -> ExitCode {
    let mut stderr = io::stderr();
    let _ = writeln!(stderr, "<{program}>: {message}");
    let _ = writeln!(stderr, "program abnormally terminated");
    ExitCode::from(1)
}

/// The switches of `geod.cpp`.
#[derive(Default)]
struct Options {
    /// `-a`: the full set of values.
    full: bool,
    /// Inverse mode: the `invgeod` name or `-I`.
    inverse: bool,
    /// `-t`: the character that starts a line copied as it is. Default `#`.
    tag: Option<u8>,
    /// `-p`: azimuths as positive values.
    pos_azi: bool,
    /// `-f`: format of the angles, in numbers and not in degrees-minutes-seconds.
    oform: Option<String>,
    /// `-F`: format of the distance.
    osform: Option<String>,
    /// `-w` and `-W`: how angles are written in degrees, minutes and seconds.
    dms: Dms,
}

impl Options {
    fn tag(&self) -> u8 {
        self.tag.unwrap_or(b'#')
    }

    fn osform(&self) -> &str {
        self.osform.as_deref().unwrap_or("%.3f")
    }
}

fn parse_args(
    args: &[String],
    options: &mut Options,
    pargs: &mut Vec<String>,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_bytes();
        if arg.first() == Some(&b'-') {
            if arg.len() == 1 {
                files.push("-".to_owned());
            }
            let mut j = 1;
            while j < arg.len() {
                match arg[j] {
                    b'a' => options.full = true,
                    b'I' => options.inverse = true,
                    b'p' => options.pos_azi = true,
                    b't' => {
                        j += 1;
                        let tag = *arg.get(j).ok_or("missing -t col. 1 tag")?;
                        options.tag = Some(tag);
                    }
                    letter @ (b'W' | b'w') => {
                        j += 1;
                        let digit = arg
                            .get(j)
                            .copied()
                            .filter(u8::is_ascii_digit)
                            .ok_or("-W argument missing or non-digit")?;
                        let width = if letter == b'W' {
                            Width::Fixed
                        } else {
                            Width::Trimmed
                        };
                        options.dms.set(usize::from(digit - b'0'), width);
                    }
                    letter @ (b'f' | b'F') => {
                        index += 1;
                        let value = args
                            .get(index)
                            .ok_or_else(|| format!("missing argument for -{}", char::from(letter)))?
                            .clone();
                        if letter == b'f' {
                            options.oform = Some(value);
                        } else {
                            options.osform = Some(value);
                        }
                    }
                    other => return Err(format!("invalid option: -{}", char::from(other))),
                }
                j += 1;
            }
        } else if arg.first() == Some(&b'+') {
            if pargs.len() >= MAX_PARGS {
                return Err("overflowed + argument table".to_owned());
            }
            pargs.push(args[index][1..].to_owned());
        } else {
            files.push(args[index].clone());
        }
        index += 1;
    }
    Ok(())
}

/// The arc or geodesic mode of `geod_set`, once its start point is set.
enum Mode {
    /// `n_alpha` azimuths, each `step` radians further round.
    Arc { steps: usize, step: f64 },
    /// `n_S` intervals of the line from the start point to the end point.
    Line { steps: usize },
}

/// `geod_set`: the ellipsoid, and the state the arc and line modes start from.
fn setup(pargs: &[String]) -> Result<(Calc, Option<Mode>), String> {
    if pargs.is_empty() {
        return Err("no arguments in initialization list".to_owned());
    }
    let ellipsoid: Vec<String> = pargs
        .iter()
        .filter(|parg| ELLIPSOID_KEYS.contains(&key_of(parg)))
        .map(|parg| format!("+{parg}"))
        .collect();
    if ellipsoid.is_empty() {
        // `pj_ell_set` has no default: without an ellipsoid argument it fails.
        return Err("ellipsoid setup failure".to_owned());
    }
    let definition = format!("+proj=longlat {}", ellipsoid.join(" "));
    let operation =
        Operation::create(&definition).map_err(|_| "ellipsoid setup failure".to_owned())?;
    let (a, es) = operation.distance_ellipsoid();
    let flattening = es / (1.0 + (1.0 - es).sqrt());
    let mut calc = Calc::new(Geod::new(a, flattening));

    let value = |key: &str| pargs.iter().find_map(|parg| arg_value(parg, key));
    if value("lat_1").is_none() {
        return Ok((calc, None));
    }
    calc.phi1 = angle(value("lat_1"));
    calc.lam1 = angle(value("lon_1"));
    if value("lat_2").is_some() {
        calc.phi2 = angle(value("lat_2"));
        calc.lam2 = angle(value("lon_2"));
        calc.inverse();
        calc.pre();
    } else {
        calc.distance = number(value("S")).unwrap_or(0.0);
        if calc.distance == 0.0 {
            return Err("incomplete geodesic/arc info".to_owned());
        }
        calc.al12 = angle(value("A"));
        calc.pre();
        calc.forward();
    }
    let arc_steps = integer(value("n_A"));
    if arc_steps > 0 {
        let step = angle(value("del_A"));
        if step == 0.0 {
            return Err("del azimuth == 0".to_owned());
        }
        let steps = usize::try_from(arc_steps).unwrap_or(0);
        return Ok((calc, Some(Mode::Arc { steps, step })));
    }
    let del_s = number(value("del_S")).unwrap_or(0.0).abs();
    let steps = if del_s != 0.0 {
        // `(int)(geod_S / del_S + .5)`
        let count = (calc.distance / del_s + 0.5).floor();
        usize::try_from(integer_of(count)).unwrap_or(0)
    } else {
        let count = integer(value("n_S"));
        if count <= 0 {
            return Err("no interval divisor selected".to_owned());
        }
        usize::try_from(count).unwrap_or(0)
    };
    Ok((calc, Some(Mode::Line { steps })))
}

/// The key of a `key=value` or bare `key` argument.
fn key_of(parg: &str) -> &str {
    parg.split_once('=').map_or(parg, |(key, _)| key)
}

/// The text after `key=` of an argument, `""` for a bare key, and `None` for
/// another key. `pj_param` reads a key this way, with its type given by the caller.
fn arg_value<'a>(parg: &'a str, key: &str) -> Option<&'a str> {
    match parg.split_once('=') {
        Some((name, text)) if name == key => Some(text),
        None if parg == key => Some(""),
        _ => None,
    }
}

/// An angle argument, read as `dmstor` reads it, in radians.
fn angle(text: Option<&str>) -> f64 {
    text.map_or(0.0, |text| dmstor(text.as_bytes()).0)
}

/// A real argument, read as `strtod` reads it.
fn number(text: Option<&str>) -> Option<f64> {
    text.map(|text| c_strtod(text.as_bytes()).0)
}

/// An integer argument, read from its leading digits as `atoi` does.
fn integer(text: Option<&str>) -> i64 {
    text.map_or(0, |text| {
        let trimmed = text.trim_start();
        let (sign, digits) = match trimmed.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };
        let end = digits
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(digits.len());
        digits[..end].parse::<i64>().map_or(0, |value| sign * value)
    })
}

/// The whole part of a finite value, as `(int)` gives it.
fn integer_of(value: f64) -> i64 {
    format!("{value:.0}").parse().unwrap_or(0)
}

/// The state of `geod.cpp`: the two points, the azimuths, the distance and
/// the line of `geod_pre`.
struct Calc {
    geod: Geod,
    phi1: f64,
    lam1: f64,
    phi2: f64,
    lam2: f64,
    al12: f64,
    al21: f64,
    distance: f64,
    line: Option<Line>,
}

impl Calc {
    fn new(geod: Geod) -> Self {
        Self {
            geod,
            phi1: 0.0,
            lam1: 0.0,
            phi2: 0.0,
            lam2: 0.0,
            al12: 0.0,
            al21: 0.0,
            distance: 0.0,
            line: None,
        }
    }

    /// `geod_pre`: the line from the first point, leaving on the first azimuth.
    fn pre(&mut self) {
        self.line = Some(self.geod.line(
            self.phi1 / DEG_TO_RAD,
            self.lam1 / DEG_TO_RAD,
            self.al12 / DEG_TO_RAD,
        ));
    }

    /// `geod_for`: the second point, `distance` metres along the line, and
    /// its back azimuth.
    fn forward(&mut self) {
        let Some(line) = &self.line else {
            return;
        };
        let (lat2, lon2, azimuth2) = line.position(self.distance);
        let azimuth2 = azimuth2 + if azimuth2 >= 0.0 { -180.0 } else { 180.0 };
        self.phi2 = lat2 * DEG_TO_RAD;
        self.lam2 = lon2 * DEG_TO_RAD;
        self.al21 = azimuth2 * DEG_TO_RAD;
    }

    /// `geod_inv`: the distance and both azimuths between the two points.
    fn inverse(&mut self) {
        let inv = self.geod.inverse(
            self.phi1 / DEG_TO_RAD,
            self.lam1 / DEG_TO_RAD,
            self.phi2 / DEG_TO_RAD,
            self.lam2 / DEG_TO_RAD,
        );
        // Back azimuth: +/-0 maps to -/+180 and +/-180 to -/+0.
        let back = inv.azimuth2 + 180.0_f64.copysign(-inv.azimuth2);
        self.al12 = inv.azimuth1 * DEG_TO_RAD;
        self.al21 = back.copysign(-inv.azimuth2) * DEG_TO_RAD;
        self.distance = inv.distance;
    }
}

/// `do_arc` and `do_geod`: the points of an arc of azimuths, or of a line
/// cut into intervals. Each line of the result ends with a newline.
fn run_mode(calc: &mut Calc, options: &Options, mode: &Mode) -> String {
    let mut out = String::new();
    match *mode {
        Mode::Arc { steps, step } => {
            push_line(&mut out, &print_ll(calc.phi2, calc.lam2, options));
            let mut azimuth = calc.al12;
            for _ in 0..steps {
                azimuth = adjlon(azimuth + step);
                calc.al12 = azimuth;
                calc.pre();
                calc.forward();
                push_line(&mut out, &print_ll(calc.phi2, calc.lam2, options));
            }
        }
        Mode::Line { steps } => {
            let (phi_end, lam_end) = (calc.phi2, calc.lam2);
            push_line(&mut out, &print_ll(calc.phi1, calc.lam1, options));
            // `geod_S = del_S = geod_S / n_S`, then `--n_S` steps.
            let del = calc.distance / usize_as_f64(steps.max(1));
            calc.distance = del;
            for _ in 1..steps {
                calc.forward();
                push_line(&mut out, &print_ll(calc.phi2, calc.lam2, options));
                calc.distance += del;
            }
            push_line(&mut out, &print_ll(phi_end, lam_end, options));
        }
    }
    out
}

fn usize_as_f64(value: usize) -> f64 {
    format!("{value}").parse().unwrap_or(0.0)
}

fn push_line(out: &mut String, text: &str) {
    out.push_str(text);
    out.push('\n');
}

/// `printLL`: a point as latitude and longitude, in `-f` numbers, or in
/// degrees, minutes and seconds with the hemisphere letters.
fn print_ll(phi: f64, lam: f64, options: &Options) -> String {
    match &options.oform {
        Some(format) => format!(
            "{}\t{}",
            number_text(format, phi * RAD_TO_DEG),
            number_text(format, lam * RAD_TO_DEG)
        ),
        None => format!(
            "{}\t{}",
            rtodms(phi, options.dms, b'N', b'S'),
            rtodms(lam, options.dms, b'E', b'W')
        ),
    }
}

/// An azimuth in radians, in a `-f` number or in degrees, minutes and seconds.
fn azimuth_text(radians: f64, options: &Options) -> String {
    match &options.oform {
        Some(format) => number_text(format, radians * RAD_TO_DEG),
        None => rtodms(radians, options.dms, 0, 0),
    }
}

/// A number in a `-f` or `-F` format. PROJ prints nothing for a bad format.
fn number_text(format: &str, value: f64) -> String {
    format_number(format, value).unwrap_or_default()
}

/// `process`: each line of `input`, as `fgets` reads it, is one record.
/// A line that starts with the tag is copied.
fn process(input: &[u8], options: &Options, calc: &mut Calc) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(line) = next_line(input, &mut pos) {
        if line.first() == Some(&options.tag()) {
            out.extend_from_slice(&line);
            continue;
        }
        let rest = parse_record(&line, options, calc);
        out.extend_from_slice(record_text(options, calc).as_bytes());
        out.extend_from_slice(&line[rest..]);
    }
    out
}

/// Reads one record and computes its geodesic. Returns the offset where the
/// values stop, so that the rest of the line can be copied after them.
fn parse_record(line: &[u8], options: &Options, calc: &mut Calc) -> usize {
    let mut s = 0;
    let take_angle = |s: &mut usize| {
        let (value, used) = dmstor(&line[*s..]);
        *s += used;
        value
    };
    calc.phi1 = take_angle(&mut s);
    calc.lam1 = take_angle(&mut s);
    if options.inverse {
        calc.phi2 = take_angle(&mut s);
        calc.lam2 = take_angle(&mut s);
        calc.inverse();
    } else {
        calc.al12 = take_angle(&mut s);
        let (distance, used) = c_strtod(&line[s..]);
        s += used;
        calc.distance = distance;
        calc.pre();
        calc.forward();
    }
    if options.pos_azi {
        if calc.al12 < 0.0 {
            calc.al12 += 2.0 * PI;
        }
        if calc.al21 < 0.0 {
            calc.al21 += 2.0 * PI;
        }
    }
    s
}

/// The values one record writes, before the rest of its line.
fn record_text(options: &Options, calc: &Calc) -> String {
    let distance = number_text(options.osform(), calc.distance);
    let az12 = azimuth_text(calc.al12, options);
    let az21 = azimuth_text(calc.al21, options);
    if options.full {
        format!(
            "{}\t{}\t{az12}\t{az21}\t{distance}",
            print_ll(calc.phi1, calc.lam1, options),
            print_ll(calc.phi2, calc.lam2, options)
        )
    } else if options.inverse {
        format!("{az12}\t{az21}\t{distance}")
    } else {
        format!("{}\t{az21}", print_ll(calc.phi2, calc.lam2, options))
    }
}

/// `fgets` with `MAXLINE`: a line is read up to its newline, or `MAX_LINE - 1`
/// bytes. A longer line is cut there and given a newline, and the rest of it
/// is skipped up to the next newline.
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
