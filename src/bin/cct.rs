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
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

use proj_rs::{
    AlternativeGridNames, AuthorityFactory, CoordinateOperation, DatabaseContext, Operation,
    ParameterValue, Transformation, WktObject, WktParser, Xyzt,
};

const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";

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
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }
    if options.version {
        println!("cct: {RELEASE}");
        return ExitCode::SUCCESS;
    }
    let operation = match resolve_operation(&options.definition) {
        Ok(operation) => operation,
        Err(error) => {
            eprintln!("cct: {error}");
            return ExitCode::from(1);
        }
    };
    let run = |reader: &mut dyn BufRead, output: &mut dyn Write| {
        transcribe(&options, &operation, reader, output)
    };
    match options.output.as_deref() {
        Some(path) => match File::create(path) {
            Ok(file) => {
                let mut output = BufWriter::new(file);
                drive(&options, &mut output, run)
            }
            Err(_) => {
                eprintln!("cct: Cannot open '{path}' for output");
                ExitCode::from(1)
            }
        },
        None => {
            let stdout = io::stdout();
            let mut output = stdout.lock();
            drive(&options, &mut output, run)
        }
    }
}

fn drive(
    options: &Options,
    output: &mut dyn Write,
    mut run: impl FnMut(&mut dyn BufRead, &mut dyn Write) -> ExitCode,
) -> ExitCode {
    if options.files.is_empty() {
        return run(&mut io::stdin().lock(), output);
    }
    for path in &options.files {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => {
                eprintln!("Cannot open file {path}");
                return ExitCode::from(1);
            }
        };
        let mut reader = BufReader::new(file);
        let code = run(&mut reader, output);
        if code != ExitCode::SUCCESS {
            return code;
        }
    }
    ExitCode::SUCCESS
}

struct Options {
    help: bool,
    version: bool,
    inverse: bool,
    /// `None` keeps PROJ's split default: 10 decimals for angles, 4 for distances.
    decimals: Option<usize>,
    z: Option<f64>,
    t: Option<f64>,
    output: Option<String>,
    definition: String,
    files: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut version = false;
        let mut inverse = false;
        let mut decimals = None;
        let mut z = None;
        let mut t = None;
        let mut output = None;
        let mut parts = Vec::new();
        let mut files = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "-h" | "--help" => help = true,
                "--version" => version = true,
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
                other if parts.is_empty() || other.starts_with('+') => parts.push(other.to_owned()),
                other => files.push(other.to_owned()),
            }
            index += 1;
        }
        if !help && !version && parts.is_empty() {
            return Err("missing +proj definition".to_owned());
        }
        Ok(Self {
            help,
            version,
            inverse,
            decimals,
            z,
            t,
            output,
            definition: parts.join(" "),
            files,
        })
    }
}

fn transcribe(
    options: &Options,
    operation: &Operation,
    input: &mut dyn BufRead,
    mut output: impl Write,
) -> ExitCode {
    let decimals_angles = options.decimals.unwrap_or(DEFAULT_ANGLE_DECIMALS);
    let decimals_distances = options.decimals.unwrap_or(DEFAULT_DISTANCE_DECIMALS);
    // `%14` / `%12` when the output is radians or degrees; `%13` for metres.
    let columns =
        if operation.angular_output(options.inverse) || operation.degree_output(options.inverse) {
            Columns::Angle
        } else {
            Columns::Distance
        };
    let mut first = true;
    for line in input.lines() {
        let Ok(line) = line else {
            eprintln!("cct: could not read input");
            return ExitCode::from(1);
        };
        let mut line = line;
        if first {
            first = false;
            if let Some(rest) = line.strip_prefix('\u{feff}') {
                line = rest.to_owned();
            }
        }
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

fn resolve_operation(definition: &str) -> Result<Operation, String> {
    let text = if let Some(path) = definition.strip_prefix('@') {
        std::fs::read_to_string(path).map_err(|_| format!("cannot open {path}"))?
    } else {
        definition.to_owned()
    };
    let trimmed = text.trim();
    if trimmed.contains("+proj=") || trimmed.contains("proj=") {
        return Operation::create(trimmed).map_err(|error| error.to_string());
    }
    if looks_like_crs(trimmed) {
        return Err("Input object is not a coordinate operation, but a CRS.".to_owned());
    }
    if trimmed.starts_with("COORDINATEOPERATION")
        || trimmed.starts_with("CONVERSION")
        || trimmed.starts_with("CONCATENATEDOPERATION")
    {
        let mut parser = WktParser::new();
        let object = parser
            .create_from_wkt(trimmed)
            .map_err(|error| error.to_string())?;
        return operation_from_wkt(object);
    }
    if let Some((auth, code)) = authority_code(trimmed) {
        return operation_from_database(&auth, &code);
    }
    operation_from_name(trimmed)
}

fn looks_like_crs(text: &str) -> bool {
    let head = text.trim_start();
    head.starts_with("GEOGCRS")
        || head.starts_with("GEODCRS")
        || head.starts_with("PROJCRS")
        || head.starts_with("VERTCRS")
        || head.starts_with("COMPOUNDCRS")
        || head.starts_with("BOUNDCRS")
        || head.starts_with("GEOCCS")
        || head.starts_with("GEOGCS")
        || head.starts_with("PROJCS")
}

fn authority_code(text: &str) -> Option<(String, String)> {
    let (auth, code) = text.split_once(':')?;
    if auth.is_empty()
        || !auth
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || code.is_empty()
        || !code.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some((auth.to_owned(), code.to_owned()))
}

fn operation_from_database(auth: &str, code: &str) -> Result<Operation, String> {
    let db = DatabaseContext::create().map_err(|error| error.to_string())?;
    let factory = AuthorityFactory::create(&db, auth);
    let operation = factory
        .create_coordinate_operation(code, AlternativeGridNames::Official)
        .map_err(|error| error.to_string())?;
    let definition = coordinate_operation_proj(&operation)?;
    Operation::create(&definition).map_err(|error| error.to_string())
}

fn operation_from_name(name: &str) -> Result<Operation, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/proj.db");
    let connection = rusqlite::Connection::open(path).map_err(|error| error.to_string())?;
    let code: i64 = connection
        .query_row(
            "SELECT code FROM coordinate_operation_view \
             WHERE name = ?1 AND auth_name = 'EPSG' \
             ORDER BY code LIMIT 1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .map_err(|_| format!("unknown operation `{name}`"))?;
    operation_from_database("EPSG", &code.to_string())
}

fn operation_from_wkt(object: WktObject) -> Result<Operation, String> {
    match object {
        WktObject::Conversion(conversion) => conversion
            .export_to_proj_string()
            .map_err(|error| error.to_string())
            .and_then(|definition| {
                Operation::create(&definition).map_err(|error| error.to_string())
            }),
        WktObject::Transformation(transformation) => {
            let definition = transformation_proj(&transformation)?;
            Operation::create(&definition).map_err(|error| error.to_string())
        }
        WktObject::Crs(_) => {
            Err("Input object is not a coordinate operation, but a CRS.".to_owned())
        }
        _ => Err("Input object is not a coordinate operation.".to_owned()),
    }
}

fn coordinate_operation_proj(operation: &CoordinateOperation) -> Result<String, String> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion
            .export_to_proj_string()
            .map_err(|error| error.to_string()),
        CoordinateOperation::Transformation(transformation) => transformation_proj(transformation),
        CoordinateOperation::Concatenated(_) => {
            Err("concatenated operation is not a single helmert".to_owned())
        }
    }
}

fn transformation_proj(transformation: &Transformation) -> Result<String, String> {
    let method = transformation.method().code();
    let convention = match method {
        "1032" | "9607" | "1038" | "1056" | "1057" | "1066" => "coordinate_frame",
        "1033" | "9606" | "1037" | "1053" | "1054" | "1065" => "position_vector",
        _ => "",
    };
    let mut parts = vec!["+proj=helmert".to_owned()];
    push_len(
        &mut parts,
        "x",
        measure(transformation, "8605", "X-axis translation"),
    );
    push_len(
        &mut parts,
        "y",
        measure(transformation, "8606", "Y-axis translation"),
    );
    push_len(
        &mut parts,
        "z",
        measure(transformation, "8607", "Z-axis translation"),
    );
    push_arcsec(
        &mut parts,
        "rx",
        measure(transformation, "8608", "X-axis rotation"),
    );
    push_arcsec(
        &mut parts,
        "ry",
        measure(transformation, "8609", "Y-axis rotation"),
    );
    push_arcsec(
        &mut parts,
        "rz",
        measure(transformation, "8610", "Z-axis rotation"),
    );
    push_ppm(
        &mut parts,
        "s",
        measure(transformation, "8611", "Scale difference"),
    );
    let rates = measure(
        transformation,
        "1040",
        "Rate of change of X-axis translation",
    )
    .is_some()
        || measure(transformation, "1043", "Rate of change of X-axis rotation").is_some()
        || measure(transformation, "1047", "Parameter reference epoch").is_some();
    if rates {
        push_len(
            &mut parts,
            "dx",
            measure(
                transformation,
                "1040",
                "Rate of change of X-axis translation",
            ),
        );
        push_len(
            &mut parts,
            "dy",
            measure(
                transformation,
                "1041",
                "Rate of change of Y-axis translation",
            ),
        );
        push_len(
            &mut parts,
            "dz",
            measure(
                transformation,
                "1042",
                "Rate of change of Z-axis translation",
            ),
        );
        push_arcsec(
            &mut parts,
            "drx",
            measure(transformation, "1043", "Rate of change of X-axis rotation"),
        );
        push_arcsec(
            &mut parts,
            "dry",
            measure(transformation, "1044", "Rate of change of Y-axis rotation"),
        );
        push_arcsec(
            &mut parts,
            "drz",
            measure(transformation, "1045", "Rate of change of Z-axis rotation"),
        );
        push_ppm(
            &mut parts,
            "ds",
            measure(transformation, "1046", "Rate of change of Scale difference"),
        );
        if let Some(epoch) = raw_measure(transformation, "1047", "Parameter reference epoch") {
            parts.push(format!("+t_epoch={}", trim_num(epoch)));
        }
    }
    if !convention.is_empty() {
        parts.push(format!("+convention={convention}"));
    }
    Ok(parts.join(" "))
}

fn measure(transformation: &Transformation, code: &str, name: &str) -> Option<f64> {
    param_pair(transformation, code, name).map(|(_, si, unit)| yearly(si, &unit))
}

fn yearly(si: f64, unit: &str) -> f64 {
    // Database factors for "per year" quantities are per SI second.
    if unit.to_ascii_lowercase().contains("per year") {
        si * 31_556_925.445
    } else {
        si
    }
}

fn raw_measure(transformation: &Transformation, code: &str, name: &str) -> Option<f64> {
    param_pair(transformation, code, name).map(|(raw, _, _)| raw)
}

fn param_pair(
    transformation: &Transformation,
    code: &str,
    name: &str,
) -> Option<(f64, f64, String)> {
    transformation.parameters().iter().find_map(|parameter| {
        let code_match = !code.is_empty() && parameter.code() == code;
        let name_match = parameter.name().eq_ignore_ascii_case(name);
        if !code_match && !name_match {
            return None;
        }
        match parameter.value() {
            ParameterValue::Measure(measure) => Some((
                measure.value(),
                measure.si_value(),
                measure.unit().name().to_owned(),
            )),
            _ => None,
        }
    })
}

fn push_len(parts: &mut Vec<String>, key: &str, metres: Option<f64>) {
    if let Some(value) = metres.filter(|value| *value != 0.0) {
        parts.push(format!("+{key}={}", trim_num(value)));
    }
}

fn push_arcsec(parts: &mut Vec<String>, key: &str, radians: Option<f64>) {
    if let Some(value) = radians.filter(|value| *value != 0.0) {
        let arcsec = value.to_degrees() * 3_600.0;
        parts.push(format!("+{key}={}", trim_num(arcsec)));
    }
}

fn push_ppm(parts: &mut Vec<String>, key: &str, si: Option<f64>) {
    if let Some(value) = si.filter(|value| *value != 0.0) {
        let ppm = value / 1e-6;
        parts.push(format!("+{key}={}", trim_num(ppm)));
    }
}

fn trim_num(value: f64) -> String {
    let mut text = format!("{value:.12}");
    if let Some(dot) = text.find('.') {
        while text.ends_with('0') && text.len() > dot + 2 {
            text.pop();
        }
    }
    text
}

fn usage() -> &'static str {
    "Usage: cct [-options]... [+operator_specs]... infile...\n\
     Angular axes are degrees. One x y [z] [t] point per line.\n\
     -o writes the coordinate lines to a file instead of stdout."
}
