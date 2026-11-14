//! `geod`: geodesics on an ellipsoid, forward from a start point and an azimuth.
//! The work is in `support/geod_cli.rs`.

#[path = "support/dms.rs"]
mod dms;
#[path = "support/geod_cli.rs"]
mod geod_cli;
#[path = "support/lists.rs"]
mod lists;

use std::process::ExitCode;

fn main() -> ExitCode {
    geod_cli::run()
}
