//! EPSG:6310 (CGRS93, geographic 3D) to EPSG:4919 (ITRF2000, geocentric): PROJ 9.9.0 builds two
//! chained candidates through the geographic CRS of the geocentric target
//! (`createOperationsGeodToGeod`, coordinateoperationfactory.cpp:5366-5387), while this port only
//! had the ballpark. Expected values: PROJ 9.9.0 `projinfo --summary` and `cs2cs -d 12`
//! (`tests/parity/expected.jsonl`, `cs2cs-h3d-geog3d-geocent-0012`), not this port.

use std::io::Write;
use std::process::{Command, Stdio};

use proj_rs::iso19111::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory, DatabaseContext,
};

const CGRS93_TO_WGS84_EPSG: &str = "CGRS93 to WGS 84 (1)";
const CGRS93_TO_WGS84_ESRI: &str = "CGRS_1993_To_WGS_1984";
const TAIL: &str = "Inverse of ITRF_2000_To_WGS_1984 + Null geographic offset from ITRF2000 to \
     MONREF_1997_3D + axis order change (geographic3D horizontal) + Conversion from ITRF2000 \
     (geographic) to ITRF2000";
const HEAD: &str = "Null geographic offset from CGRS93 (geog3D) to CGRS93 (geog2D)";

const INPUTS: [&str; 6] = [
    "35.036454237 34.016619226 1319.645",
    "34.887036867 33.822419296 1760.335",
    "35.007692759 34.634659466 781.657",
    "34.591000000 34.649000000 2115.918",
    "35.447652976 33.832216644 2813.699",
    "34.962606981 34.619493425 2840.843",
];

const PROJ_OUTPUTS: [[f64; 3]; 6] = [
    [4334345.947098, 2925378.235999, 3641936.627489],
    [4352429.565015, 2916160.014477, 3628600.977868],
    [4303683.260712, 2972751.505331, 3639014.403547],
    [4325548.216541, 2989454.135340, 3601808.078674],
    [4322878.184755, 2897431.487860, 3680069.953154],
    [4308219.133699, 2974200.474994, 3636095.899190],
];

/// Metres, the threshold of the parity suite for a length.
const TOLERANCE: f64 = 5e-4;

fn candidate_name(to_wgs84: &str) -> String {
    format!("{HEAD} + {to_wgs84} + {TAIL}")
}

/// `projinfo -s EPSG:6310 -t EPSG:4919 --summary` (PROJ 9.9.0): two candidates, in this order.
#[test]
fn cgrs93_3d_to_itrf2000_geocentric_lists_proj_candidates() {
    let db = DatabaseContext::create().expect("database");
    let source = create_from_user_input("EPSG:6310", &db).expect("source");
    let target = create_from_user_input("EPSG:4919", &db).expect("target");
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source, &target, &context)
        .expect("operations");
    let names: Vec<&str> = operations
        .iter()
        .map(|operation| operation.name())
        .collect();
    assert_eq!(
        names,
        [
            candidate_name(CGRS93_TO_WGS84_EPSG),
            candidate_name(CGRS93_TO_WGS84_ESRI),
        ]
    );
}

/// `cs2cs -d 12 EPSG:6310 +to EPSG:4919` (PROJ 9.9.0) for the six points of the parity corpus.
#[test]
fn cgrs93_3d_to_itrf2000_geocentric_cs2cs_matches_proj_values() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cs2cs"))
        .args(["-d", "12", "EPSG:6310", "+to", "EPSG:4919"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cs2cs");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(INPUTS.join("\n").as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let lines: Vec<Vec<f64>> = stdout
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse::<f64>().expect("number"))
                .collect()
        })
        .collect();
    assert_eq!(lines.len(), PROJ_OUTPUTS.len(), "{stdout}");
    for (fields, expected) in lines.iter().zip(PROJ_OUTPUTS) {
        assert_eq!(fields.len(), 3, "{stdout}");
        for (got, want) in fields.iter().zip(expected) {
            assert!(
                (got - want).abs() <= TOLERANCE,
                "got {fields:?}, PROJ {expected:?}"
            );
        }
    }
}
