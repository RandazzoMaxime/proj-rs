//! `proj-rs`: every command-line tool in one executable.
//!
//! The tool is chosen by the name the program runs under (`cs2cs` as a link to `proj-rs`) or by
//! the first argument (`proj-rs cs2cs ...`). Either way the tool sees its own name as
//! `argv[0]`, so usage and error messages name it as the PROJ original does. The tools are the
//! ones of the stand-alone binaries, which keep their own `[[bin]]` entries.
//! See `docs/PORTING-multicall.md`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the multi-call front end writes its usage to stdout or stderr"
)]

#[path = "cct.rs"]
mod cct;
#[path = "cs2cs.rs"]
mod cs2cs;
#[path = "support/dms.rs"]
mod dms;
#[path = "support/geod_cli.rs"]
mod geod_cli;
#[path = "gie.rs"]
mod gie;
#[path = "support/lists.rs"]
mod lists;
#[path = "support/proj_cli.rs"]
mod proj_cli;
#[path = "projinfo.rs"]
mod projinfo;
#[cfg(feature = "network")]
#[path = "projsync.rs"]
mod projsync;

use std::path::Path;
use std::process::ExitCode;

const RELEASE: &str = "Rel. 9.9.0, September 15th, 2026";

/// The names the tools answer to.
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

/// The name of the executable as a tool name: the last part of `argv[0]`, without `.exe`,
/// in lower case on Windows where file names do not distinguish case.
fn invoked_name(argv0: &str) -> String {
    let base = Path::new(argv0).file_name().map_or_else(
        || argv0.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let base = if cfg!(windows) {
        base.to_ascii_lowercase()
    } else {
        base
    };
    match base.strip_suffix(".exe") {
        Some(stem) => stem.to_owned(),
        None => base,
    }
}

/// Runs `tool`. `argv` is `None` when the process arguments are the tool's own (the executable
/// runs under the tool's name), and the tool's arguments, its name first, when it was named
/// on the command line.
fn run_tool(tool: &str, argv: Option<Vec<String>>) -> ExitCode {
    match tool {
        "cct" => argv.map_or_else(cct::main, cct::run_with),
        "cs2cs" => argv.map_or_else(cs2cs::main, cs2cs::run_with),
        "gie" => argv.map_or_else(gie::main, gie::run_with),
        "projinfo" => argv.map_or_else(projinfo::main, projinfo::run_with),
        #[cfg(feature = "network")]
        "projsync" => argv.map_or_else(projsync::main, projsync::run_with),
        "proj" | "invproj" => proj_cli::run(argv.unwrap_or_else(|| std::env::args().collect())),
        "geod" | "invgeod" => geod_cli::run(argv.unwrap_or_else(|| std::env::args().collect())),
        _ => unreachable!("run_tool is called with a name of TOOLS"),
    }
}

fn usage() -> String {
    format!(
        "usage: proj-rs <tool> [arguments]\n\
         \n\
         tools: {}\n\
         \n\
         Linked or copied under a tool's name, proj-rs runs that tool.\n\
         proj-rs <tool> --help prints the usage of a tool; proj-rs --list prints the tool names,\n\
         one per line, for the install scripts (tools/dist-cli.sh, tools/dist-cli.ps1).",
        TOOLS.join(", ")
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let name = invoked_name(args.first().map_or("", String::as_str));
    if TOOLS.contains(&name.as_str()) {
        return run_tool(&name, None);
    }
    let Some(first) = args.get(1) else {
        eprintln!("{}", usage());
        return ExitCode::from(1);
    };
    match first.as_str() {
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            ExitCode::SUCCESS
        }
        "--list" => {
            println!("{}", TOOLS.join("\n"));
            ExitCode::SUCCESS
        }
        "-V" | "--version" => {
            println!("proj-rs: {RELEASE}");
            ExitCode::SUCCESS
        }
        tool if TOOLS.contains(&tool) => {
            run_tool(tool, Some(args.iter().skip(1).cloned().collect()))
        }
        other => {
            eprintln!("proj-rs: '{other}' is not a tool\n{}", usage());
            ExitCode::from(1)
        }
    }
}
