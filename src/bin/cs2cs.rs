//! `cs2cs`: transform coordinates between two CRS definitions.
//!
//! `cs2cs EPSG:4326 EPSG:32631` and `cs2cs +proj=latlong +to +proj=utm ...`
//! read longitude and latitude in degrees, or easting and northing in metres,
//! one point per line. That is the order [`proj_rs::Proj::new_known_crs`] uses,
//! not the EPSG axis order.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "cs2cs writes coordinates to stdout and usage to stderr"
)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use proj_rs::Proj;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        eprintln!("{}", usage());
        return ExitCode::SUCCESS;
    }
    let (source, target) = match split_pair(&args) {
        Ok(pair) => pair,
        Err(message) => {
            eprintln!("cs2cs: {message}");
            eprintln!("{}", usage());
            return ExitCode::from(1);
        }
    };
    let proj = match Proj::new_known_crs(&source, &target, None) {
        Ok(proj) => proj,
        Err(error) => {
            eprintln!("cs2cs: {error}");
            return ExitCode::from(1);
        }
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            eprintln!("cs2cs: could not read stdin");
            return ExitCode::from(1);
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut numbers = line.split_whitespace();
        let Some(x) = numbers.next().and_then(|text| text.parse::<f64>().ok()) else {
            let _ = writeln!(stdout, "*\t*");
            continue;
        };
        let Some(y) = numbers.next().and_then(|text| text.parse::<f64>().ok()) else {
            let _ = writeln!(stdout, "*\t*");
            continue;
        };
        match proj.convert((x, y)) {
            Ok(xy) => {
                let _ = writeln!(stdout, "{:.8}\t{:.8}", xy.0, xy.1);
            }
            Err(_) => {
                let _ = writeln!(stdout, "*\t*");
            }
        }
    }
    ExitCode::SUCCESS
}

fn split_pair(args: &[String]) -> Result<(String, String), String> {
    if args.is_empty() {
        return Err("missing definition".to_owned());
    }
    if let Some(index) = args.iter().position(|arg| arg == "+to") {
        if index == 0 || index + 1 >= args.len() {
            return Err("missing +to definition".to_owned());
        }
        let source = args[..index].join(" ");
        let target = args[index + 1..].join(" ");
        if source.is_empty() || target.is_empty() {
            return Err("missing definition".to_owned());
        }
        return Ok((source, target));
    }
    if args.len() == 2 {
        return Ok((args[0].clone(), args[1].clone()));
    }
    Err("expected SOURCE TARGET or +proj=... +to +proj=...".to_owned())
}

fn usage() -> &'static str {
    "usage: cs2cs SOURCE TARGET\n       cs2cs +proj=... +to +proj=...\n\
     SOURCE and TARGET are EPSG: codes or PROJ strings.\n\
     Input is longitude latitude degrees, or easting northing metres."
}
