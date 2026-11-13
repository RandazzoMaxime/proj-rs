//! The `gie` runner: PROJ 9.9.0 `apps/gie.cpp` as a library.
//!
//! A `.gie` file is a list of `<gie>` blocks of `operation`, `accept`,
//! `expect` and `roundtrip` instructions. An operation this crate cannot build
//! is skipped. A built operation whose coordinate misses the file's tolerance
//! is a failure. The `gie` binary and the vendored-suite test both call
//! [`run_text`].

use crate::{
    lpz_dist, parse_gie_coord, xyz_dist, CreateError, CrsPipeline, Operation,
    StepError as TransformError, Xyzt,
};
use std::path::PathBuf;

/// What a `.gie` run may look at outside the text: the network switch and the
/// directories a `require_grid` name is searched in.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// `PROJ_NETWORK` is on. `require_grid` then passes for any name.
    pub network: bool,
    /// Directories searched for a `require_grid` file, besides the name as given.
    pub grid_dirs: Vec<PathBuf>,
}

/// Pass, fail and skip counts of one run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Expectations that held.
    pub passed: u64,
    /// Expectations that missed, or operations that failed to transform.
    pub failed: u64,
    /// Expectations not run: unsupported operation or `skip`.
    pub skipped: u64,
}

/// The counts and failure notes of one `.gie` text.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Totals of the text.
    pub counts: Counts,
    /// One line per failed expectation, in file order.
    pub notes: Vec<String>,
}

/// Run one `.gie` text and report its counts and failures.
#[must_use]
pub fn run_text(text: &str, config: &Config) -> Report {
    let (counts, notes) = run_gie(text, config);
    Report { counts, notes }
}

#[derive(Clone, Copy)]
enum Readiness {
    Ready,
    Unsupported,
    Rejected,
}

struct OpState {
    readiness: Readiness,
    operation: Option<Operation>,
    definition: String,
    inverse: bool,
    tolerance: f64,
    accepted: Xyzt,
    skip_rest: bool,
    crs_src: String,
    crs_dst: String,
    /// `crs_src` / `crs_dst`. Degree output is compared on the ellipsoid.
    crs_pair: bool,
    /// Destination first axis is latitude or northing.
    swap_xy: bool,
    /// The run's network switch and grid directories, for `require_grid`.
    config: Config,
}

fn run_gie(text: &str, config: &Config) -> (Counts, Vec<String>) {
    let mut counts = Counts::default();
    let mut notes = Vec::new();
    let mut state = OpState {
        config: config.clone(),
        readiness: Readiness::Unsupported,
        operation: None,
        definition: String::new(),
        inverse: false,
        tolerance: 5e-4,
        accepted: Xyzt::xy(0.0, 0.0),
        skip_rest: false,
        crs_src: String::new(),
        crs_dst: String::new(),
        crs_pair: false,
        swap_xy: false,
    };
    let mut lines = text.lines().peekable();
    let mut strict = false;
    let mut inside = false;
    let mut pending: Option<(String, String)> = None;
    let flush = |pending: &mut Option<(String, String)>,
                 state: &mut OpState,
                 counts: &mut Counts,
                 notes: &mut Vec<String>| {
        if let Some((tag, args)) = pending.take() {
            dispatch(&tag, args.trim(), state, counts, notes);
        }
    };
    while let Some(raw) = lines.next() {
        let line = strip_comment(raw).trim().to_owned();
        if line.starts_with("<gie-strict>") {
            flush(&mut pending, &mut state, &mut counts, &mut notes);
            strict = true;
            inside = true;
            continue;
        }
        if line.starts_with("</gie-strict>")
            || line.starts_with("</gie>")
            || line.starts_with("<gie>")
        {
            flush(&mut pending, &mut state, &mut counts, &mut notes);
            inside = line.starts_with("<gie");
            strict = line.starts_with("<gie-strict>");
            continue;
        }
        if !inside {
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if decorative(&line) {
            flush(&mut pending, &mut state, &mut counts, &mut notes);
            continue;
        }
        let (tag, mut args) = split_tag(&line);
        if tag.is_empty() {
            if let Some((_, buf)) = pending.as_mut() {
                buf.push(' ');
                buf.push_str(&line);
            }
            continue;
        }
        flush(&mut pending, &mut state, &mut counts, &mut notes);
        if strict {
            while args.trim_end().ends_with('\\') {
                args.pop();
                let Some(next) = lines.next() else { break };
                args.push(' ');
                args.push_str(strip_comment(next).trim());
            }
            dispatch(&tag, args.trim(), &mut state, &mut counts, &mut notes);
        } else {
            pending = Some((tag, args));
        }
    }
    flush(&mut pending, &mut state, &mut counts, &mut notes);
    (counts, notes)
}

fn dispatch(
    tag: &str,
    args: &str,
    state: &mut OpState,
    counts: &mut Counts,
    notes: &mut Vec<String>,
) {
    match tag {
        "operation" => {
            state.definition = args.to_owned();
            state.inverse = false;
            state.tolerance = 5e-4;
            state.skip_rest = false;
            state.accepted = Xyzt::xy(0.0, 0.0);
            state.crs_pair = false;
            state.swap_xy = false;
            state.crs_src.clear();
            state.crs_dst.clear();
            match Operation::create(args) {
                Ok(operation) => {
                    state.operation = Some(operation);
                    state.readiness = Readiness::Ready;
                }
                Err(
                    CreateError::UnknownProj(_)
                    | CreateError::UnknownParameter(_)
                    | CreateError::Unsupported(_),
                ) => {
                    state.operation = None;
                    state.readiness = Readiness::Unsupported;
                }
                Err(CreateError::IllegalValue(_) | CreateError::MissingArgument(_)) => {
                    state.operation = None;
                    state.readiness = Readiness::Rejected;
                }
            }
        }
        "direction" => {
            let word = args.trim();
            state.inverse = word.starts_with('i')
                || word.starts_with('I')
                || word.starts_with('r')
                || word.starts_with('R');
        }
        "tolerance" => {
            if let Some(value) = scaled_tolerance(args) {
                state.tolerance = value;
            }
        }
        "require_grid" => {
            if !grid_present(args, &state.config) {
                state.skip_rest = true;
            }
        }
        "crs_src" => {
            state.crs_src = args.to_owned();
            open_crs(state);
        }
        "crs_dst" => {
            state.crs_dst = args.to_owned();
            open_crs(state);
        }
        "accept" => {
            if let Some((coord, _)) = parse_gie_coord(args) {
                state.accepted = coord;
            }
        }
        "expect" => score_expect(args, state, counts, notes),
        "roundtrip" => score_roundtrip(args, state, counts, notes),
        "skip" => state.skip_rest = true,
        _ => {}
    }
}

fn open_crs(state: &mut OpState) {
    if state.crs_src.is_empty() || state.crs_dst.is_empty() {
        return;
    }
    // gie keeps the previous accept. The second 4979 case expects before it accepts.
    let source = std::mem::take(&mut state.crs_src);
    let target = std::mem::take(&mut state.crs_dst);
    state.inverse = false;
    state.tolerance = 5e-4;
    state.skip_rest = false;
    state.crs_pair = false;
    state.swap_xy = false;
    state.definition = format!("{source} -> {target}");
    match CrsPipeline::create(&source, &target) {
        Ok(built) => {
            state.swap_xy = built.lat_or_northing_first;
            state.crs_pair = true;
            state.operation = Some(built.operation);
            state.readiness = Readiness::Ready;
        }
        Err(
            CreateError::UnknownProj(_)
            | CreateError::UnknownParameter(_)
            | CreateError::Unsupported(_),
        ) => {
            state.operation = None;
            state.readiness = Readiness::Unsupported;
        }
        Err(CreateError::IllegalValue(_) | CreateError::MissingArgument(_)) => {
            state.operation = None;
            state.readiness = Readiness::Rejected;
        }
    }
}

fn deviation(
    state: &OpState,
    operation: &Operation,
    mut got: Xyzt,
    mut expected: Xyzt,
    inverse: bool,
) -> f64 {
    if got.x.is_nan() && expected.x.is_nan() {
        return 0.0;
    }
    if operation.angular_output(inverse) {
        expected = deg_to_rad(expected);
        let (a, es) = operation.distance_ellipsoid();
        return lpz_dist(a, es, expected, got);
    }
    // CRS pipelines leave degrees in the CRS axis order. gie converts those to
    // radians and swaps a latitude-first axis before the geodesic distance.
    if state.crs_pair && operation.degree_output(inverse) {
        expected = deg_to_rad(expected);
        got = deg_to_rad(got);
        if state.swap_xy {
            std::mem::swap(&mut expected.x, &mut expected.y);
            std::mem::swap(&mut got.x, &mut got.y);
        }
        let (a, es) = operation.distance_ellipsoid();
        return lpz_dist(a, es, expected, got);
    }
    xyz_dist(expected, got)
}

fn score_expect(args: &str, state: &OpState, counts: &mut Counts, notes: &mut Vec<String>) {
    if state.skip_rest || matches!(state.readiness, Readiness::Unsupported) {
        counts.skipped += 1;
        return;
    }
    if let Some(errno) = args.strip_prefix("failure") {
        let wanted = errno.split_whitespace().nth(1);
        let got = match state.readiness {
            Readiness::Rejected => Some("invalid_op"),
            Readiness::Ready => transform(state).err().map(errno_name),
            Readiness::Unsupported => None,
        };
        if got.is_some_and(|name| {
            wanted.is_none_or(|item| name.starts_with(item) || item.starts_with(name))
        }) {
            counts.passed += 1;
        } else {
            counts.failed += 1;
            notes.push(format!(
                "failure errno [{:?}] wanted {wanted:?} got {got:?}",
                state.definition
            ));
        }
        return;
    }
    if !matches!(state.readiness, Readiness::Ready) {
        counts.failed += 1;
        notes.push(format!("rejected [{}] expected {args}", state.definition));
        return;
    }
    let Some(operation) = state.operation.as_ref() else {
        counts.failed += 1;
        return;
    };
    let Some((expected, dims)) = parse_gie_coord(args) else {
        counts.failed += 1;
        notes.push(format!("could not parse expect {args}"));
        return;
    };
    let inverse = state.inverse;
    let input = if operation.angular_input(inverse) {
        deg_to_rad(state.accepted)
    } else {
        state.accepted
    };
    let transformed = if inverse {
        operation.inverse(input)
    } else {
        operation.forward(input)
    };
    let Ok(mut got) = transformed else {
        counts.failed += 1;
        notes.push(format!(
            "transform failed [{}] expected {args}",
            state.definition
        ));
        return;
    };
    if dims < 4 {
        got.t = 0.0;
    }
    if dims < 3 {
        got.z = 0.0;
    }
    let distance = deviation(state, operation, got, expected, inverse);
    if distance <= state.tolerance {
        counts.passed += 1;
    } else {
        counts.failed += 1;
        notes.push(format!(
            "deviation {distance} m > {} m [{}] expected {args} got {:.6} {:.6}",
            state.tolerance, state.definition, got.x, got.y
        ));
    }
}

fn score_roundtrip(args: &str, state: &OpState, counts: &mut Counts, notes: &mut Vec<String>) {
    if state.skip_rest || !matches!(state.readiness, Readiness::Ready) {
        counts.skipped += 1;
        return;
    }
    let Some(operation) = state.operation.as_ref() else {
        counts.skipped += 1;
        return;
    };
    let mut parts = args.split_whitespace();
    let trips = parts
        .next()
        .and_then(|text| text.parse::<usize>().ok())
        .unwrap_or(100);
    let tolerance = parts
        .next()
        .and_then(scaled_tolerance)
        .unwrap_or(state.tolerance);
    let inverse = state.inverse;
    let mut coord = if operation.angular_input(inverse) {
        deg_to_rad(state.accepted)
    } else {
        state.accepted
    };
    let original = coord;
    let first = step(operation, inverse, coord);
    let Ok(mut coord_now) = first else {
        counts.failed += 1;
        notes.push("roundtrip failed on the first half-step".to_owned());
        return;
    };
    coord = coord_now;
    for _ in 0..trips.saturating_sub(1) {
        let back = match step(operation, !inverse, coord) {
            Ok(value) => value,
            Err(_) => {
                counts.failed += 1;
                return;
            }
        };
        match step(operation, inverse, back) {
            Ok(value) => coord = value,
            Err(_) => {
                counts.failed += 1;
                return;
            }
        }
    }
    coord_now = match step(operation, !inverse, coord) {
        Ok(value) => value,
        Err(_) => {
            counts.failed += 1;
            return;
        }
    };
    let distance = if original.x.is_nan()
        && coord_now.x.is_nan()
        && coord_now.y.is_nan()
        && coord_now.z.is_nan()
    {
        0.0
    } else if operation.angular_input(inverse) {
        let (a, es) = operation.distance_ellipsoid();
        lpz_dist(a, es, original, coord_now)
    } else {
        xyz_dist(original, coord_now)
    };
    if distance <= tolerance {
        counts.passed += 1;
    } else {
        counts.failed += 1;
        notes.push(format!("roundtrip deviation {distance} m > {tolerance} m"));
    }
}

fn step(operation: &Operation, inverse: bool, coord: Xyzt) -> Result<Xyzt, TransformError> {
    if inverse {
        operation.inverse(coord)
    } else {
        operation.forward(coord)
    }
}

fn transform(state: &OpState) -> Result<Xyzt, TransformError> {
    let operation = state.operation.as_ref().ok_or(TransformError::NoInverse)?;
    let inverse = state.inverse;
    let input = if operation.angular_input(inverse) {
        deg_to_rad(state.accepted)
    } else {
        state.accepted
    };
    step(operation, inverse, input)
}

fn errno_name(error: TransformError) -> &'static str {
    match error {
        TransformError::OutsideDomain => "coord_transfm_outside_projection_domain",
        TransformError::OutsideGrid => "coord_transfm_outside_grid",
        TransformError::MissingTime => "coord_transfm_missing_time",
        TransformError::InvalidCoord => "coord_transfm_invalid_coord",
        TransformError::NoInverse => "no_inverse_op",
        TransformError::NonFinite => "coord_transfm_invalid_coord",
    }
}

fn deg_to_rad(coord: Xyzt) -> Xyzt {
    Xyzt {
        x: coord.x.to_radians(),
        y: coord.y.to_radians(),
        z: coord.z,
        t: coord.t,
    }
}

fn scaled_tolerance(args: &str) -> Option<f64> {
    let args = args.trim();
    let (number, rest) = take_tolerance_number(args)?;
    let unit = rest.trim();
    let scale = match unit {
        "km" => 1000.0,
        "m" => 1.0,
        "dm" => 0.1,
        "cm" => 0.01,
        "mm" => 0.001,
        "um" => 1e-6,
        "nm" => 1e-9,
        "rad" => 111_319.490_8 * (180.0 / std::f64::consts::PI),
        "deg" => 111_319.490_8,
        _ => 1.0,
    };
    Some(number * scale)
}

fn take_tolerance_number(text: &str) -> Option<(f64, &str)> {
    let mut end = 0;
    let bytes = text.as_bytes();
    if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
        end = 1;
    }
    let mut saw = false;
    let mut dot = false;
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' => {
                saw = true;
                end += 1;
            }
            b'_' => end += 1,
            b'.' if !dot => {
                dot = true;
                end += 1;
            }
            b'e' | b'E' if saw => {
                let exp = end + 1;
                let mut cursor = exp;
                if cursor < bytes.len() && (bytes[cursor] == b'+' || bytes[cursor] == b'-') {
                    cursor += 1;
                }
                let digits = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                if cursor > digits {
                    end = cursor;
                }
                break;
            }
            _ => break,
        }
    }
    if !saw {
        return None;
    }
    let value = text[..end].replace('_', "").parse().ok()?;
    Some((value, &text[end..]))
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn decorative(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_alphanumeric() || first == '+' || first == '<' {
        return false;
    }
    line.chars().take(5).filter(|ch| *ch == first).count() >= 5
}

/// The directories `common::fixture_search_path` puts on `PROJ_DATA`. A
/// require_grid name may be a bare filename.
fn split_tag(line: &str) -> (String, String) {
    const TAGS: &[&str] = &[
        "operation",
        "accept",
        "expect",
        "roundtrip",
        "direction",
        "tolerance",
        "require_grid",
        "ignore",
        "skip",
        "banner",
        "verbose",
        "echo",
        "crs_src",
        "crs_dst",
        "use_proj4_init_rules",
    ];
    for tag in TAGS {
        if line == *tag
            || line.starts_with(&format!("{tag} "))
            || line.starts_with(&format!("{tag}\t"))
        {
            return ((*tag).to_owned(), line[tag.len()..].trim().to_owned());
        }
    }
    (String::new(), line.to_owned())
}

/// Whether a `require_grid` name is present. Network on passes any name. Otherwise
/// the name is looked for under each of `config.grid_dirs`, by its full path and
/// by its bare file name, and as a path of its own.
fn grid_present(name: &str, config: &Config) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    if config.network {
        return true;
    }
    let file_name = std::path::Path::new(name);
    for directory in &config.grid_dirs {
        if directory.join(name).is_file() {
            return true;
        }
        if let Some(bare) = file_name.file_name() {
            if directory.join(bare).is_file() {
                return true;
            }
        }
    }
    file_name.is_file()
}

/// The configuration a `gie` process takes from its environment: the network
/// switch of the default context and the data directories, which are where a
/// `require_grid` file is looked for.
#[must_use]
pub fn config_from_env() -> Config {
    Config {
        network: crate::network::context().enabled,
        grid_dirs: crate::network::data_directories(),
    }
}
