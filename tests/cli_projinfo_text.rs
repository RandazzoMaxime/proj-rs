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

/// `projinfo -o PROJJSON` writes any object, not only a CRS: an ellipsoid or an operation
/// (PROJ 9.9.0 `outputObject`, `proj_as_projjson`).
#[test]
fn projjson_writes_ellipsoid_and_operation() {
    let ellipsoid = projinfo_stdout(
        &["-k", "ellipsoid", "EPSG:1024", "-o", "PROJJSON", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert_eq!(
        ellipsoid,
        "{\n  \"$schema\": \"https://proj.org/schemas/v0.7/projjson.schema.json\",\n  \"type\": \"Ellipsoid\",\n  \"name\": \"CGCS2000\",\n  \"semi_major_axis\": 6378137,\n  \"inverse_flattening\": 298.257222101,\n  \"id\": {\n    \"authority\": \"EPSG\",\n    \"code\": 1024\n  }\n}\n"
    );
    let operation = projinfo_stdout(
        &["-k", "operation", "EPSG:11182", "-o", "PROJJSON", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert!(
        operation.starts_with("{\n  \"$schema\": \"https://proj.org/schemas/v0.7/projjson.schema.json\",\n  \"type\": \"Transformation\",\n  \"name\": \"ETRS89-BIH [BH_ETRS89] to ETRF2000 (1)\",\n"),
        "{operation}"
    );
}

/// `--single-line` puts a WKT on one line, as PROJ 9.9.0 does; the multi-line default stays.
#[test]
fn single_line_flag_joins_wkt2_lines() {
    let single = projinfo_stdout(
        &["EPSG:4618", "-o", "WKT2:2019", "--single-line", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert_eq!(single.lines().count(), 1, "{single}");
    assert!(single.starts_with("GEOGCRS[\"SAD69\","), "{single}");
    let multi = projinfo_stdout(
        &["EPSG:4618", "-o", "WKT2:2019", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert!(multi.lines().count() > 1, "{multi}");
}

/// A PROJ string without `+type=crs` is a PROJ-based operation, even with `+towgs84`, not a
/// bound CRS (PROJ 9.9.0 `createFromPROJString`, the `unexpectedStructure` branch).
#[test]
fn proj_string_without_type_crs_is_a_proj_based_operation() {
    let text = projinfo_stdout(
        &[
            "+proj=utm +zone=41 +ellps=intl +towgs84=-164,-17,-104,0,0,0,0 +units=m +no_defs",
            "-o",
            "WKT2:2019",
            "-q",
        ],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert_eq!(
        text,
        "CONVERSION[\"PROJ-based coordinate operation\",\n    METHOD[\"PROJ-based operation method: +proj=utm +zone=41 +ellps=intl +towgs84=-164,-17,-104,0,0,0,0 +units=m +no_defs\"]]\n"
    );
}

/// A conversion is left out of the WKT1 dialects (PROJ 9.9.0 `outputObject`, the
/// `!Conversion` guard), so `-o WKT1_GDAL` prints nothing for it.
#[test]
fn conversion_is_left_out_of_wkt1() {
    let text = projinfo_stdout(
        &["-k", "operation", "EPSG:3856", "-o", "WKT1_GDAL", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert_eq!(text, "", "{text:?}");
}

/// The WKT2:2019 of a vertical datum keeps its anchor epoch, as `ANCHOREPOCH` after `ANCHOR`
/// (PROJ 9.9.0 `VerticalReferenceFrame::_exportToWKT`).
#[test]
fn vertical_datum_writes_its_anchor_epoch() {
    let text = projinfo_stdout(
        &["EPSG:11329", "-o", "WKT2:2019", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert!(
        text.contains("VDATUM[\"Japanese Geodetic Datum 2024 (vertical)\",\n            ANCHOREPOCH[2024.415]]"),
        "{text}"
    );
}

/// A vertical offset grid that PROJ reads under another name keeps its operation as a
/// `DERIVED_FROM` identifier with no version (`createSimilarPropertiesOperation`).
#[test]
fn vertical_grid_operation_takes_the_proj_grid_name() {
    let text = projinfo_stdout(
        &["-k", "operation", "EPSG:7863", "-o", "WKT2:2019", "-q"],
        &[("PROJINFO_NO_GRID_CHECK", "YES")],
    );
    assert!(text.contains("ID[\"DERIVED_FROM(EPSG)\",7863]"), "{text}");
    assert!(
        text.contains(
            "PARAMETERFILE[\"Vertical offset file\",\"nz_linz_dublht1960-nzvd2016.tif\"]"
        ),
        "{text}"
    );
    assert!(!text.contains("VERSION[\"LINZ-NZ DUBL\"]"), "{text}");
}
