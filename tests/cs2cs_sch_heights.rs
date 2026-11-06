//! Heights through PROJ-string pairs in `cs2cs`, against PROJ 9.9.0.
//!
//! The expected values are the output of the PROJ 9.9.0 `cs2cs` binary, run with
//! `-f %.7f`. PROJ carries the third axis through a pair: a projection that computes
//! a height (`+proj=sch`, `+proj=cart`) sets it, and a 2D datum shift
//! (`+towgs84`) leaves it as it came in.

use std::io::Write;
use std::process::{Command, Stdio};

const FORMAT: &[&str] = &["-f", "%.7f"];

fn run(args: &[&str], input: &str) -> Vec<Vec<f64>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cs2cs"));
    command
        .args(args)
        .args(FORMAT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn cs2cs");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse::<f64>().expect("number"))
                .collect()
        })
        .collect()
}

/// Each row is compared to PROJ's value within the 7-decimal print precision.
fn assert_rows(args: &[&str], input: &str, expected: &[[f64; 3]]) {
    let got = run(args, input);
    assert_eq!(got.len(), expected.len(), "row count for {args:?}");
    for (row, (got, want)) in got.iter().zip(expected).enumerate() {
        assert_eq!(got.len(), 3, "row {row} has three fields for {args:?}");
        for axis in 0..3 {
            assert!(
                (got[axis] - want[axis]).abs() <= 2e-7,
                "row {row} axis {axis} of {args:?}: got {}, PROJ {}",
                got[axis],
                want[axis]
            );
        }
    }
}

const LATLONG_TO_SCH: &[&str] = &[
    "+proj=latlong",
    "+datum=WGS84",
    "+to",
    "+proj=sch",
    "+datum=WGS84",
    "+plat_0=30.0",
    "+plon_0=45.0",
    "+phdg_0=-12.0",
    "+nodefs",
];

/// PROJ `test_cs2cs_various.yaml#71`, and the 3D rows that carry its height.
#[test]
fn sch_forward_sets_the_height() {
    assert_rows(
        LATLONG_TO_SCH,
        "0 0 0\n45 45 100\n30 45 -50\n",
        &[
            [-1_977_112.0305592, 5_551_475.1418378, 6_595.725_658_3],
            [1_630_035.539_140_1, -342_353.634_343_7, 228.344_562_0],
            [1_974_596.263_972_0, 787_409.868_764_9, 723.002_887_8],
        ],
    );
}

/// PROJ `test_cs2cs_various.yaml#72`, and the 3D rows that carry its height.
#[test]
fn sch_inverse_sets_the_height() {
    assert_rows(
        &[
            "+proj=sch",
            "+datum=WGS84",
            "+plat_0=30.0",
            "+plon_0=45.0",
            "+phdg_0=-12.0",
            "+nodefs",
            "+to",
            "+proj=latlong",
            "+datum=WGS84",
        ],
        "0 0 2\n0 1000 0\n1000 0 5\n1000 1000 -3\n",
        &[
            [45.0, 30.0, 2.0],
            [44.989_862_5, 29.998_124_0, -0.000_361_7],
            [44.997_845_0, 30.008_823_8, 5.0],
            [44.987_706_6, 30.006_947_7, -3.000_522_8],
        ],
    );
}

/// `+h_0` is the average altitude: it raises the sphere, and the height of the sch
/// output and of the inverse with it.
#[test]
fn sch_average_altitude_moves_the_height() {
    assert_rows(
        &[
            "+proj=latlong",
            "+datum=WGS84",
            "+to",
            "+proj=sch",
            "+datum=WGS84",
            "+plat_0=30",
            "+plon_0=45",
            "+phdg_0=-12",
            "+h_0=100",
            "+nodefs",
        ],
        "45 45 10\n30 45 0\n",
        &[
            [1_630_061.221_131_2, -342_359.028_172_3, 38.344_565_1],
            [1_974_627.318_123_0, 787_422.216_515_4, 673.002_857_7],
        ],
    );
    assert_rows(
        &[
            "+proj=sch",
            "+datum=WGS84",
            "+plat_0=30",
            "+plon_0=45",
            "+phdg_0=-12",
            "+h_0=100",
            "+nodefs",
            "+to",
            "+proj=latlong",
            "+datum=WGS84",
        ],
        "0 0 10\n0 1000 0\n",
        &[
            [45.0, 30.0, 110.0],
            [44.989_862_7, 29.998_124_1, 99.999_638_3],
        ],
    );
}

/// A geocentric source is 3D: its height is the height of the geodetic output.
#[test]
fn cart_source_sets_the_height() {
    assert_rows(
        &[
            "+proj=cart",
            "+ellps=WGS84",
            "+to",
            "+proj=latlong",
            "+datum=WGS84",
        ],
        "5000000 1000000 3000000\n",
        &[[11.309_932_5, 30.651_881_5, -456_535.563_957_8]],
    );
}

/// A projected source keeps its height into `+proj=sch`, which sets its own.
#[test]
fn utm_to_sch_takes_the_sch_height() {
    assert_rows(
        &[
            "+proj=utm",
            "+zone=32",
            "+datum=WGS84",
            "+to",
            "+proj=sch",
            "+datum=WGS84",
            "+plat_0=30",
            "+plon_0=45",
            "+phdg_0=-12",
            "+nodefs",
        ],
        "500000 3300000 10\n",
        &[[1_352_821.605_715_7, 3_210_562.472_296_6, 4_437.869_221_2]],
    );
}

/// A 2D datum shift moves the position and keeps the height, as PROJ does with
/// `+towgs84` on a projected target.
#[test]
fn towgs84_target_keeps_the_height() {
    assert_rows(
        &[
            "+proj=latlong",
            "+datum=WGS84",
            "+to",
            "+proj=utm",
            "+zone=32",
            "+a=6378249.2",
            "+b=6356515",
            "+towgs84=-263.0,6.0,431.0",
        ],
        "10 34 100\n",
        &[[592_302.982_726_3, 3_762_148.736_867_4, 100.0]],
    );
}
