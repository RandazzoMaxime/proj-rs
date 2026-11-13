//! `proj`: project coordinates, and with `-S` print scale factors.
//! The work is in `support/proj_cli.rs`.

#[path = "support/dms.rs"]
mod dms;
#[path = "support/proj_cli.rs"]
mod proj_cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    proj_cli::run()
}
