//! `projinfo`: describe one geographic EPSG CRS from the embedded `proj.db`.
//!
//! `--version`, `-o PROJ`, `-o WKT2:2019`, and `-o PROJJSON` follow
//! `projinfo_lib.cpp`. Any other `-o` dialect is an error: this port does not
//! write it. A source/target operation is not a CRS export.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "projinfo writes CRS text to stdout and errors to stderr"
)]

use std::fmt::Write as _;
use std::process::ExitCode;

use rusqlite::{params, Connection, OptionalExtension};

const PROJ_DB: &[u8] = include_bytes!("../../data/proj.db");
const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";
const PROJJSON_SCHEMA: &str = "https://proj.org/schemas/v0.7/projjson.schema.json";

fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(Report::Usage(message)) => {
            eprintln!("projinfo: {message}");
            eprintln!("{}", usage());
            ExitCode::from(1)
        }
        Err(Report::Plain(message)) => {
            eprintln!("projinfo: {message}");
            ExitCode::from(1)
        }
    }
}

enum Report {
    Usage(String),
    Plain(String),
}

fn run(args: &[String]) -> Result<String, Report> {
    let options = Options::parse(args).map_err(Report::Usage)?;
    if options.help {
        return Ok(usage().to_owned());
    }
    if options.version {
        return Ok(format!("projinfo: {RELEASE}"));
    }
    let code = options
        .code
        .as_deref()
        .ok_or_else(|| Report::Usage("missing EPSG code".to_owned()))?;
    let connection = open_db().map_err(Report::Plain)?;
    let crs = load_geographic(&connection, code).map_err(Report::Plain)?;
    if options.quiet && options.dialects.len() != 1 {
        return Err(Report::Usage(
            "-q can only be used with a single output format".to_owned(),
        ));
    }
    Ok(render(&crs, &options))
}

fn usage() -> &'static str {
    "usage: projinfo [-o formats] [-q] [--version] EPSG:code\n\
     -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2019).\n\
     Other dialects (WKT1, WKT2:2015, SQL, all) are an error."
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dialect {
    Proj,
    Wkt2_2019,
    ProjJson,
}

struct Options {
    help: bool,
    version: bool,
    quiet: bool,
    dialects: Vec<Dialect>,
    code: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut version = false;
        let mut quiet = false;
        let mut proj = false;
        let mut wkt = false;
        let mut json = false;
        let mut saw_format = false;
        let mut code = None;
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            if arg == "-h" || arg == "--help" || arg == "-?" {
                help = true;
            } else if arg == "--version" {
                version = true;
            } else if arg == "-q" || arg == "--quiet" {
                quiet = true;
            } else if arg == "-o" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for -o".to_owned())?;
                saw_format = true;
                for piece in value.split(',') {
                    match classify_dialect(piece)? {
                        Some(Dialect::Proj) => proj = true,
                        Some(Dialect::Wkt2_2019) => wkt = true,
                        Some(Dialect::ProjJson) => json = true,
                        None => {
                            proj = true;
                            wkt = true;
                        }
                    }
                }
            } else if let Some(parsed) = epsg_code(arg) {
                if code.is_some() {
                    return Err("a CRS pair is not built; pass one EPSG code".to_owned());
                }
                code = Some(parsed);
            } else if arg.starts_with('-') {
                return Err(format!("unrecognized option {arg}"));
            } else {
                return Err(format!(
                    "`{arg}` is not an EPSG code this port can describe"
                ));
            }
            index += 1;
        }
        let mut dialects = Vec::new();
        if !saw_format {
            proj = true;
            wkt = true;
        }
        if proj {
            dialects.push(Dialect::Proj);
        }
        if wkt {
            dialects.push(Dialect::Wkt2_2019);
        }
        if json {
            dialects.push(Dialect::ProjJson);
        }
        if saw_format && dialects.is_empty() && !help && !version {
            return Err("no output format remains enabled".to_owned());
        }
        Ok(Self {
            help,
            version,
            quiet,
            dialects,
            code,
        })
    }
}

/// `None` is `default`, which turns on PROJ and WKT2:2019.
fn classify_dialect(text: &str) -> Result<Option<Dialect>, String> {
    let folded = text.trim().to_ascii_lowercase();
    const UNSUPPORTED: &[&str] = &[
        "all",
        "wkt_all",
        "wkt-all",
        "wkt2_2015",
        "wkt2-2015",
        "wkt2:2015",
        "wkt2_2015_simplified",
        "wkt2-2015_simplified",
        "wkt2:2015_simplified",
        "wkt2_2019_simplified",
        "wkt2-2019_simplified",
        "wkt2:2019_simplified",
        "wkt2_2018_simplified",
        "wkt2-2018_simplified",
        "wkt2:2018_simplified",
        "wkt1_gdal",
        "wkt1-gdal",
        "wkt1:gdal",
        "wkt1_esri",
        "wkt1-esri",
        "wkt1:esri",
        "sql",
    ];
    if UNSUPPORTED.contains(&folded.as_str()) {
        return Err(format!("cannot write dialect {text}"));
    }
    Ok(match folded.as_str() {
        "default" => None,
        "proj" => Some(Dialect::Proj),
        "wkt2_2019" | "wkt2-2019" | "wkt2:2019" | "wkt2_2018" | "wkt2-2018" | "wkt2:2018" => {
            Some(Dialect::Wkt2_2019)
        }
        "projjson" => Some(Dialect::ProjJson),
        _ => return Err(format!("unrecognized value for option -o: {text}")),
    })
}

fn epsg_code(text: &str) -> Option<String> {
    let (authority, code) = text.split_once(':')?;
    if authority.eq_ignore_ascii_case("epsg")
        && !code.is_empty()
        && code.bytes().all(|byte| byte.is_ascii_digit())
    {
        Some(code.to_owned())
    } else {
        None
    }
}

fn open_db() -> Result<Connection, String> {
    let mut connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .deserialize_bytes(rusqlite::MAIN_DB, PROJ_DB)
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

struct GeoCrs {
    code: String,
    name: String,
    dimension: i64,
    cs_type: String,
    deprecated: bool,
    datum: Datum,
    meridian: PrimeMeridian,
    axes: Vec<Axis>,
    usages: Vec<Usage>,
}

struct Datum {
    code: String,
    name: String,
    ensemble: bool,
    /// SQLite text of `ensemble_accuracy`, so WKT keeps `2.0` rather than `2`.
    accuracy: Option<String>,
    members: Vec<(String, String)>,
    ellipsoid: Ellipsoid,
    anchor: Option<String>,
    anchor_epoch: Option<f64>,
    frame_epoch: Option<f64>,
}

struct Ellipsoid {
    name: String,
    semi_major: f64,
    inverse_flattening: Option<f64>,
    semi_minor: Option<f64>,
    unit: Unit,
}

struct PrimeMeridian {
    name: String,
    longitude: f64,
    unit: Unit,
}

struct Axis {
    name: String,
    abbrev: String,
    direction: String,
    unit: Unit,
}

struct Unit {
    name: String,
    factor: f64,
    kind: UnitKind,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum UnitKind {
    Length,
    Angle,
    Scale,
    Time,
    Other,
}

struct Usage {
    scope: String,
    area: String,
    south: Option<f64>,
    north: Option<f64>,
    west: Option<f64>,
    east: Option<f64>,
}

fn load_geographic(connection: &Connection, code: &str) -> Result<GeoCrs, String> {
    let row = connection
        .query_row(
            "SELECT name, type, deprecated,
                    datum_auth_name, CAST(datum_code AS TEXT),
                    coordinate_system_auth_name, CAST(coordinate_system_code AS TEXT)
             FROM geodetic_crs
             WHERE auth_name = 'EPSG' AND CAST(code AS TEXT) = ?1",
            params![code],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((name, crs_type, deprecated, datum_auth, datum_code, cs_auth, cs_code)) = row else {
        return Err(missing_object(connection, code)?);
    };
    if crs_type != "geographic 2D" && crs_type != "geographic 3D" {
        return Err(format!(
            "EPSG:{code} is {crs_type}; this port writes a geographic CRS only"
        ));
    }
    let (datum_auth, datum_code) = datum_auth
        .zip(datum_code)
        .ok_or_else(|| format!("EPSG:{code} has no datum row this port can write"))?;
    let (cs_auth, cs_code) = cs_auth
        .zip(cs_code)
        .ok_or_else(|| format!("EPSG:{code} has no coordinate system this port can write"))?;
    let datum = load_datum(connection, &datum_auth, &datum_code)?;
    let (cs_type, dimension) = connection
        .query_row(
            "SELECT type, dimension FROM coordinate_system WHERE auth_name = ?1 AND CAST(code AS TEXT) = ?2",
            params![cs_auth, cs_code],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if cs_type != "ellipsoidal" {
        return Err(format!(
            "EPSG:{code} coordinate system `{cs_type}` is not ellipsoidal"
        ));
    }
    let axes = load_axes(connection, &cs_auth, &cs_code)?;
    if i64::try_from(axes.len()).ok() != Some(dimension) {
        return Err(format!(
            "EPSG:{code} axis count does not match its coordinate system"
        ));
    }
    let usages = if code == "4326" {
        // EPSG changed 4326 from extent 1262 ("World.") to the country list in
        // 2830. PROJ still exports the old extent so WKT2 stays stable.
        vec![load_fixed_usage(connection, "1262", "1183")?]
    } else {
        load_usages(connection, code)?
    };
    Ok(GeoCrs {
        code: code.to_owned(),
        name,
        dimension,
        cs_type,
        deprecated: deprecated != 0,
        datum,
        meridian: load_meridian(connection, &datum_auth, &datum_code)?,
        axes,
        usages,
    })
}

fn missing_object(connection: &Connection, code: &str) -> Result<String, String> {
    let found = connection
        .query_row(
            "SELECT table_name, type FROM crs_view WHERE auth_name = 'EPSG' AND CAST(code AS TEXT) = ?1",
            params![code],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(match found {
        Some((table, crs_type)) => {
            format!("EPSG:{code} is {crs_type} ({table}); this port writes a geographic CRS only")
        }
        None => format!("EPSG:{code} is not in proj.db"),
    })
}

fn load_datum(connection: &Connection, auth: &str, code: &str) -> Result<Datum, String> {
    let (name, ell_auth, ell_code, accuracy, anchor, anchor_epoch, frame_epoch) = connection
        .query_row(
            "SELECT name, ellipsoid_auth_name, CAST(ellipsoid_code AS TEXT),
                    CAST(ensemble_accuracy AS TEXT), anchor, anchor_epoch, frame_reference_epoch
             FROM geodetic_datum
             WHERE auth_name = ?1 AND CAST(code AS TEXT) = ?2",
            params![auth, code],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, Option<f64>>(6)?,
                ))
            },
        )
        .map_err(|error| format!("datum {auth}:{code}: {error}"))?;
    let members = connection
        .prepare(
            "SELECT d.name, CAST(d.code AS TEXT)
             FROM geodetic_datum_ensemble_member m
             JOIN geodetic_datum d
               ON d.auth_name = m.member_auth_name AND d.code = m.member_code
             WHERE m.ensemble_auth_name = ?1 AND CAST(m.ensemble_code AS TEXT) = ?2
             ORDER BY m.sequence",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![auth, code], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let ensemble = !members.is_empty();
    if ensemble && accuracy.is_none() {
        return Err(format!(
            "datum {auth}:{code} is an ensemble without accuracy"
        ));
    }
    Ok(Datum {
        code: code.to_owned(),
        name,
        ensemble,
        accuracy,
        members,
        ellipsoid: load_ellipsoid(connection, &ell_auth, &ell_code)?,
        anchor,
        anchor_epoch,
        frame_epoch,
    })
}

fn load_ellipsoid(connection: &Connection, auth: &str, code: &str) -> Result<Ellipsoid, String> {
    connection
        .query_row(
            "SELECT e.name, e.semi_major_axis, e.inv_flattening, e.semi_minor_axis,
                    u.code, u.name, u.conv_factor, u.type
             FROM ellipsoid e
             JOIN unit_of_measure u
               ON u.auth_name = e.uom_auth_name AND u.code = e.uom_code
             WHERE e.auth_name = ?1 AND CAST(e.code AS TEXT) = ?2",
            params![auth, code],
            |row| {
                Ok(Ellipsoid {
                    name: row.get(0)?,
                    semi_major: row.get(1)?,
                    inverse_flattening: row.get(2)?,
                    semi_minor: row.get(3)?,
                    unit: unit_from_row(row, 4)?,
                })
            },
        )
        .map_err(|error| format!("ellipsoid {auth}:{code}: {error}"))
}

fn load_meridian(
    connection: &Connection,
    datum_auth: &str,
    datum_code: &str,
) -> Result<PrimeMeridian, String> {
    connection
        .query_row(
            "SELECT p.name, p.longitude, u.code, u.name, u.conv_factor, u.type
             FROM geodetic_datum d
             JOIN prime_meridian p
               ON p.auth_name = d.prime_meridian_auth_name AND p.code = d.prime_meridian_code
             JOIN unit_of_measure u
               ON u.auth_name = p.uom_auth_name AND u.code = p.uom_code
             WHERE d.auth_name = ?1 AND CAST(d.code AS TEXT) = ?2",
            params![datum_auth, datum_code],
            |row| {
                Ok(PrimeMeridian {
                    name: row.get(0)?,
                    longitude: row.get(1)?,
                    unit: unit_from_row(row, 2)?,
                })
            },
        )
        .map_err(|error| format!("prime meridian of {datum_auth}:{datum_code}: {error}"))
}

fn load_axes(connection: &Connection, auth: &str, code: &str) -> Result<Vec<Axis>, String> {
    let rows = connection
        .prepare(
            "SELECT name, abbrev, orientation, uom_auth_name, CAST(uom_code AS TEXT)
             FROM axis
             WHERE coordinate_system_auth_name = ?1 AND CAST(coordinate_system_code AS TEXT) = ?2
             ORDER BY coordinate_system_order",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![auth, code], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut axes = Vec::new();
    for (name, abbrev, direction, uom_auth, uom_code) in rows {
        let (uom_auth, uom_code) = uom_auth
            .zip(uom_code)
            .ok_or_else(|| format!("axis `{name}` has no unit"))?;
        axes.push(Axis {
            name,
            abbrev,
            direction,
            unit: load_unit(connection, &uom_auth, &uom_code)?,
        });
    }
    Ok(axes)
}

fn load_unit(connection: &Connection, auth: &str, code: &str) -> Result<Unit, String> {
    connection
        .query_row(
            "SELECT code, name, conv_factor, type FROM unit_of_measure
             WHERE auth_name = ?1 AND CAST(code AS TEXT) = ?2",
            params![auth, code],
            |row| unit_from_row(row, 0),
        )
        .map_err(|error| format!("unit {auth}:{code}: {error}"))
}

fn unit_from_row(row: &rusqlite::Row<'_>, index: usize) -> Result<Unit, rusqlite::Error> {
    let code = sql_text(row, index)?;
    let mut name: String = row.get(index + 1)?;
    let mut factor: f64 = row.get(index + 2)?;
    let type_name: String = row.get(index + 3)?;
    if name == "degree (supplier to define representation)" {
        name = "degree".to_owned();
    }
    let degree = std::f64::consts::PI / 180.0;
    if code == "9107" || code == "9108" {
        factor = degree;
    }
    let arcsec = degree / 3600.0;
    if (factor - degree).abs() < 1e-10 * degree {
        factor = degree;
    } else if (factor - arcsec).abs() < 1e-10 * arcsec {
        factor = arcsec;
    }
    let kind = match type_name.as_str() {
        "length" => UnitKind::Length,
        "angle" => UnitKind::Angle,
        "scale" => UnitKind::Scale,
        "time" => UnitKind::Time,
        _ => UnitKind::Other,
    };
    Ok(Unit { name, factor, kind })
}

fn sql_text(row: &rusqlite::Row<'_>, index: usize) -> Result<String, rusqlite::Error> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Text(bytes) => Ok(String::from_utf8_lossy(bytes).into_owned()),
        rusqlite::types::ValueRef::Integer(value) => Ok(value.to_string()),
        rusqlite::types::ValueRef::Real(value) => Ok(format_g(value, 15)),
        rusqlite::types::ValueRef::Null => Err(rusqlite::Error::InvalidColumnType(
            index,
            String::new(),
            rusqlite::types::Type::Text,
        )),
        rusqlite::types::ValueRef::Blob(_) => Err(rusqlite::Error::InvalidColumnType(
            index,
            String::new(),
            rusqlite::types::Type::Text,
        )),
    }
}

fn load_usages(connection: &Connection, code: &str) -> Result<Vec<Usage>, String> {
    let mut rows = connection
        .prepare(
            "SELECT scope.scope, extent.description,
                    extent.south_lat, extent.north_lat, extent.west_lon, extent.east_lon,
                    CASE WHEN scope.scope LIKE '%large scale%' THEN 0 ELSE 1 END,
                    usage.auth_name, CAST(usage.code AS TEXT)
             FROM usage
             JOIN extent
               ON usage.extent_auth_name = extent.auth_name AND usage.extent_code = extent.code
             JOIN scope
               ON usage.scope_auth_name = scope.auth_name AND usage.scope_code = scope.code
             WHERE usage.object_table_name = 'geodetic_crs'
               AND usage.object_auth_name = 'EPSG'
               AND CAST(usage.object_code AS TEXT) = ?1
               AND NOT (usage.extent_auth_name = 'PROJ' AND usage.extent_code = 'EXTENT_UNKNOWN')
               AND NOT (usage.scope_auth_name = 'PROJ' AND usage.scope_code = 'SCOPE_UNKNOWN')",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![code], |row| {
            Ok((
                Usage {
                    scope: row.get(0)?,
                    area: row.get(1)?,
                    south: row.get(2)?,
                    north: row.get(3)?,
                    west: row.get(4)?,
                    east: row.get(5)?,
                },
                row.get::<_, i64>(6)?,
                row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    rows.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    Ok(rows.into_iter().map(|(usage, _, _, _)| usage).collect())
}

fn load_fixed_usage(connection: &Connection, extent: &str, scope: &str) -> Result<Usage, String> {
    connection
        .query_row(
            "SELECT scope.scope, extent.description,
                    extent.south_lat, extent.north_lat, extent.west_lon, extent.east_lon
             FROM extent, scope
             WHERE extent.auth_name = 'EPSG' AND CAST(extent.code AS TEXT) = ?1
               AND scope.auth_name = 'EPSG' AND CAST(scope.code AS TEXT) = ?2",
            params![extent, scope],
            |row| {
                Ok(Usage {
                    scope: row.get(0)?,
                    area: row.get(1)?,
                    south: row.get(2)?,
                    north: row.get(3)?,
                    west: row.get(4)?,
                    east: row.get(5)?,
                })
            },
        )
        .map_err(|error| format!("EPSG:4326 extent: {error}"))
}

fn render(crs: &GeoCrs, options: &Options) -> String {
    let mut blocks = Vec::new();
    if crs.deprecated {
        blocks.push("Warning: object is deprecated".to_owned());
    }
    for dialect in &options.dialects {
        let body = match dialect {
            Dialect::Proj => proj_string(crs),
            Dialect::Wkt2_2019 => wkt_crs(crs),
            Dialect::ProjJson => json_crs(crs),
        };
        if options.quiet {
            blocks.push(body);
        } else {
            let header = match dialect {
                Dialect::Proj => "PROJ.4 string:",
                Dialect::Wkt2_2019 => "WKT2:2019 string:",
                Dialect::ProjJson => "PROJJSON:",
            };
            blocks.push(format!("{header}\n{body}"));
        }
    }
    blocks.join("\n\n")
}

fn proj_string(crs: &GeoCrs) -> String {
    let mut parts = vec!["+proj=longlat".to_owned()];
    match crs.datum.code.as_str() {
        "6326" => parts.push("+datum=WGS84".to_owned()),
        "6267" => parts.push("+datum=NAD27".to_owned()),
        "6269" => parts.push("+datum=NAD83".to_owned()),
        _ => {
            parts.push(ellipsoid_proj(&crs.datum.ellipsoid));
            let longitude = crs.meridian.longitude * crs.meridian.unit.factor;
            if longitude != 0.0 {
                parts.push(format!("+pm={}", format_g(longitude.to_degrees(), 15)));
            }
        }
    }
    parts.push("+no_defs".to_owned());
    parts.push("+type=crs".to_owned());
    parts.join(" ")
}

fn ellipsoid_proj(ellipsoid: &Ellipsoid) -> String {
    let metres = ellipsoid.semi_major * ellipsoid.unit.factor;
    let rf = flattening(ellipsoid);
    let minor = ellipsoid
        .semi_minor
        .map(|axis| axis * ellipsoid.unit.factor);
    if let Some(name) = well_known_ellps(metres, rf, minor) {
        return format!("+ellps={name}");
    }
    if let Some(rf) = rf.filter(|value| *value != 0.0) {
        format!("+a={} +rf={}", format_g(metres, 15), format_g(rf, 15))
    } else if let Some(minor) = minor {
        format!("+a={} +b={}", format_g(metres, 15), format_g(minor, 15))
    } else {
        format!("+R={}", format_g(metres, 15))
    }
}

fn flattening(ellipsoid: &Ellipsoid) -> Option<f64> {
    if let Some(rf) = ellipsoid.inverse_flattening {
        return Some(rf);
    }
    let minor = ellipsoid.semi_minor?;
    let major = ellipsoid.semi_major * ellipsoid.unit.factor;
    let minor = minor * ellipsoid.unit.factor;
    if major == minor {
        Some(0.0)
    } else {
        Some(major / (major - minor))
    }
}

fn well_known_ellps(a: f64, rf: Option<f64>, b: Option<f64>) -> Option<&'static str> {
    const TABLE: &[(&str, f64, f64, bool)] = &[
        ("MERIT", 6_378_137.0, 298.257, true),
        ("SGS85", 6_378_136.0, 298.257, true),
        ("GRS80", 6_378_137.0, 298.257_222_101, true),
        ("IAU76", 6_378_140.0, 298.257, true),
        ("airy", 6_377_563.396, 299.324_964_6, true),
        ("APL4.9", 6_378_137.0, 298.25, true),
        ("NWL9D", 6_378_145.0, 298.25, true),
        ("mod_airy", 6_377_340.189, 6_356_034.446, false),
        ("andrae", 6_377_104.43, 300.0, true),
        ("danish", 6_377_019.256_3, 300.0, true),
        ("aust_SA", 6_378_160.0, 298.25, true),
        ("GRS67", 6_378_160.0, 298.247_167_427, true),
        ("GSK2011", 6_378_136.5, 298.256_415_1, true),
        ("bessel", 6_377_397.155, 299.152_812_8, true),
        ("bess_nam", 6_377_483.865, 299.152_812_8, true),
        ("clrk66", 6_378_206.4, 6_356_583.8, false),
        ("clrk80", 6_378_249.145, 293.4663, true),
        ("clrk80ign", 6_378_249.2, 293.466_021_293_626_9, true),
        ("CPM", 6_375_738.7, 334.29, true),
        ("delmbr", 6_376_428.0, 311.5, true),
        ("engelis", 6_378_136.05, 298.2566, true),
        ("evrst30", 6_377_276.345, 300.8017, true),
        ("evrst48", 6_377_304.063, 300.8017, true),
        ("evrst56", 6_377_301.243, 300.8017, true),
        ("evrst69", 6_377_295.664, 300.8017, true),
        ("evrstSS", 6_377_298.556, 300.8017, true),
        ("fschr60", 6_378_166.0, 298.3, true),
        ("fschr60m", 6_378_155.0, 298.3, true),
        ("fschr68", 6_378_150.0, 298.3, true),
        ("helmert", 6_378_200.0, 298.3, true),
        ("hough", 6_378_270.0, 297.0, true),
        ("intl", 6_378_388.0, 297.0, true),
        ("krass", 6_378_245.0, 298.3, true),
        ("kaula", 6_378_163.0, 298.24, true),
        ("lerch", 6_378_139.0, 298.257, true),
        ("mprts", 6_397_300.0, 191.0, true),
        ("new_intl", 6_378_157.5, 6_356_772.2, false),
        ("plessis", 6_376_523.0, 6_355_863.0, false),
        ("PZ90", 6_378_136.0, 298.257_84, true),
        ("SEasia", 6_378_155.0, 6_356_773.320_5, false),
        ("walbeck", 6_376_896.0, 6_355_834.846_7, false),
        ("WGS60", 6_378_165.0, 298.3, true),
        ("WGS66", 6_378_145.0, 298.25, true),
        ("WGS72", 6_378_135.0, 298.26, true),
        ("WGS84", 6_378_137.0, 298.257_223_563, true),
        ("sphere", 6_370_997.0, 6_370_997.0, false),
    ];
    let b = b.or_else(|| {
        rf.filter(|value| *value != 0.0)
            .map(|value| a * (1.0 - 1.0 / value))
    });
    TABLE
        .iter()
        .find(|(_, major, second, is_rf)| {
            if (a - major).abs() >= 1e-10 * major.abs() {
                return false;
            }
            if *is_rf {
                rf.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            } else {
                b.is_some_and(|value| (value - second).abs() < 1e-10 * second.abs())
            }
        })
        .map(|(name, _, _, _)| *name)
}

fn wkt_crs(crs: &GeoCrs) -> String {
    let mut wkt = Wkt::default();
    wkt.start("GEOGCRS");
    wkt.quoted(&crs.name);
    if let Some(epoch) = crs.datum.frame_epoch {
        wkt.start("DYNAMIC");
        wkt.start("FRAMEEPOCH");
        wkt.token(&format_g(round_epoch(epoch), 15));
        wkt.end();
        wkt.end();
    }
    write_datum(&mut wkt, &crs.datum);
    wkt.start("PRIMEM");
    wkt.quoted(&crs.meridian.name);
    wkt.token(&format_g(crs.meridian.longitude, 15));
    write_unit(&mut wkt, &crs.meridian.unit);
    wkt.end();
    wkt.start("CS");
    wkt.token(&crs.cs_type);
    wkt.token(&crs.dimension.to_string());
    wkt.end();
    wkt.start("");
    for (index, axis) in crs.axes.iter().enumerate() {
        write_axis(&mut wkt, axis, index + 1);
    }
    wkt.end();
    for usage in &crs.usages {
        write_usage(&mut wkt, usage);
    }
    wkt.start("ID");
    wkt.quoted("EPSG");
    wkt.token(&crs.code);
    wkt.end();
    wkt.end();
    wkt.out
}

fn write_datum(wkt: &mut Wkt, datum: &Datum) {
    if datum.ensemble {
        wkt.start("ENSEMBLE");
        wkt.quoted(&datum.name);
        for (name, _code) in &datum.members {
            wkt.start("MEMBER");
            wkt.quoted(name);
            wkt.end();
        }
        write_ellipsoid(wkt, &datum.ellipsoid);
        if let Some(accuracy) = &datum.accuracy {
            wkt.start("ENSEMBLEACCURACY");
            wkt.token(accuracy);
            wkt.end();
        }
        wkt.end();
        return;
    }
    wkt.start("DATUM");
    wkt.quoted(&datum.name);
    write_ellipsoid(wkt, &datum.ellipsoid);
    if let Some(anchor) = &datum.anchor {
        wkt.start("ANCHOR");
        wkt.quoted(anchor);
        wkt.end();
    }
    if let Some(epoch) = datum.anchor_epoch {
        wkt.start("ANCHOREPOCH");
        wkt.token(&format_g(round_epoch(epoch), 15));
        wkt.end();
    }
    wkt.end();
}

fn write_ellipsoid(wkt: &mut Wkt, ellipsoid: &Ellipsoid) {
    wkt.start("ELLIPSOID");
    wkt.quoted(&ellipsoid.name);
    wkt.token(&format_g(ellipsoid.semi_major, 15));
    let rf = flattening(ellipsoid).unwrap_or(0.0);
    wkt.token(&format_g(rf, 15));
    write_unit(wkt, &ellipsoid.unit);
    wkt.end();
}

fn write_axis(wkt: &mut Wkt, axis: &Axis, order: usize) {
    wkt.start("AXIS");
    wkt.quoted(&axis_designation(&axis.name, &axis.abbrev));
    wkt.token(&axis.direction);
    wkt.start("ORDER");
    wkt.token(&order.to_string());
    wkt.end();
    write_unit(wkt, &axis.unit);
    wkt.end();
}

fn axis_designation(name: &str, abbrev: &str) -> String {
    let mut text = String::new();
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        text.extend(first.to_lowercase());
        text.push_str(chars.as_str());
    }
    if !abbrev.is_empty() && name != "Latitude" && name != "Longitude" {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push('(');
        text.push_str(abbrev);
        text.push(')');
    }
    text
}

fn write_usage(wkt: &mut Wkt, usage: &Usage) {
    wkt.start("USAGE");
    wkt.start("SCOPE");
    wkt.quoted(&usage.scope);
    wkt.end();
    if !usage.area.is_empty() {
        wkt.start("AREA");
        wkt.quoted(&usage.area);
        wkt.end();
    }
    if let (Some(south), Some(west), Some(north), Some(east)) =
        (usage.south, usage.west, usage.north, usage.east)
    {
        wkt.start("BBOX");
        wkt.token(&format_g(south, 15));
        wkt.token(&format_g(west, 15));
        wkt.token(&format_g(north, 15));
        wkt.token(&format_g(east, 15));
        wkt.end();
    }
    wkt.end();
}

fn write_unit(wkt: &mut Wkt, unit: &Unit) {
    let keyword = match unit.kind {
        UnitKind::Length => "LENGTHUNIT",
        UnitKind::Angle => "ANGLEUNIT",
        UnitKind::Scale => "SCALEUNIT",
        UnitKind::Time => "TIMEUNIT",
        UnitKind::Other => "UNIT",
    };
    wkt.start(keyword);
    wkt.quoted(&unit.name);
    wkt.token(&format_g(unit.factor, 15));
    wkt.end();
}

#[derive(Default)]
struct Wkt {
    out: String,
    indent: usize,
    has_child: Vec<bool>,
    empty: Vec<bool>,
}

impl Wkt {
    fn start(&mut self, keyword: &str) {
        if let Some(has) = self.has_child.last_mut() {
            if *has {
                self.out.push(',');
            }
            *has = true;
        }
        if self.indent > 0 && !keyword.is_empty() && !self.out.is_empty() {
            self.out.push('\n');
            self.out.push_str(&" ".repeat(self.indent * 4));
        }
        if !keyword.is_empty() {
            self.out.push_str(keyword);
            self.out.push('[');
        }
        self.indent += 1;
        self.has_child.push(false);
        self.empty.push(keyword.is_empty());
    }

    fn end(&mut self) {
        let empty = self.empty.pop().expect("wkt node");
        self.has_child.pop().expect("wkt node");
        self.indent = self.indent.saturating_sub(1);
        if !empty {
            self.out.push(']');
        }
    }

    fn token(&mut self, text: &str) {
        if let Some(has) = self.has_child.last_mut() {
            if *has {
                self.out.push(',');
            }
            *has = true;
        }
        self.out.push_str(text);
    }

    fn quoted(&mut self, text: &str) {
        let mut escaped = String::new();
        for ch in text.chars() {
            if ch == '"' {
                escaped.push('"');
            }
            escaped.push(ch);
        }
        self.token(&format!("\"{escaped}\""));
    }
}

fn json_crs(crs: &GeoCrs) -> String {
    let mut json = Json::default();
    json.start_obj();
    json.key("$schema");
    json.string(PROJJSON_SCHEMA);
    json.key("type");
    json.string("GeographicCRS");
    json.key("name");
    json.string(&crs.name);
    if crs.datum.ensemble {
        json.key("datum_ensemble");
        json_ensemble(&mut json, &crs.datum);
    } else {
        json.key("datum");
        json_datum(&mut json, &crs.datum, &crs.meridian);
    }
    json.key("coordinate_system");
    json.start_obj();
    json.key("subtype");
    json.string(&crs.cs_type);
    json.key("axis");
    json.start_array();
    for axis in &crs.axes {
        json.start_obj();
        json.key("name");
        json.string(&axis.name);
        json.key("abbreviation");
        json.string(&axis.abbrev);
        json.key("direction");
        json.string(&axis.direction);
        json.key("unit");
        json_unit(&mut json, &axis.unit);
        json.end_obj();
    }
    json.end_array();
    json.end_obj();
    match crs.usages.as_slice() {
        [] => {}
        [usage] => json_usage(&mut json, usage),
        many => {
            json.key("usages");
            json.start_array();
            for usage in many {
                json.start_obj();
                json_usage(&mut json, usage);
                json.end_obj();
            }
            json.end_array();
        }
    }
    json_id(&mut json, "EPSG", &crs.code);
    json.end_obj();
    json.out
}

fn json_ensemble(json: &mut Json, datum: &Datum) {
    json.start_obj();
    json.key("name");
    json.string(&datum.name);
    json.key("members");
    json.start_array();
    for (name, code) in &datum.members {
        json.start_obj();
        json.key("name");
        json.string(name);
        json_id(json, "EPSG", code);
        json.end_obj();
    }
    json.end_array();
    json.key("ellipsoid");
    json_ellipsoid(json, &datum.ellipsoid);
    if let Some(accuracy) = &datum.accuracy {
        json.key("accuracy");
        json.string(accuracy);
    }
    json_id(json, "EPSG", &datum.code);
    json.end_obj();
}

fn json_datum(json: &mut Json, datum: &Datum, meridian: &PrimeMeridian) {
    json.start_obj();
    json.key("type");
    json.string(if datum.frame_epoch.is_some() {
        "DynamicGeodeticReferenceFrame"
    } else {
        "GeodeticReferenceFrame"
    });
    json.key("name");
    json.string(&datum.name);
    if let Some(anchor) = &datum.anchor {
        json.key("anchor");
        json.string(anchor);
    }
    if let Some(epoch) = datum.anchor_epoch {
        json.key("anchor_epoch");
        json.number(round_epoch(epoch));
    }
    if let Some(epoch) = datum.frame_epoch {
        json.key("frame_reference_epoch");
        json.number(round_epoch(epoch));
    }
    json.key("ellipsoid");
    json_ellipsoid(json, &datum.ellipsoid);
    if meridian.name != "Greenwich" {
        json.key("prime_meridian");
        json.start_obj();
        json.key("name");
        json.string(&meridian.name);
        json.key("longitude");
        json.number(meridian.longitude);
        json.end_obj();
    }
    json.end_obj();
}

fn json_ellipsoid(json: &mut Json, ellipsoid: &Ellipsoid) {
    json.start_obj();
    json.key("name");
    json.string(&ellipsoid.name);
    let sphere = ellipsoid
        .inverse_flattening
        .is_some_and(|value| value == 0.0)
        || (ellipsoid.inverse_flattening.is_none()
            && ellipsoid
                .semi_minor
                .is_some_and(|minor| minor == ellipsoid.semi_major));
    if sphere {
        json.key("radius");
        json_length(json, ellipsoid.semi_major, &ellipsoid.unit);
    } else {
        json.key("semi_major_axis");
        json_length(json, ellipsoid.semi_major, &ellipsoid.unit);
        if let Some(rf) = ellipsoid.inverse_flattening {
            json.key("inverse_flattening");
            json.number(rf);
        } else if let Some(minor) = ellipsoid.semi_minor {
            json.key("semi_minor_axis");
            json_length(json, minor, &ellipsoid.unit);
        }
    }
    json.end_obj();
}

fn json_length(json: &mut Json, value: f64, unit: &Unit) {
    if unit.name == "metre" {
        json.number(value);
        return;
    }
    json.start_obj();
    json.key("value");
    json.number(value);
    json.key("unit");
    json_unit(json, unit);
    json.end_obj();
}

fn json_unit(json: &mut Json, unit: &Unit) {
    if unit.name == "metre" || unit.name == "degree" {
        json.string(&unit.name);
        return;
    }
    json.start_obj();
    json.key("type");
    json.string(match unit.kind {
        UnitKind::Length => "LinearUnit",
        UnitKind::Angle => "AngularUnit",
        UnitKind::Scale => "ScaleUnit",
        UnitKind::Time => "TimeUnit",
        UnitKind::Other => "Unit",
    });
    json.key("name");
    json.string(&unit.name);
    json.key("conversion_factor");
    json.number(unit.factor);
    json.end_obj();
}

fn json_usage(json: &mut Json, usage: &Usage) {
    if !usage.scope.is_empty() {
        json.key("scope");
        json.string(&usage.scope);
    }
    if !usage.area.is_empty() {
        json.key("area");
        json.string(&usage.area);
    }
    if let (Some(south), Some(west), Some(north), Some(east)) =
        (usage.south, usage.west, usage.north, usage.east)
    {
        json.key("bbox");
        json.start_obj();
        json.key("south_latitude");
        json.number(south);
        json.key("west_longitude");
        json.number(west);
        json.key("north_latitude");
        json.number(north);
        json.key("east_longitude");
        json.number(east);
        json.end_obj();
    }
}

fn json_id(json: &mut Json, authority: &str, code: &str) {
    json.key("id");
    json.start_obj();
    json.key("authority");
    json.string(authority);
    json.key("code");
    if let Ok(number) = code.parse::<i32>() {
        json.raw(&number.to_string());
    } else {
        json.string(code);
    }
    json.end_obj();
}

#[derive(Default)]
struct Json {
    out: String,
    pad: String,
    stack: Vec<bool>,
    wait: bool,
}

impl Json {
    fn emit(&mut self) {
        if self.wait {
            self.wait = false;
            return;
        }
        if let Some(first) = self.stack.last_mut() {
            if !*first {
                self.out.push(',');
            }
            self.out.push('\n');
            self.out.push_str(&self.pad);
            *first = false;
        }
    }

    fn start_obj(&mut self) {
        self.emit();
        self.out.push('{');
        self.pad.push_str("  ");
        self.stack.push(true);
    }

    fn end_obj(&mut self) {
        self.pad.truncate(self.pad.len().saturating_sub(2));
        let first = self.stack.pop().expect("json object");
        if !first {
            self.out.push('\n');
            self.out.push_str(&self.pad);
        }
        self.out.push('}');
    }

    fn start_array(&mut self) {
        self.emit();
        self.out.push('[');
        self.pad.push_str("  ");
        self.stack.push(true);
    }

    fn end_array(&mut self) {
        self.pad.truncate(self.pad.len().saturating_sub(2));
        let first = self.stack.pop().expect("json array");
        if !first {
            self.out.push('\n');
            self.out.push_str(&self.pad);
        }
        self.out.push(']');
    }

    fn key(&mut self, name: &str) {
        self.emit();
        self.out.push('"');
        push_escaped(&mut self.out, name);
        self.out.push_str("\": ");
        self.wait = true;
    }

    fn string(&mut self, value: &str) {
        self.emit();
        self.out.push('"');
        push_escaped(&mut self.out, value);
        self.out.push('"');
    }

    fn number(&mut self, value: f64) {
        self.raw(&format_g(value, 15));
    }

    fn raw(&mut self, value: &str) {
        self.emit();
        self.out.push_str(value);
    }
}

fn push_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if u32::from(other) < 0x20 => {
                let _ = write!(out, "\\u{code:04X}", code = u32::from(other));
            }
            other => out.push(other),
        }
    }
}

fn round_epoch(year: f64) -> f64 {
    if (1000.0 * year - (1000.0 * year).round()).abs() <= 1e-3 {
        (1000.0 * year).round() / 1000.0
    } else {
        year
    }
}

/// `%.15g`, with `0` for zero. PROJ's WKT writer uses the same precision.
fn format_g(value: f64, precision: usize) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    if !value.is_finite() || precision == 0 {
        return value.to_string();
    }
    let negative = value.is_sign_negative();
    let value = value.abs();
    let mut exp = decade(value);
    let digits_pow = i32::try_from(precision.saturating_sub(1)).unwrap_or(14);
    let mut scale = pow10(digits_pow);
    let mut significand = value / pow10(exp);
    let mut rounded = (significand * scale).round();
    if rounded >= scale * 10.0 {
        exp += 1;
        significand = value / pow10(exp);
        rounded = (significand * scale).round();
        if rounded >= scale * 10.0 {
            rounded = (scale * 10.0) - 1.0;
        }
    }
    if rounded < scale {
        // A value just under a power of ten can land in the previous decade.
        scale = pow10(digits_pow);
        while rounded < scale && scale > 1.0 {
            rounded *= 10.0;
            exp -= 1;
            scale /= 10.0;
        }
    }
    let mut digits = format!("{rounded:.0}");
    if digits.len() > precision {
        digits.truncate(precision);
    }
    while digits.len() < precision {
        digits.push('0');
    }
    let body = place_digits(&digits, exp, precision);
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

fn decade(value: f64) -> i32 {
    let mut exp = 0i32;
    let mut scaled = value;
    if scaled >= 10.0 {
        while scaled >= 10.0 && exp < 400 {
            scaled /= 10.0;
            exp += 1;
        }
    } else {
        while scaled < 1.0 && exp > -400 {
            scaled *= 10.0;
            exp -= 1;
        }
    }
    exp
}

fn pow10(exp: i32) -> f64 {
    10.0_f64.powi(exp)
}

fn place_digits(digits: &str, exp: i32, precision: usize) -> String {
    let precision_i = i32::try_from(precision).unwrap_or(15);
    if exp < -4 || exp >= precision_i {
        let mut scientific = String::new();
        scientific.push_str(&digits[..1]);
        let rest = digits[1..].trim_end_matches('0');
        if !rest.is_empty() {
            scientific.push('.');
            scientific.push_str(rest);
        }
        let sign = if exp >= 0 { "+" } else { "-" };
        let exponent = exp.unsigned_abs();
        scientific.push('e');
        scientific.push_str(sign);
        if exponent < 10 {
            scientific.push('0');
        }
        scientific.push_str(&exponent.to_string());
        return scientific;
    }
    let mut text = String::new();
    if exp >= 0 {
        let split = usize::try_from(exp).unwrap_or(0) + 1;
        let split = split.min(digits.len());
        text.push_str(&digits[..split]);
        let rest = digits[split..].trim_end_matches('0');
        if !rest.is_empty() {
            text.push('.');
            text.push_str(rest);
        }
    } else {
        text.push_str("0.");
        let zeros = usize::try_from(-exp - 1).unwrap_or(0);
        text.extend(std::iter::repeat_n('0', zeros));
        text.push_str(digits.trim_end_matches('0'));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{classify_dialect, format_g, load_geographic, open_db, Dialect};

    #[test]
    fn format_g_matches_proj_wkt_numbers() {
        assert_eq!(format_g(0.0, 15), "0");
        assert_eq!(format_g(1.0, 15), "1");
        assert_eq!(format_g(-90.0, 15), "-90");
        assert_eq!(format_g(-180.0, 15), "-180");
        assert_eq!(format_g(6_378_137.0, 15), "6378137");
        assert_eq!(format_g(298.257_223_563, 15), "298.257223563");
        assert_eq!(
            format_g(std::f64::consts::PI / 180.0, 15),
            "0.0174532925199433"
        );
    }

    #[test]
    fn unsupported_dialect_is_an_error() {
        let error = classify_dialect("WKT1:GDAL").expect_err("dialect");
        assert!(error.contains("cannot write dialect"), "{error}");
        assert!(classify_dialect("SQL").is_err());
        assert!(classify_dialect("all").is_err());
        assert_eq!(
            classify_dialect("WKT2:2019").expect("wkt"),
            Some(Dialect::Wkt2_2019)
        );
    }

    #[test]
    fn epsg_4326_matches_the_projinfo_sample() {
        let connection = open_db().expect("proj.db");
        let crs = load_geographic(&connection, "4326").expect("4326");
        let proj = super::proj_string(&crs);
        assert_eq!(proj, "+proj=longlat +datum=WGS84 +no_defs +type=crs");
        let wkt = super::wkt_crs(&crs);
        assert!(
            wkt.contains("GEOGCRS[\"WGS 84\",")
                && wkt.contains("ENSEMBLEACCURACY[2.0]]")
                && wkt.contains("ID[\"EPSG\",4326]]"),
            "{wkt}"
        );
        assert!(wkt.contains("AREA[\"World.\"]"), "{wkt}");
        let json = super::json_crs(&crs);
        assert!(json.contains("\"type\": \"GeographicCRS\""), "{json}");
        assert!(json.contains("\"accuracy\": \"2.0\""), "{json}");
        assert!(json.contains("\"code\": 4326"), "{json}");
        assert!(
            json.starts_with(
                "{\n  \"$schema\": \"https://proj.org/schemas/v0.7/projjson.schema.json\""
            ),
            "{json}"
        );
    }
}
