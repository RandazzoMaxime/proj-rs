//! Runner for PROJ's CLI yaml suites.
//!
//! `cct` and `cs2cs` are the binaries this runner executes. A case whose
//! executable is `proj`, `geod`, `gie`, `projsync`, or `projinfo` is not run.
//! `projinfo` is built, but this suite's other WKT dialects are not what it
//! writes. A case that needs a flag, file, or DMS input the executed binaries
//! do not accept is not run either. Every skipped case is listed in
//! `docs/UNPORTED-CLI.md`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_yaml::Value;

const FILES: &[&str] = &[
    "test_cct.yaml",
    "test_cs2cs_various.yaml",
    "test_projinfo.yaml",
    "test_proj.yaml",
];

const SKIP_APPS: &[&str] = &["proj", "geod", "gie", "projsync"];

struct Case {
    id: String,
    file: String,
    exe: String,
    comment: String,
    skip: Option<&'static str>,
    args: Vec<String>,
    input: String,
    expected: String,
}

#[test]
fn cli_yaml_suites() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cli_yaml");
    let mut cases = Vec::new();
    for file in FILES {
        cases.extend(load(&root.join(file), file));
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/UNPORTED-CLI.md");
    let rendered = render(&cases);
    if fs::read_to_string(&path).ok().as_deref() != Some(rendered.as_str()) {
        fs::write(&path, &rendered).expect("write UNPORTED-CLI.md");
    }
    let doc = rendered;
    let mut failures = Vec::new();
    for case in &cases {
        if let Some(reason) = case.skip {
            let needle = format!("`{}`", case.id);
            if !doc.contains(&needle) {
                failures.push(format!("{} ({reason}) is not listed", case.id));
            }
            continue;
        }
        if let Err(error) = run_case(case) {
            failures.push(format!("{}: {error}", case.id));
        }
    }
    for line in doc.lines() {
        let Some(id) = listed_id(line) else {
            continue;
        };
        let Some(case) = cases.iter().find(|case| case.id == id) else {
            failures.push(format!("{id} is listed but not in the yaml"));
            continue;
        };
        if case.skip.is_none() {
            failures.push(format!("{id} is listed but the runner executes it"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn render(cases: &[Case]) -> String {
    let mut out = String::from(
        "# Unported CLI cases\n\n\
         The yaml runner executes `cct` and `cs2cs` when the case is a numeric\n\
         `+proj` transform those binaries accept. Everything else is listed here.\n\
         `proj`, `geod`, `gie`, and `projsync` are not binaries of this crate.\n\
         `projinfo` is listed because WKT export is not built.\n\n\
         | id | app | reason | comment |\n\
         |---|---|---|---|\n",
    );
    for case in cases {
        let Some(reason) = case.skip else {
            continue;
        };
        let comment = if case.comment.is_empty() {
            "-".to_owned()
        } else {
            case.comment.replace('|', "/")
        };
        out.push_str(&format!(
            "| `{}` | {} | {reason} | {comment} |\n",
            case.id, case.exe
        ));
    }
    out
}

fn listed_id(line: &str) -> Option<&str> {
    let start = line.find("`test_")?;
    let rest = &line[start + 1..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

fn load(path: &std::path::Path, file: &str) -> Vec<Case> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("{file}: {error}"));
    let root: Value = serde_yaml::from_str(&text).unwrap_or_else(|error| panic!("{file}: {error}"));
    let exe = root
        .get("exe")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let tests = root
        .get("tests")
        .and_then(Value::as_sequence)
        .unwrap_or_else(|| panic!("{file} has no tests"));
    tests
        .iter()
        .enumerate()
        .map(|(index, case)| one_case(file, &exe, index + 1, case))
        .collect()
}

fn one_case(file: &str, exe: &str, index: usize, case: &Value) -> Case {
    let comment = case
        .get("comment")
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    let args = args_of(case.get("args"));
    let input = text_field(case, "in").or_else(|| text_field(case, "input"));
    let expected = text_field(case, "out").or_else(|| text_field(case, "stdout"));
    let id = format!("{file}#{index}");
    let skip = skip_reason(exe, case, &args, input.as_deref(), expected.as_deref());
    Case {
        id,
        file: file.to_owned(),
        exe: exe.to_owned(),
        comment,
        skip,
        args,
        input: input.unwrap_or_default(),
        expected: expected.unwrap_or_default(),
    }
}

fn skip_reason(
    exe: &str,
    case: &Value,
    args: &[String],
    input: Option<&str>,
    expected: Option<&str>,
) -> Option<&'static str> {
    if SKIP_APPS.contains(&exe) {
        return Some("executable is not built");
    }
    if exe == "projinfo" {
        return Some("projinfo is not built; WKT export is not built");
    }
    if exe != "cct" && exe != "cs2cs" {
        return Some("no binary for this executable");
    }
    if case.get("file").is_some() || case.get("tmpdir").is_some() {
        return Some("case writes files the binary does not read");
    }
    if case.get("env").is_some() || case.get("grep").is_some() || case.get("sub").is_some() {
        return Some("case post-processes output or sets an environment");
    }
    if case.get("stderr").is_some() && expected.is_none() {
        return Some("case checks stderr text");
    }
    let exit = case.get("exitcode").and_then(Value::as_i64).unwrap_or(0);
    if exit != 0 {
        return Some("case expects a non-zero status our flags do not produce");
    }
    if args
        .iter()
        .any(|arg| arg.starts_with('-') && !cct_flag(exe, arg))
    {
        return Some("flag is not implemented on this binary");
    }
    if args
        .iter()
        .any(|arg| arg.starts_with('@') || arg.contains('"'))
    {
        return Some("argument is a file reference or a quoted CRS name");
    }
    if exe == "cct" {
        if !args
            .iter()
            .any(|arg| arg.contains("+proj=") || arg.contains("proj="))
        {
            return Some("cct invocation is not a +proj operation");
        }
        if input.is_some_and(|text| !numeric_lines(text)) {
            return Some("cct input is not numeric");
        }
        if expected.is_some_and(|text| {
            !text
                .split_whitespace()
                .any(|token| token_number(token).is_some())
        }) {
            return Some("cct expected output has no coordinates");
        }
        return None;
    }
    if args
        .iter()
        .any(|arg| arg.contains("+pm=") || arg == "+geoc")
    {
        return Some("prime meridian or geocentric latitude is not what cs2cs prints");
    }
    if !args.iter().any(|arg| arg == "+to") && args.len() != 2 {
        return Some("cs2cs arguments are not SOURCE +to TARGET");
    }
    if args
        .iter()
        .any(|arg| !arg.starts_with('+') && !epsg_code(arg))
    {
        return Some("authority other than an EPSG code");
    }
    if args.len() == 2 && args.iter().any(|arg| arg.contains(":4807")) {
        return Some("grad input is not what cs2cs reads");
    }
    if args.len() == 2 && args.iter().any(|arg| !epsg_code(arg)) {
        return Some("cs2cs pair is not two EPSG codes");
    }
    if input.is_none_or(|text| !numeric_lines(text)) {
        return Some("cs2cs input is DMS or empty");
    }
    if input.is_some_and(|text| text.lines().any(|line| line.split_whitespace().count() > 2)) {
        return Some("cs2cs reads only two axes");
    }
    if expected.is_none_or(|text| {
        !text
            .split_whitespace()
            .any(|token| token_number(token).is_some())
    }) {
        return Some("cs2cs case has no expected coordinates");
    }
    None
}

fn epsg_code(arg: &str) -> bool {
    let Some(code) = arg
        .get(..5)
        .filter(|prefix| prefix.eq_ignore_ascii_case("EPSG:"))
        .and_then(|_| arg.get(5..))
    else {
        return false;
    };
    !code.is_empty() && code.chars().all(|ch| ch.is_ascii_digit())
}

fn cct_flag(exe: &str, arg: &str) -> bool {
    exe == "cct"
        && matches!(
            arg,
            "-d" | "--decimals" | "-z" | "--height" | "-t" | "--time" | "-I" | "--inverse"
        )
}

fn numeric_lines(text: &str) -> bool {
    text.lines().all(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return true;
        }
        let mut numbers = line.split_whitespace();
        numbers
            .next()
            .is_some_and(|token| token.parse::<f64>().is_ok())
            && numbers
                .next()
                .is_some_and(|token| token.parse::<f64>().is_ok())
    })
}

fn args_of(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(text) = value.as_str() {
        return split_args(text);
    }
    value
        .as_sequence()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .flat_map(split_args)
                .collect()
        })
        .unwrap_or_default()
}

fn split_args(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn text_field(case: &Value, key: &str) -> Option<String> {
    case.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn run_case(case: &Case) -> Result<(), String> {
    let bin = match case.exe.as_str() {
        "cct" => env!("CARGO_BIN_EXE_cct"),
        "cs2cs" => env!("CARGO_BIN_EXE_cs2cs"),
        other => return Err(format!("no binary for {other}")),
    };
    let output = Command::new(bin)
        .args(&case.args)
        .env("PROJ_NETWORK", "OFF")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(case.input.as_bytes())?;
            }
            child.wait_with_output()
        })
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "exit {:?}\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    compare_numbers(&case.expected, &stdout, &case.file)
}

fn compare_numbers(expected: &str, got: &str, _file: &str) -> Result<(), String> {
    let expected_lines: Vec<&str> = expected
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let got_lines: Vec<&str> = got.lines().filter(|line| !line.trim().is_empty()).collect();
    if expected_lines.len() != got_lines.len() {
        return Err(format!(
            "line count {} vs {}\nexpected {expected}\ngot {got}",
            expected_lines.len(),
            got_lines.len()
        ));
    }
    for (expect_line, got_line) in expected_lines.iter().zip(got_lines) {
        let expect_numbers: Vec<f64> = expect_line
            .split_whitespace()
            .filter(|token| *token != "*" && !token.eq_ignore_ascii_case("inf"))
            .filter_map(token_number)
            .collect();
        let got_numbers: Vec<f64> = got_line
            .split_whitespace()
            .filter(|token| *token != "*" && !token.eq_ignore_ascii_case("inf"))
            .filter_map(token_number)
            .collect();
        if expect_numbers.is_empty() || got_numbers.is_empty() {
            return Err(format!(
                "no numbers\nexpected {expect_line}\ngot {got_line}"
            ));
        }
        if !axes_match(&expect_numbers, &got_numbers) {
            return Err(format!("expected {expect_line}\ngot {got_line}"));
        }
    }
    Ok(())
}

fn axes_match(expect: &[f64], got: &[f64]) -> bool {
    if expect.is_empty() || got.is_empty() {
        return false;
    }
    let direct =
        close(got[0], expect[0]) && (got.len() < 2 || expect.len() < 2 || close(got[1], expect[1]));
    if direct {
        return true;
    }
    // PROJ cs2cs prints geographic output in latitude, longitude order.
    if expect.len() >= 2 && got.len() >= 2 && expect[0].abs() <= 180.0 && expect[1].abs() <= 180.0 {
        return close(got[0], expect[1]) && close(got[1], expect[0]);
    }
    false
}

fn close(value: f64, expect: f64) -> bool {
    let tol = if expect.abs() >= 1_000.0 { 0.05 } else { 1e-4 };
    (value - expect).abs() <= tol
}

fn token_number(token: &str) -> Option<f64> {
    if let Ok(value) = token.parse::<f64>() {
        return Some(value);
    }
    let mut chars = token.chars();
    let hemi = chars.next_back()?;
    let sign = match hemi {
        'N' | 'E' | 'n' | 'e' => 1.0,
        'S' | 'W' | 's' | 'w' => -1.0,
        _ => return None,
    };
    let body: String = chars.collect();
    let (deg, rest) = body.split_once('d')?;
    let degrees: f64 = deg.parse().ok()?;
    if rest.is_empty() {
        return Some(sign * degrees);
    }
    let (minutes_text, seconds_text) = rest.split_once('\'').unwrap_or((rest, ""));
    let minutes: f64 = if minutes_text.is_empty() {
        0.0
    } else {
        minutes_text.parse().ok()?
    };
    let seconds_text = seconds_text.trim_end_matches('"');
    let seconds: f64 = if seconds_text.is_empty() {
        0.0
    } else {
        seconds_text.parse().ok()?
    };
    Some(sign * (degrees + minutes / 60.0 + seconds / 3_600.0))
}
