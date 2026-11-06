//! What the ledger-driven doc tests share: PROJ's totals and the open lists.
#![allow(
    dead_code,
    reason = "each test crate uses a different part of this module"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// `TEST()` names per PROJ 9.9.0 unit file, ported or not.
pub(crate) const UNIT_TOTALS: [(&str, usize); 6] = [
    ("test_c_api.cpp", 152),
    ("test_crs.cpp", 197),
    ("test_factory.cpp", 113),
    ("test_io.cpp", 454),
    ("test_operation.cpp", 162),
    ("test_operationfactory.cpp", 274),
];
/// Cases per PROJ CLI, from the yaml files.
pub(crate) const CLI_TOTALS: [(&str, usize); 4] =
    [("cct", 18), ("cs2cs", 143), ("proj", 4), ("projinfo", 111)];

pub(crate) fn read(name: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)).expect(name)
}

/// Open unit tests per PROJ file, from `docs/UNPORTED-UNIT.md`.
pub(crate) fn unit_open() -> BTreeMap<&'static str, usize> {
    let mut open = BTreeMap::new();
    let mut file = "";
    for line in read("docs/UNPORTED-UNIT.md").lines() {
        if let Some(name) = line.strip_prefix("## ") {
            file = UNIT_TOTALS
                .iter()
                .map(|(n, _)| *n)
                .find(|n| *n == name)
                .unwrap_or("");
        } else if line.starts_with("- `") && !file.is_empty() {
            *open.entry(file).or_default() += 1;
        }
    }
    open
}

/// Open CLI cases per program, from `docs/UNPORTED-CLI.md`.
pub(crate) fn cli_open() -> BTreeMap<&'static str, usize> {
    let mut open = BTreeMap::new();
    for line in read("docs/UNPORTED-CLI.md")
        .lines()
        .filter(|l| l.starts_with("| `test_"))
    {
        let app = line.split('|').nth(2).map_or("", str::trim);
        if let Some((name, _)) = CLI_TOTALS.iter().find(|(n, _)| *n == app) {
            *open.entry(*name).or_default() += 1;
        }
    }
    open
}

/// `(file, cases, passed)` for each vendored `gie` file, from `tests/gie/files.txt`.
pub(crate) fn gie_files() -> Vec<(String, usize, usize)> {
    read("tests/gie/files.txt")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.to_owned();
            Some((
                name,
                fields.next()?.parse().ok()?,
                fields.next()?.parse().ok()?,
            ))
        })
        .collect()
}

/// The data search path of PROJ's test runs (`proj_test_set_properties` in
/// `cmake/ProjTest.cmake`): the vendored data directory, then its `tests`
/// fixtures. `PROJ_DATA` takes this value, as it does in PROJ's ctest runs.
pub(crate) fn fixture_search_path() -> String {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let separator = if cfg!(windows) { ";" } else { ":" };
    format!(
        "{}{separator}{}",
        data.display(),
        data.join("tests").display()
    )
}

/// Put the fixtures on `PROJ_DATA` for this process, with no user-writable
/// directory. The library reads `PROJ_DATA` on each lookup, so the call may
/// come after another test has already resolved a grid.
pub(crate) fn use_fixture_search_path() {
    std::env::set_var("PROJ_DATA", fixture_search_path());
    std::env::set_var("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES");
}
