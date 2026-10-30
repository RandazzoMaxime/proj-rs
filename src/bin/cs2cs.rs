//! `cs2cs`: transform coordinates between two CRS definitions.
//!
//! `cs2cs EPSG:4326 EPSG:32631` and `cs2cs +proj=latlong +to +proj=utm ...`
//! read longitude and latitude in degrees, or easting and northing in metres,
//! one point per line. That is the order [`proj_rs::Proj::new_known_crs`] uses,
//! not the EPSG axis order. `-d` sets the printed decimals for x, y, and z.
//! A geographic degree result with no `-d` or `-f` is DMS. A projected result
//! with no format is `%.2f`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "cs2cs writes coordinates to stdout and usage to stderr"
)]

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

use proj_rs::iso19111::{BestOperation, OnlyBest};
use proj_rs::{Area, CrsPipeline, Operation, Proj, Xyzt};
use rusqlite::Connection;

const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";

fn main() -> ExitCode {
    let prog = program_name();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", usage(&prog));
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        eprintln!("{}", usage(&prog));
        return ExitCode::SUCCESS;
    }
    let opts = match parse(&args) {
        Ok(opts) => opts,
        Err(error) => {
            if error.plain {
                eprintln!("{}", error.message);
            } else {
                report(&prog, error.code, &error.message, None);
            }
            return exit_code(error.code);
        }
    };
    if opts.list {
        eprintln!("cs2cs: listing is not built in proj-rs");
        return ExitCode::SUCCESS;
    }
    if opts.bbox.is_some() && opts.area_name.is_some() {
        eprintln!("ERROR: --bbox and --area are exclusive");
        return ExitCode::from(1);
    }
    let catalog = Catalog::open();
    let area = if let Some(name) = &opts.area_name {
        match catalog.area_bbox(name) {
            Ok(area) => Some(area),
            Err(error) => {
                eprintln!("{}", error.message);
                return exit_code(error.code);
            }
        }
    } else {
        opts.bbox
    };
    let (source, target, src_side, dst_side) = match resolve_pair(&opts, &catalog) {
        Ok(pair) => pair,
        Err(error) => {
            report(&prog, error.code, &error.message, None);
            return exit_code(error.code);
        }
    };
    let engine = match build_engine(&source, &target, area, opts.only_best) {
        Ok(engine) => engine,
        Err(error) => {
            report(
                &prog,
                3,
                &format!("cannot initialize transformation\ncause: {error}"),
                None,
            );
            return ExitCode::from(3);
        }
    };
    let lat_first = engine.lat_first() && dst_side.layout == Layout::DegreeLongLat;
    let _selection = (
        opts.authority.as_deref(),
        opts.accuracy,
        opts.only_best,
        opts.allow_ballpark,
        opts.promote_3d,
        opts.source_epoch,
        opts.target_epoch,
        opts.debug_level,
    );
    let style = style_for(&opts, dst_side.layout);
    let mut stdout = io::stdout().lock();
    if opts.monitor {
        let _ = writeln!(stdout, "{} ---- From Coordinate System ----", opts.tag);
        let _ = writeln!(stdout, "{source}");
        let _ = writeln!(stdout, "{} ---- To Coordinate System ----", opts.tag);
        let _ = writeln!(stdout, "{target}");
    }
    let job = Job {
        engine: &engine,
        style: &style,
        echo: opts.echo,
        reverse_out: opts.reverse_out,
        error_text: &opts.error_text,
        tag: opts.tag,
        src_dynamic: src_side.dynamic && opts.source_epoch.is_none(),
        dst_dynamic: dst_side.dynamic && opts.target_epoch.is_none(),
        epoch: opts.source_epoch.or(opts.target_epoch),
        degree_in: src_side.layout == Layout::DegreeLongLat,
        reverse_in: opts.reverse_in,
        lat_first,
    };
    if opts.files.is_empty() {
        return read_buf(io::stdin().lock(), &job, &mut stdout);
    }
    for file in &opts.files {
        if file == "-" {
            let code = read_buf(io::stdin().lock(), &job, &mut stdout);
            if code != ExitCode::SUCCESS {
                return code;
            }
            continue;
        }
        match File::open(file) {
            Ok(handle) => {
                let code = read_buf(BufReader::new(handle), &job, &mut stdout);
                if code != ExitCode::SUCCESS {
                    return code;
                }
            }
            Err(error) => report(&prog, -2, &format!("input file: {file}"), Some(&error)),
        }
    }
    let _ = _selection;
    ExitCode::SUCCESS
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one cs2cs input or output switch"
)]
struct Job<'a> {
    engine: &'a Engine,
    style: &'a Style,
    echo: bool,
    reverse_out: bool,
    error_text: &'a str,
    tag: char,
    src_dynamic: bool,
    dst_dynamic: bool,
    degree_in: bool,
    reverse_in: bool,
    /// Geographic output whose first axis is latitude. DMS then prints N/S first.
    lat_first: bool,
    /// `--s_epoch`, else `--t_epoch`. It replaces the input time, as PROJ's `set v_4` does.
    epoch: Option<f64>,
}

fn read_buf<R: BufRead>(reader: R, job: &Job<'_>, stdout: &mut impl Write) -> ExitCode {
    for (index, line) in reader.lines().enumerate() {
        let Ok(line) = line else {
            eprintln!("cs2cs: could not read input");
            return ExitCode::from(1);
        };
        let slot = if index == 0 {
            LineSlot::First
        } else {
            LineSlot::Later
        };
        emit_line(&line, slot, job, stdout);
    }
    ExitCode::SUCCESS
}

enum LineSlot {
    First,
    Later,
}

fn emit_line(line: &str, slot: LineSlot, job: &Job<'_>, stdout: &mut impl Write) {
    let mut logical = line;
    if matches!(slot, LineSlot::First) {
        if let Some(rest) = line.strip_prefix('\u{feff}') {
            logical = rest;
        }
    }
    if logical.starts_with(job.tag) {
        let _ = writeln!(stdout, "{line}");
        return;
    }
    let text_kind = if job.degree_in {
        AxisText::Dms
    } else {
        AxisText::Decimal
    };
    let order = if job.reverse_in {
        AxisOrder::Reversed
    } else {
        AxisOrder::Forward
    };
    let parsed = parse_point(logical, text_kind, order);
    if !parsed.had_time && (job.src_dynamic || job.dst_dynamic) {
        if job.src_dynamic {
            eprintln!(
                "Input coordinates lack a coordinate epoch, whereas the source CRS is dynamic. Results might be inaccurate."
            );
        } else {
            eprintln!(
                "Input coordinates lack a coordinate epoch, whereas the destination CRS is dynamic. Results might be inaccurate."
            );
        }
    }
    let drawn = if parsed.bad {
        Drawn::Failed(parsed.z)
    } else {
        let t = job.epoch.unwrap_or(parsed.t);
        match job.engine.apply(parsed.x, parsed.y, parsed.z, t) {
            Ok((x, y, z)) => Drawn::Value(x, y, z),
            Err(_) => Drawn::Failed(f64::INFINITY),
        }
    };
    let cut = parsed.tail;
    let text = render_point(job, drawn, &logical[..cut], &logical[cut..]);
    let _ = writeln!(stdout, "{text}");
}

enum Drawn {
    Value(f64, f64, f64),
    Failed(f64),
}

fn render_point(job: &Job<'_>, drawn: Drawn, echo_prefix: &str, tail: &str) -> String {
    let mut out = String::new();
    if job.echo {
        out.push_str(echo_prefix);
        out.push('\t');
    }
    let z = match drawn {
        Drawn::Failed(z) => {
            out.push_str(job.error_text);
            z
        }
        Drawn::Value(x, y, z) => {
            match job.style.xy {
                XyStyle::Dms { fract, form } => {
                    let (left, right, left_pos, left_neg, right_pos, right_neg) =
                        if job.lat_first && !job.reverse_out {
                            (x, y, 'N', 'S', 'E', 'W')
                        } else if job.reverse_out {
                            (y, x, 'N', 'S', 'E', 'W')
                        } else {
                            (x, y, 'E', 'W', 'N', 'S')
                        };
                    out.push_str(&rtodms(left, left_pos, left_neg, fract, form));
                    out.push('\t');
                    out.push_str(&rtodms(right, right_pos, right_neg, fract, form));
                }
                XyStyle::Number(fmt) => {
                    let (left, right) = if job.reverse_out { (y, x) } else { (x, y) };
                    out.push_str(&format_number(&fmt, left));
                    out.push('\t');
                    out.push_str(&format_number(&fmt, right));
                }
            }
            z
        }
    };
    out.push(' ');
    out.push_str(&format_number(&job.style.z, z));
    out.push_str(tail);
    out
}

enum Engine {
    /// Two `+proj` definitions. Inverse of the source, then forward of the target.
    /// Height and a geocentric third axis go through the operations.
    Pair(Box<SidePair>),
    /// Two EPSG codes. The pipeline already speaks authority axis order and units.
    Pipe(CrsPipeline),
    /// Names and mixed definitions, through the CRS facade, which carries the third axis.
    Facade(Box<Proj>),
    /// `--only-best` chose an operation whose grid is missing. Each point fails with PROJ's message.
    Failing(String),
}

impl Engine {
    fn lat_first(&self) -> bool {
        match self {
            Self::Pipe(pipe) => pipe.lat_or_northing_first,
            Self::Pair(_) | Self::Facade(_) | Self::Failing(_) => false,
        }
    }

    fn apply(&self, x: f64, y: f64, z: f64, t: f64) -> Result<(f64, f64, f64), ()> {
        match self {
            Self::Pair(pair) => pair
                .apply(x, y, z, t)
                .map(|coord| (coord.x, coord.y, coord.z)),
            Self::Pipe(pipe) => pipe
                .operation
                .forward(Xyzt { x, y, z, t })
                .map(|coord| (coord.x, coord.y, coord.z))
                .map_err(|_| ()),
            Self::Facade(proj) => proj.convert_xyz(x, y, z).map_err(|_| ()),
            Self::Failing(message) => {
                eprintln!("{message}");
                Err(())
            }
        }
    }
}

fn build_engine(
    source: &str,
    target: &str,
    area: Option<Area>,
    only_best: Option<bool>,
) -> Result<Engine, String> {
    if area.is_none() && !projish(source) && !projish(target) {
        if let Some(engine) = select_pipeline(source, target, only_best) {
            return Ok(engine);
        }
    }
    let source = &canonical_side(source);
    let target = &canonical_side(target);
    if projish(source) && projish(target) {
        return SidePair::open(source, target)
            .map(Box::new)
            .map(Engine::Pair);
    }
    if epsg_token(source) && epsg_token(target) {
        if let Ok(pipe) = CrsPipeline::create(source, target) {
            return Ok(Engine::Pipe(pipe));
        }
    }
    Proj::new_known_crs(source, target, area)
        .map(Box::new)
        .map(Engine::Facade)
        .map_err(|error| error.to_string())
}

/// The operation PROJ's `proj_create_crs_to_crs` picks for the pair, or `None`
/// when that selection fails and the caller keeps its previous path.
fn select_pipeline(source: &str, target: &str, only_best: Option<bool>) -> Option<Engine> {
    let mode = if only_best == Some(true) {
        OnlyBest::On
    } else {
        OnlyBest::Off
    };
    match proj_rs::iso19111::best_pipeline_with(source, target, mode).ok()? {
        BestOperation::Pipeline {
            definition,
            lat_or_northing_first,
        } => {
            let operation = Operation::create(&definition).ok()?;
            Some(Engine::Pipe(CrsPipeline {
                operation,
                lat_or_northing_first,
            }))
        }
        BestOperation::MissingGrid { message } => Some(Engine::Failing(message)),
    }
}

/// A name or compound that the library resolves to EPSG codes becomes that
/// code, so the EPSG pipeline builder can take it. A CRS with no EPSG identity,
/// such as `IAU_2015:49935`, becomes its PROJ string.
fn canonical_side(text: &str) -> String {
    if epsg_token(text) || projish(text) {
        return text.to_owned();
    }
    let Ok(db) = proj_rs::DatabaseContext::create() else {
        return text.to_owned();
    };
    match proj_rs::epsg_identity(text, &db) {
        Ok(Some(identity)) => identity,
        Ok(None) => proj_rs::iso19111::create_from_user_input(text, &db)
            .and_then(|crs| crs.export_to_proj_string())
            .unwrap_or_else(|_| text.to_owned()),
        Err(_) => text.to_owned(),
    }
}

fn projish(definition: &str) -> bool {
    explicit_proj(definition).is_some()
        || definition.contains("init=")
        || definition.contains("proj=")
}

fn epsg_token(text: &str) -> bool {
    let text = text.trim();
    let Some((auth, code)) = text.split_once(':') else {
        return false;
    };
    auth.eq_ignore_ascii_case("EPSG")
        && !code.is_empty()
        && code
            .split('+')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

struct SidePair {
    source: Operation,
    target: Operation,
    src_scale: Option<f64>,
    dst_scale: Option<f64>,
    src_geoc: Option<Operation>,
    dst_geoc: Option<Operation>,
}

impl SidePair {
    fn open(source: &str, target: &str) -> Result<Self, String> {
        // A raw ellipse on one side does not pull in the other side's
        // `+towgs84` or `+datum` grid. Both sides must name a datum.
        let source_def = if names_datum(source) && !names_datum(target) {
            strip_datum(source)
        } else {
            source.to_owned()
        };
        let target_def = if names_datum(target) && !names_datum(source) {
            strip_datum(target)
        } else {
            target.to_owned()
        };
        let (source_op, src_scale, src_geoc) = open_side(&source_def)?;
        let (target_op, dst_scale, dst_geoc) = open_side(&target_def)?;
        Ok(Self {
            source: source_op,
            target: target_op,
            src_scale,
            dst_scale,
            src_geoc,
            dst_geoc,
        })
    }

    fn apply(&self, x: f64, y: f64, z: f64, t: f64) -> Result<Xyzt, ()> {
        let mut coord = Xyzt { x, y, z, t };
        if let Some(scale) = self.src_scale {
            coord.x *= scale;
            coord.y *= scale;
        }
        if let Some(geoc) = &self.src_geoc {
            coord = geoc.inverse(coord).map_err(|_| ())?;
        }
        coord = self.source.inverse(coord).map_err(|_| ())?;
        coord = self.target.forward(coord).map_err(|_| ())?;
        if let Some(geoc) = &self.dst_geoc {
            coord = geoc.forward(coord).map_err(|_| ())?;
        }
        if let Some(scale) = self.dst_scale {
            let out = 1.0 / scale;
            coord.x *= out;
            coord.y *= out;
        }
        Ok(coord)
    }
}

fn open_side(definition: &str) -> Result<(Operation, Option<f64>, Option<Operation>), String> {
    let name = explicit_proj(definition);
    let geographic = matches!(
        name.as_deref(),
        Some("latlong" | "longlat" | "latlon" | "lonlat")
    );
    let geoc = geographic && has_flag(definition, "geoc");
    let cleaned = if geoc {
        strip_flag(definition, "geoc")
    } else {
        definition.to_owned()
    };
    let operation = Operation::create(&cleaned).map_err(|error| error.to_string())?;
    let angular = geographic || (operation.angular_input(false) && operation.angular_output(false));
    let scale = if angular {
        Some(if grad_units(&cleaned) {
            std::f64::consts::PI / 200.0
        } else {
            std::f64::consts::PI / 180.0
        })
    } else {
        None
    };
    let geoc_op = if geoc {
        Some(Operation::create(&geoc_definition(&cleaned)).map_err(|error| error.to_string())?)
    } else {
        None
    };
    Ok((operation, scale, geoc_op))
}

fn names_datum(definition: &str) -> bool {
    definition.split_whitespace().any(|token| {
        let bare = token.trim_start_matches('+');
        bare.starts_with("datum=")
            || bare.starts_with("towgs84=")
            || bare == "towgs84"
            || bare.starts_with("nadgrids=")
            || bare == "nadgrids"
    })
}

fn strip_datum(definition: &str) -> String {
    let mut parts = Vec::new();
    for token in definition.split_whitespace() {
        let bare = token.trim_start_matches('+');
        if bare.starts_with("towgs84=")
            || bare == "towgs84"
            || bare.starts_with("nadgrids=")
            || bare == "nadgrids"
        {
            continue;
        }
        if let Some(name) = bare.strip_prefix("datum=") {
            if let Some(ellps) = datum_ellipsoid(name) {
                parts.push(format!("+ellps={ellps}"));
            }
            continue;
        }
        parts.push(token.to_owned());
    }
    parts.join(" ")
}

fn datum_ellipsoid(name: &str) -> Option<&'static str> {
    Some(match name {
        "WGS84" => "WGS84",
        "NAD83" | "GRS80" => "GRS80",
        "GGRS87" => "GRS80",
        "NAD27" => "clrk66",
        "potsdam" => "bessel",
        "carthage" => "clrk80ign",
        "hermannskogel" => "bessel",
        "ire65" => "mod_airy",
        "nzgd49" => "intl",
        "OSGB36" => "airy",
        _ => return None,
    })
}

fn has_flag(definition: &str, flag: &str) -> bool {
    definition.split_whitespace().any(|token| {
        let bare = token.trim_start_matches('+');
        bare == flag
    })
}

fn strip_flag(definition: &str, flag: &str) -> String {
    definition
        .split_whitespace()
        .filter(|token| token.trim_start_matches('+') != flag)
        .collect::<Vec<_>>()
        .join(" ")
}

fn grad_units(definition: &str) -> bool {
    definition.split_whitespace().any(|token| {
        let bare = token.trim_start_matches('+');
        matches!(bare, "units=grad" | "units=grads" | "units=gon" | "units=g")
    })
}

fn geoc_definition(definition: &str) -> String {
    let mut parts = vec!["+proj=geoc".to_owned()];
    for token in definition.split_whitespace() {
        let bare = token.trim_start_matches('+');
        let keep = bare.starts_with("ellps=")
            || bare.starts_with("datum=")
            || bare.starts_with("a=")
            || bare.starts_with("b=")
            || bare.starts_with("rf=")
            || bare.starts_with("f=")
            || bare.starts_with("es=")
            || bare.starts_with("e=")
            || bare.starts_with("R=");
        if keep {
            if token.starts_with('+') {
                parts.push((*token).to_owned());
            } else {
                parts.push(format!("+{token}"));
            }
        }
    }
    parts.join(" ")
}

struct Parsed {
    x: f64,
    y: f64,
    z: f64,
    /// Input time, or `+inf` when the line has none, as PROJ's `HUGE_VAL`.
    t: f64,
    tail: usize,
    had_time: bool,
    bad: bool,
}

#[derive(Clone, Copy)]
enum AxisText {
    Dms,
    Decimal,
}

#[derive(Clone, Copy)]
enum AxisOrder {
    Forward,
    Reversed,
}

fn parse_point(line: &str, text: AxisText, order: AxisOrder) -> Parsed {
    let (x, i1) = read_axis(line, 0, text);
    let (y, i2) = read_axis(line, i1, text);
    let (z, i3) = read_number(line, i2);
    let had_time = read_number_consumed(line, i3);
    let t = if had_time {
        read_number(line, i3).0
    } else {
        f64::INFINITY
    };
    let mut x = x;
    let mut y = y;
    if !y.is_finite() {
        x = f64::INFINITY;
    }
    let bad = !x.is_finite() || !y.is_finite();
    if matches!(order, AxisOrder::Reversed) {
        std::mem::swap(&mut x, &mut y);
    }
    Parsed {
        x,
        y,
        z,
        t,
        tail: i3,
        had_time,
        bad,
    }
}

fn program_name() -> String {
    let exe = std::env::args()
        .next()
        .unwrap_or_else(|| "cs2cs".to_owned());
    std::path::Path::new(&exe)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("cs2cs")
        .to_owned()
}

fn usage(prog: &str) -> String {
    format!(
        "{RELEASE}\n\
         usage: {prog} [-dDeEfIlrstvwW [args]]\n\
                       [[--area name_or_code] | [--bbox west_long,south_lat,east_long,north_lat]]\n\
                       [--authority {{name}}] [--3d]\n\
                       [--accuracy {{accuracy}}] [--only-best[=yes|=no]] [--no-ballpark]\n\
                       [--s_epoch {{epoch}}] [--t_epoch {{epoch}}]\n\
                       [+opt[=arg] ...] [+to +opt[=arg] ...] [file ...]"
    )
}

fn report(prog: &str, code: i32, message: &str, os: Option<&std::io::Error>) {
    if std::env::var("PROJ_DISPLAY_PROGRAM_NAME").ok().as_deref() != Some("NO") {
        eprintln!("{RELEASE}");
        eprint!("<{prog}>: ");
    }
    eprintln!();
    if code == 2 || code == -2 {
        if let Some(os) = os {
            let errno = os.raw_os_error().unwrap_or(0);
            eprintln!("Sys errno: {errno}: {os}");
        }
    }
    eprintln!("{message}");
    if code > 0 {
        eprintln!("program abnormally terminated");
    } else {
        eprintln!();
    }
}

fn exit_code(code: i32) -> ExitCode {
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

#[derive(Debug)]
struct AppError {
    code: i32,
    message: String,
    plain: bool,
}

fn fail(code: i32, message: impl Into<String>) -> AppError {
    AppError {
        code,
        message: message.into(),
        plain: false,
    }
}

fn fail_plain(code: i32, message: impl Into<String>) -> AppError {
    AppError {
        code,
        message: message.into(),
        plain: true,
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one cs2cs command-line switch"
)]
#[derive(Debug)]
struct Options {
    inverse: bool,
    echo: bool,
    reverse_in: bool,
    reverse_out: bool,
    monitor: bool,
    allow_ballpark: bool,
    promote_3d: bool,
    dms_wide: bool,
    list: bool,
    dms_fract: i32,
    tag: char,
    error_text: String,
    number_fmt: Option<NumFmt>,
    debug_level: Option<i32>,
    area_name: Option<String>,
    bbox: Option<Area>,
    authority: Option<String>,
    accuracy: Option<f64>,
    only_best: Option<bool>,
    source_epoch: Option<f64>,
    target_epoch: Option<f64>,
    source: String,
    target: String,
    files: Vec<String>,
}

fn parse(args: &[String]) -> Result<Options, AppError> {
    let mut opts = Options {
        inverse: program_name().starts_with("inv"),
        echo: false,
        reverse_in: false,
        reverse_out: false,
        monitor: false,
        allow_ballpark: true,
        promote_3d: false,
        dms_wide: false,
        list: false,
        dms_fract: 3,
        tag: '#',
        error_text: "*\t*".to_owned(),
        number_fmt: None,
        debug_level: None,
        area_name: None,
        bbox: None,
        authority: None,
        accuracy: None,
        only_best: None,
        source_epoch: None,
        target_epoch: None,
        source: String::new(),
        target: String::new(),
        files: Vec::new(),
    };
    let proj4 = args.iter().any(|arg| arg.starts_with('+'));
    let mut have_to = false;
    let mut raw_fmt: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--area" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --area"))?;
            opts.area_name = Some(value.clone());
        } else if arg == "--bbox" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --bbox"))?;
            opts.bbox = Some(parse_bbox(value)?);
        } else if arg == "--authority" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --authority"))?;
            opts.authority = Some(value.clone());
        } else if arg == "--accuracy" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --accuracy"))?;
            opts.accuracy = Some(parse_locale_f64(value).map_err(|message| {
                fail_plain(1, format!("Invalid value for option --accuracy: {message}"))
            })?);
        } else if arg == "--no-ballpark" {
            opts.allow_ballpark = false;
        } else if arg == "--only-best" || arg == "--only-best=yes" {
            opts.only_best = Some(true);
        } else if arg == "--only-best=no" {
            opts.only_best = Some(false);
        } else if arg == "--3d" {
            opts.promote_3d = true;
        } else if arg == "--s_epoch" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --s_epoch"))?;
            opts.source_epoch = Some(
                parse_locale_f64(value)
                    .map_err(|message| fail(3, format!("invalid --s_epoch: {message}")))?,
            );
        } else if arg == "--t_epoch" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| fail(1, "missing argument for --t_epoch"))?;
            opts.target_epoch = Some(
                parse_locale_f64(value)
                    .map_err(|message| fail(3, format!("invalid --t_epoch: {message}")))?,
            );
        } else if arg.starts_with('-') {
            match cluster(arg, &mut i, args, &mut opts, &mut raw_fmt)? {
                Cluster::Stay => {}
                Cluster::List => return Ok(opts),
            }
        } else if !proj4 {
            if opts.source.is_empty() {
                opts.source = arg.to_owned();
            } else if opts.target.is_empty() {
                opts.target = arg.to_owned();
            } else {
                opts.files.push(arg.to_owned());
            }
        } else if arg == "+to" {
            have_to = true;
        } else if arg.starts_with('+') {
            let slot = if have_to {
                &mut opts.target
            } else {
                &mut opts.source
            };
            if !slot.is_empty() {
                slot.push(' ');
            }
            slot.push_str(arg);
        } else if !have_to {
            opts.source = arg.to_owned();
        } else if opts.target.is_empty() {
            opts.target = arg.to_owned();
        } else {
            opts.files.push(arg.to_owned());
        }
        i += 1;
    }
    if let Some(fmt) = raw_fmt {
        opts.number_fmt =
            Some(parse_num_fmt(&fmt).ok_or_else(|| fail(3, "invalid format string"))?);
    }
    Ok(opts)
}

enum Cluster {
    Stay,
    List,
}

fn cluster(
    arg: &str,
    i: &mut usize,
    args: &[String],
    opts: &mut Options,
    raw_fmt: &mut Option<String>,
) -> Result<Cluster, AppError> {
    let chars: Vec<char> = arg.chars().collect();
    let mut j = 0;
    loop {
        j += 1;
        if j >= chars.len() {
            if arg == "-" {
                opts.files.push("-".to_owned());
            }
            break;
        }
        match chars[j] {
            'v' => opts.monitor = true,
            'I' => opts.inverse = true,
            'E' => opts.echo = true,
            'r' => opts.reverse_in = true,
            's' => opts.reverse_out = true,
            't' => {
                let Some(tag) = chars.get(j + 1).copied() else {
                    return Err(fail(1, "missing -t col. 1 tag"));
                };
                opts.tag = tag;
                j += 1;
            }
            'l' => {
                let next = chars.get(j + 1).copied().unwrap_or('\0');
                if next == '\0' || matches!(next, 'p' | 'P' | '=' | 'e' | 'u' | 'm') {
                    opts.list = true;
                    return Ok(Cluster::List);
                }
                return Err(fail(1, format!("invalid list option: l{next}")));
            }
            'e' => {
                *i += 1;
                let value = args
                    .get(*i)
                    .ok_or_else(|| fail(1, "missing argument for -e"))?;
                opts.error_text = value.clone();
                break;
            }
            'W' | 'w' => {
                let digit = chars.get(j + 1).copied().unwrap_or('\0');
                let after = chars.get(j + 2).copied().unwrap_or('\0');
                if digit.is_ascii_digit()
                    && digit <= '8'
                    && (after == '\0' || !after.is_ascii_digit())
                {
                    opts.dms_fract = i32::try_from(digit.to_digit(10).unwrap_or(0)).unwrap_or(0);
                    opts.dms_wide = chars[j] == 'W';
                    j += 1;
                } else {
                    return Err(fail(1, "-W argument missing or not in range [0,8]"));
                }
            }
            'f' => {
                *i += 1;
                let value = args
                    .get(*i)
                    .ok_or_else(|| fail(1, "missing argument for -f"))?;
                *raw_fmt = Some(value.clone());
                break;
            }
            'D' => {
                *i += 1;
                let value = args
                    .get(*i)
                    .ok_or_else(|| fail(1, "missing argument for -D"))?;
                opts.debug_level = Some(atoi(value));
                break;
            }
            'd' => {
                *i += 1;
                let value = args
                    .get(*i)
                    .ok_or_else(|| fail(1, "missing argument for -d"))?;
                let digits = atoi(value);
                *raw_fmt = Some(format!("%.{digits}f"));
                break;
            }
            other => return Err(fail(1, format!("invalid option: -{other}"))),
        }
    }
    Ok(Cluster::Stay)
}

fn atoi(text: &str) -> i32 {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut sign = 1i32;
    if bytes.first() == Some(&b'+') {
        i = 1;
    } else if bytes.first() == Some(&b'-') {
        sign = -1;
        i = 1;
    }
    let mut value = 0i32;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        any = true;
        value = value
            .saturating_mul(10)
            .saturating_add(i32::from(bytes[i] - b'0'));
        i += 1;
    }
    if any {
        value * sign
    } else {
        0
    }
}

fn parse_locale_f64(text: &str) -> Result<f64, String> {
    let (value, next, consumed) = strtod_full(text.trim(), 0);
    if !consumed || next != text.trim().len() || !value.is_finite() {
        return Err(text.to_owned());
    }
    Ok(value)
}

fn parse_bbox(text: &str) -> Result<Area, AppError> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 4 {
        return Err(fail_plain(
            1,
            format!("Incorrect number of values for option --bbox: {text}"),
        ));
    }
    let mut values = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        values[index] = parse_locale_f64(part).map_err(|message| {
            fail_plain(
                1,
                format!("Invalid value for option --bbox: {text}, {message}"),
            )
        })?;
    }
    let [west, south, east, north] = values;
    const MARGIN: f64 = 10.0;
    if south < -90.0 - MARGIN && west.abs() <= 90.0 && east.abs() <= 90.0 {
        eprintln!("Warning: suspicious south latitude: {south}");
    }
    if north > 90.0 + MARGIN && west.abs() <= 90.0 && east.abs() <= 90.0 {
        eprintln!("Warning: suspicious north latitude: {north}");
    }
    Ok(Area::new(west, south, east, north))
}

fn resolve_pair(
    opts: &Options,
    catalog: &Catalog,
) -> Result<(String, String, Side, Side), AppError> {
    let mut source = opts.source.clone();
    let mut target = opts.target.clone();
    if opts.inverse {
        std::mem::swap(&mut source, &mut target);
    }
    if source.is_empty() && target.is_empty() {
        return Err(fail(3, "missing source and target coordinate systems"));
    }
    if target.is_empty() {
        let side = catalog.layout(&source);
        if side.layout != Layout::Projected {
            return Err(fail(
                3,
                "missing target CRS and source CRS is not a projected CRS",
            ));
        }
        target = catalog.geographic_base(&source).ok_or_else(|| {
            fail(
                3,
                "missing target CRS and source CRS is not a projected CRS",
            )
        })?;
    } else if source.is_empty() {
        let side = catalog.layout(&target);
        if side.layout != Layout::Projected {
            return Err(fail(
                3,
                "missing source CRS and target CRS is not a projected CRS",
            ));
        }
        source = catalog.geographic_base(&target).ok_or_else(|| {
            fail(
                3,
                "missing source CRS and target CRS is not a projected CRS",
            )
        })?;
    }
    let src_side = catalog.layout(&source);
    let dst_side = catalog.layout(&target);
    Ok((source, target, src_side, dst_side))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    DegreeLongLat,
    Projected,
    Other,
}

#[derive(Clone, Copy)]
struct Side {
    layout: Layout,
    dynamic: bool,
}

impl Side {
    fn other() -> Self {
        Self {
            layout: Layout::Other,
            dynamic: false,
        }
    }
}

struct Catalog {
    db: Option<Connection>,
}

impl Catalog {
    fn open() -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/proj.db");
        Self {
            db: Connection::open(path).ok(),
        }
    }

    fn layout(&self, def: &str) -> Side {
        if let Some(name) = explicit_proj(def) {
            return layout_from_proj(&name, def);
        }
        if let Some(code) = init_epsg(def) {
            return self.authority("EPSG", code);
        }
        if let Some((auth, code)) = first_auth(def) {
            return self.authority(&auth, code);
        }
        self.by_name(def.trim())
    }

    fn authority(&self, auth: &str, code: i64) -> Side {
        let Some(db) = &self.db else {
            return Side::other();
        };
        let Ok(kind) = query_type(db, auth, code) else {
            return Side::other();
        };
        self.layout_of_type(db, auth, code, &kind, 0)
    }

    fn layout_of_type(
        &self,
        db: &Connection,
        auth: &str,
        code: i64,
        kind: &str,
        depth: u8,
    ) -> Side {
        if depth > 4 {
            return Side::other();
        }
        let dynamic = datum_is_dynamic(db, auth, code);
        if kind.starts_with("geographic") {
            let layout = if axis_is_degrees(db, auth, code).unwrap_or(true) {
                Layout::DegreeLongLat
            } else {
                Layout::Other
            };
            return Side { layout, dynamic };
        }
        if kind == "projected" {
            let dynamic = dynamic || projected_datum_dynamic(db, auth, code);
            return Side {
                layout: Layout::Projected,
                dynamic,
            };
        }
        if kind == "compound" {
            if let Some((horiz_auth, horiz_code)) = compound_horizontal(db, auth, code) {
                if let Ok(horiz_kind) = query_type(db, &horiz_auth, horiz_code) {
                    return self.layout_of_type(
                        db,
                        &horiz_auth,
                        horiz_code,
                        &horiz_kind,
                        depth + 1,
                    );
                }
            }
        }
        Side {
            layout: Layout::Other,
            dynamic,
        }
    }

    fn by_name(&self, name: &str) -> Side {
        if name.is_empty() {
            return Side::other();
        }
        let Some(db) = &self.db else {
            return Side::other();
        };
        if let Some((code, kind)) = named_row(
            db,
            "SELECT code, type FROM crs_view WHERE auth_name = 'EPSG' AND name = ?1 AND deprecated = 0 ORDER BY code",
            name,
        ) {
            return self.layout_of_type(db, "EPSG", code, &kind, 0);
        }
        if let Some((auth, code, kind)) = named_any(db, name) {
            return self.layout_of_type(db, &auth, code, &kind, 0);
        }
        Side::other()
    }

    fn geographic_base(&self, def: &str) -> Option<String> {
        if explicit_proj(def).is_some() {
            return Some(synthesize_longlat(def));
        }
        let (auth, code) =
            first_auth(def).or_else(|| init_epsg(def).map(|code| ("EPSG".to_owned(), code)))?;
        let (base_auth, base_code) = projected_base(self.db.as_ref()?, &auth, code)?;
        Some(format!("{base_auth}:{base_code}"))
    }

    fn area_bbox(&self, area: &str) -> Result<Area, AppError> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| fail_plain(1, "ERROR: Cannot create database connection: proj.db"))?;
        if !area.contains(' ') {
            if let Some((auth, code)) = area.split_once(':') {
                if let Ok(code) = code.parse::<i64>() {
                    if let Some(area) = extent_code(db, auth, code) {
                        return Ok(area);
                    }
                }
            }
        }
        let rows = extent_name(db, area);
        match rows.len() {
            0 => Err(fail_plain(1, "No area of use matching provided name")),
            1 => Ok(rows[0].1),
            _ => {
                let mut message =
                    String::from("Several candidates area of use matching provided name :");
                for (label, _) in &rows {
                    message.push('\n');
                    message.push_str("  ");
                    message.push_str(label);
                }
                Err(fail_plain(1, message))
            }
        }
    }
}

fn layout_from_proj(name: &str, def: &str) -> Side {
    let layout = match name {
        "latlong" | "longlat" | "latlon" | "lonlat" if degree_units(def) => Layout::DegreeLongLat,
        "geocent" | "cart" => Layout::Other,
        "latlong" | "longlat" | "latlon" | "lonlat" => Layout::Other,
        _ => Layout::Projected,
    };
    Side {
        layout,
        dynamic: false,
    }
}

fn degree_units(def: &str) -> bool {
    for token in def.split_whitespace() {
        let token = token.trim_start_matches('+');
        let Some(unit) = token.strip_prefix("units=") else {
            continue;
        };
        return matches!(
            unit.to_ascii_lowercase().as_str(),
            "deg" | "degree" | "degrees" | "d"
        );
    }
    true
}

fn explicit_proj(def: &str) -> Option<String> {
    for token in def.split_whitespace() {
        let token = token.trim_start_matches('+');
        if let Some(value) = token.strip_prefix("proj=") {
            if !value.is_empty() {
                return Some(value.to_ascii_lowercase());
            }
        }
    }
    None
}

fn init_epsg(def: &str) -> Option<i64> {
    for token in def.split_whitespace() {
        let token = token.trim_start_matches('+');
        let Some(rest) = token.strip_prefix("init=") else {
            continue;
        };
        let rest = rest
            .strip_prefix("epsg:")
            .or_else(|| rest.strip_prefix("EPSG:"))?;
        return rest.split(['+', ':']).next()?.parse().ok();
    }
    None
}

fn first_auth(text: &str) -> Option<(String, i64)> {
    let text = text.trim();
    if text.is_empty() || text.contains("proj=") || text.split_whitespace().count() != 1 {
        return None;
    }
    let piece = text.split('+').next()?.trim();
    let (auth, code) = piece.split_once(':')?;
    if auth.is_empty()
        || !auth
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    Some((auth.to_owned(), code.parse().ok()?))
}

fn synthesize_longlat(def: &str) -> String {
    let mut out = String::from("+proj=longlat");
    for token in def.split_whitespace() {
        let bare = token.trim_start_matches('+');
        let key = bare.split('=').next().unwrap_or("");
        if keep_geographic_key(key) {
            out.push(' ');
            if token.starts_with('+') {
                out.push_str(token);
            } else {
                out.push('+');
                out.push_str(bare);
            }
        }
    }
    out
}

fn keep_geographic_key(key: &str) -> bool {
    matches!(
        key,
        "ellps"
            | "datum"
            | "a"
            | "b"
            | "rf"
            | "f"
            | "es"
            | "e"
            | "R"
            | "towgs84"
            | "nadgrids"
            | "pm"
    ) || key.starts_with("R_")
}

fn query_type(db: &Connection, auth: &str, code: i64) -> Result<String, rusqlite::Error> {
    db.query_row(
        "SELECT type FROM crs_view WHERE auth_name = ?1 AND code = ?2",
        (auth, code),
        |row| row.get(0),
    )
}

fn axis_is_degrees(db: &Connection, auth: &str, code: i64) -> Option<bool> {
    let factor: f64 = db
        .query_row(
            "SELECT u.conv_factor
             FROM geodetic_crs g
             JOIN axis a
               ON a.coordinate_system_auth_name = g.coordinate_system_auth_name
              AND a.coordinate_system_code = g.coordinate_system_code
              AND a.coordinate_system_order = 1
             JOIN unit_of_measure u
               ON u.auth_name = a.uom_auth_name
              AND u.code = a.uom_code
             WHERE g.auth_name = ?1 AND g.code = ?2",
            (auth, code),
            |row| row.get(0),
        )
        .ok()?;
    Some((factor - std::f64::consts::PI / 180.0).abs() < 1e-10)
}

fn datum_is_dynamic(db: &Connection, auth: &str, code: i64) -> bool {
    db.query_row(
        "SELECT d.frame_reference_epoch
         FROM geodetic_crs g
         JOIN geodetic_datum d
           ON d.auth_name = g.datum_auth_name AND d.code = g.datum_code
         WHERE g.auth_name = ?1 AND g.code = ?2",
        (auth, code),
        |row| row.get::<_, Option<f64>>(0),
    )
    .ok()
    .flatten()
    .is_some()
}

fn projected_datum_dynamic(db: &Connection, auth: &str, code: i64) -> bool {
    let Ok((base_auth, base_code)) = db.query_row(
        "SELECT geodetic_crs_auth_name, geodetic_crs_code
         FROM projected_crs WHERE auth_name = ?1 AND code = ?2",
        (auth, code),
        |row| Ok((row.get::<_, String>(0)?, cell_i64(row, 1)?)),
    ) else {
        return false;
    };
    datum_is_dynamic(db, &base_auth, base_code)
}

fn projected_base(db: &Connection, auth: &str, code: i64) -> Option<(String, i64)> {
    db.query_row(
        "SELECT geodetic_crs_auth_name, geodetic_crs_code
         FROM projected_crs WHERE auth_name = ?1 AND code = ?2",
        (auth, code),
        |row| Ok((row.get::<_, String>(0)?, cell_i64(row, 1)?)),
    )
    .ok()
}

fn compound_horizontal(db: &Connection, auth: &str, code: i64) -> Option<(String, i64)> {
    db.query_row(
        "SELECT horiz_crs_auth_name, horiz_crs_code
         FROM compound_crs WHERE auth_name = ?1 AND code = ?2",
        (auth, code),
        |row| Ok((row.get::<_, String>(0)?, cell_i64(row, 1)?)),
    )
    .ok()
}

fn cell_i64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<i64> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Integer(value) => Ok(value),
        rusqlite::types::ValueRef::Real(value) => float_to_i64(value),
        rusqlite::types::ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes);
            text.parse::<i64>()
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
        }
        _ => Err(rusqlite::Error::InvalidColumnType(
            index,
            String::new(),
            rusqlite::types::Type::Integer,
        )),
    }
}

fn float_to_i64(value: f64) -> rusqlite::Result<i64> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(rusqlite::Error::InvalidColumnType(
            0,
            String::new(),
            rusqlite::types::Type::Integer,
        ));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "a whole CRS code read from proj.db fits in i64"
    )]
    Ok(value as i64)
}

fn named_row(db: &Connection, sql: &str, name: &str) -> Option<(i64, String)> {
    let mut statement = db.prepare(sql).ok()?;
    let rows = statement
        .query_map([name], |row| {
            Ok((cell_i64(row, 0)?, row.get::<_, String>(1)?))
        })
        .ok()?;
    let mut best: Option<(i64, String)> = None;
    for row in rows {
        let Ok(row) = row else {
            continue;
        };
        best = Some(match best {
            Some(current) if rank(&current.1) <= rank(&row.1) => current,
            _ => row,
        });
    }
    best
}

fn named_any(db: &Connection, name: &str) -> Option<(String, i64, String)> {
    let mut statement = db
        .prepare(
            "SELECT auth_name, code, type FROM crs_view
             WHERE name = ?1 AND deprecated = 0 ORDER BY code",
        )
        .ok()?;
    let direct = statement
        .query_map([name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                cell_i64(row, 1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .ok()?;
    if let Some(found) = fold_named(direct) {
        return Some(found);
    }
    let mut alias = db
        .prepare(
            "SELECT c.auth_name, c.code, c.type
             FROM alias_name a
             JOIN crs_view c ON c.auth_name = a.auth_name AND c.code = a.code
             WHERE a.alt_name = ?1 AND c.deprecated = 0
             ORDER BY c.code",
        )
        .ok()?;
    let rows = alias
        .query_map([name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                cell_i64(row, 1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .ok()?;
    fold_named(rows)
}

fn fold_named<I>(rows: I) -> Option<(String, i64, String)>
where
    I: IntoIterator<Item = rusqlite::Result<(String, i64, String)>>,
{
    let mut best: Option<(String, i64, String)> = None;
    for row in rows {
        let Ok(row) = row else {
            continue;
        };
        best = Some(match best {
            Some(current) if rank(&current.2) <= rank(&row.2) => current,
            _ => row,
        });
    }
    best
}

fn rank(kind: &str) -> u8 {
    match kind {
        "geographic 2D" => 0,
        "geographic 3D" => 1,
        "geocentric" => 2,
        "projected" => 3,
        "compound" => 4,
        _ => 5,
    }
}

fn extent_code(db: &Connection, auth: &str, code: i64) -> Option<Area> {
    db.query_row(
        "SELECT west_lon, south_lat, east_lon, north_lat
         FROM extent WHERE auth_name = ?1 AND code = ?2",
        (auth, code),
        |row| {
            Ok(Area::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        },
    )
    .ok()
}

fn extent_name(db: &Connection, name: &str) -> Vec<(String, Area)> {
    let Ok(mut statement) = db.prepare(
        "SELECT auth_name, code, description, west_lon, south_lat, east_lon, north_lat
         FROM extent WHERE name = ?1 AND deprecated = 0 ORDER BY code",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([name], |row| {
        let auth: String = row.get(0)?;
        let code = cell_i64(row, 1)?;
        let description: String = row.get(2)?;
        let area = Area::new(row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?);
        Ok((format!("{auth}:{code} : {description}"), area))
    }) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok).collect()
}

#[derive(Clone, Copy)]
enum SecondForm {
    Compact,
    Fixed,
}

#[derive(Clone, Copy)]
enum XyStyle {
    Dms { fract: i32, form: SecondForm },
    Number(NumFmt),
}

struct Style {
    xy: XyStyle,
    z: NumFmt,
}

fn style_for(opts: &Options, layout: Layout) -> Style {
    if let Some(fmt) = opts.number_fmt {
        return Style {
            z: fmt,
            xy: XyStyle::Number(fmt),
        };
    }
    if layout == Layout::DegreeLongLat {
        return Style {
            xy: XyStyle::Dms {
                fract: opts.dms_fract,
                form: if opts.dms_wide {
                    SecondForm::Fixed
                } else {
                    SecondForm::Compact
                },
            },
            z: fixed_fmt(3),
        };
    }
    let fmt = fixed_fmt(2);
    Style {
        xy: XyStyle::Number(fmt),
        z: fmt,
    }
}

fn fixed_fmt(precision: usize) -> NumFmt {
    NumFmt {
        plus: false,
        zero_pad: false,
        width: None,
        precision: Some(precision),
        spec: b'f',
    }
}

#[derive(Clone, Copy, Debug)]
struct NumFmt {
    plus: bool,
    zero_pad: bool,
    width: Option<usize>,
    precision: Option<usize>,
    spec: u8,
}

fn parse_num_fmt(fmt: &str) -> Option<NumFmt> {
    let bytes = fmt.as_bytes();
    if bytes.first() != Some(&b'%') || bytes.len() < 2 {
        return None;
    }
    let mut i = 1;
    let plus = if bytes.get(i) == Some(&b'+') {
        i += 1;
        true
    } else {
        false
    };
    let mut width = None;
    let mut zero_pad = false;
    if bytes.get(i).is_some_and(u8::is_ascii_digit) {
        zero_pad = bytes[i] == b'0';
        let mut value = 0usize;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            value = value
                .saturating_mul(10)
                .saturating_add(usize::from(bytes[i] - b'0'));
            if value > 1000 {
                return None;
            }
            i += 1;
        }
        width = Some(value);
    }
    let mut precision = None;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        if bytes.get(i).is_some_and(u8::is_ascii_digit) {
            let mut value = 0usize;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                value = value
                    .saturating_mul(10)
                    .saturating_add(usize::from(bytes[i] - b'0'));
                if value > 1000 {
                    return None;
                }
                i += 1;
            }
            precision = Some(value);
        } else {
            precision = Some(0);
        }
    }
    let spec = *bytes.get(i)?;
    if i + 1 != bytes.len() || !matches!(spec, b'e' | b'E' | b'f' | b'F' | b'g' | b'G') {
        return None;
    }
    Some(NumFmt {
        plus,
        zero_pad,
        width,
        precision,
        spec,
    })
}

fn format_number(fmt: &NumFmt, value: f64) -> String {
    let precision = fmt.precision.unwrap_or(6);
    let upper = matches!(fmt.spec, b'E' | b'F' | b'G');
    if value.is_nan() {
        let body = if upper { "NAN" } else { "nan" };
        return pad_number(body, fmt);
    }
    if value.is_infinite() {
        let mut body = if value.is_sign_negative() {
            if upper {
                "-INF"
            } else {
                "-inf"
            }
        } else if fmt.plus {
            if upper {
                "+INF"
            } else {
                "+inf"
            }
        } else if upper {
            "INF"
        } else {
            "inf"
        }
        .to_owned();
        if upper {
            body.make_ascii_uppercase();
        }
        return pad_number(&body, fmt);
    }
    let mut body = match fmt.spec {
        b'e' | b'E' => format!("{value:.precision$e}"),
        _ => format!("{value:.precision$}"),
    };
    if upper {
        body.make_ascii_uppercase();
    }
    if fmt.plus && !body.starts_with('-') {
        body.insert(0, '+');
    }
    pad_number(&body, fmt)
}

fn pad_number(body: &str, fmt: &NumFmt) -> String {
    let Some(width) = fmt.width.filter(|width| *width > 0) else {
        return body.to_owned();
    };
    if body.len() >= width {
        return body.to_owned();
    }
    if !fmt.zero_pad {
        return format!("{body:>width$}");
    }
    let pad = width - body.len();
    if let Some(rest) = body.strip_prefix(['-', '+']) {
        let sign = &body[..body.len() - rest.len()];
        format!("{sign}{:0>pad$}{rest}", "")
    } else {
        format!("{body:0>width$}")
    }
}

fn rtodms(degrees: f64, pos: char, neg: char, fract: i32, form: SecondForm) -> String {
    let fract = fract.clamp(0, 8);
    let res = 10f64.powi(fract);
    let res60 = res * 60.0;
    let mut radians = degrees.to_radians();
    let conv = 180.0 * 3600.0 * res / std::f64::consts::PI;
    let mut prefixed = String::new();
    let sign = if radians < 0.0 {
        radians = -radians;
        if pos == '\0' {
            prefixed.push('-');
            '\0'
        } else {
            neg
        }
    } else {
        pos
    };
    let mut ticks = (radians * conv + 0.5).floor();
    let sec = (ticks / res) % 60.0;
    ticks = (ticks / res60).floor();
    let min = float_to_i32(ticks % 60.0);
    ticks = (ticks / 60.0).floor();
    let deg = float_to_i32(ticks);
    let sign_text = if sign == '\0' {
        String::new()
    } else {
        sign.to_string()
    };
    let fract_usize = usize::try_from(fract).unwrap_or(0);
    if matches!(form, SecondForm::Fixed) {
        let sec_width = fract_usize + 2 + usize::from(fract > 0);
        let sec_text = zero_pad(&format!("{sec:.fract_usize$}"), sec_width);
        prefixed.push_str(&format!("{deg}d{min:02}'{sec_text}\"{sign_text}"));
    } else if sec != 0.0 {
        let sec_text = strip_fraction_zeros(&format!("{sec:.fract_usize$}"));
        prefixed.push_str(&format!("{deg}d{min}'{sec_text}\"{sign_text}"));
    } else if min != 0 {
        prefixed.push_str(&format!("{deg}d{min}'{sign_text}"));
    } else {
        prefixed.push_str(&format!("{deg}d{sign_text}"));
    }
    prefixed
}

fn zero_pad(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_owned()
    } else {
        format!("{text:0>width$}")
    }
}

fn strip_fraction_zeros(text: &str) -> String {
    let Some(dot) = text.find('.') else {
        return text.to_owned();
    };
    let bytes = text.as_bytes();
    let mut end = bytes.len();
    while end > dot + 1 && bytes[end - 1] == b'0' {
        end -= 1;
    }
    if end == dot + 1 {
        end = dot;
    }
    text[..end].to_owned()
}

fn float_to_i32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let truncated = value.trunc();
    if truncated >= f64::from(i32::MAX) {
        i32::MAX
    } else if truncated <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::as_conversions,
            reason = "the truncated DMS field is inside i32"
        )]
        {
            truncated as i32
        }
    }
}

const DEGREE_SCALE: [f64; 3] = [1.0, 1.0 / 60.0, 1.0 / 3600.0];

fn read_axis(text: &str, start: usize, kind: AxisText) -> (f64, usize) {
    match kind {
        AxisText::Dms => dmstor_degrees(text, start),
        AxisText::Decimal => {
            let (value, next, _) = strtod_full(text, start);
            (value, next)
        }
    }
}

fn read_number(text: &str, start: usize) -> (f64, usize) {
    let (value, next, _) = strtod_full(text, start);
    (value, next)
}

fn read_number_consumed(text: &str, start: usize) -> bool {
    let (_, _, consumed) = strtod_full(text, start);
    consumed
}

fn dmstor_degrees(text: &str, start: usize) -> (f64, usize) {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let token_start = i;
    let mut count = 0;
    while i < bytes.len() && count < 63 {
        if bytes[i] == 0xC2 && i + 1 < bytes.len() && bytes[i + 1] == 0xB0 {
            i += 2;
            count += 2;
            continue;
        }
        if bytes[i].is_ascii_graphic() {
            i += 1;
            count += 1;
            continue;
        }
        break;
    }
    let token = &text[token_start..i];
    let (value, used) = parse_dms_token(token);
    (value, token_start + used.min(token.len()))
}

fn parse_dms_token(token: &str) -> (f64, usize) {
    let bytes = token.as_bytes();
    let mut s = 0;
    let mut sign = 1.0;
    if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
        if bytes[0] == b'-' {
            sign = -1.0;
        }
        s = 1;
    }
    let mut value = 0.0;
    let mut level = 0i32;
    while level < 3 {
        if s >= bytes.len() || !(bytes[s].is_ascii_digit() || bytes[s] == b'.') {
            break;
        }
        let (number, width) = strtod_strict(&token[s..]);
        if width == 0 {
            break;
        }
        if !number.is_finite() {
            return (f64::INFINITY, s);
        }
        s += width;
        match dms_marker(bytes, s) {
            DmsMark::Unit { which, adv } => {
                if which < level {
                    return (f64::INFINITY, s);
                }
                let scale = DEGREE_SCALE[usize::try_from(which).unwrap_or(0)];
                value += number * scale;
                s += adv;
                level = which + 1;
            }
            DmsMark::Radians => {
                if level != 0 {
                    return (f64::INFINITY, s);
                }
                s += 1;
                value = number.to_degrees();
                level = 4;
            }
            DmsMark::Plain => {
                let scale = DEGREE_SCALE[usize::try_from(level).unwrap_or(0)];
                value += number * scale;
                level = 4;
            }
        }
    }
    if s < bytes.len() && is_hemisphere(bytes[s]) {
        sign = if matches!(bytes[s], b'S' | b's' | b'W' | b'w') {
            -1.0
        } else {
            1.0
        };
        s += 1;
    }
    (value * sign, s)
}

enum DmsMark {
    Unit { which: i32, adv: usize },
    Radians,
    Plain,
}

fn dms_marker(bytes: &[u8], s: usize) -> DmsMark {
    if s >= bytes.len() {
        return DmsMark::Plain;
    }
    if bytes[s] == b'D' || bytes[s] == b'd' || bytes[s] == 0xB0 {
        return DmsMark::Unit { which: 0, adv: 1 };
    }
    if bytes[s] == b'\'' {
        return DmsMark::Unit { which: 1, adv: 1 };
    }
    if bytes[s] == b'"' {
        return DmsMark::Unit { which: 2, adv: 1 };
    }
    if s + 1 < bytes.len() && bytes[s] == 0xC2 && bytes[s + 1] == 0xB0 {
        return DmsMark::Unit { which: 0, adv: 2 };
    }
    if bytes[s] == b'r' || bytes[s] == b'R' {
        return DmsMark::Radians;
    }
    DmsMark::Plain
}

fn is_hemisphere(byte: u8) -> bool {
    matches!(byte, b'N' | b'n' | b'E' | b'e' | b'S' | b's' | b'W' | b'w')
}

fn strtod_full(text: &str, start: usize) -> (f64, usize, bool) {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return (0.0, start, false);
    }
    let (value, width) = strtod_strict(&text[i..]);
    if width == 0 {
        if let Some((value, width)) = parse_inf_nan(&text[i..]) {
            return (value, i + width, true);
        }
        return (0.0, start, false);
    }
    (value, i + width, true)
}

fn strtod_strict(text: &str) -> (f64, usize) {
    let bytes = text.as_bytes();
    let mut i = 0;
    if bytes.first() == Some(&b'+') || bytes.first() == Some(&b'-') {
        i = 1;
    }
    let mut saw_digit = false;
    let mut saw_dot = false;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => {
                saw_digit = true;
                i += 1;
            }
            b'.' if !saw_dot => {
                saw_dot = true;
                i += 1;
            }
            b'e' | b'E' if saw_digit => {
                let mark = i;
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                let exp = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i == exp {
                    i = mark;
                }
                break;
            }
            _ => break,
        }
    }
    if !saw_digit {
        return (0.0, 0);
    }
    let value = text[..i].parse::<f64>().unwrap_or(f64::INFINITY);
    (value, i)
}

fn parse_inf_nan(text: &str) -> Option<(f64, usize)> {
    let lower = text.to_ascii_lowercase();
    let (sign, rest, sign_len) = if let Some(rest) = lower.strip_prefix('-') {
        (-1.0, rest, 1)
    } else if let Some(rest) = lower.strip_prefix('+') {
        (1.0, rest, 1)
    } else {
        (1.0, lower.as_str(), 0)
    };
    if let Some(rest) = rest.strip_prefix("infinity") {
        let _ = rest;
        return Some((sign * f64::INFINITY, sign_len + "infinity".len()));
    }
    if rest.starts_with("inf") {
        return Some((sign * f64::INFINITY, sign_len + 3));
    }
    if rest.starts_with("nan") {
        return Some((f64::NAN, sign_len + 3));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dms_west_and_plain_degrees() {
        let (value, next) = dmstor_degrees("79d58'00.000\"W 37d", 0);
        assert!((value + 79.0 + 58.0 / 60.0).abs() < 1e-12, "{value}");
        assert!(next > 10, "{next}");
        let (plain, _) = dmstor_degrees("46.5", 0);
        assert!((plain - 46.5).abs() < 1e-12);
    }

    #[test]
    fn rtodms_matches_proj_stripping_and_width() {
        assert_eq!(
            rtodms(-79.0 - 58.0 / 60.0, 'E', 'W', 3, SecondForm::Compact),
            "79d58'W"
        );
        assert_eq!(
            rtodms(37.0 + 2.0 / 60.0, 'N', 'S', 3, SecondForm::Compact),
            "37d2'N"
        );
        assert_eq!(rtodms(0.0, 'E', 'W', 0, SecondForm::Fixed), "0d00'00\"E");
        assert_eq!(
            rtodms(0.0, 'N', 'S', 8, SecondForm::Fixed),
            "0d00'00.00000000\"N"
        );
        assert_eq!(
            rtodms(
                4.0 + 53.0 / 60.0 + 49.2 / 3600.0,
                'E',
                'W',
                3,
                SecondForm::Compact
            ),
            "4d53'49.2\"E"
        );
    }

    #[test]
    fn dash_d_sets_precision_and_to_still_splits() {
        let args = [
            "+proj=latlong".to_owned(),
            "+ellps=WGS84".to_owned(),
            "-d".to_owned(),
            "8".to_owned(),
            "+to".to_owned(),
            "+proj=utm".to_owned(),
            "+zone=31".to_owned(),
        ];
        let opts = parse(&args).expect("parse");
        assert_eq!(opts.source, "+proj=latlong +ellps=WGS84");
        assert_eq!(opts.target, "+proj=utm +zone=31");
        let fmt = opts.number_fmt.expect("fmt");
        assert_eq!(format_number(&fmt, 1.2), "1.20000000");
        let three = parse(&[
            "-d".to_owned(),
            "3".to_owned(),
            "EPSG:4326".to_owned(),
            "EPSG:32631".to_owned(),
        ])
        .expect("parse 3");
        let fmt = three.number_fmt.expect("fmt 3");
        assert_eq!(format_number(&fmt, 1.2), "1.200");
    }

    #[test]
    fn bad_w_is_rejected() {
        for arg in ["-W", "-W9", "-W10", "-w"] {
            let error = parse(&[arg.to_owned()]).expect_err(arg);
            assert_eq!(error.code, 1, "{arg}");
            assert!(
                error
                    .message
                    .contains("-W argument missing or not in range [0,8]"),
                "{}",
                error.message
            );
        }
        let opts = parse(&[
            "-W8".to_owned(),
            "+proj=latlong".to_owned(),
            "+to".to_owned(),
            "+proj=latlong".to_owned(),
        ])
        .expect("W8");
        assert!(opts.dms_wide);
        assert_eq!(opts.dms_fract, 8);
        assert!(opts.number_fmt.is_none());
    }

    #[test]
    fn epsg_layout_uses_proj_db() {
        let catalog = Catalog::open();
        assert_eq!(catalog.layout("EPSG:4326").layout, Layout::DegreeLongLat);
        assert_eq!(catalog.layout("EPSG:32631").layout, Layout::Projected);
        assert_eq!(catalog.layout("EPSG:4807").layout, Layout::Other);
        assert_eq!(catalog.layout("NAD27").layout, Layout::DegreeLongLat);
        assert!(catalog.layout("ITRF2014").dynamic);
        assert!(!catalog.layout("NAD27").dynamic);
        assert_eq!(
            catalog.layout("+proj=latlong +ellps=WGS84").layout,
            Layout::DegreeLongLat
        );
        assert_eq!(
            catalog.layout("+proj=utm +zone=31 +ellps=WGS84").layout,
            Layout::Projected
        );
    }
}
