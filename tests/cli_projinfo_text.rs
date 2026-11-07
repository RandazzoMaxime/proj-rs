//! `projinfo` text, byte for byte, as PROJ 9.9.0 writes it. The expected texts are the
//! stdout of PROJ 9.9.0's `projinfo` for the same arguments and environment.

use std::process::Command;

/// The stdout of `projinfo` with `args`. The grid check is on unless `env` turns it off
/// (`PROJINFO_NO_GRID_CHECK`), and the other variables of `env` are set.
fn projinfo_stdout(args: &[&str], env: &[(&str, &str)]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_projinfo"));
    command.args(args);
    command.env_remove("PROJINFO_NO_GRID_CHECK");
    for (name, value) in env {
        command.env(name, value);
    }
    let output = command.output().expect("run projinfo");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

/// The summary ends with one newline, as PROJ's `std::endl` leaves it: no blank line after
/// the last candidate (projinfo_lib.cpp:1156-1159).
#[test]
fn summary_ends_with_one_newline() {
    let text = projinfo_stdout(
        &[
            "-s",
            "EPSG:4230",
            "-t",
            "EPSG:4258",
            "--area",
            "EPSG:3237",
            "--summary",
        ],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert_eq!(
        text,
        "Candidate operations found: 1\nEPSG:1626, ED50 to ETRS89 (4), 1.0 m, Denmark - onshore.\n"
    );
}

/// With the grid check on, a candidate whose grid file is missing ends its line with
/// `, at least one grid missing` before any `time-dependent operation` note
/// (projinfo_lib.cpp:945-962). The fixture directory holds no such grid.
#[test]
fn summary_names_a_missing_grid_when_the_grid_check_is_on() {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
    let text = projinfo_stdout(
        &[
            "-s",
            "EPSG:23030",
            "-t",
            "EPSG:25830",
            "--bbox",
            "-6,40,-5,41",
            "--grid-check",
            "known_available",
            "--hide-ballpark",
            "--summary",
        ],
        &[("PROJ_DATA", data)],
    );
    assert_eq!(
        text,
        "Candidate operations found: 2\n\
         unknown id, Inverse of UTM zone 30N + ED50 to ETRS89 (12) + UTM zone 30N, 0.2 m, Spain - mainland, Balearic Islands and Ceuta - onshore., at least one grid missing\n\
         unknown id, Inverse of UTM zone 30N + ED50 to ETRS89 (7) + UTM zone 30N, 1.5 m, Spain - onshore mainland except northwest (north of 41°30'N and west of 4°30'W).\n"
    );
}

/// A WKT1 block is followed by a blank line (projinfo_lib.cpp:774, 799), and the output ends
/// there. The WKT of a projected CRS ends in its AUTHORITY, so the text ends in `]]` and two
/// newlines.
#[test]
fn wkt1_block_ends_with_a_blank_line() {
    let text = projinfo_stdout(&["-o", "WKT1_GDAL", "EPSG:32631"], &[]);
    assert!(
        text.ends_with("AUTHORITY[\"EPSG\",\"32631\"]]\n\n"),
        "{text:?}"
    );
    assert!(!text.ends_with("\n\n\n"), "{text:?}");
}
