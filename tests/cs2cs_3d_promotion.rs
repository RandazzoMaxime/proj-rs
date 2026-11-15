//! `cs2cs --3d` promotes a CRS to 3D before the pair is built, against PROJ 9.9.0.
//!
//! The expected values are the output of the PROJ 9.9.0 `cs2cs` binary, run with
//! `-d 10`, kept as printed. `EPSG:7680` to `EPSG:2657` was 110 m off before the promotion
//! reached the pair builder; the other pairs already matched and guard against regressions.
//! `EPSG:8698` to `EPSG:32427` is not here: its pivot choice is another gap, see
//! `docs/PORTING-cs2cs-pivot-candidates.md`.

use std::io::Write;
use std::process::{Command, Stdio};

const TOLERANCE: f64 = 1e-6;

fn run(args: &[&str], input: &str) -> Vec<Vec<f64>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cs2cs"));
    command
        .args(["-d", "10"])
        .args(args)
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

fn assert_rows(args: &[&str], input: &str, expected: &[[&str; 3]]) {
    let got = run(args, input);
    assert_eq!(got.len(), expected.len(), "row count for {args:?}");
    for (row, (got, want)) in got.iter().zip(expected).enumerate() {
        assert_eq!(got.len(), 3, "row {row} has three fields for {args:?}");
        for axis in 0..3 {
            let want: f64 = want[axis].parse().expect("PROJ value");
            assert!(
                (got[axis] - want).abs() <= TOLERANCE,
                "row {row} axis {axis} of {args:?}: got {}, PROJ {want}",
                got[axis],
            );
        }
    }
}

/// PROJ: `--3d EPSG:7680 +to EPSG:2657`. Without the promotion the planimetry is 110 m off.
#[test]
fn promoted_geographic_source_matches_proj_7680_to_2657() {
    assert_rows(
        &["--3d", "EPSG:7680", "+to", "EPSG:2657"],
        "-81.225428215 5.868397821 2354.525\n\
         63.216043391 47.456406908 1135.438\n\
         6.130562278 179.093816936 -73.080\n",
        &[
            [
                "-9556458.0448540933",
                "22623766.5795235001",
                "2354.5250000000",
            ],
            [
                "7194863.5762028666",
                "22431910.7737489343",
                "1135.4380000000",
            ],
            [
                "18040766.0791723095",
                "34310299.7494523972",
                "-73.0800000000",
            ],
        ],
    );
}

/// PROJ: `--3d EPSG:8936 +to EPSG:6272`, already matched.
#[test]
fn promoted_pair_8936_to_6272_still_matches_proj() {
    assert_rows(
        &["--3d", "EPSG:8936", "+to", "EPSG:6272"],
        "15.372066441 -26.979371431 595.329\n\
         -42.950268704 -119.189543625 3985.461\n",
        &[
            ["2380519.7429586528", "6061364.3279995965", "595.3290000000"],
            [
                "-4153905.3688792102",
                "-2661782.7983902562",
                "3985.4610000000",
            ],
        ],
    );
}

/// PROJ: `--3d EPSG:7659 +to EPSG:5274`, already matched.
#[test]
fn promoted_pair_7659_to_5274_still_matches_proj() {
    assert_rows(
        &["--3d", "EPSG:7659", "+to", "EPSG:5274"],
        "89.999000000 -179.999000000 283.590\n\
         68.069301229 155.985802328 488.198\n\
         -19.437216333 54.649918395 835.588\n",
        &[
            [
                "10002048.7353378236",
                "14500074.7364115510",
                "283.5900000000",
            ],
            [
                "11039632.4296024833",
                "16773862.9622519035",
                "488.1980000000",
            ],
            [
                "-2199748.7780055702",
                "15836936.1888513248",
                "835.5880000000",
            ],
        ],
    );
}

/// PROJ: `--3d EPSG:9002 +to EPSG:2310`, already matched.
#[test]
fn promoted_pair_9002_to_2310_still_matches_proj() {
    assert_rows(
        &["--3d", "EPSG:9002", "+to", "EPSG:2310"],
        "-65.151562367 117.805378165 1254.169\n\
         70.585064002 170.563638553 2009.318\n\
         -26.606355475 -5.941475281 3523.707\n",
        &[
            [
                "-161136.2902433943",
                "2699821.8238026025",
                "1254.1690000000",
            ],
            [
                "1844552.0912770457",
                "18278397.2611395307",
                "2009.3180000000",
            ],
            [
                "-3913139.0262135472",
                "-6227592.8419147860",
                "3523.7070000000",
            ],
        ],
    );
}
