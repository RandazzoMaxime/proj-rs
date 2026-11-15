//! `proj` and `proj -I` on `+proj=geocent` and `+proj=cart`, and `proj -S` on both, checked
//! against PROJ 9.9.0's `proj`.
//!
//! The expected text of each case was written by the reference binary (`build/bin/proj`, the
//! same `PROJ_DATA` as the parity suite) on the same arguments and input. In the `proj` app the
//! operation is created with `break_cs2cs_recursion`: `geocent` is then the identity on the 2D
//! interface (radians in, radians out, scaled by the frame's units), not the geodetic to
//! geocentric conversion that `cs2cs` and `cct` run.

use std::io::Write;
use std::process::{Command, Stdio};

mod common;

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    input: &'static str,
    stdout: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "geocent_fwd_wgs84",
        args: &["-f", "%.12f", "+proj=geocent", "+ellps=WGS84"],
        input: "-27.488549169 -44.423151428\n0 0\n10 20\n180 -89\n",
        stdout: "-0.479765689595\t-0.775330256531\n\
                 0.000000000000\t0.000000000000\n\
                 0.174532925199\t0.349065850399\n\
                 3.141592653590\t-1.553343034275\n",
    },
    Case {
        name: "geocent_fwd_sphere",
        args: &["-f", "%.12f", "+proj=geocent", "+R=6371000"],
        input: "-27.488549169 -44.423151428\n10 20\n",
        stdout: "-0.479765689595\t-0.775330256531\n\
                 0.174532925199\t0.349065850399\n",
    },
    Case {
        name: "geocent_fwd_km",
        args: &["-f", "%.12f", "+proj=geocent", "+ellps=WGS84", "+units=km"],
        input: "-27.488549169 -44.423151428\n",
        stdout: "-0.000479765690\t-0.000775330257\n",
    },
    Case {
        name: "geocent_fwd_default_format",
        args: &["+proj=geocent", "+ellps=WGS84"],
        input: "-27.488549169 -44.423151428\n",
        stdout: "-0.48\t-0.78\n",
    },
    Case {
        name: "geocent_inv_wgs84",
        args: &["-I", "-f", "%.12f", "+proj=geocent", "+ellps=WGS84"],
        input: "-0.165754237759 -0.035429443936\n0.5 0.25\n",
        stdout: "-9.497018259999\t-2.029957608028\n\
                 28.647889756541\t14.323944878271\n",
    },
    Case {
        name: "geocent_inv_km",
        args: &[
            "-I",
            "-f",
            "%.12f",
            "+proj=geocent",
            "+ellps=WGS84",
            "+units=km",
        ],
        input: "-0.165754237759 -0.035429443936\n",
        stdout: "-137.018259998689\t-2029.957608028167\n",
    },
    Case {
        name: "geocent_factors_wgs84",
        args: &["-S", "-f", "%.12f", "+proj=geocent", "+ellps=WGS84"],
        input: "-27.488549169 -44.423151428\n10 20\n",
        stdout: "-0.479765689595\t-0.775330256531\t<1.00179 1.39789 1.40039 19.0018 1.39789 1.00179>\n\
                 0.174532925199\t0.349065850399\t<1.00556 1.06376 1.06967 3.22355 1.06376 1.00556>\n",
    },
    Case {
        name: "geocent_factors_sphere",
        args: &["-S", "-f", "%.12f", "+proj=geocent", "+R=6371000"],
        input: "10 20\n-27.488549169 -44.423151428\n",
        stdout: "0.174532925199\t0.349065850399\t<1 1.06418 1.06418 3.56336 1.06418 1>\n\
                 -0.479765689595\t-0.775330256531\t<1 1.40019 1.40019 19.1957 1.40019 1>\n",
    },
    Case {
        name: "cart_factors_wgs84",
        args: &["-S", "-f", "%.12f", "+proj=cart", "+ellps=WGS84"],
        input: "-2.002236084 19.304674784\n4.787658742 30.358892160\n",
        stdout: "6018046.524288287386\t-210389.968667302368\t<2.10856e+06 6.37814e+06 1.34487e+13 60.4094 6.37814e+06 2.10856e+06>\n\
                 5489037.200659614988\t459736.702165359049\t<3.2236e+06 6.37814e+06 2.05606e+13 38.3601 6.37814e+06 3.2236e+06>\n",
    },
    Case {
        name: "cart_factors_sphere",
        args: &["-S", "-f", "%.12f", "+proj=cart", "+R=6371000"],
        input: "-2.002236084 19.304674784\n",
        stdout: "6009113.017382428050\t-210077.654658025887\t<2.1062e+06 6.371e+06 1.34186e+13 60.4094 6.371e+06 2.1062e+06>\n",
    },
];

#[test]
fn proj_geocent_and_cart_match_reference_output() {
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
        if !output.status.success() || !common::same_cli_text(&stdout, case.stdout) {
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
