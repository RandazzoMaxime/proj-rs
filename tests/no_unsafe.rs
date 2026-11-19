//! No new `unsafe` in the repository.
//!
//! The rule of `CLAUDE.md` and `CONTRIBUTING.md` is that the repository has no `unsafe` code, except
//! in `capi/` (the C API, which cannot be written without it): `capi/` is not counted here.
//! The `unsafe` that existed when the rule was made is listed in `tools/unsafe-debt.txt`, one
//! line `path count` per file, and it only goes down: this test fails when a file not in the list
//! has an `unsafe`, when a listed file has more than its count (a new `unsafe`), and when a listed
//! file has fewer (the list is then lowered, so the debt cannot grow back).
//! `UPDATE_UNSAFE_DEBT=1 cargo test --test no_unsafe` rewrites the list, and a reviewer reads
//! its diff: it must only shrink.
//!
//! An `unsafe` is the word `unsafe` in code: comments, doc comments and string literals do not
//! count.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LIST: &str = "tools/unsafe-debt.txt";

/// The number of `unsafe` words in the code of a Rust source, outside comments and strings.
fn count_unsafe(source: &str) -> usize {
    let chars: Vec<char> = source.chars().collect();
    let mut count = 0;
    let mut i = 0;
    let word_char = |c: char| c.is_alphanumeric() || c == '_';
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            // Block comments nest in Rust.
            let mut depth = 1;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if c == 'r'
            && matches!(next, Some('"' | '#'))
            && (i == 0 || !word_char(chars[i - 1]))
        {
            // A raw string: r"..." or r#"..."#.
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                j += 1;
                loop {
                    if j >= chars.len() {
                        break;
                    }
                    if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
        } else if c == '\'' {
            // A character literal ('u', '\n', '\u{..}') but not a lifetime ('a).
            let literal_end = if next == Some('\\') {
                chars[i + 2..]
                    .iter()
                    .position(|&x| x == '\'')
                    .map(|p| i + 2 + p)
            } else if chars.get(i + 2) == Some(&'\'') {
                Some(i + 2)
            } else {
                None
            };
            i = literal_end.map_or(i + 1, |end| end + 1);
        } else if c == 'u'
            && (i == 0 || !word_char(chars[i - 1]))
            && chars[i..].starts_with(&['u', 'n', 's', 'a', 'f', 'e'])
            && !chars.get(i + 6).copied().is_some_and(word_char)
        {
            count += 1;
            i += 6;
        } else {
            i += 1;
        }
    }
    count
}

fn rust_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            // Build output, vendored sources and the worktrees of agents are not the repository.
            if matches!(
                name.as_ref(),
                "target" | ".git" | ".claude" | "node_modules"
            ) || path == root.join("capi")
            {
                continue;
            }
            rust_files(root, &path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn counts() -> BTreeMap<String, usize> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(root, root, &mut files);
    files
        .iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path).ok()?;
            let n = count_unsafe(&source);
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            (n > 0).then_some((relative, n))
        })
        .collect()
}

fn listed() -> BTreeMap<String, usize> {
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(LIST)).unwrap_or_default();
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (path, n) = line.rsplit_once(' ').expect("`path count`");
            (path.to_owned(), n.parse().expect("a count"))
        })
        .collect()
}

#[test]
fn the_counter_ignores_comments_and_strings() {
    assert_eq!(
        count_unsafe("// unsafe\n/// unsafe\n/* unsafe /* unsafe */ */ fn f() {}"),
        0
    );
    assert_eq!(
        count_unsafe("let s = \"unsafe\"; let r = r#\"unsafe \" unsafe\"#;"),
        0
    );
    assert_eq!(
        count_unsafe("#![warn(unsafe_op_in_unsafe_fn)] #[allow(unsafe_code)]"),
        0
    );
    assert_eq!(count_unsafe("let c = '\"'; let l: &'static str = \"\";"), 0);
    assert_eq!(
        count_unsafe("unsafe { f() } unsafe extern \"C\" {} unsafe impl T for U {}"),
        3
    );
    assert_eq!(count_unsafe("#[unsafe(no_mangle)] pub unsafe fn f() {}"), 2);
}

#[test]
fn no_new_unsafe() {
    let now = counts();
    if std::env::var_os("UPDATE_UNSAFE_DEBT").is_some() {
        let mut text = String::from(
            "# The `unsafe` the repository still has, per file. It only goes down: see tests/no_unsafe.rs.\n",
        );
        for (path, n) in &now {
            text.push_str(&format!("{path} {n}\n"));
        }
        fs::write(Path::new(env!("CARGO_MANIFEST_DIR")).join(LIST), text).expect("write the list");
        return;
    }
    let debt = listed();
    let mut problems = Vec::new();
    for (path, n) in &now {
        match debt.get(path) {
            None => problems.push(format!(
                "{path}: {n} unsafe in a file that has none in the list"
            )),
            Some(allowed) if n > allowed => {
                problems.push(format!("{path}: {n} unsafe, the list allows {allowed}"));
            }
            Some(allowed) if n < allowed => problems.push(format!(
                "{path}: {n} unsafe, the list says {allowed}: lower it (UPDATE_UNSAFE_DEBT=1)"
            )),
            Some(_) => {}
        }
    }
    for path in debt.keys().filter(|path| !now.contains_key(*path)) {
        problems.push(format!(
            "{path}: no unsafe left, remove it from the list (UPDATE_UNSAFE_DEBT=1)"
        ));
    }
    assert!(
        problems.is_empty(),
        "The repository has no `unsafe` (CLAUDE.md). New ones are refused:\n{}",
        problems.join("\n")
    );
}
