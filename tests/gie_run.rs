//! Vendored PROJ 9.9 `gie` suite.
//!
//! The runner is `proj_rs::gie`. `tests/gie/passing.txt` is the ratchet: the
//! live pass count must equal that file.

mod common;

use proj_rs::gie::{run_text, Config, Counts};
use std::fs;
use std::path::PathBuf;

/// Files PROJ runs only when the network is on: `test/CMakeLists.txt` adds them
/// with `PROJ_NETWORK=ON` and no test without it.
const NETWORK_FILES: &[&str] = &["nkg.gie", "epsg_grid.gie", "tinshift_gpkg_network.gie"];

fn network_enabled() -> bool {
    std::env::var("PROJ_NETWORK").is_ok_and(|value| {
        value.eq_ignore_ascii_case("on")
            || value.eq_ignore_ascii_case("yes")
            || value.eq_ignore_ascii_case("true")
    })
}

/// The directories `common::fixture_search_path` puts on `PROJ_DATA`, which is
/// where a `require_grid` name is looked for.
fn test_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Config {
        network: network_enabled(),
        grid_dirs: vec![root.join("data"), root.join("data").join("tests")],
    }
}

fn main_counts() -> (Counts, Vec<(String, Counts)>, Vec<String>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/gie");
    let mut files: Vec<_> = fs::read_dir(&root)
        .expect("vendored gie directory")
        .map(|entry| entry.expect("gie entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "gie"))
        .collect();
    files.sort();
    let config = test_config();
    let mut total = Counts::default();
    let mut per_file = Vec::new();
    let mut failures = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        if !network_enabled() && NETWORK_FILES.contains(&name.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read gie");
        let report = run_text(&text, &config);
        for note in report.notes {
            failures.push(format!("{name}: {note}"));
        }
        total.passed += report.counts.passed;
        total.failed += report.counts.failed;
        total.skipped += report.counts.skipped;
        per_file.push((name, report.counts));
    }
    (total, per_file, failures)
}

#[test]
fn vendored_gie_ratchet() {
    common::use_fixture_search_path();
    let (total, per_file, failures) = main_counts();
    for (name, counts) in &per_file {
        println!(
            "{name} passed={} failed={} skipped={}",
            counts.passed, counts.failed, counts.skipped
        );
    }
    println!(
        "total passed={} failed={} skipped={}",
        total.passed, total.failed, total.skipped
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/gie/passing.txt");
    let mode = if network_enabled() {
        "network"
    } else {
        "offline"
    };
    let expected: u64 = fs::read_to_string(&path)
        .expect("passing.txt")
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some(mode)).then(|| fields.next()?.parse().ok())?
        })
        .expect("passing.txt has no count for this mode");
    assert_eq!(
        total.passed, expected,
        "update the {mode} line of tests/gie/passing.txt to the live pass count"
    );
    // `files.txt` holds `name cases passed` for every file, network ones included.
    // The progress chart in the README is drawn from it.
    let files =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/gie/files.txt"))
            .expect("files.txt");
    for (name, counts) in &per_file {
        let line = files
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
            .unwrap_or_else(|| panic!("tests/gie/files.txt has no line for {name}"));
        let numbers: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .map(|n| n.parse().expect("number"))
            .collect();
        assert_eq!(
            numbers,
            [
                counts.passed + counts.failed + counts.skipped,
                counts.passed
            ],
            "update the {name} line of tests/gie/files.txt (name cases passed)"
        );
    }
    let builtins = per_file
        .iter()
        .find(|(name, _)| name == "builtins.gie")
        .expect("builtins");
    assert_eq!(builtins.1.failed, 0);
    assert_eq!(builtins.1.skipped, 0, "builtins.gie still skips operations");
}
