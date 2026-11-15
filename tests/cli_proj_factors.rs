//! `proj -S` (scale factors) checked against PROJ 9.9.0's `proj`.
//!
//! The expected text of each case was written by the reference binary
//! (`build/bin/proj`, `PROJ_DATA` of the reference build), run on the same
//! arguments and input. The comparison is exact, byte for byte: the omega
//! column is the last digit that a numerical derivative can move.

use std::io::Write;
use std::process::{Command, Stdio};

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static str,
    stdout: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "merc_grs80",
        args: &["-S", "+proj=merc", "+ellps=GRS80"],
        input: "100 -80\n2 49\n0 0\n-170 60\n",
        stdout: "11131949.08\t-15496570.74\t<5.74005 5.74005 32.9481 1.18992e-06 5.74005 5.74005>\n\
                 222638.98\t6242596.00\t<1.52134 1.52134 2.31449 0 1.52134 1.52134>\n\
                 0.00\t0.00\t<1 1 1 0 1 1>\n\
                 -18924313.43\t8362698.55\t<1.99497 1.99497 3.97992 0 1.99497 1.99497>\n",
    },
    Case {
        name: "tmerc_grs80_k",
        args: &[
            "-S",
            "+proj=tmerc",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
        ],
        input: "2 49\n100 -80\n0 0\n",
        stdout: "426857.99\t5427937.52\t<0.999666 0.999666 0.999332 1.47927e-06 0.999666 0.999666>\n\
                 1613554.81\t-10135394.32\t<1.01478 1.01478 1.02978 1.18983e-06 1.01478 1.01478>\n\
                 166021.44\t0.00\t<1.00098 1.00098 1.00196 0 1.00098 1.00098>\n",
    },
    Case {
        name: "lcc_grs80",
        args: &[
            "-S",
            "+proj=lcc",
            "+lat_1=44",
            "+lat_2=49",
            "+lat_0=46.5",
            "+lon_0=3",
            "+x_0=700000",
            "+y_0=6600000",
            "+ellps=GRS80",
        ],
        input: "2 49\n3 46.5\n-10 30\n",
        stdout: "626830.16\t6878250.03\t<1 1 1 8.53774e-07 1 1>\n\
                 700000.00\t6600000.00\t<0.999051 0.999051 0.998103 8.54585e-07 0.999051 0.999051>\n\
                 -596307.46\t4853062.08\t<1.03816 1.03816 1.07777 1.16304e-06 1.03816 1.03816>\n",
    },
    Case {
        name: "stere_polar_wgs84",
        args: &["-S", "+proj=stere", "+lat_0=90", "+ellps=WGS84"],
        input: "2 49\n0 0\n100 -80\n",
        stdout: "166733.31\t-4774617.24\t<1.13956 1.13956 1.2986 0 1.13956 1.13956>\n\
                 0.00\t-12713600.10\t<1.99331 1.99331 3.97328 0 1.99331 1.99331>\n\
                 142167013.21\t25067880.19\t<129.918 129.918 16878.7 1.18959e-06 129.918 129.918>\n",
    },
    Case {
        name: "robin_sphere",
        args: &["-S", "+proj=robin", "+R=6371000"],
        input: "2 49\n100 -80\n0 0\n",
        stdout: "164939.07\t5217398.91\t<0.932956 1.13049 1.05463 11.0038 1.13063 0.932788>\n\
                 5863278.40\t-8093403.95\t<1.10016 3.03658 1.97977 84.2275 3.16873 0.624783>\n\
                 0.00\t-0.00\t<0.960765 0.8487 0.815402 7.10153 0.960765 0.8487>\n",
    },
    Case {
        name: "merc_units_ft",
        args: &["-S", "+proj=merc", "+ellps=GRS80", "+units=ft"],
        input: "2 49\n0 0\n",
        stdout: "730442.85\t20480958.00\t<1.52134 1.52134 2.31449 0 1.52134 1.52134>\n\
                 0.00\t0.00\t<1 1 1 0 1 1>\n",
    },
    Case {
        name: "tmerc_units_ft",
        args: &[
            "-S",
            "+proj=tmerc",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
            "+units=ft",
        ],
        input: "2 49\n",
        stdout: "1400452.72\t17808193.97\t<0.999666 0.999666 0.999332 1.47927e-06 0.999666 0.999666>\n",
    },
];

#[test]
fn proj_factors_match_reference_output() {
    let mut failures = Vec::new();
    for case in CASES {
        let mut child = Command::new(env!("CARGO_BIN_EXE_proj"))
            .args(case.args)
            .env("PROJ_NETWORK", "OFF")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn proj");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(case.input.as_bytes())
            .expect("write input");
        let output = child.wait_with_output().expect("proj output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || stdout != case.stdout {
            failures.push(format!(
                "{}: status {:?}\n got:      {stdout:?}\n expected: {:?}",
                case.name,
                output.status.code(),
                case.stdout
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
