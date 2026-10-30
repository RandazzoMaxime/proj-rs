//! Runner for PROJ's CLI yaml suites.
//!
//! Every case is executed. A case that matches the yaml leaves
//! `docs/UNPORTED-CLI.md`. A case that does not is listed there. A case that
//! was already absent from that file and then fails is a regression.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_yaml::Value;

const FILES: &[&str] = &[
    "test_cct.yaml",
    "test_cs2cs_various.yaml",
    "test_projinfo.yaml",
    "test_proj.yaml",
];

struct Case {
    id: String,
    exe: String,
    comment: String,
    args: Vec<String>,
    input: String,
    stdout: Option<String>,
    stderr: Option<String>,
    combined: bool,
    exitcode: i32,
    files: Vec<(String, String)>,
    tmp_copies: Vec<String>,
    env: Vec<(String, String)>,
    global_env: Vec<(String, String)>,
    steps: Vec<Step>,
}

enum Step {
    Grep(String),
    GrepNot(String),
    Sub(String, String),
    Head(usize),
    Tail(usize),
    Sort,
}

#[test]
fn cli_yaml_suites() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cli_yaml");
    let mut cases = Vec::new();
    for file in FILES {
        cases.extend(load(&root.join(file), file));
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/UNPORTED-CLI.md");
    let previous = fs::read_to_string(&path).unwrap_or_default();
    let mut failures = Vec::new();
    let mut regressions = Vec::new();
    for case in &cases {
        if let Err(error) = run_case(case) {
            if !previous.contains(&format!("`{}`", case.id)) {
                regressions.push(format!("{}: {error}", case.id));
            }
            failures.push((case, error));
        }
    }
    let rendered = render(&failures);
    if fs::read_to_string(&path).ok().as_deref() != Some(rendered.as_str()) {
        fs::write(&path, &rendered).expect("write UNPORTED-CLI.md");
    }
    assert!(
        regressions.is_empty(),
        "previously passing CLI cases failed:\n{}",
        regressions.join("\n")
    );
}

fn render(failures: &[(&Case, String)]) -> String {
    let mut out = String::from(
        "# Unported CLI cases\n\n\
         The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.\n\
         A row remains when that case does not match the yaml.\n\n\
         | id | app | reason | comment |\n\
         |---|---|---|---|\n",
    );
    for (case, error) in failures {
        let comment = if case.comment.is_empty() {
            "-".to_owned()
        } else {
            case.comment.replace('|', "/")
        };
        let mut reason = error.replace(['\n', '\r', '|'], " ");
        if reason.len() > 160 {
            reason.truncate(160);
        }
        out.push_str(&format!(
            "| `{}` | {} | {reason} | {comment} |\n",
            case.id, case.exe
        ));
    }
    out
}

fn load(path: &Path, file: &str) -> Vec<Case> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("{file}: {error}"));
    let root: Value = serde_yaml::from_str(&text).unwrap_or_else(|error| panic!("{file}: {error}"));
    let exe = root
        .get("exe")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let global_env = env_pairs(root.get("env"));
    let tests = root
        .get("tests")
        .and_then(Value::as_sequence)
        .unwrap_or_else(|| panic!("{file} has no tests"));
    tests
        .iter()
        .enumerate()
        .map(|(index, case)| one_case(file, &exe, &global_env, index + 1, case))
        .collect()
}

fn one_case(
    file: &str,
    exe: &str,
    global_env: &[(String, String)],
    index: usize,
    case: &Value,
) -> Case {
    let comment = case
        .get("comment")
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    let input = text_field(case, "in")
        .or_else(|| text_field(case, "input"))
        .unwrap_or_default();
    let stdout = text_field(case, "stdout");
    let stderr = text_field(case, "stderr");
    let out = text_field(case, "out");
    let (stdout, combined) = if let Some(out) = out {
        (Some(out), true)
    } else {
        (stdout, false)
    };
    Case {
        id: format!("{file}#{index}"),
        exe: exe.to_owned(),
        comment,
        args: args_of(case.get("args")),
        input,
        stdout,
        stderr,
        combined,
        exitcode: case
            .get("exitcode")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .try_into()
            .unwrap_or(1),
        files: files_of(case.get("file")),
        tmp_copies: copies_of(case.get("tmpdir")),
        env: env_pairs(case.get("env")),
        global_env: global_env.to_vec(),
        steps: steps_of(case),
    }
}

fn text_field(case: &Value, key: &str) -> Option<String> {
    match case.get(key) {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Bool(value)) => Some(value.to_string()),
        Some(Value::Number(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn env_pairs(value: Option<&Value>) -> Vec<(String, String)> {
    let Some(mapping) = value.and_then(Value::as_mapping) else {
        return Vec::new();
    };
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?.to_owned();
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Bool(flag) => flag.to_string(),
                Value::Number(number) => number.to_string(),
                Value::Null => String::new(),
                _ => return None,
            };
            Some((key, text))
        })
        .collect()
}

fn files_of(value: Option<&Value>) -> Vec<(String, String)> {
    let Some(value) = value else {
        return Vec::new();
    };
    let items = if value.is_sequence() {
        value.as_sequence().cloned().unwrap_or_default()
    } else {
        vec![value.clone()]
    };
    items
        .iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_owned();
            let content = match item.get("content")? {
                Value::String(text) => text.clone(),
                Value::Null => String::new(),
                other => other.as_str().unwrap_or("").to_owned(),
            };
            Some((name, content))
        })
        .collect()
}

fn copies_of(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|item| item.get("copy"))
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn steps_of(case: &Value) -> Vec<Step> {
    let Some(mapping) = case.as_mapping() else {
        return Vec::new();
    };
    let mut steps = Vec::new();
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            continue;
        };
        match name {
            "grep" => push_patterns(value, &mut steps, LineKeep::Matching),
            "grep-v" => push_patterns(value, &mut steps, LineKeep::Other),
            "sub" => {
                if let Some(pair) = value.as_sequence() {
                    if let (Some(pat), Some(repl)) = (
                        pair.first().and_then(Value::as_str),
                        pair.get(1).and_then(Value::as_str),
                    ) {
                        steps.push(Step::Sub(pat.to_owned(), repl.to_owned()));
                    }
                }
            }
            "head" => {
                if let Some(count) = value.as_u64() {
                    steps.push(Step::Head(usize::try_from(count).unwrap_or(0)));
                }
            }
            "tail" => {
                if let Some(count) = value.as_u64() {
                    steps.push(Step::Tail(usize::try_from(count).unwrap_or(0)));
                }
            }
            "sort" => steps.push(Step::Sort),
            _ => {}
        }
    }
    steps
}

enum LineKeep {
    Matching,
    Other,
}

fn push_patterns(value: &Value, steps: &mut Vec<Step>, keep: LineKeep) {
    let patterns = if let Some(text) = value.as_str() {
        vec![text.to_owned()]
    } else {
        value
            .as_sequence()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    for pattern in patterns {
        match keep {
            LineKeep::Matching => steps.push(Step::Grep(pattern)),
            LineKeep::Other => steps.push(Step::GrepNot(pattern)),
        }
    }
}

fn args_of(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(text) = value.as_str() {
        return split_shell(text);
    }
    value
        .as_sequence()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn split_shell(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut quote = None;
    let mut any = false;
    while let Some(ch) = chars.next() {
        if let Some(mark) = quote {
            if ch == mark {
                quote = None;
                any = true;
            } else if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                any = true;
            }
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    any = true;
                }
            }
            ch if ch.is_whitespace() => {
                if any {
                    out.push(std::mem::take(&mut current));
                    any = false;
                }
            }
            ch => {
                current.push(ch);
                any = true;
            }
        }
    }
    if any {
        out.push(current);
    }
    out
}

fn run_case(case: &Case) -> Result<(), String> {
    let bin = match case.exe.as_str() {
        "cct" => env!("CARGO_BIN_EXE_cct"),
        "cs2cs" => env!("CARGO_BIN_EXE_cs2cs"),
        "projinfo" => env!("CARGO_BIN_EXE_projinfo"),
        "proj" => env!("CARGO_BIN_EXE_proj"),
        other => return Err(format!("no binary for {other}")),
    };
    let scratch = std::env::temp_dir().join(format!(
        "proj-rs-cli-{}-{}",
        std::process::id(),
        case.id.replace(['#', ':'], "_")
    ));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).map_err(|error| error.to_string())?;
    let outcome = run_in(case, bin, &scratch);
    let _ = fs::remove_dir_all(&scratch);
    outcome
}

fn run_in(case: &Case, bin: &str, scratch: &Path) -> Result<(), String> {
    let tmp = scratch.join("tmp");
    fs::create_dir_all(&tmp).map_err(|error| error.to_string())?;
    let tmp_text = tmp.to_string_lossy().replace('\\', "/");
    let pwd = scratch.to_string_lossy().replace('\\', "/");
    for source in &case.tmp_copies {
        let source = subst(source, &tmp_text, &pwd);
        let from = PathBuf::from(&source);
        let name = from
            .file_name()
            .ok_or_else(|| format!("tmpdir copy {source}"))?;
        fs::copy(&from, tmp.join(name)).map_err(|error| format!("copy {source}: {error}"))?;
    }
    for (name, content) in &case.files {
        let name = subst(name, &tmp_text, &pwd);
        let path = scratch.join(&name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut content = content.clone();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        fs::write(&path, content).map_err(|error| format!("write {name}: {error}"))?;
    }
    let args: Vec<String> = case
        .args
        .iter()
        .map(|arg| subst(arg, &tmp_text, &pwd))
        .collect();
    let mut command = Command::new(bin);
    command
        .args(&args)
        .current_dir(scratch)
        .env("PROJ_NETWORK", "OFF")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in case.global_env.iter().chain(case.env.iter()) {
        command.env(key, subst_env(value, &tmp_text, &pwd));
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(case.input.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    let code = output.status.code().unwrap_or(1);
    if code != case.exitcode {
        return Err(format!(
            "exit {code} expected {}\n{}",
            case.exitcode,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    let mut stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
    if case.combined {
        stdout.push_str(&stderr);
        stderr.clear();
    }
    for step in &case.steps {
        apply_step(step, &mut stdout);
        apply_step(step, &mut stderr);
    }
    if let Some(expected) = &case.stdout {
        compare_stream(&case.exe, expected, &stdout)?;
    }
    if let Some(expected) = &case.stderr {
        compare_text(expected, &stderr).map_err(|error| format!("stderr {error}"))?;
    }
    Ok(())
}

fn proj_data_dir() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .to_string_lossy()
        .replace('\\', "/")
}

fn subst(text: &str, tmp: &str, pwd: &str) -> String {
    let data = proj_data_dir();
    text.replace("${PROJ_DATA}", &data)
        .replace("$PROJ_DATA", &data)
        .replace("${PWD}", pwd)
        .replace("$PWD", pwd)
        .replace("${tmpdir}", tmp)
        .replace("$tmpdir", tmp)
}

fn subst_env(value: &str, tmp: &str, pwd: &str) -> String {
    // PROJ joins search paths with `:`. Windows needs `;` before `$tmpdir` expands
    // to a drive letter, or that colon would be rewritten too.
    let value = if std::path::MAIN_SEPARATOR == '\\' {
        value.replace(':', ";")
    } else {
        value.to_owned()
    };
    subst(&value, tmp, pwd)
}

fn apply_step(step: &Step, text: &mut String) {
    match step {
        Step::Grep(pattern) => {
            *text = filter_lines(text, pattern, LineKeep::Matching);
        }
        Step::GrepNot(pattern) => {
            *text = filter_lines(text, pattern, LineKeep::Other);
        }
        Step::Sub(pattern, replacement) => {
            *text = text
                .lines()
                .map(|line| re_sub(pattern, replacement, line))
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
        }
        Step::Head(count) => {
            *text = text.lines().take(*count).collect::<Vec<_>>().join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
        }
        Step::Tail(count) => {
            let lines: Vec<&str> = text.lines().collect();
            let start = lines.len().saturating_sub(*count);
            *text = lines[start..].join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
        }
        Step::Sort => {
            let mut lines: Vec<&str> = text.lines().collect();
            lines.sort_unstable();
            *text = lines.join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
        }
    }
}

fn filter_lines(text: &str, pattern: &str, keep: LineKeep) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let hit = re_search(pattern, line);
        let write = match keep {
            LineKeep::Matching => hit,
            LineKeep::Other => !hit,
        };
        if write {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn compare_stream(exe: &str, expected: &str, got: &str) -> Result<(), String> {
    if exe == "projinfo" || exe == "proj" || !coordinate_text(expected) {
        return compare_text(expected, got);
    }
    compare_numbers(expected, got)
}

fn coordinate_text(text: &str) -> bool {
    text.lines().any(|line| {
        line.split_whitespace()
            .any(|token| token_number(token).is_some())
    }) && !text.contains("GEOGCRS")
        && !text.contains("PROJ.4")
        && !text.contains("WKT")
}

fn compare_text(expected: &str, got: &str) -> Result<(), String> {
    let expected_lines = content_lines(expected);
    let got_lines = content_lines(got);
    if expected_lines != got_lines {
        return Err(format!("text mismatch\nexpected:\n{expected}\ngot:\n{got}"));
    }
    Ok(())
}

fn content_lines(text: &str) -> Vec<&str> {
    let text = text.trim_end_matches(['\n', '\r']);
    if text.is_empty() {
        Vec::new()
    } else {
        text.lines().collect()
    }
}

fn compare_numbers(expected: &str, got: &str) -> Result<(), String> {
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

fn re_search(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    (0..=text.len()).any(|start| match_at(&pattern, 0, &text, start).is_some())
}

fn re_sub(pattern: &str, replacement: &str, text: &str) -> String {
    let pattern: Vec<char> = pattern.chars().collect();
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index <= chars.len() {
        if let Some(end) = match_at(&pattern, 0, &chars, index) {
            if end > index || pattern.is_empty() {
                out.push_str(replacement);
                index = end.max(index + 1);
                if end == index - 1 && end == chars.len() {
                    break;
                }
                continue;
            }
        }
        if index == chars.len() {
            break;
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

fn match_at(pattern: &[char], from: usize, text: &[char], at: usize) -> Option<usize> {
    if from == pattern.len() {
        return Some(at);
    }
    if top_alt(&pattern[from..]) {
        for alternative in split_alt(&pattern[from..]) {
            if let Some(end) = match_at(alternative, 0, text, at) {
                return Some(end);
            }
        }
        return None;
    }
    let (width, kind) = atom(&pattern[from..])?;
    let optional = pattern.get(from + width) == Some(&'?');
    let next = from + width + usize::from(optional);
    match kind {
        Atom::AnyStar => {
            let mut end = text.len();
            loop {
                if let Some(done) = match_at(pattern, next, text, end) {
                    return Some(done);
                }
                if end == at {
                    break;
                }
                end -= 1;
            }
            None
        }
        Atom::Any => {
            if at < text.len() {
                if let Some(done) = match_at(pattern, next, text, at + 1) {
                    return Some(done);
                }
            }
            if optional {
                match_at(pattern, next, text, at)
            } else {
                None
            }
        }
        Atom::Lit(ch) => {
            if at < text.len() && text[at] == ch {
                if let Some(done) = match_at(pattern, next, text, at + 1) {
                    return Some(done);
                }
            }
            if optional {
                match_at(pattern, next, text, at)
            } else {
                None
            }
        }
        Atom::Group(inner) => {
            if let Some(end) = match_at(&inner, 0, text, at) {
                if let Some(done) = match_at(pattern, next, text, end) {
                    return Some(done);
                }
            }
            if optional {
                match_at(pattern, next, text, at)
            } else {
                None
            }
        }
    }
}

enum Atom {
    Lit(char),
    Any,
    AnyStar,
    Group(Vec<char>),
}

fn atom(pattern: &[char]) -> Option<(usize, Atom)> {
    if pattern.is_empty() {
        return None;
    }
    if pattern[0] == '(' {
        let mut depth = 0;
        for (index, ch) in pattern.iter().enumerate() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((index + 1, Atom::Group(pattern[1..index].to_vec())));
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    if pattern[0] == '.' && pattern.get(1) == Some(&'*') {
        return Some((2, Atom::AnyStar));
    }
    if pattern[0] == '.' {
        return Some((1, Atom::Any));
    }
    if pattern[0] == '\\' {
        return pattern.get(1).map(|ch| (2, Atom::Lit(*ch)));
    }
    Some((1, Atom::Lit(pattern[0])))
}

fn top_alt(pattern: &[char]) -> bool {
    let mut depth = 0;
    for ch in pattern {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '|' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn split_alt(pattern: &[char]) -> Vec<&[char]> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, ch) in pattern.iter().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '|' if depth == 0 => {
                parts.push(&pattern[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&pattern[start..]);
    parts
}
