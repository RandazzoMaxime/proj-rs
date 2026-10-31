//! The counts in `README.md` and `ROADMAP.md` come from the two ledgers.
//!
//! A number sits between `<!--gen:NAME-->` and `<!--/gen-->`. This test
//! recomputes it from `docs/UNPORTED-UNIT.md` and `docs/UNPORTED-CLI.md` and
//! fails when the text differs. `UPDATE_DOCS=1 cargo test --test docs_in_sync`
//! rewrites the files instead.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// `TEST()` names per PROJ 9.9.0 unit file, ported or not.
const UNIT_TOTALS: [(&str, usize); 6] = [
    ("test_c_api.cpp", 152),
    ("test_crs.cpp", 197),
    ("test_factory.cpp", 113),
    ("test_io.cpp", 454),
    ("test_operation.cpp", 162),
    ("test_operationfactory.cpp", 274),
];
/// Cases per PROJ CLI, from the yaml files.
const CLI_TOTALS: [(&str, usize); 4] =
    [("cct", 18), ("cs2cs", 143), ("proj", 4), ("projinfo", 111)];

fn read(name: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)).expect(name)
}

fn icon(open: usize, total: usize) -> &'static str {
    if open == 0 {
        "✅"
    } else if open * 2 > total {
        "🔴"
    } else {
        "🟡"
    }
}

/// Every generated value, by name.
fn values() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut unit_open: BTreeMap<&str, usize> = BTreeMap::new();
    let mut file = "";
    for line in read("docs/UNPORTED-UNIT.md").lines() {
        if let Some(name) = line.strip_prefix("## ") {
            file = UNIT_TOTALS
                .iter()
                .map(|(n, _)| *n)
                .find(|n| *n == name)
                .unwrap_or("");
        } else if line.starts_with("- `") && !file.is_empty() {
            *unit_open.entry(file).or_default() += 1;
        }
    }
    let mut cli_open: BTreeMap<&str, usize> = BTreeMap::new();
    for line in read("docs/UNPORTED-CLI.md")
        .lines()
        .filter(|l| l.starts_with("| `test_"))
    {
        let app = line.split('|').nth(2).map_or("", str::trim);
        if let Some((name, _)) = CLI_TOTALS.iter().find(|(n, _)| *n == app) {
            *cli_open.entry(name).or_default() += 1;
        }
    }
    let unit_total: usize = UNIT_TOTALS.iter().map(|(_, t)| t).sum();
    let open_total: usize = unit_open.values().sum();
    out.insert("unit_ported".into(), (unit_total - open_total).to_string());
    out.insert("unit_open".into(), open_total.to_string());
    out.insert("unit_icon".into(), icon(open_total, unit_total).into());
    let mut by_size: Vec<_> = unit_open.iter().collect();
    by_size.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let largest: Vec<String> = by_size
        .iter()
        .take(5)
        .map(|(n, c)| format!("`{}` ({c})", n.trim_end_matches(".cpp")))
        .collect();
    out.insert("unit_largest".into(), largest.join(", "));
    for (name, total) in UNIT_TOTALS {
        let open = unit_open.get(name).copied().unwrap_or(0);
        let stem = name.trim_end_matches(".cpp");
        out.insert(format!("{stem}_ported"), (total - open).to_string());
        out.insert(format!("{stem}_open"), open.to_string());
    }
    let cli_total: usize = CLI_TOTALS.iter().map(|(_, t)| t).sum();
    let cli_open_total: usize = cli_open.values().sum();
    out.insert("cli_total".into(), cli_total.to_string());
    out.insert(
        "cli_matched".into(),
        (cli_total - cli_open_total).to_string(),
    );
    out.insert("cli_open".into(), cli_open_total.to_string());
    for (name, total) in CLI_TOTALS {
        let open = cli_open.get(name).copied().unwrap_or(0);
        out.insert(format!("cli_{name}_ported"), (total - open).to_string());
        out.insert(format!("cli_{name}_open"), open.to_string());
        out.insert(format!("cli_{name}_icon"), icon(open, total).into());
    }
    let mut apps: Vec<_> = cli_open.iter().filter(|(_, c)| **c > 0).collect();
    apps.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let parts: Vec<String> = apps.iter().map(|(n, c)| format!("`{n}` {c}")).collect();
    out.insert("cli_breakdown".into(), parts.join(", "));
    out
}

/// `text` with every `<!--gen:NAME-->…<!--/gen-->` span recomputed.
fn render(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--gen:") {
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        let name_end = tail.find("-->").expect("open marker end");
        let name = &tail["<!--gen:".len()..name_end];
        let body = &tail[name_end + 3..];
        let close = body.find("<!--/gen-->").expect("close marker");
        let value = values
            .get(name)
            .unwrap_or_else(|| panic!("unknown gen value {name}"));
        out.push_str(&tail[..name_end + 3]);
        out.push_str(value);
        out.push_str("<!--/gen-->");
        rest = &body[close + "<!--/gen-->".len()..];
    }
    out.push_str(rest);
    out
}

#[test]
fn counts_in_docs_match_the_ledgers() {
    let values = values();
    for name in ["README.md", "ROADMAP.md"] {
        let current = read(name);
        let expected = render(&current, &values);
        if std::env::var_os("UPDATE_DOCS").is_some() {
            fs::write(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name),
                &expected,
            )
            .expect(name);
        } else {
            assert!(
                current == expected,
                "{name} is out of date with the ledgers: run `UPDATE_DOCS=1 cargo test --test docs_in_sync`"
            );
        }
    }
}
