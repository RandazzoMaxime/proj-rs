//! What the ledger-driven doc tests share: PROJ's totals and the open lists.
#![allow(
    dead_code,
    reason = "each test crate uses a different part of this module"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub(crate) mod tmp_db;

/// `TEST()` names per PROJ 9.9.0 unit file, ported or not.
pub(crate) const UNIT_TOTALS: [(&str, usize); 24] = [
    ("test_c_api.cpp", 152),
    ("test_crs.cpp", 197),
    ("test_factory.cpp", 113),
    ("test_io.cpp", 454),
    ("test_operation.cpp", 162),
    ("test_operationfactory.cpp", 274),
    ("gie_self_tests.cpp", 20),
    ("pj_phi2_test.cpp", 1),
    ("proj_angular_io_test.cpp", 5),
    ("proj_context_test.cpp", 5),
    ("proj_errno_string_test.cpp", 3),
    ("test_common.cpp", 13),
    ("test_coordinates.cpp", 3),
    ("test_datum.cpp", 22),
    ("test_defmodel.cpp", 13),
    ("test_grids.cpp", 10),
    ("test_metadata.cpp", 12),
    ("test_misc.cpp", 1),
    ("test_network.cpp", 26),
    ("test_polyhedral.cpp", 5),
    ("test_projinfo_lib.cpp", 5),
    ("test_tinshift.cpp", 1),
    ("test_tmerczoned.cpp", 8),
    ("test_util.cpp", 2),
];
/// Cases per PROJ CLI, from the yaml files in `tests/cli_yaml` (and `test_gie.yaml`, which
/// `tests/gie_cli.rs` runs).
pub(crate) const CLI_TOTALS: [(&str, usize); 6] = [
    ("cct", 18),
    ("cs2cs", 197),
    ("gie", 4),
    ("invproj", 1),
    ("proj", 494),
    ("projinfo", 111),
];

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
