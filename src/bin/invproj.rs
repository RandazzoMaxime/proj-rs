//! `invproj`: the inverse of `proj`, the same program run in inverse mode.
//! The work is in `support/proj_cli.rs`.

#[path = "support/dms.rs"]
mod dms;
#[path = "support/lists.rs"]
mod lists;
#[path = "support/proj_cli.rs"]
mod proj_cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    proj_cli::run()
}
