//! `projsync`: download the PROJ data files the database does not have, from the index
//! `files.geojson` of the endpoint. A port of PROJ 9.9.0 `src/apps/projsync.cpp`.
//!
//! The index, the selection and the downloads are in `proj_rs::projsync`. This file reads the
//! options, prints, and exits as the C++ program does: 1 on a usage or download error.
//! `--help` and `--version` are not in PROJ 9.9.0; `--user-directory` is a name for the default
//! target, `--user-writable-directory`.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use proj_rs::projsync::{
    self, Bbox, Decision, Index, Query, SpatialTest, VersionFiltering, WorldCoverage,
    INDEX_FILE_NAME,
};
use proj_rs::ProjBuilder;

const USAGE: &str = "usage: projsync \n\
          [--endpoint URL]\n\
          [--local-geojson-file FILENAME]\n\
          ([--user-writable-directory] | [--system-directory] | [--target-dir DIRNAME])\n\
          [--bbox west_long,south_lat,east_long,north_lat]\n\
          [--spatial-test contains|intersects]\n\
          [--source-id ID] [--area-of-use NAME]\n\
          [--file NAME]\n\
          [--all] [--exclude-world-coverage]\n\
          [--quiet | --verbose] [--dry-run] [--list-files]\n\
          [--no-version-filtering]\n";

/// Whether the program prints its progress, `-q` and `--quiet` turn it off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Chatter {
    #[default]
    Normal,
    Quiet,
}

/// The options, as the command line gives them.
#[derive(Debug, Default)]
struct Options {
    endpoint: Option<String>,
    target_dir: Option<PathBuf>,
    local_index: Option<PathBuf>,
    list_files: bool,
    dry_run: bool,
    chatter: Chatter,
    verbose: bool,
    query: Query,
}

fn say(text: &str) {
    let _ = writeln!(io::stdout().lock(), "{text}");
}

fn warn(text: &str) {
    let _ = writeln!(io::stderr(), "{text}");
}

fn usage_error() -> ExitCode {
    let _ = write!(io::stderr(), "{USAGE}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse(&args) {
        Ok(options) => options,
        Err(code) => return code,
    };
    run(&options)
}

/// Read the options. `Err` is the exit code after the message is printed.
fn parse(args: &[String]) -> Result<Options, ExitCode> {
    let mut options = Options::default();
    let mut bbox: Option<Bbox> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || iter.next().cloned();
        match arg.as_str() {
            "--endpoint" => options.endpoint = Some(value().ok_or_else(usage_error)?),
            "--user-writable-directory" | "--user-directory" => {}
            "--system-directory" => {
                options.target_dir = Some(system_directory().ok_or_else(|| {
                    warn("--system-directory: no PROJ_DATA or PROJ_LIB directory is set");
                    ExitCode::from(1)
                })?);
            }
            "--target-dir" => {
                options.target_dir = Some(PathBuf::from(value().ok_or_else(usage_error)?));
            }
            "--local-geojson-file" => {
                options.local_index = Some(PathBuf::from(value().ok_or_else(usage_error)?));
            }
            "--list-files" => options.list_files = true,
            "--source-id" => options.query.source_id = value().ok_or_else(usage_error)?,
            "--area-of-use" => options.query.area_of_use = value().ok_or_else(usage_error)?,
            "--file" => options.query.file = value().ok_or_else(usage_error)?,
            "--bbox" => {
                let text = value().ok_or_else(usage_error)?;
                bbox = Some(parse_bbox(&text)?);
            }
            "--spatial-test" => {
                let text = value().ok_or_else(usage_error)?;
                options.query.spatial_test = match text.to_ascii_lowercase().as_str() {
                    "contains" => SpatialTest::Contains,
                    "intersects" => SpatialTest::Intersects,
                    _ => {
                        warn(&format!(
                            "Unrecognized value for option --spatial-test: {text}"
                        ));
                        return Err(usage_error());
                    }
                };
            }
            "--dry-run" => options.dry_run = true,
            "--exclude-world-coverage" => {
                options.query.world_coverage = WorldCoverage::Exclude;
            }
            "--all" => options.query.all = true,
            "--no-version-filtering" => {
                options.query.version_filtering = VersionFiltering::Ignore;
            }
            "-q" | "--quiet" => options.chatter = Chatter::Quiet,
            "--verbose" => options.verbose = true,
            "--version" => {
                say(&format!(
                    "projsync {} (Rust port of PROJ 9.9.0)",
                    env!("CARGO_PKG_VERSION")
                ));
                return Err(ExitCode::SUCCESS);
            }
            "--help" => {
                let _ = write!(io::stdout().lock(), "{USAGE}");
                return Err(ExitCode::SUCCESS);
            }
            _ => return Err(usage_error()),
        }
    }
    options.query.bbox = bbox;
    let asked = options.list_files
        || !options.query.file.is_empty()
        || !options.query.source_id.is_empty()
        || !options.query.area_of_use.is_empty()
        || options.query.bbox.is_some()
        || options.query.all;
    if !asked {
        warn(
            "At least one of --list-files, --file, --source-id, --area-of-use, --bbox or \
             --all must be specified.\n",
        );
        return Err(usage_error());
    }
    Ok(options)
}

/// `--bbox west,south,east,north`. A wrong count or a number that does not parse is a usage
/// error.
fn parse_bbox(text: &str) -> Result<Bbox, ExitCode> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 4 {
        warn(&format!(
            "Incorrect number of values for option --bbox: {text}"
        ));
        return Err(usage_error());
    }
    let mut numbers = [0.0_f64; 4];
    for (slot, part) in numbers.iter_mut().zip(&parts) {
        *slot = match part.trim().parse() {
            Ok(number) => number,
            Err(error) => {
                warn(&format!("Invalid value for option --bbox: {text}, {error}"));
                return Err(usage_error());
            }
        };
    }
    let [west, south, east, north] = numbers;
    Ok(Bbox::new(west, south, east, north))
}

/// The first directory of `PROJ_DATA`, else of `PROJ_LIB`.
fn system_directory() -> Option<PathBuf> {
    ["PROJ_DATA", "PROJ_LIB"]
        .iter()
        .filter_map(std::env::var_os)
        .find(|value| !value.is_empty())
        .and_then(|value| std::env::split_paths(&value).find(|path| !path.as_os_str().is_empty()))
}

fn trim_trailing_slash(text: &str) -> &str {
    text.strip_suffix('/').unwrap_or(text)
}

fn run(options: &Options) -> ExitCode {
    let builder = ProjBuilder::new();
    let endpoint = trim_trailing_slash(
        &options
            .endpoint
            .clone()
            .unwrap_or_else(|| builder.get_url_endpoint().unwrap_or_default()),
    )
    .to_owned();
    let target_dir = match &options.target_dir {
        Some(dir) => PathBuf::from(trim_trailing_slash(&dir.to_string_lossy()).to_owned()),
        None => builder.user_writable_directory(),
    };

    if options.chatter == Chatter::Normal && !options.list_files {
        say(&format!(
            "Downloading from {endpoint} into {}",
            target_dir.display()
        ));
    }

    let index_path = match &options.local_index {
        Some(path) => path.clone(),
        None => {
            let url = format!("{endpoint}/{INDEX_FILE_NAME}");
            if projsync::download(&target_dir, &url).is_err() {
                warn(&format!("Cannot download {INDEX_FILE_NAME}"));
                return ExitCode::from(1);
            }
            target_dir.join(INDEX_FILE_NAME)
        }
    };
    let text = match std::fs::read_to_string(&index_path) {
        Ok(text) => text,
        Err(_) => {
            warn(&format!("Cannot open {}", index_path.display()));
            return ExitCode::from(1);
        }
    };

    if options.list_files {
        say("filename,area_of_use,source_id,file_size");
    }

    let (data_version, data_version_text) = match projsync::database_data_version() {
        Some((major, minor, text)) => (Some((major, minor)), text),
        None => (None, String::new()),
    };
    let index = match Index::parse(&text) {
        Ok(index) => index,
        Err(error) => {
            warn(&format!("Error: {error}"));
            return ExitCode::from(1);
        }
    };
    let selection = projsync::select(&index, &options.query, data_version);

    let mut to_download: Vec<(String, u64)> = Vec::new();
    for decision in &selection.decisions {
        match *decision {
            Decision::TooRecent { entry, version } => {
                if options.verbose {
                    say(&format!(
                        "Skipping {} as it is only useful starting with PROJ-data {version} \
                         and we are targeting {data_version_text}",
                        entry.name
                    ));
                }
            }
            Decision::Removed { entry, version } => {
                if options.verbose {
                    say(&format!(
                        "Skipping {} as it is no longer useful starting with PROJ-data \
                         {version} and we are targeting {data_version_text}",
                        entry.name
                    ));
                }
            }
            Decision::Matched(entry) => {
                if options.list_files {
                    say(&format!(
                        "{},{},{},{}",
                        entry.name,
                        entry.area_of_use.as_deref().unwrap_or(""),
                        entry.source_id.as_deref().unwrap_or(""),
                        entry.file_size
                    ));
                    continue;
                }
                let url = format!("{endpoint}/{}", entry.name);
                if projsync::is_download_needed(&target_dir, &url) {
                    to_download.push((url, entry.file_size));
                } else if options.chatter == Chatter::Normal {
                    say(&format!("{url} already downloaded."));
                }
            }
        }
    }

    let total: u64 = to_download.iter().map(|(_, size)| size).sum();
    if options.chatter == Chatter::Normal && !options.list_files && total > 0 {
        if total > 1024 * 1024 {
            say(&format!(
                "Total size to download: {} MB",
                total / (1024 * 1024)
            ));
        } else {
            say(&format!("Total to download: {total} bytes"));
        }
    }
    for (number, (url, _)) in to_download.iter().enumerate() {
        if options.chatter == Chatter::Normal {
            let verb = if options.dry_run {
                "Would download"
            } else {
                "Downloading"
            };
            say(&format!(
                "{verb} {url}... ({} / {})",
                number + 1,
                to_download.len()
            ));
        }
        if !options.dry_run && projsync::download(&target_dir, url).is_err() {
            warn(&format!("Cannot download {url}"));
            return ExitCode::from(1);
        }
    }

    let query = &options.query;
    if !query.source_id.is_empty() && !selection.found_source_id {
        return unknown_value("--source-id", &query.source_id, &selection.source_ids);
    }
    if !query.area_of_use.is_empty() && !selection.found_area_of_use {
        return unknown_value("--area-of-use", &query.area_of_use, &selection.areas_of_use);
    }
    if !query.file.is_empty() && !selection.found_file {
        return unknown_value("--file", &query.file, &selection.files);
    }
    ExitCode::SUCCESS
}

/// The message of a query value no file has, with the values there are.
fn unknown_value(
    option: &str,
    value: &str,
    known: &std::collections::BTreeSet<String>,
) -> ExitCode {
    warn(&format!(
        "Warning: '{value}' is a unknown value for {option}."
    ));
    warn("Known values are:");
    for entry in known {
        warn(&format!("  {entry}"));
    }
    ExitCode::from(1)
}
