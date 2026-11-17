//! `proj-rs`, the one executable that holds every tool, against the stand-alone binaries.
//!
//! Each case runs a tool three ways and expects the same standard output, standard error and
//! exit status: the stand-alone binary (`CARGO_BIN_EXE_cs2cs`), `proj-rs <tool> ...`, and
//! `proj-rs` started under the tool's name (a symbolic link on Unix, a copy named `<tool>.exe` on
//! Windows). The usage and error texts name the program, so a wrong `argv[0]` shows.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MULTICALL: &str = env!("CARGO_BIN_EXE_proj-rs");

/// The tools of the executable, in the order `proj-rs --list` prints them.
const TOOLS: &[&str] = &[
    "cct",
    "cs2cs",
    "geod",
    "gie",
    "invgeod",
    "invproj",
    "proj",
    "projinfo",
    #[cfg(feature = "network")]
    "projsync",
];

/// The stand-alone binary of a tool.
fn standalone(tool: &str) -> &'static str {
    match tool {
        "cct" => env!("CARGO_BIN_EXE_cct"),
        "cs2cs" => env!("CARGO_BIN_EXE_cs2cs"),
        "geod" => env!("CARGO_BIN_EXE_geod"),
        "gie" => env!("CARGO_BIN_EXE_gie"),
        "invgeod" => env!("CARGO_BIN_EXE_invgeod"),
        "invproj" => env!("CARGO_BIN_EXE_invproj"),
        "proj" => env!("CARGO_BIN_EXE_proj"),
        "projinfo" => env!("CARGO_BIN_EXE_projinfo"),
        #[cfg(feature = "network")]
        "projsync" => env!("CARGO_BIN_EXE_projsync"),
        other => panic!("no stand-alone binary for {other}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(program: &Path, args: &[&str], input: &str) -> Outcome {
    let mut child = Command::new(program)
        .args(args)
        .env("PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY", "YES")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write the input");
    let output = child.wait_with_output().expect("wait");
    Outcome {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `proj-rs` under the name of `tool`, in a directory of its own.
fn named_like(tool: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("proj-rs-multicall-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch directory");
    named(&dir, tool)
}

#[cfg(windows)]
fn named(dir: &Path, file: &str) -> PathBuf {
    let path = dir.join(format!("{file}.exe"));
    std::fs::copy(MULTICALL, &path).expect("copy proj-rs");
    path
}

#[cfg(not(windows))]
fn named(dir: &Path, file: &str) -> PathBuf {
    let path = dir.join(file);
    let _ = std::fs::remove_file(&path);
    std::os::unix::fs::symlink(MULTICALL, &path).expect("link proj-rs");
    path
}

struct Case {
    tool: &'static str,
    args: &'static [&'static str],
    input: &'static str,
}

const POINTS: &str = "2.2945 48.8584\n2 49\n";

const CASES: &[Case] = &[
    Case {
        tool: "cs2cs",
        args: &[],
        input: "",
    },
    Case {
        tool: "cs2cs",
        args: &["--version"],
        input: "",
    },
    Case {
        tool: "cs2cs",
        args: &["--bogus"],
        input: "",
    },
    Case {
        tool: "cs2cs",
        args: &[
            "-d",
            "3",
            "+proj=longlat",
            "+datum=WGS84",
            "+to",
            "EPSG:2154",
        ],
        input: POINTS,
    },
    Case {
        tool: "cs2cs",
        args: &["EPSG:4326", "EPSG:99999999"],
        input: "",
    },
    Case {
        tool: "proj",
        args: &[],
        input: "",
    },
    Case {
        tool: "proj",
        args: &["+proj=utm", "+zone=31", "+ellps=WGS84"],
        input: POINTS,
    },
    Case {
        tool: "proj",
        args: &["+proj=nonexistent"],
        input: "",
    },
    Case {
        tool: "invproj",
        args: &["+proj=utm", "+zone=31"],
        input: "500000 5400000\n",
    },
    Case {
        tool: "invproj",
        args: &[],
        input: "",
    },
    Case {
        tool: "cct",
        args: &["--version"],
        input: "",
    },
    Case {
        tool: "cct",
        args: &["+proj=utm", "+zone=31", "+ellps=GRS80"],
        input: "2 49 0 0\n",
    },
    Case {
        tool: "cct",
        args: &["--bogus"],
        input: "",
    },
    Case {
        tool: "projinfo",
        args: &["EPSG:4326"],
        input: "",
    },
    Case {
        tool: "projinfo",
        args: &["-q", "EPSG:99999"],
        input: "",
    },
    Case {
        tool: "geod",
        args: &[],
        input: "",
    },
    Case {
        tool: "geod",
        args: &["+ellps=WGS84", "-p"],
        input: "40 0 90 1000\n",
    },
    Case {
        tool: "invgeod",
        args: &[],
        input: "",
    },
    Case {
        tool: "invgeod",
        args: &["+ellps=WGS84"],
        input: "40 0 40 1\n",
    },
    Case {
        tool: "gie",
        args: &[],
        input: "",
    },
    Case {
        tool: "gie",
        args: &["--version"],
        input: "",
    },
    Case {
        tool: "gie",
        args: &["/nonexistent.gie"],
        input: "",
    },
];

#[test]
fn every_tool_behaves_the_same_in_the_three_ways_to_start_it() {
    for case in CASES {
        let alone = run(Path::new(standalone(case.tool)), case.args, case.input);
        let mut named = vec![case.tool];
        named.extend_from_slice(case.args);
        let by_argument = run(Path::new(MULTICALL), &named, case.input);
        let by_name = run(&named_like(case.tool), case.args, case.input);
        let label = format!("{} {:?}", case.tool, case.args);
        assert_eq!(
            by_argument, alone,
            "proj-rs <tool> against the stand-alone: {label}"
        );
        assert_eq!(
            by_name, alone,
            "proj-rs under the tool's name against the stand-alone: {label}"
        );
    }
}

#[test]
fn usage_names_the_tool_that_was_asked_for() {
    for tool in ["cs2cs", "proj", "invproj", "geod", "invgeod", "gie"] {
        let by_argument = run(Path::new(MULTICALL), &[tool], "");
        let text = format!("{}{}", by_argument.stdout, by_argument.stderr);
        assert!(text.contains(tool), "{tool}: {text}");
        assert!(
            !text.contains("proj-rs"),
            "{tool} must not name the executable: {text}"
        );
    }
}

#[test]
fn list_prints_the_tools_one_per_line() {
    let out = run(Path::new(MULTICALL), &["--list"], "");
    assert_eq!(out.status, Some(0));
    let names: Vec<&str> = out.stdout.lines().collect();
    assert_eq!(names, TOOLS);
}

#[test]
fn without_a_tool_it_prints_the_list_and_fails() {
    let out = run(Path::new(MULTICALL), &[], "");
    assert_eq!(out.status, Some(1));
    for tool in TOOLS {
        assert!(out.stderr.contains(tool), "{tool}: {}", out.stderr);
    }
    let out = run(Path::new(MULTICALL), &["nosuchtool", "EPSG:4326"], "");
    assert_eq!(out.status, Some(1));
    assert!(
        out.stderr.contains("'nosuchtool' is not a tool"),
        "{}",
        out.stderr
    );
}

#[test]
fn help_and_version_succeed() {
    let help = run(Path::new(MULTICALL), &["--help"], "");
    assert_eq!(help.status, Some(0));
    assert!(
        help.stdout.contains("usage: proj-rs <tool>"),
        "{}",
        help.stdout
    );
    let version = run(Path::new(MULTICALL), &["--version"], "");
    assert_eq!(version.status, Some(0));
    assert!(
        version.stdout.starts_with("proj-rs: Rel. 9.9.0"),
        "{}",
        version.stdout
    );
}

/// File names do not distinguish case on Windows, and `.exe` is not part of the tool's name.
#[cfg(windows)]
#[test]
fn windows_names_ignore_case_and_extension() {
    let dir = std::env::temp_dir().join(format!("proj-rs-multicall-case-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch directory");
    let upper = dir.join("CS2CS.EXE");
    std::fs::copy(MULTICALL, &upper).expect("copy proj-rs");
    let alone = run(Path::new(standalone("cs2cs")), &["--version"], "");
    assert_eq!(run(&upper, &["--version"], ""), alone);
}
