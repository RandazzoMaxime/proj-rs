//! `geod` and `invgeod` checked against PROJ 9.9.0's `geod`.
//!
//! The expected text and status of each case were written by the reference
//! binary (`build/bin/geod`, `PROJ_DATA` of the reference build), run on the same
//! arguments and input. PROJ 9.9.0 has no `geod` case in its `test/cli` yaml
//! files, so these cases are not PROJ's own: they record the reference's
//! behaviour, errors included.

use std::io::Write;
use std::process::{Command, Stdio};

/// One run: arguments, input, and what the reference wrote and returned.
struct Case {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    input: &'static str,
    stdout: &'static str,
    status: i32,
}

const CASES: &[Case] = &[
    Case {
        name: "forward_default",
        program: "geod",
        args: &["+ellps=GRS80"],
        input: "40 -74 45 1000\n",
        stdout: "40d0'22.925\"N\t73d59'30.187\"W\t-134d59'40.836\"\n",
        status: 0,
    },
    Case {
        name: "forward_full",
        program: "geod",
        args: &["-a", "+ellps=GRS80"],
        input: "40 -74 45 1000\n",
        stdout: "40dN\t74dW\t40d0'22.925\"N\t73d59'30.187\"W\t45d\t-134d59'40.836\"\t1000.000\n",
        status: 0,
    },
    Case {
        name: "forward_decimal",
        program: "geod",
        args: &["+ellps=GRS80", "-f", "%.6f"],
        input: "40 -74 45 1000\n",
        stdout: "40.006368\t-73.991719\t-134.994677\n",
        status: 0,
    },
    Case {
        name: "forward_positive_azimuth_wgs84",
        program: "geod",
        args: &["-p", "+ellps=WGS84"],
        input: "40 -74 -45 1000\n",
        stdout: "40d0'22.925\"N\t74d0'29.813\"W\t134d59'40.836\"\n",
        status: 0,
    },
    Case {
        name: "forward_seconds_two_decimals",
        program: "geod",
        args: &["-w2", "+ellps=GRS80"],
        input: "40 -74 45 1000\n",
        stdout: "40d0'22.92\"N\t73d59'30.19\"W\t-134d59'40.84\"\n",
        status: 0,
    },
    Case {
        name: "forward_seconds_fixed_width",
        program: "geod",
        args: &["-W2", "+ellps=GRS80"],
        input: "40 -74 45 1000\n",
        stdout: "40d00'22.92\"N\t73d59'30.19\"W\t-134d59'40.84\"\n",
        status: 0,
    },
    Case {
        name: "forward_tag_line_copied",
        program: "geod",
        args: &["-t%", "+ellps=GRS80"],
        input: "%comment\n40 -74 45 1000\n",
        stdout: "%comment\n40d0'22.925\"N\t73d59'30.187\"W\t-134d59'40.836\"\n",
        status: 0,
    },
    Case {
        name: "forward_distance_format_and_tail",
        program: "geod",
        args: &["+ellps=GRS80", "-F", "%.1f"],
        input: "40 -74 45 1000 # tail\n",
        stdout: "40d0'22.925\"N\t73d59'30.187\"W\t-134d59'40.836\" # tail\n",
        status: 0,
    },
    Case {
        name: "forward_near_pole",
        program: "geod",
        args: &["+ellps=GRS80"],
        input: "89.5 10 0 1000000\n",
        stdout: "81d32'46.854\"N\t170dW\t0d\n",
        status: 0,
    },
    Case {
        name: "forward_dms_input",
        program: "geod",
        args: &["+ellps=GRS80"],
        input: "40d30'15\" -74d15'W 45 1000\n",
        stdout: "40d30'37.923\"N\t74d14'29.965\"W\t-134d59'40.491\"\n",
        status: 0,
    },
    Case {
        name: "error_no_ellipsoid",
        program: "geod",
        args: &["+foo=1", "-f", "%.6f"],
        input: "40 -74 45 1000\n",
        stdout: "",
        status: 1,
    },
    Case {
        name: "forward_missing_distance_is_zero",
        program: "geod",
        args: &["+ellps=GRS80"],
        input: "40 -74 45\n",
        stdout: "40dN\t74dW\t-135d\n",
        status: 0,
    },
    Case {
        name: "forward_custom_axes",
        program: "geod",
        args: &["+a=6378137", "+f=0.00335281066474748"],
        input: "10 10 20 1000\n",
        stdout: "10d0'30.585\"N\t10d0'11.23\"E\t-159d59'58.049\"\n",
        status: 0,
    },
    Case {
        name: "forward_sphere",
        program: "geod",
        args: &["+R=6371000"],
        input: "10 10 20 1000\n",
        stdout: "10d0'30.423\"N\t10d0'11.244\"E\t-159d59'58.047\"\n",
        status: 0,
    },
    Case {
        name: "forward_semi_major_only",
        program: "geod",
        args: &["+a=6378137"],
        input: "10 10 20 1000\n",
        stdout: "10d0'30.389\"N\t10d0'11.232\"E\t-159d59'58.049\"\n",
        status: 0,
    },
    Case {
        name: "inverse_default",
        program: "invgeod",
        args: &["+ellps=GRS80"],
        input: "40 -74 45 -80\n",
        stdout: "-39d36'33.559\"\t136d19'52.483\"\t742348.165\n",
        status: 0,
    },
    Case {
        name: "inverse_full",
        program: "invgeod",
        args: &["-a", "+ellps=GRS80"],
        input: "40 -74 45 -80\n",
        stdout: "40dN\t74dW\t45dN\t80dW\t-39d36'33.559\"\t136d19'52.483\"\t742348.165\n",
        status: 0,
    },
    Case {
        name: "inverse_decimal",
        program: "invgeod",
        args: &["+ellps=GRS80", "-f", "%.8f"],
        input: "40 -74 45 -80\n",
        stdout: "-39.60932194\t136.33124536\t742348.165\n",
        status: 0,
    },
    Case {
        name: "inverse_rf_axes",
        program: "invgeod",
        args: &["+a=6378137", "+rf=298.257"],
        input: "10 10 20 20\n",
        stdout: "42d59'34.638\"\t-134d24'9.797\"\t1541856.430\n",
        status: 0,
    },
    Case {
        name: "inverse_tag_comment",
        program: "invgeod",
        args: &["-t#", "+ellps=GRS80"],
        input: "# hi\n40 -74 45 -80\n",
        stdout: "# hi\n-39d36'33.559\"\t136d19'52.483\"\t742348.165\n",
        status: 0,
    },
    Case {
        name: "inverse_flag_I_on_geod",
        program: "geod",
        args: &["-I", "+ellps=GRS80"],
        input: "40 -74 45 -80\n",
        stdout: "-39d36'33.559\"\t136d19'52.483\"\t742348.165\n",
        status: 0,
    },
    Case {
        name: "arc_of_azimuths",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+A=30", "+S=1000", "+n_A=3", "+del_A=10"],
        input: "",
        stdout: "40d0'28.078\"N\t73d59'38.919\"W\n40d0'24.836\"N\t73d59'32.899\"W\n40d0'20.839\"N\t73d59'27.703\"W\n40d0'16.21\"N\t73d59'23.488\"W\n",
        status: 0,
    },
    Case {
        name: "line_between_points",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+lat_2=45", "+lon_2=-80", "+n_S=4"],
        input: "",
        stdout: "40dN\t74dW\n41d16'44.507\"N\t75d24'44.048\"W\n42d32'23.166\"N\t76d52'49.091\"W\n43d46'50.369\"N\t78d24'29.411\"W\n45dN\t80dW\n",
        status: 0,
    },
    Case {
        name: "line_from_distance",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+A=45", "+S=3000", "+n_S=3"],
        input: "",
        stdout: "40dN\t74dW\n40d0'22.925\"N\t73d59'30.187\"W\n40d0'45.848\"N\t73d59'0.369\"W\n40d1'8.768\"N\t73d58'30.545\"W\n",
        status: 0,
    },
    Case {
        name: "line_step_size",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+lat_2=45", "+lon_2=-80", "+del_S=500000"],
        input: "",
        stdout: "40dN\t74dW\n45dN\t80dW\n",
        status: 0,
    },
    Case {
        name: "error_files_with_line_mode",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+A=45", "+S=3000", "+n_S=3", "missing.txt"],
        input: "",
        stdout: "",
        status: 1,
    },
    Case {
        name: "error_no_interval_divisor",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+A=45", "+S=3000"],
        input: "",
        stdout: "",
        status: 1,
    },
    Case {
        name: "error_no_end_point_or_distance",
        program: "geod",
        args: &["+ellps=GRS80", "+lat_1=40", "+lon_1=-74", "+n_S=3"],
        input: "",
        stdout: "",
        status: 1,
    },
    Case {
        name: "error_bad_option",
        program: "geod",
        args: &["-z", "+ellps=GRS80"],
        input: "40 -74 45 1000\n",
        stdout: "",
        status: 1,
    },
    Case {
        name: "error_unknown_ellipsoid",
        program: "geod",
        args: &["+ellps=NOPE"],
        input: "40 -74 45 1000\n",
        stdout: "",
        status: 1,
    },
];

#[test]
fn geod_and_invgeod_match_reference_output() {
    let mut failures = Vec::new();
    for case in CASES {
        let exe = match case.program {
            "geod" => env!("CARGO_BIN_EXE_geod"),
            "invgeod" => env!("CARGO_BIN_EXE_invgeod"),
            other => panic!("no binary for {other}"),
        };
        let mut child = Command::new(exe)
            .args(case.args)
            .env("PROJ_NETWORK", "OFF")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn geod");
        if let Some(mut stdin) = child.stdin.take() {
            // A program that exits before reading its input closes the pipe:
            // that is its own behaviour, and its status is what is compared.
            let _ = stdin.write_all(case.input.as_bytes());
        }
        let output = child.wait_with_output().expect("geod output");
        let status = output.status.code().unwrap_or(1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout != case.stdout || status != case.status {
            failures.push(format!(
                "{}: status {status} (expected {}), stdout {stdout:?} (expected {:?})",
                case.name, case.status, case.stdout
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
