//! PROJ `test/cli/test_projsync.sh`, against a local server (`tests/net_support`) instead of
//! `cdn.proj.org`, and an index of our own (`tests/projsync/files.geojson`, the same shape as
//! PROJ's `files.geojson`) instead of the CDN one.
//!
//! The expected output of each case is what PROJ 9.9.0 `projsync` prints for the same index
//! (the reference build, `--local-geojson-file`, `--endpoint https://cdn.proj.org`, data
//! version 1.25). Here the endpoint is the local server, which is the only text that differs.
//! The exit code and the messages on stderr are compared as well.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "net_support/mod.rs"]
#[expect(dead_code, reason = "this file does not inspect the requests")]
mod net_support;

use net_support::{LocalServer, Route};

const INDEX: &str = include_str!("projsync/files.geojson");
const DUMMY: &str = include_str!("projsync/dummy.geojson");

/// The endpoint the reference output was recorded with.
const REFERENCE_ENDPOINT: &str = "https://cdn.proj.org";

/// `projsync --list-files`, after the header.
const LIST: &str = "\
at_bev_README.txt,,at_bev,3042
fr_ign_README.txt,France,fr_ign,1000
fr_ign_ntf_r93.tif,France - onshore - mainland and Corsica.,fr_ign,12345
us_nga_egm96_15.tif,World,us_nga,999
nz_linz_nzgeoid2009.tif,New Zealand,nz_linz,5000
us_noaa_alaska.tif,Alaska,us_noaa,7000
au_ga_GDA2020_to_AVWS.tif,Australia,au_ga,60878523
";

/// `--all --dry-run`.
const ALL: &str = "\
Total size to download: 58 MB
Would download https://cdn.proj.org/at_bev_README.txt... (1 / 7)
Would download https://cdn.proj.org/fr_ign_README.txt... (2 / 7)
Would download https://cdn.proj.org/fr_ign_ntf_r93.tif... (3 / 7)
Would download https://cdn.proj.org/us_nga_egm96_15.tif... (4 / 7)
Would download https://cdn.proj.org/nz_linz_nzgeoid2009.tif... (5 / 7)
Would download https://cdn.proj.org/us_noaa_alaska.tif... (6 / 7)
Would download https://cdn.proj.org/au_ga_GDA2020_to_AVWS.tif... (7 / 7)
";

/// `--source-id fr_ign --dry-run`.
const SOURCE_FR_IGN: &str = "\
Total to download: 13345 bytes
Would download https://cdn.proj.org/fr_ign_README.txt... (1 / 2)
Would download https://cdn.proj.org/fr_ign_ntf_r93.tif... (2 / 2)
";

/// `--source-id non_existing`, on stderr.
const UNKNOWN_SOURCE: &str = "\
Warning: 'non_existing' is a unknown value for --source-id.
Known values are:
  at_bev
  au_ga
  fr_ign
  nz_linz
  us_nga
  us_noaa
";

/// `--area-of-use non_existing`, on stderr. The empty area is the one of `at_bev_README.txt`.
const UNKNOWN_AREA: &str = "\
Warning: 'non_existing' is a unknown value for --area-of-use.
Known values are:
  
  Alaska
  Australia
  France
  France - onshore - mainland and Corsica.
  New Zealand
  World
";

/// `--file non_existing`, on stderr.
const UNKNOWN_FILE: &str = "\
Warning: 'non_existing' is a unknown value for --file.
Known values are:
  at_bev_README.txt
  au_ga_GDA2020_to_AVWS.tif
  fr_ign_README.txt
  fr_ign_ntf_r93.tif
  nz_linz_nzgeoid2009.tif
  us_nga_egm96_15.tif
  us_noaa_alaska.tif
";

/// `--file ntf_r93 --dry-run`.
const FILE_NTF: &str = "\
Total to download: 12345 bytes
Would download https://cdn.proj.org/fr_ign_ntf_r93.tif... (1 / 1)
";

/// `--bbox 2,49,3,50 --dry-run`.
const BBOX_FRANCE: &str = "\
Total to download: 13344 bytes
Would download https://cdn.proj.org/fr_ign_ntf_r93.tif... (1 / 2)
Would download https://cdn.proj.org/us_nga_egm96_15.tif... (2 / 2)
";

/// `--bbox 2,49,3,50 --exclude-world-coverage --dry-run`.
const BBOX_FRANCE_NO_WORLD: &str = "\
Total to download: 12345 bytes
Would download https://cdn.proj.org/fr_ign_ntf_r93.tif... (1 / 1)
";

/// `--bbox` windows across the antimeridian: `170,-90,-170,90`, `170,-90,190,90` and
/// `-190,-90,-170,90` give the same four files.
const BBOX_ANTIMERIDIAN: &str = "\
Total size to download: 58 MB
Would download https://cdn.proj.org/us_nga_egm96_15.tif... (1 / 4)
Would download https://cdn.proj.org/nz_linz_nzgeoid2009.tif... (2 / 4)
Would download https://cdn.proj.org/us_noaa_alaska.tif... (3 / 4)
Would download https://cdn.proj.org/au_ga_GDA2020_to_AVWS.tif... (4 / 4)
";

/// `--source-id au_ga --verbose --dry-run --local-geojson-file dummy.geojson`.
const VERBOSE_DUMMY: &str = "\
Skipping removed_in_1.7 as it is no longer useful starting with PROJ-data 1.7 and we are targeting 1.25
Skipping added_in_99_99.tif as it is only useful starting with PROJ-data 99.99 and we are targeting 1.25
Total size to download: 58 MB
Would download https://cdn.proj.org/without_version.tif... (1 / 1)
";

/// A test directory, removed when the test ends.
struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("proj-rs-projsync-{name}-{}", std::process::id()));
        let _removed = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// The server of the test: the index and the data files.
fn server() -> LocalServer {
    LocalServer::spawn(vec![
        Route {
            path: "/files.geojson",
            status: 200,
            body: INDEX.as_bytes().to_vec(),
        },
        Route {
            path: "/fr_ign_ntf_r93.tif",
            status: 200,
            body: b"proj-rs test grid".to_vec(),
        },
    ])
}

/// Run `projsync` with `args` and the endpoint of `server`, into `target`.
fn projsync(server: &LocalServer, target: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_projsync"))
        .arg("--endpoint")
        .arg(server.url())
        .arg("--target-dir")
        .arg(target)
        .args(args)
        .output()
        .expect("projsync runs")
}

/// The text the reference printed, with the endpoint of `server`.
fn expected(text: &str, server: &LocalServer) -> String {
    text.replace(REFERENCE_ENDPOINT, server.url())
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The stdout of a run, without its first line: `Downloading from <endpoint> into <dir>`. The
/// directory differs from the reference run, so the line is checked on its own.
fn body_of(output: &Output, target: &Path, server: &LocalServer) -> String {
    let stdout = stdout_of(output);
    let (first, rest) = stdout.split_once('\n').unwrap_or((stdout.as_str(), ""));
    assert_eq!(
        first,
        format!(
            "Downloading from {} into {}",
            server.url(),
            target.display()
        ),
        "the first line names the endpoint and the target"
    );
    rest.to_owned()
}

#[test]
fn list_files_fetches_the_index_and_records_it() {
    let dir = TestDir::new("list");
    let server = server();
    let output = projsync(&server, dir.path(), &["--list-files"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);
    let (header, rest) = stdout.split_once('\n').expect("header");
    assert_eq!(header, "filename,area_of_use,source_id,file_size");
    assert_eq!(rest, LIST);
    assert!(dir.path().join("files.geojson").is_file());
    assert!(dir.path().join("cache.db").is_file(), "the record is kept");
}

#[test]
fn all_dry_run_lists_every_file() {
    let dir = TestDir::new("all");
    let server = server();
    let output = projsync(&server, dir.path(), &["--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(
        body_of(&output, dir.path(), &server),
        expected(ALL, &server)
    );
}

#[test]
fn source_id_selects_by_part_of_the_id() {
    let dir = TestDir::new("source");
    let server = server();
    let output = projsync(&server, dir.path(), &["--source-id", "fr_ign", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(
        body_of(&output, dir.path(), &server),
        expected(SOURCE_FR_IGN, &server)
    );
}

#[test]
fn unknown_source_id_fails_and_lists_the_known_ones() {
    let dir = TestDir::new("unknown-source");
    let server = server();
    let output = projsync(&server, dir.path(), &["--source-id", "non_existing"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr_of(&output), UNKNOWN_SOURCE);
}

#[test]
fn area_of_use_selects_by_part_of_the_name() {
    let dir = TestDir::new("area");
    let server = server();
    let output = projsync(
        &server,
        dir.path(),
        &["--area-of-use", "France", "--dry-run"],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(
        body_of(&output, dir.path(), &server),
        expected(SOURCE_FR_IGN, &server)
    );
}

#[test]
fn unknown_area_of_use_fails_and_lists_the_known_ones() {
    let dir = TestDir::new("unknown-area");
    let server = server();
    let output = projsync(&server, dir.path(), &["--area-of-use", "non_existing"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr_of(&output), UNKNOWN_AREA);
}

#[test]
fn file_downloads_once_and_then_says_it_is_already_there() {
    let dir = TestDir::new("file");
    let server = server();
    let file = dir.path().join("fr_ign_ntf_r93.tif");
    let url = format!("{}/fr_ign_ntf_r93.tif", server.url());

    let dry = projsync(&server, dir.path(), &["--file", "ntf_r93", "--dry-run"]);
    assert_eq!(
        body_of(&dry, dir.path(), &server),
        expected(FILE_NTF, &server)
    );

    let first = projsync(&server, dir.path(), &["--file", "ntf_r93"]);
    assert!(first.status.success(), "{}", stderr_of(&first));
    assert_eq!(
        body_of(&first, dir.path(), &server),
        format!("Total to download: 12345 bytes\nDownloading {url}... (1 / 1)\n")
    );
    assert_eq!(
        std::fs::read(&file).expect("downloaded"),
        b"proj-rs test grid"
    );

    let second = projsync(&server, dir.path(), &["--file", "ntf_r93"]);
    assert!(second.status.success(), "{}", stderr_of(&second));
    assert_eq!(
        body_of(&second, dir.path(), &server),
        format!("{url} already downloaded.\n")
    );
    let _removed = std::fs::remove_file(&file);
}

#[test]
fn unknown_file_fails_and_lists_the_known_ones() {
    let dir = TestDir::new("unknown-file");
    let server = server();
    let output = projsync(&server, dir.path(), &["--file", "non_existing"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr_of(&output), UNKNOWN_FILE);
}

#[test]
fn bbox_selects_the_grids_that_overlap_it() {
    let dir = TestDir::new("bbox");
    let server = server();
    let output = projsync(&server, dir.path(), &["--bbox", "2,49,3,50", "--dry-run"]);
    assert_eq!(
        body_of(&output, dir.path(), &server),
        expected(BBOX_FRANCE, &server)
    );
}

#[test]
fn exclude_world_coverage_drops_the_world_grids() {
    let dir = TestDir::new("world");
    let server = server();
    let output = projsync(
        &server,
        dir.path(),
        &[
            "--bbox",
            "2,49,3,50",
            "--exclude-world-coverage",
            "--dry-run",
        ],
    );
    assert_eq!(
        body_of(&output, dir.path(), &server),
        expected(BBOX_FRANCE_NO_WORLD, &server)
    );
}

#[test]
fn bbox_across_the_antimeridian_finds_the_pacific_grids() {
    let dir = TestDir::new("antimeridian");
    let server = server();
    for window in ["170,-90,-170,90", "170,-90,190,90", "-190,-90,-170,90"] {
        let output = projsync(&server, dir.path(), &["--bbox", window, "--dry-run"]);
        assert_eq!(
            body_of(&output, dir.path(), &server),
            expected(BBOX_ANTIMERIDIAN, &server),
            "--bbox {window}"
        );
    }
}

#[test]
fn verbose_reports_the_files_left_out_by_version() {
    let dir = TestDir::new("verbose");
    let server = server();
    let local = dir.path().join("dummy.geojson");
    std::fs::write(&local, DUMMY).expect("local index");
    let output = Command::new(env!("CARGO_BIN_EXE_projsync"))
        .arg("--endpoint")
        .arg(server.url())
        .arg("--target-dir")
        .arg(dir.path())
        .args([
            "--source-id",
            "au_ga",
            "--verbose",
            "--dry-run",
            "--local-geojson-file",
        ])
        .arg(&local)
        .output()
        .expect("projsync runs");
    assert!(output.status.success(), "{}", stderr_of(&output));
    let body = stdout_of(&output);
    let (_first, rest) = body.split_once('\n').expect("first line");
    assert_eq!(rest, expected(VERBOSE_DUMMY, &server));
}

#[test]
fn a_request_needs_a_selection() {
    let dir = TestDir::new("usage");
    let server = server();
    let output = projsync(&server, dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("At least one of --list-files"));
}

#[test]
fn unknown_option_prints_usage_and_fails() {
    let dir = TestDir::new("options");
    let server = server();
    let output = projsync(&server, dir.path(), &["--no-such-option"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).starts_with("usage: projsync"));
}

#[test]
fn help_and_version_exit_cleanly() {
    let dir = TestDir::new("help");
    let server = server();
    let help = projsync(&server, dir.path(), &["--help"]);
    assert!(help.status.success());
    assert!(stdout_of(&help).starts_with("usage: projsync"));
    let version = projsync(&server, dir.path(), &["--version"]);
    assert!(version.status.success());
    assert!(stdout_of(&version).starts_with("projsync "));
}

/// The index reader, without a process: the nesting limit, the features that are used, and
/// the window of `--bbox` brought into the range of the grids.
mod library {
    use proj_rs::projsync::{Bbox, Geometry, Index};

    #[test]
    fn a_document_nested_too_deeply_is_refused() {
        let open = "[".repeat(200);
        let close = "]".repeat(200);
        let text = format!("{{\"features\": {open}{close}}}");
        let error = Index::parse(&text).expect_err("too deep");
        assert_eq!(error.to_string(), "Too deep nesting in JSON content");
    }

    #[test]
    fn a_document_without_features_is_refused() {
        let error = Index::parse("{\"type\": \"FeatureCollection\"}").expect_err("no features");
        assert_eq!(error.to_string(), "no features member");
    }

    #[test]
    fn only_features_with_a_name_are_read() {
        let text = r#"{"features": [
            {"properties": {"name": "a.tif", "source_id": "s", "file_size": 7}, "geometry": null},
            {"properties": {"source_id": "s"}, "geometry": null},
            {"properties": {"name": 3}},
            "not a feature"
        ]}"#;
        let index = Index::parse(text).expect("index");
        assert_eq!(index.entries().len(), 1);
        let entry = &index.entries()[0];
        assert_eq!(entry.name, "a.tif");
        assert_eq!(entry.file_size, 7);
        assert_eq!(entry.geometry, Geometry::Absent);
    }

    #[test]
    fn a_fractional_file_size_is_not_an_unsigned_integer() {
        let text = r#"{"features": [{"properties": {"name": "a.tif", "file_size": 7.5}}]}"#;
        let index = Index::parse(text).expect("index");
        assert_eq!(index.entries()[0].file_size, 0);
    }

    #[test]
    fn a_window_across_the_antimeridian_is_shifted_as_projsync_does() {
        assert_eq!(
            Bbox::new(170.0, -90.0, -170.0, 90.0),
            Bbox {
                west: 170.0,
                south: -90.0,
                east: 190.0,
                north: 90.0
            }
        );
        assert_eq!(
            Bbox::new(190.0, 0.0, 200.0, 1.0),
            Bbox {
                west: -170.0,
                south: 0.0,
                east: -160.0,
                north: 1.0
            }
        );
        assert_eq!(
            Bbox::new(-190.0, -90.0, -170.0, 90.0),
            Bbox {
                west: 170.0,
                south: -90.0,
                east: 190.0,
                north: 90.0
            }
        );
    }
}
