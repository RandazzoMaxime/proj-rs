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

use std::process::ExitCode;

const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";

fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(Report::Usage(message)) => {
            eprintln!("{message}");
            eprintln!("{}", usage());
            ExitCode::from(1)
        }
        Err(Report::Plain(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

enum Report {
    Usage(String),
    Plain(String),
}

fn run(args: &[String]) -> Result<String, Report> {
    if args.len() >= 2 && args[0] == "completion" {
        return complete_shell(&args[2..])
            .map(|text| text.trim_end_matches('\n').to_owned())
            .map_err(Report::Plain);
    }
    let mut options = Options::parse(args).map_err(Report::Usage)?;
    if options.help {
        return Ok(usage().to_owned());
    }
    if options.version {
        return Ok(format!("projinfo: {RELEASE}"));
    }
    if options.searchpaths {
        return Ok(searchpaths().join("\n"));
    }
    if options.remote_data {
        return remote_data().map_err(Report::Plain);
    }
    if options.area.is_some() && options.bbox.is_some() {
        return Err(Report::Plain(
            "ERROR: --bbox and --area are exclusive".to_owned(),
        ));
    }
    if options.dump_db_structure && options.positional == 1 && !options.format_given {
        options.dialects = vec![Dialect::Sql];
        options.quiet = true;
    }
    if options.dialects.contains(&Dialect::Sql) && options.output_id.is_none() {
        if !options.all {
            return Err(Report::Plain(
                "ERROR: --output-id AUTH:CODE must be specified when SQL output is enabled."
                    .to_owned(),
            ));
        }
        eprintln!(
            "WARNING: SQL output disable since --output-id AUTH:CODE has not been specified."
        );
        options.dialects.retain(|dialect| *dialect != Dialect::Sql);
    }
    let mut lines = Vec::new();
    if options.dump_db_structure {
        let db = database().map_err(Report::Plain)?;
        lines.extend(
            db.database_structure()
                .map_err(|error| Report::Plain(error.to_string()))?,
        );
    }
    if let Some(filter) = options.list_crs.as_deref() {
        let db = database().map_err(Report::Plain)?;
        lines.extend(list_crs(&options, filter, &db)?);
    }
    if options.source.is_some() && options.target.is_some() {
        lines.push(candidates(&options).map_err(Report::Plain)?);
    } else if let Some(code) = options.code.as_deref() {
        if options.quiet && options.dialects.len() != 1 {
            return Err(Report::Usage(
                "-q can only be used with a single output format".to_owned(),
            ));
        }
        lines.push(describe(&options, code).map_err(Report::Plain)?);
    } else if !(options.dump_db_structure || options.list_crs.is_some()) {
        return Err(Report::Usage("missing EPSG code".to_owned()));
    }
    lines.retain(|line| !line.is_empty());
    Ok(lines.join("\n"))
}

fn database() -> Result<proj_rs::DatabaseContext, String> {
    proj_rs::DatabaseContext::create().map_err(|error| error.to_string())
}

/// The grid search path, one directory per line.
fn searchpaths() -> Vec<String> {
    let Ok(proj) = proj_rs::Proj::new("+proj=longlat") else {
        return Vec::new();
    };
    proj.lib_info()
        .map(|info| {
            info.searchpath
                .split(if cfg!(windows) { ';' } else { ':' })
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn remote_data() -> Result<String, String> {
    let proj = proj_rs::Proj::new("+proj=longlat").map_err(|error| error.to_string())?;
    if proj.network_enabled() {
        let endpoint = proj.get_url_endpoint().map_err(|error| error.to_string())?;
        Ok(format!("Status: enabled\nURL: {endpoint}"))
    } else {
        Ok(
            "Status: disabled\nReason: not enabled in proj.ini or PROJ_NETWORK=ON not specified"
                .to_owned(),
        )
    }
}

fn usage() -> &'static str {
    "usage: projinfo [-o formats] [-q] [--version] EPSG:code\n\
     -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2019).\n\
     Other dialects (WKT1, WKT2:2015, SQL, all) are an error."
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dialect {
    Proj,
    Wkt2_2015,
    Wkt2_2019,
    Wkt1Gdal,
    Wkt1Esri,
    ProjJson,
    Sql,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one independent projinfo switch"
)]
struct Options {
    help: bool,
    version: bool,
    quiet: bool,
    single_line: bool,
    summary: bool,
    searchpaths: bool,
    remote_data: bool,
    dump_db_structure: bool,
    format_given: bool,
    dialects: Vec<Dialect>,
    code: Option<String>,
    source: Option<String>,
    target: Option<String>,
    positional: usize,
    kind: Option<ObjectKind>,
    list_crs: Option<String>,
    authorities: Vec<String>,
    area: Option<String>,
    bbox: Option<String>,
    spatial_test: Option<SpatialTest>,
    crs_extent_use: Option<proj_rs::CrsExtentUse>,
    accuracy: Option<f64>,
    output_id: Option<(String, String)>,
    all: bool,
}

/// The `-k` value: which kind of object a name or code describes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectKind {
    Crs,
    Operation,
    Ellipsoid,
    Datum,
    Ensemble,
}

/// The `--spatial-test` value. `contains` is PROJ's default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SpatialTest {
    #[default]
    Contains,
    Intersects,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut help = false;
        let mut version = false;
        let mut quiet = false;
        let mut proj = false;
        let mut wkt2015 = false;
        let mut wkt = false;
        let mut wkt1 = false;
        let mut wkt1_esri = false;
        let mut json = false;
        let mut saw_format = false;
        let mut code = None;
        let mut source = None;
        let mut target = None;
        let mut single_line = false;
        let mut summary = false;
        let mut searchpaths = false;
        let mut remote_data = false;
        let mut dump_db_structure = false;
        let mut kind = None;
        let mut list_crs = None;
        let mut authorities = Vec::new();
        let mut area = None;
        let mut bbox = None;
        let mut spatial_test = None;
        let mut crs_extent_use = None;
        let mut accuracy = None;
        let mut output_id = None;
        let mut all = false;
        let mut sql = false;
        let mut positional = 0;
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            if arg == "-h" || arg == "--help" || arg == "-?" {
                help = true;
            } else if arg == "--version" {
                version = true;
            } else if arg == "-q" || arg == "--quiet" {
                quiet = true;
            } else if arg == "--single-line" {
                single_line = true;
            } else if arg == "--summary" {
                summary = true;
            } else if matches!(
                arg,
                "--grid-check"
                    | "--pivot-crs"
                    | "--main-db-path"
                    | "--aux-db-path"
                    | "--s_epoch"
                    | "--t_epoch"
            ) {
                index += 1;
            } else if matches!(
                arg,
                "--3d"
                    | "--hide-ballpark"
                    | "--show-superseded"
                    | "--normalize-axis-order"
                    | "--boundcrs-to-wgs84"
                    | "--allow-ellipsoidal-height-as-vertical-crs"
                    | "--identify"
                    | "--c-ify"
            ) {
            } else if arg == "--crs-extent-use" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                crs_extent_use = Some(match value.to_ascii_lowercase().as_str() {
                    "none" => proj_rs::CrsExtentUse::Ignore,
                    "both" => proj_rs::CrsExtentUse::Both,
                    "intersection" => proj_rs::CrsExtentUse::Intersection,
                    "smallest" => proj_rs::CrsExtentUse::Smallest,
                    _ => {
                        return Err(format!(
                            "Unrecognized value for option --crs-extent-use: {value}"
                        ))
                    }
                });
            } else if arg == "--accuracy" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                accuracy = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid value for option --accuracy: {value}"))?,
                );
            } else if arg == "--output-id" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                let tokens: Vec<&str> = value.split(':').collect();
                let [authority, code] = tokens.as_slice() else {
                    return Err("Invalid value for option --output-id".to_owned());
                };
                output_id = Some(((*authority).to_owned(), (*code).to_owned()));
            } else if arg == "--searchpaths" {
                searchpaths = true;
            } else if arg == "--remote-data" {
                remote_data = true;
            } else if arg == "--dump-db-structure" {
                dump_db_structure = true;
            } else if arg == "--list-crs" {
                list_crs = Some(match args.get(index + 1) {
                    Some(next) if !next.starts_with('-') => {
                        index += 1;
                        next.clone()
                    }
                    _ => String::new(),
                });
            } else if arg == "-k" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for -k".to_owned())?;
                kind = Some(classify_kind(value)?);
            } else if arg == "--area" {
                index += 1;
                area = Some(
                    args.get(index)
                        .ok_or_else(|| format!("missing value for {arg}"))?
                        .clone(),
                );
            } else if arg == "--bbox" {
                index += 1;
                bbox = Some(
                    args.get(index)
                        .ok_or_else(|| format!("missing value for {arg}"))?
                        .clone(),
                );
            } else if arg == "--authority" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                authorities = value.split(',').map(str::to_owned).collect();
            } else if arg == "--spatial-test" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                spatial_test = Some(match value.to_ascii_lowercase().as_str() {
                    "contains" => SpatialTest::Contains,
                    "intersects" => SpatialTest::Intersects,
                    _ => {
                        return Err(format!(
                            "Unrecognized value for option --spatial-test: {value}"
                        ))
                    }
                });
            } else if arg == "-s" || arg == "--source-crs" {
                index += 1;
                source = Some(
                    args.get(index)
                        .ok_or_else(|| format!("missing value for {arg}"))?
                        .clone(),
                );
            } else if arg == "-t" || arg == "--target-crs" {
                index += 1;
                target = Some(
                    args.get(index)
                        .ok_or_else(|| format!("missing value for {arg}"))?
                        .clone(),
                );
            } else if arg == "-o" {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for -o".to_owned())?;
                saw_format = true;
                for piece in value.split(',') {
                    if piece.eq_ignore_ascii_case("all") {
                        all = true;
                        sql = true;
                        proj = true;
                        wkt2015 = true;
                        wkt = true;
                        wkt1 = true;
                        wkt1_esri = true;
                        json = true;
                        continue;
                    }
                    match classify_dialect(piece)? {
                        Some(Dialect::Proj) => proj = true,
                        Some(Dialect::Wkt2_2015) => wkt2015 = true,
                        Some(Dialect::Wkt2_2019) => wkt = true,
                        Some(Dialect::Wkt1Gdal) => wkt1 = true,
                        Some(Dialect::Wkt1Esri) => wkt1_esri = true,
                        Some(Dialect::ProjJson) => json = true,
                        Some(Dialect::Sql) => sql = true,
                        None => {
                            proj = true;
                            wkt = true;
                        }
                    }
                }
            } else if arg.starts_with('-') {
                return Err(format!("unrecognized option {arg}"));
            } else {
                positional += 1;
                if source.is_none() {
                    source = Some(arg.to_owned());
                } else if target.is_none() {
                    target = Some(arg.to_owned());
                } else {
                    return Err(format!("Too many parameters: {arg}"));
                }
                if code.is_none() {
                    code = Some(arg.to_owned());
                }
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
        if wkt2015 {
            dialects.push(Dialect::Wkt2_2015);
        }
        if wkt {
            dialects.push(Dialect::Wkt2_2019);
        }
        if wkt1 {
            dialects.push(Dialect::Wkt1Gdal);
        }
        if wkt1_esri {
            dialects.push(Dialect::Wkt1Esri);
        }
        if json {
            dialects.push(Dialect::ProjJson);
        }
        if sql {
            dialects.push(Dialect::Sql);
        }
        if saw_format && dialects.is_empty() && !help && !version {
            return Err("no output format remains enabled".to_owned());
        }
        Ok(Self {
            help,
            version,
            quiet,
            single_line,
            summary,
            searchpaths,
            remote_data,
            dump_db_structure,
            format_given: saw_format,
            dialects,
            code,
            source,
            target,
            positional,
            kind,
            list_crs,
            authorities,
            area,
            bbox,
            spatial_test,
            crs_extent_use,
            accuracy,
            output_id,
            all,
        })
    }
}

fn classify_kind(text: &str) -> Result<ObjectKind, String> {
    Ok(match text.to_ascii_lowercase().as_str() {
        "crs" | "srs" => ObjectKind::Crs,
        "operation" => ObjectKind::Operation,
        "ellipsoid" => ObjectKind::Ellipsoid,
        "datum" => ObjectKind::Datum,
        "ensemble" => ObjectKind::Ensemble,
        _ => return Err(format!("Unrecognized value for option -k: {text}")),
    })
}

/// `None` is `default`, which turns on PROJ and WKT2:2019.
fn classify_dialect(text: &str) -> Result<Option<Dialect>, String> {
    let folded = text.trim().to_ascii_lowercase();
    const UNSUPPORTED: &[&str] = &[
        "wkt_all",
        "wkt-all",
        "wkt2_2015_simplified",
        "wkt2-2015_simplified",
        "wkt2:2015_simplified",
        "wkt2_2019_simplified",
        "wkt2-2019_simplified",
        "wkt2:2019_simplified",
        "wkt2_2018_simplified",
        "wkt2-2018_simplified",
        "wkt2:2018_simplified",
    ];
    if UNSUPPORTED.contains(&folded.as_str()) {
        return Err(format!("cannot write dialect {text}"));
    }
    Ok(match folded.as_str() {
        "default" => None,
        "all" => None,
        "proj" => Some(Dialect::Proj),
        "wkt2_2015" | "wkt2-2015" | "wkt2:2015" => Some(Dialect::Wkt2_2015),
        "wkt2_2019" | "wkt2-2019" | "wkt2:2019" | "wkt2_2018" | "wkt2-2018" | "wkt2:2018" => {
            Some(Dialect::Wkt2_2019)
        }
        "wkt1_gdal" | "wkt1-gdal" | "wkt1:gdal" => Some(Dialect::Wkt1Gdal),
        "wkt1_esri" | "wkt1-esri" | "wkt1:esri" => Some(Dialect::Wkt1Esri),
        "projjson" => Some(Dialect::ProjJson),
        "sql" => Some(Dialect::Sql),
        _ => return Err(format!("unrecognized value for option -o: {text}")),
    })
}

/// Shell completion, as `suggestCompletion` in `projinfo_lib.cpp`. `args`
/// are the words after `projinfo completion projinfo`.
fn complete_shell(args: &[String]) -> Result<String, String> {
    const KNOWN_OPTIONS: &[&str] = &[
        "-o",
        "-k",
        "--summary",
        "-q",
        "--area",
        "--bbox",
        "--spatial-test",
        "--crs-extent-use",
        "--grid-check",
        "--pivot-crs",
        "--show-superseded",
        "--hide-ballpark",
        "--accuracy",
        "--allow-ellipsoidal-height-as-vertical-crs",
        "--boundcrs-to-wgs84",
        "--authority",
        "--main-db-path",
        "--aux-db-path",
        "--identify",
        "--3d",
        "--output-id",
        "--c-ify",
        "--single-line",
        "--searchpaths",
        "--remote-data",
        "--list-crs",
        "--dump-db-structure",
        "-s",
        "--s_epoch",
        "-t",
        "--t_epoch",
    ];
    if args.iter().any(String::is_empty) {
        return Ok(String::new());
    }
    let db = database()?;
    let authorities = || db.authorities().unwrap_or_default();
    let line = |words: &[String]| format!("{}\n", words.join(" "));
    let Some(last) = args.last() else {
        let words: Vec<String> = authorities()
            .into_iter()
            .map(|authority| format!("{authority}:"))
            .collect();
        return Ok(line(&words));
    };
    let option_is =
        |option: &str| last == option || (args.len() >= 2 && args[args.len() - 2] == option);
    let mut words: Vec<String> = Vec::new();
    if args.len() == 1 && !last.starts_with('-') && !last.contains(':') {
        words.extend(
            authorities()
                .into_iter()
                .filter(|authority| authority.starts_with(last.as_str()))
                .map(|authority| format!("{authority}:")),
        );
    }
    if option_is("-k") {
        return Ok("crs operation datum ensemble ellipsoid\n".to_owned());
    }
    if option_is("-o") {
        let text = if last.starts_with("WKT1:") {
            "GDAL ESRI"
        } else if last.starts_with("WKT2:") {
            "2019 2015"
        } else {
            "all PROJ WKT2:2019 WKT2:2015 WKT1:GDAL WKT1:ESRI PROJJSON SQL"
        };
        return Ok(format!("{text}\n"));
    }
    if option_is("--spatial-test") {
        return Ok("contains intersects\n".to_owned());
    }
    if option_is("--crs-extent-use") {
        return Ok("none both intersection smallest\n".to_owned());
    }
    if option_is("--grid-check") {
        return Ok("none discard_missing sort known_available\n".to_owned());
    }
    if option_is("--pivot-crs") {
        if last.ends_with(':') {
            return Ok(String::new());
        }
        let mut pivots = vec![
            "always".to_owned(),
            "if_no_direct_transformation".to_owned(),
            "never".to_owned(),
        ];
        pivots.extend(
            authorities()
                .into_iter()
                .map(|authority| format!("{authority}:")),
        );
        return Ok(line(&pivots));
    }
    if last.starts_with('-') {
        if KNOWN_OPTIONS.contains(&last.as_str()) {
            return Ok(String::new());
        }
        let options: Vec<String> = KNOWN_OPTIONS
            .iter()
            .map(|option| (*option).to_owned())
            .collect();
        return Ok(line(&options));
    }
    if let Some(colon) = last.find(':') {
        let authority = &last[..colon];
        let code_start = &last[colon + 1..];
        let mut suggestions = Vec::new();
        let mut code = String::new();
        for info in crs_infos(&db, authority).unwrap_or_default() {
            if !info.deprecated && (code_start.is_empty() || info.code.starts_with(code_start)) {
                if suggestions.is_empty() {
                    code.clone_from(&info.code);
                }
                suggestions.push(format!("{} -- {}", info.code, info.name));
            }
        }
        if suggestions.len() == 1 {
            suggestions = vec![code];
        }
        let escaped: Vec<String> = suggestions.iter().map(|text| escape_space(text)).collect();
        return Ok(line(&escaped));
    }
    let last_arg = completion_phrase(args);
    for authority in ["EPSG", ""] {
        for info in crs_infos(&db, authority).unwrap_or_default() {
            if info.deprecated || !info.name.starts_with(last_arg.as_str()) {
                continue;
            }
            let mut name = info.name.clone();
            if last == "+" || last == "/" {
                if let Some(pos) = name.find(last.as_str()) {
                    if name.as_bytes().get(pos + 1) == Some(&b' ') {
                        name = name[pos + 2..].to_owned();
                    }
                }
            }
            words.push(escape_space(&name));
        }
        if !words.is_empty() {
            return Ok(line(&words));
        }
    }
    if let Some(pos) = last_arg.find(" +") {
        let horizontal = last_arg[..pos].to_owned();
        return Ok(vertical_suggestions(&db, &horizontal).unwrap_or_default());
    }
    Ok(String::new())
}

/// The name prefix to complete: the last word, or the quoted phrase that
/// ends the command line (`"NAD83 / California Albers +`).
fn completion_phrase(args: &[String]) -> String {
    let mut phrase = args.last().cloned().unwrap_or_default();
    for index in (0..args.len()).rev() {
        let word = &args[index];
        if word.len() >= 2 && word.ends_with('"') {
            break;
        }
        if word.len() >= 2 && word.starts_with('"') {
            phrase = word[1..].to_owned();
            for next in &args[index + 1..] {
                phrase.push(' ');
                phrase.push_str(next);
            }
            break;
        }
    }
    phrase
}

/// Vertical CRS names that share the area of use of a horizontal CRS.
fn vertical_suggestions(db: &proj_rs::DatabaseContext, horizontal: &str) -> Result<String, String> {
    let candidates = proj_rs::AuthorityFactory::create(db, "")
        .create_objects_from_name(
            horizontal,
            &[
                proj_rs::ObjectType::Geographic2dCrs,
                proj_rs::ObjectType::ProjectedCrs,
                proj_rs::ObjectType::EngineeringCrs,
            ],
            proj_rs::NameMatch::Approximate,
            2,
        )
        .map_err(|error| error.to_string())?;
    let Some(proj_rs::AuthorityObject::Crs(crs)) = candidates.first() else {
        return Ok(String::new());
    };
    let [domain] = crs.domains() else {
        return Ok(String::new());
    };
    if domain.bounding_box().is_none() {
        return Ok(String::new());
    }
    let vertical_authority = crs
        .identifiers()
        .first()
        .map(|identifier| identifier.code_space().to_owned())
        .unwrap_or_default();
    let horizontal_area = domain.description().map(cut_at_dash).unwrap_or_default();
    let infos = crs_infos(db, &vertical_authority).unwrap_or_default();
    let mut words = Vec::new();
    for attempt in 0..2 {
        if !words.is_empty() {
            break;
        }
        for info in &infos {
            if info.deprecated
                || info.west_lon.is_none()
                || info.type_name != "vertical"
                || info.name.starts_with("EPSG example")
            {
                continue;
            }
            let area = cut_at_dash(&info.area_name);
            let matches = match attempt {
                0 if !horizontal_area.is_empty() => area == horizontal_area,
                1 => area == "World.",
                _ => false,
            };
            if matches {
                words.push(escape_space(&info.name));
            }
        }
    }
    if words.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{}\n", words.join(" ")))
}

/// PROJ's area text is cut before the first ` -`: `Spain - onshore` is `Spain`.
fn cut_at_dash(text: &str) -> String {
    match text.find(" -") {
        Some(pos) => text[..pos].to_owned(),
        None => text.to_owned(),
    }
}

fn escape_space(text: &str) -> String {
    text.replace(' ', "\\ ")
}

/// The CRS list of `authority`. An empty name lists every authority.
fn crs_infos(
    db: &proj_rs::DatabaseContext,
    authority: &str,
) -> Result<Vec<proj_rs::CrsInfo>, String> {
    if !authority.is_empty() {
        return proj_rs::AuthorityFactory::create(db, authority)
            .get_crs_info_list()
            .map_err(|error| error.to_string());
    }
    let mut all = Vec::new();
    for name in db.authorities().map_err(|error| error.to_string())? {
        all.extend(
            proj_rs::AuthorityFactory::create(db, name)
                .get_crs_info_list()
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(all)
}

/// `--list-crs`, as `projinfo_lib.cpp` lists it. `filter` is the comma
/// separated type list; the bbox or area narrows the CRSs by extent.
fn list_crs(
    options: &Options,
    filter: &str,
    db: &proj_rs::DatabaseContext,
) -> Result<Vec<String>, Report> {
    let mut allow_deprecated = false;
    let mut types: Vec<&str> = Vec::new();
    for token in filter.split(',').filter(|token| !token.is_empty()) {
        let found: &[&str] = match token.to_ascii_lowercase().as_str() {
            "allow_deprecated" => {
                allow_deprecated = true;
                &[]
            }
            "geodetic" => &["geographic 2D", "geographic 3D", "geocentric"],
            "geocentric" => &["geocentric"],
            "geographic" => &["geographic 2D", "geographic 3D"],
            "geographic_2d" => &["geographic 2D"],
            "geographic_3d" => &["geographic 3D"],
            "vertical" => &["vertical"],
            "projected" => &["projected"],
            "compound" => &["compound"],
            "engineering" => &["engineering"],
            _ => {
                return Err(Report::Usage(format!(
                    "Unrecognized value for option --list-crs: {token}"
                )))
            }
        };
        types.extend_from_slice(found);
    }
    let scope = scope(db, options)?;
    let authorities: Vec<String> = if options.authorities.is_empty() {
        vec![String::new()]
    } else {
        options.authorities.clone()
    };
    let mut lines = Vec::new();
    for authority in authorities {
        for info in crs_infos(db, &authority).map_err(Report::Plain)? {
            if info.deprecated && !allow_deprecated {
                continue;
            }
            if !types.is_empty() && !types.contains(&info.type_name.as_str()) {
                continue;
            }
            if !scope.admits(&info, options.spatial_test.unwrap_or_default()) {
                continue;
            }
            let mut line = format!("{}:{} \"{}\"", info.authority, info.code, info.name);
            if info.deprecated {
                line.push_str(" [deprecated]");
            }
            lines.push(line);
        }
    }
    Ok(lines)
}

/// The extent that `--bbox` or `--area` names, as `makeBboxFilter`.
enum Scope {
    /// No bbox and no area.
    Anywhere,
    /// An extent. A CRS without a bbox is dropped; an extent without a box
    /// keeps every CRS that has one.
    Extent(Option<proj_rs::BoundingBox>),
    /// Several areas matched the name: a CRS is kept when its area of use
    /// contains the text, ignoring case.
    AreaName(String),
}

impl Scope {
    fn admits(&self, info: &proj_rs::CrsInfo, spatial_test: SpatialTest) -> bool {
        match self {
            Self::Anywhere => true,
            Self::Extent(filter) => {
                let Some(crs_box) = crs_box(info) else {
                    return false;
                };
                match (filter, spatial_test) {
                    (None, _) => true,
                    (Some(extent), SpatialTest::Contains) => extent.contains(&crs_box),
                    (Some(extent), SpatialTest::Intersects) => extent.intersects(&crs_box),
                }
            }
            Self::AreaName(name) => info
                .area_name
                .to_ascii_lowercase()
                .contains(&name.to_ascii_lowercase()),
        }
    }
}

fn scope(db: &proj_rs::DatabaseContext, options: &Options) -> Result<Scope, Report> {
    if let Some(bbox) = &options.bbox {
        return Ok(Scope::Extent(Some(
            parse_bbox(bbox).map_err(Report::Usage)?,
        )));
    }
    let Some(area) = &options.area else {
        return Ok(Scope::Anywhere);
    };
    let plain = |error: proj_rs::FactoryError| Report::Plain(error.to_string());
    let parts: Vec<&str> = area.split(':').collect();
    if !area.contains(' ') && parts.len() == 2 {
        let domain = proj_rs::AuthorityFactory::create(db, parts[0])
            .create_extent(parts[1])
            .map_err(plain)?;
        return Ok(Scope::Extent(domain.bounding_box()));
    }
    let factory = proj_rs::AuthorityFactory::create(db, "");
    let mut matches = factory
        .list_area_of_use_from_name(area, proj_rs::NameMatch::Exact)
        .map_err(plain)?;
    if matches.len() != 1 {
        matches = factory
            .list_area_of_use_from_name(area, proj_rs::NameMatch::Approximate)
            .map_err(plain)?;
        if matches.is_empty() {
            return Err(Report::Plain(
                "No area of use matching provided name".to_owned(),
            ));
        }
    }
    if let [(authority, code)] = matches.as_slice() {
        let domain = proj_rs::AuthorityFactory::create(db, authority.as_str())
            .create_extent(code)
            .map_err(plain)?;
        return Ok(Scope::Extent(domain.bounding_box()));
    }
    Ok(Scope::AreaName(area.clone()))
}

/// The bounding box of a CRS from `--list-crs`, when it has one.
fn crs_box(info: &proj_rs::CrsInfo) -> Option<proj_rs::BoundingBox> {
    Some(proj_rs::BoundingBox::new(
        info.west_lon?,
        info.south_lat?,
        info.east_lon?,
        info.north_lat?,
    ))
}

fn parse_bbox(text: &str) -> Result<proj_rs::BoundingBox, String> {
    if text.split(',').count() != 4 {
        return Err(format!(
            "Incorrect number of values for option --bbox: {text}"
        ));
    }
    let values: Vec<f64> = text
        .split(',')
        .map(|value| value.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("Invalid value for option --bbox: {text}"))?;
    let [west, south, east, north] = values.as_slice() else {
        return Err(format!("Invalid value for option --bbox: {text}"));
    };
    Ok(proj_rs::BoundingBox::new(*west, *south, *east, *north))
}

fn candidates(options: &Options) -> Result<String, String> {
    let db = proj_rs::DatabaseContext::create().map_err(|error| error.to_string())?;
    let source_text = options.source.as_deref().unwrap_or("");
    let target_text = options.target.as_deref().unwrap_or("");
    let source = load_named(&db, source_text)?;
    let target = load_named(&db, target_text)?;
    let area = candidate_area(&db, options)?;
    let spatial_test = options.spatial_test.unwrap_or_default();
    let operations =
        candidate_operations(&db, &source, &target, options, area.as_ref(), spatial_test)?;
    let mut note = String::new();
    if options.spatial_test.is_none() {
        if let Ok(partial) = candidate_operations(
            &db,
            &source,
            &target,
            options,
            area.as_ref(),
            SpatialTest::Intersects,
        ) {
            if partial.len() > operations.len() {
                note = format!(
                    "Note: using '--spatial-test intersects' would bring more results ({})\n",
                    partial.len()
                );
            } else if let ([only], [first]) = (partial.as_slice(), operations.as_slice()) {
                if only.name() != first.name() {
                    note = "Note: using '--spatial-test intersects' would bring more relevant results.\n"
                        .to_owned();
                }
            }
        }
    }
    if options.quiet && !operations.is_empty() {
        return Ok(operation_bodies(&source, &operations[0], options));
    }
    let mut out = format!("Candidate operations found: {}\n{note}", operations.len());
    for (index, operation) in operations.iter().enumerate() {
        if !options.summary {
            if index > 0 {
                out.push('\n');
            }
            out.push_str("-------------------------------------\n");
            out.push_str(&format!("Operation No. {}:\n\n", index + 1));
        }
        out.push_str(&operation_heading(operation));
        out.push('\n');
        if options.summary {
            continue;
        }
        if options.dialects.contains(&Dialect::Proj) {
            if let Some(proj) = operation_proj(&source, operation, proj_lines(options)) {
                out.push_str("\nPROJ string:\n");
                out.push_str(&proj);
                out.push('\n');
            }
        }
        if options.dialects.contains(&Dialect::Wkt2_2019) {
            if let Ok(wkt) = operation_wkt(operation, wkt_lines(options)) {
                out.push_str("\nWKT2:2019 string:\n");
                out.push_str(&wkt);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

/// The area of interest of the candidates, as `makeBboxFilter` resolves it with several matches an error.
fn candidate_area(
    db: &proj_rs::DatabaseContext,
    options: &Options,
) -> Result<Option<proj_rs::ObjectDomain>, String> {
    if let Some(bbox) = &options.bbox {
        let bbox = parse_bbox(bbox)?;
        return Ok(Some(proj_rs::ObjectDomain::from_bounding_box(bbox)));
    }
    let Some(area) = &options.area else {
        return Ok(None);
    };
    let retrieval = |error: proj_rs::FactoryError| match error {
        proj_rs::FactoryError::NotFound { object, .. } => {
            format!("Area of use retrieval failed: {object} not found")
        }
        other => format!("Area of use retrieval failed: {other}"),
    };
    let parts: Vec<&str> = area.split(':').collect();
    if !area.contains(' ') && parts.len() == 2 {
        let domain = proj_rs::AuthorityFactory::create(db, parts[0])
            .create_extent(parts[1])
            .map_err(retrieval)?;
        return Ok(Some(domain));
    }
    let factory = proj_rs::AuthorityFactory::create(db, "");
    let mut matches = factory
        .list_area_of_use_from_name(area, proj_rs::NameMatch::Exact)
        .map_err(retrieval)?;
    if matches.len() != 1 {
        matches = factory
            .list_area_of_use_from_name(area, proj_rs::NameMatch::Approximate)
            .map_err(retrieval)?;
    }
    match matches.as_slice() {
        [] => Err("No area of use matching provided name".to_owned()),
        [(authority, code)] => proj_rs::AuthorityFactory::create(db, authority.as_str())
            .create_extent(code)
            .map(Some)
            .map_err(retrieval),
        several => {
            let mut text = "Several candidates area of use matching provided name :\n".to_owned();
            for (authority, code) in several {
                let domain = proj_rs::AuthorityFactory::create(db, authority.as_str())
                    .create_extent(code)
                    .map_err(retrieval)?;
                text.push_str(&format!(
                    "  {authority}:{code} : {}\n",
                    domain.description().unwrap_or_default()
                ));
            }
            Err(text)
        }
    }
}

/// The candidate operations between two CRS for the area, spatial test and options given.
fn candidate_operations(
    db: &proj_rs::DatabaseContext,
    source: &proj_rs::Crs,
    target: &proj_rs::Crs,
    options: &Options,
    area: Option<&proj_rs::ObjectDomain>,
    spatial_test: SpatialTest,
) -> Result<Vec<proj_rs::CoordinateOperation>, String> {
    let criterion = match spatial_test {
        SpatialTest::Contains => proj_rs::SpatialCriterion::StrictContainment,
        SpatialTest::Intersects => proj_rs::SpatialCriterion::PartialIntersection,
    };
    let mut context = proj_rs::CoordinateOperationContext::create(db, "EPSG");
    context
        .set_area_of_interest(area.cloned())
        .set_spatial_criterion(criterion)
        .set_crs_extent_use(options.crs_extent_use.unwrap_or_default())
        .set_desired_accuracy(options.accuracy.unwrap_or(0.0));
    proj_rs::CoordinateOperationFactory::create()
        .create_operations(source, target, &context)
        .map_err(|error| error.to_string())
}

fn proj_lines(options: &Options) -> proj_rs::ProjLines {
    if options.single_line {
        proj_rs::ProjLines::Single
    } else {
        proj_rs::ProjLines::Multi
    }
}

fn wkt_lines(options: &Options) -> proj_rs::WktLines {
    if options.single_line {
        proj_rs::WktLines::Single
    } else {
        proj_rs::WktLines::Multi
    }
}

/// The text `-q` prints for one object: its PROJ string, then its WKT2:2019, with no headers.
fn operation_bodies(
    source: &proj_rs::Crs,
    operation: &proj_rs::CoordinateOperation,
    options: &Options,
) -> String {
    let mut out = String::new();
    if options.dialects.contains(&Dialect::Proj) {
        if let Some(proj) = operation_proj(source, operation, proj_lines(options)) {
            out.push_str(&proj);
            out.push('\n');
        }
    }
    if options.dialects.contains(&Dialect::Wkt2_2019) {
        if let Ok(wkt) = operation_wkt(operation, wkt_lines(options)) {
            out.push_str(&wkt);
            out.push('\n');
        }
    }
    out
}

fn load_named(db: &proj_rs::DatabaseContext, text: &str) -> Result<proj_rs::Crs, String> {
    proj_rs::create_from_user_input(text, db).map_err(|error| error.to_string())
}

fn operation_heading(operation: &proj_rs::CoordinateOperation) -> String {
    let id = operation
        .identifiers()
        .first()
        .map(|identifier| format!("{}:{}", identifier.code_space(), identifier.code()))
        .unwrap_or_else(|| "unknown id".to_owned());
    let accuracy = match (operation.accuracy_text(), operation.accuracy(), operation) {
        (Some(text), _, _) => format!("{text} m"),
        (None, Some(value), _) => format!("{value} m"),
        (None, None, proj_rs::CoordinateOperation::Conversion(_)) => "0 m".to_owned(),
        _ => "unknown accuracy".to_owned(),
    };
    let domain = operation.domains().first();
    let mut line = format!("{id}, {}, {accuracy}", operation.name());
    if let Some(area) = domain.and_then(proj_rs::ObjectDomain::area) {
        line.push_str(", ");
        line.push_str(area.trim_end_matches('.'));
    }
    line.push('.');
    line
}

fn operation_proj(
    source: &proj_rs::Crs,
    operation: &proj_rs::CoordinateOperation,
    lines: proj_rs::ProjLines,
) -> Option<String> {
    match operation {
        proj_rs::CoordinateOperation::Conversion(conversion) => conversion_proj(source, conversion),
        proj_rs::CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(lines).ok()
        }
        proj_rs::CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(lines).ok()
        }
    }
}

fn conversion_proj(source: &proj_rs::Crs, conversion: &proj_rs::Conversion) -> Option<String> {
    let mut raw = conversion.export_to_proj_string().ok()?;
    if !raw.contains("+ellps=") && !raw.contains("+datum=") {
        raw.push_str(&ellps_suffix(source));
    }
    if !north_then_east(source) {
        return Some(raw);
    }
    Some(format!(
        "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad +step {raw}"
    ))
}

fn ellps_suffix(crs: &proj_rs::Crs) -> String {
    let name = match crs {
        proj_rs::Crs::Geographic(crs) => crs.ellipsoid().map(|ellipsoid| ellipsoid.name()),
        proj_rs::Crs::Projected(crs) => {
            crs.base_crs().ellipsoid().map(|ellipsoid| ellipsoid.name())
        }
        _ => None,
    };
    let Some(name) = name else {
        return String::new();
    };
    let token = match name {
        "WGS 84" => "WGS84",
        "GRS 1980" => "GRS80",
        "GRS 1967" | "GRS 1967 Modified" => "GRS67",
        "Clarke 1866" => "clrk66",
        "International 1924" => "intl",
        "Bessel 1841" => "bessel",
        _ => return String::new(),
    };
    format!(" +ellps={token}")
}

fn north_then_east(crs: &proj_rs::Crs) -> bool {
    let proj_rs::Crs::Geographic(geographic) = crs else {
        return false;
    };
    let axes = geographic.coordinate_system().axes();
    axes.len() >= 2
        && axes[0].direction().eq_ignore_ascii_case("north")
        && axes[1].direction().eq_ignore_ascii_case("east")
}

fn operation_wkt(
    operation: &proj_rs::CoordinateOperation,
    lines: proj_rs::WktLines,
) -> Result<String, String> {
    let proj_rs::CoordinateOperation::Conversion(conversion) = operation else {
        return Err("not a conversion".to_owned());
    };
    let mut formatter = proj_rs::WktFormatter::create(proj_rs::WktConvention::Wkt2_2019);
    formatter.set_lines(lines);
    conversion
        .export_to_wkt(&mut formatter)
        .map_err(|error| error.to_string())
}

fn describe(options: &Options, text: &str) -> Result<String, String> {
    let db = database()?;
    let object = build_object(&db, text, options.kind)?;
    render(&object, &db, options)
}

/// The object `text` names, as `buildObject` reads it. A `-k` kind reads
/// `auth:code` as a URN; a name without `:` is looked up by name.
fn build_object(
    db: &proj_rs::DatabaseContext,
    text: &str,
    kind: Option<ObjectKind>,
) -> Result<proj_rs::AuthorityObject, String> {
    match kind {
        Some(ObjectKind::Operation) => Err(
            "-k operation is not supported yet: coordinate operations are not described".to_owned(),
        ),
        Some(kind @ (ObjectKind::Ellipsoid | ObjectKind::Datum | ObjectKind::Ensemble)) => {
            let tokens: Vec<&str> = text.split(':').collect();
            if let [authority, code] = tokens.as_slice() {
                let factory = proj_rs::AuthorityFactory::create(db, *authority);
                let object = match kind {
                    ObjectKind::Ellipsoid => factory
                        .create_ellipsoid(code)
                        .map(proj_rs::AuthorityObject::Ellipsoid),
                    ObjectKind::Datum => factory
                        .create_datum(code)
                        .map(proj_rs::AuthorityObject::Datum),
                    _ => factory
                        .create_datum_ensemble(code)
                        .map(proj_rs::AuthorityObject::DatumEnsemble),
                };
                return object.map_err(|error| error.to_string());
            }
            if text.contains(':') {
                return create_crs(db, text);
            }
            named_object(db, text, kind)
        }
        Some(ObjectKind::Crs) if !text.contains(':') && !looks_like_wkt(text) => {
            named_object(db, text, ObjectKind::Crs)
        }
        Some(_) => create_crs(db, text),
        None => create_default(db, text),
    }
}

/// Text that can only be an object name: no WKT, PROJ string, JSON, or `AUTH:code`.
fn name_like(text: &str) -> bool {
    !text.contains([':', '[', '{', '=']) && !text.trim_start().starts_with('+')
}

/// Whether `text` opens with a WKT keyword and `[`.
fn looks_like_wkt(text: &str) -> bool {
    let Some((keyword, _)) = text.trim_start().split_once('[') else {
        return false;
    };
    let keyword = keyword.trim_end();
    !keyword.is_empty()
        && keyword
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '_')
}

/// `createFromUserInput` with no `-k`: the text as WKT, PROJ, `AUTH:code`, or
/// CRS name first. A name that matches none of those is searched over every
/// object type PROJ tries.
fn create_default(
    db: &proj_rs::DatabaseContext,
    text: &str,
) -> Result<proj_rs::AuthorityObject, String> {
    match proj_rs::create_from_user_input(text, db) {
        Ok(crs) => Ok(proj_rs::AuthorityObject::Crs(crs)),
        Err(_) if name_like(text) => search_default(db, text)
            .map_err(|list| format!("input string: parsing of '{text}' failed: {list}"))?
            .ok_or_else(|| {
                format!(
                    "input string: parsing of '{text}' failed: unrecognized format / unknown name"
                )
            }),
        Err(error) => Err(error.to_string()),
    }
}

/// The object types of `searchObject`'s second pass.
const OTHER_NAME_TYPES: [proj_rs::ObjectType; 4] = [
    proj_rs::ObjectType::Ellipsoid,
    proj_rs::ObjectType::Datum,
    proj_rs::ObjectType::DatumEnsemble,
    proj_rs::ObjectType::CoordinateOperation,
];

/// The four passes of PROJ's name search: CRSs, then the other types, each
/// exact, then both approximate. A text of four bytes or fewer gets only the
/// exact passes, and an approximate match must have a name shorter than twice
/// the text.
fn search_default(
    db: &proj_rs::DatabaseContext,
    text: &str,
) -> Result<Option<proj_rs::AuthorityObject>, String> {
    let factory = proj_rs::AuthorityFactory::create(db, "");
    let passes = if text.len() <= 4 { 2 } else { 4 };
    for pass in 0..passes {
        let approximate = pass >= 2;
        let mode = if approximate {
            proj_rs::NameMatch::Approximate
        } else {
            proj_rs::NameMatch::Exact
        };
        let (types, ties): (&[proj_rs::ObjectType], Ties) = if pass % 2 == 0 {
            (&[proj_rs::ObjectType::Crs], Ties::Dimension)
        } else {
            (&OTHER_NAME_TYPES, Ties::Name)
        };
        let found = factory
            .create_objects_from_name(text, types, mode, 10)
            .map_err(|error| error.to_string())?;
        if let Some(object) = settle(found, text, ties)? {
            if !approximate || object.name().len() < 2 * text.len() {
                return Ok(Some(object));
            }
        }
    }
    Ok(None)
}

/// How a name lookup breaks a tie between several matches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ties {
    /// The match whose name is equivalent to the text.
    Name,
    /// For CRSs, the 2D geographic match, then the 3D one, before the name.
    Dimension,
}

/// The match a lookup settles on: its only match, else the one `ties` picks.
/// `None` when nothing matched. Several matches that `ties` cannot break are
/// an error that lists their names.
fn settle(
    found: Vec<proj_rs::AuthorityObject>,
    text: &str,
    ties: Ties,
) -> Result<Option<proj_rs::AuthorityObject>, String> {
    if found.len() <= 1 {
        return Ok(found.into_iter().next());
    }
    if ties == Ties::Dimension {
        for dimension in [2, 3] {
            if let Some(object) = found
                .iter()
                .find(|object| geographic_axes(object) == Some(dimension))
            {
                return Ok(Some(object.clone()));
            }
        }
    }
    if let Some(object) = found
        .iter()
        .find(|object| proj_rs::equivalent_name_bigger_differences(object.name(), text))
    {
        return Ok(Some(object.clone()));
    }
    let mut message = String::from("several objects matching this name: ");
    for (index, object) in found.iter().enumerate() {
        if message.len() > 200 {
            message.push_str(", ...");
            break;
        }
        if index > 0 {
            message.push_str(", ");
        }
        message.push_str(object.name());
    }
    Err(message)
}

fn geographic_axes(object: &proj_rs::AuthorityObject) -> Option<usize> {
    match object {
        proj_rs::AuthorityObject::Crs(proj_rs::Crs::Geographic(crs)) => {
            Some(crs.coordinate_system().axes().len())
        }
        _ => None,
    }
}

fn create_crs(
    db: &proj_rs::DatabaseContext,
    text: &str,
) -> Result<proj_rs::AuthorityObject, String> {
    proj_rs::create_from_user_input(text, db)
        .map(proj_rs::AuthorityObject::Crs)
        .map_err(|error| error.to_string())
}

/// The CRS, ellipsoid, datum, or ensemble whose name is `text`: an exact name
/// first, then a substring, as `buildObject` searches with `-k`.
fn named_object(
    db: &proj_rs::DatabaseContext,
    text: &str,
    kind: ObjectKind,
) -> Result<proj_rs::AuthorityObject, String> {
    let object_type = match kind {
        ObjectKind::Crs => proj_rs::ObjectType::Crs,
        ObjectKind::Ellipsoid => proj_rs::ObjectType::Ellipsoid,
        ObjectKind::Datum => proj_rs::ObjectType::Datum,
        _ => proj_rs::ObjectType::DatumEnsemble,
    };
    let factory = proj_rs::AuthorityFactory::create(db, "");
    for mode in [proj_rs::NameMatch::Exact, proj_rs::NameMatch::Approximate] {
        let found = factory
            .create_objects_from_name(text, &[object_type], mode, 10)
            .map_err(|error| error.to_string())?;
        if let Some(object) =
            settle(found, text, Ties::Name).map_err(|list| format!("input string: {list}"))?
        {
            return Ok(object);
        }
    }
    Err(format!("input string: {text}: no matching object"))
}

fn render(
    object: &proj_rs::AuthorityObject,
    db: &proj_rs::DatabaseContext,
    options: &Options,
) -> Result<String, String> {
    let mut blocks = Vec::new();
    if object.is_deprecated() {
        blocks.push("Warning: object is deprecated".to_owned());
    }
    for dialect in &options.dialects {
        let body = match dialect {
            Dialect::Proj => match proj_text(object, db)? {
                Some(body) => body,
                None => continue,
            },
            Dialect::ProjJson => match object {
                proj_rs::AuthorityObject::Crs(crs) => {
                    crs.export_to_json(db).map_err(|error| error.to_string())?
                }
                _ => return Err("cannot write PROJJSON for this object".to_owned()),
            },
            Dialect::Sql => match sql_text(object, db, options) {
                Ok(body) => body,
                Err(error) => {
                    eprintln!("Error when exporting to SQL: {error}");
                    continue;
                }
            },
            Dialect::Wkt2_2015 => wkt(object, db, proj_rs::WktConvention::Wkt2_2015)?,
            Dialect::Wkt2_2019 => wkt(object, db, proj_rs::WktConvention::Wkt2_2019)?,
            Dialect::Wkt1Gdal => {
                format!("{}\n", wkt(object, db, proj_rs::WktConvention::Wkt1Gdal)?)
            }
            Dialect::Wkt1Esri => {
                format!("{}\n", wkt(object, db, proj_rs::WktConvention::Wkt1Esri)?)
            }
        };
        if options.quiet {
            blocks.push(body);
        } else {
            let header = match dialect {
                Dialect::Proj if matches!(object, proj_rs::AuthorityObject::Crs(_)) => {
                    "PROJ.4 string:"
                }
                Dialect::Proj => "PROJ string:",
                Dialect::Wkt2_2015 => "WKT2:2015 string:",
                Dialect::Wkt2_2019 => "WKT2:2019 string:",
                Dialect::Wkt1Gdal => "WKT1:GDAL string:",
                Dialect::Wkt1Esri => "WKT1:ESRI string:",
                Dialect::ProjJson => "PROJJSON:",
                Dialect::Sql => "SQL:",
            };
            blocks.push(if body.is_empty() {
                header.to_owned()
            } else {
                format!("{header}\n{body}")
            });
        }
    }
    Ok(blocks.join("\n\n"))
}

/// The insert statements for `object` under `--output-id`, one per line. An
/// object that is already known under that name has none.
fn sql_text(
    object: &proj_rs::AuthorityObject,
    db: &proj_rs::DatabaseContext,
    options: &Options,
) -> Result<String, String> {
    let Some((authority, code)) = options.output_id.as_ref() else {
        return Err("--output-id AUTH:CODE must be specified".to_owned());
    };
    let allowed = if options.authorities.is_empty() {
        vec!["EPSG".to_owned(), "PROJ".to_owned()]
    } else {
        options.authorities.clone()
    };
    let statements = proj_rs::insert_statements(db, object, authority, code, &allowed)
        .map_err(|error| error.to_string())?;
    Ok(statements.join("\n"))
}

/// The PROJ string of `object`. A datum has none, so it prints no block.
fn proj_text(
    object: &proj_rs::AuthorityObject,
    db: &proj_rs::DatabaseContext,
) -> Result<Option<String>, String> {
    match object {
        proj_rs::AuthorityObject::Crs(crs) => crs
            .create_bound_crs_to_wgs84_if_possible(db)
            .export_to_proj_string()
            .map(Some)
            .map_err(|error| error.to_string()),
        proj_rs::AuthorityObject::Ellipsoid(ellipsoid) => {
            Ok(Some(ellipsoid.export_to_proj_string()))
        }
        _ => Ok(None),
    }
}

fn wkt(
    object: &proj_rs::AuthorityObject,
    db: &proj_rs::DatabaseContext,
    convention: proj_rs::WktConvention,
) -> Result<String, String> {
    let mut formatter = proj_rs::WktFormatter::create(convention).with_database(db.clone());
    let text = match object {
        proj_rs::AuthorityObject::Crs(crs) => crs.export_to_wkt(&mut formatter),
        proj_rs::AuthorityObject::Ellipsoid(ellipsoid) => ellipsoid.export_to_wkt(&mut formatter),
        proj_rs::AuthorityObject::Datum(datum) => datum.export_to_wkt(&mut formatter),
        proj_rs::AuthorityObject::DatumEnsemble(ensemble) => ensemble.export_to_wkt(&mut formatter),
        _ => return Err("cannot write WKT for this object".to_owned()),
    };
    text.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_dialect_is_an_error() {
        assert!(classify_dialect("WKT1:GDAL").is_ok());
        assert!(classify_dialect("WKT_ALL")
            .expect_err("wkt all")
            .contains("cannot write dialect"));
        assert_eq!(classify_dialect("SQL").expect("sql"), Some(Dialect::Sql));
        assert_eq!(classify_dialect("all").expect("all"), None);
        assert_eq!(
            classify_dialect("WKT2:2019").expect("wkt"),
            Some(Dialect::Wkt2_2019)
        );
    }

    #[test]
    fn epsg_4326_matches_the_projinfo_sample() {
        let text = describe(
            &Options::parse(&["EPSG:4326".to_owned()]).expect("options"),
            "EPSG:4326",
        )
        .expect("describe");
        assert!(text.starts_with("PROJ.4 string:\n+proj=longlat +datum=WGS84 +no_defs +type=crs\n"));
        assert!(text.contains("ENSEMBLEACCURACY[2.0]]"));
    }

    fn well_formed(west: f64, south: f64, east: f64, north: f64) -> proj_rs::BoundingBox {
        proj_rs::BoundingBox::new(west, south, east, north)
    }

    #[test]
    fn contains_follows_the_antimeridian_rules() {
        let europe = well_formed(-10.0, 35.0, 5.0, 45.0);
        assert!(europe.contains(&well_formed(-5.0, 36.0, 0.0, 40.0)));
        assert!(!europe.contains(&well_formed(-20.0, 36.0, 0.0, 40.0)));
        let pacific = well_formed(170.0, -10.0, -170.0, 10.0);
        assert!(pacific.contains(&well_formed(175.0, 0.0, -175.0, 5.0)));
        let world = well_formed(-180.0, -90.0, 180.0, 90.0);
        assert!(world.contains(&well_formed(-5.0, 36.0, 0.0, 40.0)));
        assert!(world.contains(&well_formed(-180.0, -90.0, 180.0, 90.0)));
        assert!(!world.contains(&well_formed(10.0, 0.0, 10.0, 5.0)));
    }

    #[test]
    fn intersects_follows_the_antimeridian_rules() {
        let europe = well_formed(-10.0, 35.0, 5.0, 45.0);
        assert!(europe.intersects(&well_formed(0.0, 40.0, 20.0, 50.0)));
        assert!(!europe.intersects(&well_formed(6.0, 35.0, 20.0, 45.0)));
        let pacific = well_formed(170.0, -10.0, -170.0, 10.0);
        assert!(pacific.intersects(&well_formed(-175.0, 0.0, -160.0, 5.0)));
        assert!(!pacific.intersects(&well_formed(0.0, 0.0, 10.0, 5.0)));
    }

    #[test]
    fn bbox_needs_four_values() {
        assert!(parse_bbox("1,2,3")
            .expect_err("three values")
            .contains("Incorrect number of values"));
        assert!(parse_bbox("1,x,3,4")
            .expect_err("text")
            .contains("Invalid value"));
        assert_eq!(
            parse_bbox("1,2,3,4").expect("four"),
            well_formed(1.0, 2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn completion_phrase_joins_a_quoted_start() {
        let words = |items: &[&str]| {
            items
                .iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            completion_phrase(&words(&["\"ETRS89-FRA [RGF93 v1]", "/"])),
            "ETRS89-FRA [RGF93 v1] /"
        );
        assert_eq!(completion_phrase(&words(&["EGM"])), "EGM");
    }

    #[test]
    fn unknown_kind_is_rejected() {
        assert_eq!(classify_kind("Datum").expect("kind"), ObjectKind::Datum);
        assert!(classify_kind("grid")
            .expect_err("kind")
            .contains("Unrecognized value for option -k"));
    }
}
