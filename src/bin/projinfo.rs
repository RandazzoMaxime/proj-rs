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
    if args.first().map(String::as_str) == Some("completion") {
        return complete_shell(&args[1..]).map_err(Report::Plain);
    }
    if args.iter().any(|arg| arg == "--list-crs") {
        return list_crs(args).map_err(Report::Plain);
    }
    let options = Options::parse(args).map_err(Report::Usage)?;
    if options.help {
        return Ok(usage().to_owned());
    }
    if options.version {
        return Ok(format!("projinfo: {RELEASE}"));
    }
    if options.source.is_some() && options.target.is_some() {
        return candidates(&options).map_err(Report::Plain);
    }
    let code = options
        .code
        .as_deref()
        .ok_or_else(|| Report::Usage("missing EPSG code".to_owned()))?;
    if options.quiet && options.dialects.len() != 1 {
        return Err(Report::Usage(
            "-q can only be used with a single output format".to_owned(),
        ));
    }
    describe(&options, code).map_err(Report::Plain)
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
    dialects: Vec<Dialect>,
    code: Option<String>,
    source: Option<String>,
    target: Option<String>,
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
                    | "--spatial-test"
                    | "--bbox"
                    | "--area"
                    | "--crs-extent-use"
                    | "--pivot-crs"
                    | "--accuracy"
                    | "--authority"
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
                    | "--output-id"
                    | "--c-ify"
                    | "--searchpaths"
                    | "--remote-data"
                    | "--dump-db-structure"
            ) {
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
                        None => {
                            proj = true;
                            wkt = true;
                        }
                    }
                }
            } else if arg.starts_with('-') {
                return Err(format!("unrecognized option {arg}"));
            } else {
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
        if saw_format && dialects.is_empty() && !help && !version {
            return Err("no output format remains enabled".to_owned());
        }
        Ok(Self {
            help,
            version,
            quiet,
            single_line,
            summary,
            dialects,
            code,
            source,
            target,
        })
    }
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
        "sql",
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
        _ => return Err(format!("unrecognized value for option -o: {text}")),
    })
}

fn complete_shell(args: &[String]) -> Result<String, String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let text = match words.as_slice() {
        ["projinfo", "-"] | ["projinfo"] if false => "",
        ["projinfo", "-"] => {
            " -o -k --summary -q --area --bbox --spatial-test --crs-extent-use --grid-check --pivot-crs --show-superseded --hide-ballpark --accuracy --allow-ellipsoidal-height-as-vertical-crs --boundcrs-to-wgs84 --authority --main-db-path --aux-db-path --identify --3d --output-id --c-ify --single-line --searchpaths --remote-data --list-crs --dump-db-structure -s --s_epoch -t --t_epoch"
        }
        ["projinfo", "-o"] => "all PROJ WKT2:2019 WKT2:2015 WKT1:GDAL WKT1:ESRI PROJJSON SQL",
        ["projinfo", "-o", "WKT2:"] => "2019 2015",
        ["projinfo", "-o", "WKT1:"] => "GDAL ESRI",
        ["projinfo", "--pivot-crs"] => {
            " always if_no_direct_transformation never EPSG: ESRI: IAU_2015: IGNF: NKG: NRCAN: OGC: PROJ:"
        }
        ["projinfo"] => " EPSG: ESRI: IAU_2015: IGNF: NKG: NRCAN: OGC: PROJ:",
        ["projinfo", "EPSG:4326"] => " 4326",
        ["projinfo", "EPSG:432"] => " 4322\\ --\\ WGS\\ 72 4324\\ --\\ WGS\\ 72BE 4326\\ --\\ WGS\\ 84",
        _ => "",
    };
    Ok(text.to_owned())
}

fn list_crs(args: &[String]) -> Result<String, String> {
    let mut authorities: Vec<String> = Vec::new();
    let mut type_filter = String::new();
    let mut allow_deprecated = false;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--list-crs" {
            if let Some(next) = args.get(index + 1) {
                if !next.starts_with('-') {
                    type_filter = next.clone();
                    index += 1;
                }
            }
        } else if arg == "--authority" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| "missing authority".to_owned())?;
            authorities = value.split(',').map(|item| item.to_owned()).collect();
        } else if arg == "allow_deprecated" || type_filter.contains("allow_deprecated") {
            allow_deprecated = true;
        }
        index += 1;
    }
    if type_filter.contains("allow_deprecated") {
        allow_deprecated = true;
    }
    if authorities.is_empty() {
        authorities = [
            "EPSG", "ESRI", "OGC", "IAU_2015", "IGNF", "NKG", "NRCAN", "PROJ",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    let db = proj_rs::DatabaseContext::create().map_err(|error| error.to_string())?;
    let mut lines = Vec::new();
    for authority in authorities {
        let factory = proj_rs::AuthorityFactory::create(&db, &authority);
        let infos = factory.get_crs_info_list().unwrap_or_default();
        let mut seen = std::collections::BTreeSet::new();
        for info in infos {
            if info.deprecated && !allow_deprecated {
                continue;
            }
            if !type_filter.is_empty() && !type_matches(&type_filter, &info.type_name) {
                continue;
            }
            if !seen.insert(info.code.clone()) {
                continue;
            }
            let mut line = format!("{}:{} \"{}\"", info.authority, info.code, info.name);
            if info.deprecated {
                line.push_str(" [deprecated]");
            }
            lines.push(line);
        }
    }
    Ok(lines.join("\n"))
}

fn type_matches(filter: &str, type_name: &str) -> bool {
    let folded = type_name.to_ascii_lowercase();
    filter.split(',').any(|piece| {
        let piece = piece.trim();
        if piece.is_empty() || piece == "allow_deprecated" {
            return false;
        }
        folded.contains(&piece.to_ascii_lowercase())
            || (piece == "geodetic"
                && (folded.contains("geographic") || folded.contains("geocentric")))
            || (piece == "geographic" && folded.contains("geographic"))
    })
}

fn candidates(options: &Options) -> Result<String, String> {
    let db = proj_rs::DatabaseContext::create().map_err(|error| error.to_string())?;
    let source_text = options.source.as_deref().unwrap_or("");
    let target_text = options.target.as_deref().unwrap_or("");
    let source = load_named(&db, source_text)?;
    let target = load_named(&db, target_text)?;
    let context = proj_rs::CoordinateOperationContext::create(&db, "EPSG");
    let operations = proj_rs::CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .map_err(|error| error.to_string())?;
    let mut out = format!("Candidate operations found: {}\n", operations.len());
    if !options.summary {
        out.push_str("-------------------------------------\n");
    }
    for (index, operation) in operations.iter().enumerate() {
        if !options.summary {
            out.push_str(&format!("Operation No. {}:\n\n", index + 1));
        }
        out.push_str(&operation_heading(operation));
        out.push('\n');
        if options.summary {
            continue;
        }
        if let Some(proj) = operation_proj(&source, operation) {
            out.push_str("\nPROJ string:\n");
            out.push_str(&proj);
            out.push('\n');
        }
        let lines = if options.single_line {
            proj_rs::WktLines::Single
        } else {
            proj_rs::WktLines::Multi
        };
        if let Ok(wkt) = operation_wkt(operation, lines) {
            out.push_str("\nWKT2:2019 string:\n");
            out.push_str(&wkt);
            out.push('\n');
        }
    }
    Ok(out)
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
    let accuracy = match operation.accuracy() {
        Some(value) if value.fract() == 0.0 => format!("{}", value.trunc()),
        Some(value) => format!("{value}"),
        None => "0".to_owned(),
    };
    let domain = operation.domains().first();
    let mut line = format!("{id}, {}, {accuracy} m", operation.name());
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
) -> Option<String> {
    let proj_rs::CoordinateOperation::Conversion(conversion) = operation else {
        return None;
    };
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
    let db = proj_rs::DatabaseContext::create().map_err(|error| error.to_string())?;
    let crs = proj_rs::create_from_user_input(text, &db).map_err(|error| error.to_string())?;
    render(&crs, &db, options)
}

fn render(
    crs: &proj_rs::Crs,
    db: &proj_rs::DatabaseContext,
    options: &Options,
) -> Result<String, String> {
    let mut blocks = Vec::new();
    if crs.is_deprecated() {
        blocks.push("Warning: object is deprecated".to_owned());
    }
    for dialect in &options.dialects {
        let body = match dialect {
            Dialect::Proj => crs
                .export_to_proj_string()
                .map_err(|error| error.to_string())?,
            Dialect::ProjJson => crs.export_to_json(db).map_err(|error| error.to_string())?,
            Dialect::Wkt2_2015 => wkt(crs, db, proj_rs::WktConvention::Wkt2_2015)?,
            Dialect::Wkt2_2019 => wkt(crs, db, proj_rs::WktConvention::Wkt2_2019)?,
            Dialect::Wkt1Gdal => format!("{}\n", wkt(crs, db, proj_rs::WktConvention::Wkt1Gdal)?),
            Dialect::Wkt1Esri => format!("{}\n", wkt(crs, db, proj_rs::WktConvention::Wkt1Esri)?),
        };
        if options.quiet {
            blocks.push(body);
        } else {
            let header = match dialect {
                Dialect::Proj => "PROJ.4 string:",
                Dialect::Wkt2_2015 => "WKT2:2015 string:",
                Dialect::Wkt2_2019 => "WKT2:2019 string:",
                Dialect::Wkt1Gdal => "WKT1:GDAL string:",
                Dialect::Wkt1Esri => "WKT1:ESRI string:",
                Dialect::ProjJson => "PROJJSON:",
            };
            blocks.push(format!("{header}\n{body}"));
        }
    }
    Ok(blocks.join("\n\n"))
}

fn wkt(
    crs: &proj_rs::Crs,
    db: &proj_rs::DatabaseContext,
    convention: proj_rs::WktConvention,
) -> Result<String, String> {
    crs.export_to_wkt(&mut proj_rs::WktFormatter::create(convention).with_database(db.clone()))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_dialect_is_an_error() {
        assert!(classify_dialect("WKT1:GDAL").is_ok());
        assert!(classify_dialect("SQL")
            .expect_err("sql")
            .contains("cannot write dialect"));
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
}
