//! `proj -S` (scale factors) checked against PROJ 9.9.0's `proj`.
//!
//! The expected text of each case was written by the reference binary
//! (`build/bin/proj`, `PROJ_DATA` of the reference build), run on the same
//! arguments and input. The numbers are read back and compared by the rule of
//! `common::tolerance` (`docs/PARITY.md`), not as text.

use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, ExitStatus, Stdio};

mod common;

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
    Case {
        name: "adams_hemi_wgs84",
        args: &["-S", "+proj=adams_hemi", "+ellps=WGS84"],
        input: "20.195460075 35.040542787\n11.206358475 2.397954734\n16.032816862 -22.918072285\n",
        stdout: "2044799.83\t4136394.41\t<1.12841 1.12841 1.2733 9.63019e-06 1.12841 1.12841>\n\
                 1250931.10\t269557.87\t<1.0101 1.0101 1.0203 5.34576e-06 1.0101 1.0101>\n\
                 1721565.96\t-2633806.38\t<1.05954 1.05954 1.12263 2.18903e-05 1.05954 1.05954>\n",
    },
    Case {
        name: "apian_wgs84",
        args: &["-S", "+proj=apian", "+ellps=WGS84"],
        input: "11.021235012 -4.345572209\n-8.957425588 25.280142744\n-23.615048222 -34.457130732\n",
        stdout: "1224060.14\t-483746.89\t<1.00007 1.00065 1.00065 0.668396 1.00618 0.994506>\n\
                 -919174.44\t2814172.62\t<1.00154 1.02101 1.02101 3.36135 1.04053 0.981238>\n\
                 -2265063.12\t-3835750.25\t<1.01815 1.06393 1.06393 11.173 1.13729 0.935504>\n",
    },
    Case {
        name: "crast_wgs84",
        args: &["-S", "+proj=crast", "+ellps=WGS84"],
        input: "-10.689237309 -13.964244651\n29.614926289 27.417737714\n-22.670934567 -12.774012770\n",
        stdout: "-1132165.68\t-1589004.53\t<1.02071 0.980439 1 3.19313 1.02826 0.972516>\n\
                 2896470.19\t3110096.88\t<1.03218 0.98977 1 12.114 1.11172 0.899504>\n\
                 -2411807.03\t-1453828.01\t<1.02335 0.97991 1 4.95107 1.04417 0.957701>\n",
    },
    Case {
        name: "eck4_wgs84",
        args: &["-S", "+proj=eck4", "+ellps=WGS84"],
        input: "5.151619025 5.581842957\n-6.248542615 3.967968069\n-17.839612189 -31.059657794\n",
        stdout: "483370.91\t735102.19\t<1.18079 0.846895 1 18.9563 1.18079 0.84689>\n\
                 -586843.49\t522808.87\t<1.18246 0.845697 1 19.1162 1.18246 0.845693>\n\
                 -1578876.67\t-3972299.17\t<1.07896 0.928107 1 9.13563 1.08308 0.923294>\n",
    },
    Case {
        name: "fouc_wgs84",
        args: &["-S", "+proj=fouc", "+ellps=WGS84"],
        input: "15.830648637 15.655752290\n9.361698074 41.886159525\n25.696845704 47.696136970\n",
        stdout: "1665403.91\t1753715.38\t<1.02472 0.98145 1 6.60032 1.05932 0.943999>\n\
                 676719.86\t4882140.35\t<1.15449 0.872236 1 17.3979 1.16464 0.858635>\n\
                 1610602.92\t5638975.67\t<1.25715 0.836531 1 29.649 1.2991 0.769763>\n",
    },
    Case {
        name: "gnom_wgs84",
        args: &["-S", "+proj=gnom", "+ellps=WGS84"],
        input: "-23.310663682 -20.440707631\n-12.669400929 19.980423218\n21.408091007 -9.367606967\n",
        stdout: "-2749343.71\t-2572974.39\t<1.23301 1.28049 1.56197 8.68429 1.34828 1.15849>\n\
                 -1433953.49\t2361653.52\t<1.15331 1.12173 1.28961 4.83192 1.18453 1.08871>\n\
                 2501437.78\t-1123184.96\t<1.09658 1.17257 1.2835 5.15919 1.18511 1.08302>\n",
    },
    Case {
        name: "lagrng_wgs84",
        args: &["-S", "+proj=lagrng", "+ellps=WGS84"],
        input: "-26.025973673 44.509175809\n-20.767777151 -27.362742726\n-18.544327500 -18.477345357\n",
        stdout: "-1387424.42\t2763310.32\t<0.677326 0.677326 0.45877 0 0.677326 0.677326>\n\
                 -1141259.72\t-1589149.22\t<0.5589 0.5589 0.31237 0 0.5589 0.5589>\n\
                 -1027451.66\t-1051227.17\t<0.527063 0.527063 0.277795 0 0.527063 0.527063>\n",
    },
    Case {
        name: "murd2_default_ellps",
        args: &["-S", "+proj=murd2", "+lat_1=30", "+lat_2=45"],
        input: "-20.253039938 13.786614425\n-15.024483346 -44.035274777\n16.155187184 -42.581289645\n",
        stdout: "-2355516.96\t2345853.88\t<1.19294 1.08405 1.2932 5.4821 1.19294 1.08405>\n\
                 -8093543.91\t-37318883.64\t<46.151 6.75962 311.964 96.2305 46.151 6.75962>\n\
                 7611314.80\t-30928612.44\t<33.7037 5.776 194.673 90.0465 33.7037 5.776>\n",
    },
    Case {
        name: "putp4p_wgs84",
        args: &["-S", "+proj=putp4p", "+ellps=WGS84"],
        input: "-4.922333245 -14.449766868\n-21.580388017 -28.141707125\n2.670963351 26.531781491\n",
        stdout: "-468423.31\t-1834322.20\t<1.13285 0.882788 1 14.2683 1.13296 0.882641>\n\
                 -1928332.86\t-3538070.44\t<1.10367 0.91031 1 12.3407 1.11394 0.897716>\n\
                 240966.64\t3340703.68\t<1.10403 0.90583 1 11.3374 1.10418 0.905653>\n",
    },
    Case {
        name: "rpoly_wgs84",
        args: &["-S", "+proj=rpoly", "+ellps=WGS84"],
        input: "5.989808030 10.949474847\n-9.995336165 -16.757200474\n22.589943082 27.143837894\n",
        stdout: "654579.12\t1225388.96\t<1.00527 0.999901 1.00517 0.306623 1.00527 0.999901>\n\
                 -1064752.70\t-1892180.10\t<1.01394 0.999368 1.0133 0.829572 1.01394 0.999368>\n\
                 2219786.41\t3221281.00\t<1.06105 0.991976 1.05254 3.85629 1.06105 0.991976>\n",
    },
    Case {
        name: "wag2_wgs84",
        args: &["-S", "+proj=wag2", "+ellps=WGS84"],
        input: "-5.098609598 21.960549638\n4.459607557 44.090830715\n4.192714515 -19.383650366\n",
        stdout: "-501866.21\t2631480.68\t<1.06657 0.953407 1.01672 6.49976 1.0672 0.952694>\n\
                 382091.58\t5199994.43\t<1.00953 1.0716 1.08136 3.79496 1.07491 1.006>\n\
                 416829.29\t-2325067.66\t<1.06993 0.946747 1.01287 7.04123 1.07023 0.946397>\n",
    },
    Case {
        name: "wink2_default_ellps",
        args: &["-S", "+proj=wink2", "+lat_1=40"],
        input: "-19.229148084 49.422195108\n-27.478923002 4.774684814\n-9.035527203 40.481379529\n",
        stdout: "-1638296.35\t5979042.22\t<1.02909 1.1766 1.20571 9.31181 1.19111 1.01226>\n\
                 -2697835.21\t593477.79\t<1.1161 0.885022 0.987711 13.277 1.11621 0.884877>\n\
                 -809528.04\t4942868.15\t<1.05675 1.05814 1.11755 1.92883 1.07509 1.0395>\n",
    },
    Case {
        name: "aea_grs80",
        args: &["-S", "+proj=aea", "+lat_1=29", "+lat_2=45", "+ellps=GRS80"],
        input: "2 49\n3 46.5\n-10 30\n",
        stdout: "148492.79\t5254960.90\t<0.985455 1.01476 1 1.67886 1.01476 0.985455>\n\
                 231311.52\t4981651.42\t<0.995413 1.00461 1 0.526883 1.00461 0.995413>\n\
                 -961110.53\t3185432.85\t<1.00209 0.99791 1 0.239759 1.00209 0.99791>\n",
    },
    Case {
        name: "rhealpix_default_ellps",
        args: &["-S", "+proj=rhealpix", "+north_square=1", "+south_square=2"],
        input: "10.657782218 41.046693782\n26.336828942 -31.566692498\n-22.666876625 12.088698817\n",
        stdout: "1185092.65\t4916211.20\t<0.891769 1.32404 1.18074 22.4993 1.32404 0.891769>\n\
                 2928525.06\t-3916375.21\t<1.00695 1.17259 1.18074 8.71715 1.17259 1.00695>\n\
                 -2520444.52\t1565159.57\t<1.15472 1.02253 1.18074 6.9618 1.15472 1.02253>\n",
    },
];

fn run_proj(
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    input: &str,
) -> (ExitStatus, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_proj"))
        .args(args)
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
        .write_all(input.as_bytes())
        .expect("write input");
    let output = child.wait_with_output().expect("proj output");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn proj_factors_match_reference_output() {
    let mut failures = Vec::new();
    for case in CASES {
        let (status, stdout) = run_proj(case.args, case.input);
        if !status.success() || !common::same_cli_text(&stdout, case.stdout) {
            failures.push(format!(
                "{}: status {:?}\n got:      {stdout:?}\n expected: {:?}",
                case.name,
                status.code(),
                case.stdout
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// One row per case: `kind|name|extra args|three points|expected stdout`. `kind` is `sphere`
/// (PROJ's setup sets `P->es = 0`) or `ellipsoid` (a control that must keep the ellipsoid).
/// Points are `lon lat` joined by `;`; the expected lines are joined by the two characters `\n`.
const SPHERE_CASES: &str = include_str!("cli_proj_factors/sphere.tsv");

/// The x and y columns are compared in metres, the factor column by the factor rule.
fn same_xy_and_factors(got: &str, expected: &str) -> bool {
    let (got, expected): (Vec<&str>, Vec<&str>) =
        (got.lines().collect(), expected.lines().collect());
    got.len() == expected.len()
        && got.iter().zip(&expected).all(|(g, e)| {
            let (g, e): (Vec<&str>, Vec<&str>) = (g.split('\t').collect(), e.split('\t').collect());
            g.len() == 3
                && e.len() == 3
                && close(g[0], e[0])
                && close(g[1], e[1])
                && common::same_cli_text(g[2], e[2])
        })
}

fn close(a: &str, b: &str) -> bool {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(a), Ok(b)) => common::tolerance::close(common::tolerance::Quantity::Metres, a, b),
        _ => false,
    }
}

#[test]
fn proj_factors_sphere_projections_match_reference_output() {
    let mut failures = Vec::new();
    for line in SPHERE_CASES.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        let [kind, name, extra, points, expected] = parts[..] else {
            panic!("malformed row: {line}");
        };
        let proj = format!("+proj={name}");
        let mut args = vec!["-S", "-f", "%.12f", proj.as_str(), "+ellps=WGS84"];
        args.extend(extra.split_whitespace());
        let input: String = points.split(';').map(|p| format!("{p}\n")).collect();
        let expected = expected.replace("\\n", "\n");
        let (status, stdout) = run_proj(&args, &input);
        if !status.success() || !same_xy_and_factors(&stdout, &expected) {
            failures.push(format!(
                "{kind} {name}: status {:?}\n got:      {stdout:?}\n expected: {expected:?}",
                status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The rule accepts the rounding of `proj -S` (the MSVC output of CI log of PR 217, whose omega
/// moves by finite differences) and refuses a changed token or a real difference.
#[test]
fn comparison_accepts_libm_rounding_only() {
    let linux = "552535.620906571625\t1135669.254347730894\t<0.528892 0.528892 0.279727 7.99022e-06 0.528892 0.528892>\n-1598415.287275458686\t2812926.457895270083\t<0.685653 0.685653 0.47012 2.67065e-05 0.685653 0.685653>\n";
    let msvc = "552535.620906571625\t1135669.254347730894\t<0.528892 0.528892 0.279727 8.15166e-06 0.528892 0.528892>\n-1598415.287275458686\t2812926.457895271014\t<0.685653 0.685653 0.47012 2.67065e-05 0.685653 0.685653>\n";
    assert!(common::same_cli_text(msvc, linux));
    assert!(!common::same_cli_text(
        &msvc.replace("0.279727", "0.279737"),
        linux
    ));
    assert!(!common::same_cli_text(
        &msvc.replace("552535.62", "552535.72"),
        linux
    ));
    assert!(!common::same_cli_text(&msvc.replace('<', "("), linux));
    assert!(!common::same_cli_text("*\t*\n", "inf\tinf\n"));
    assert!(common::same_cli_text(
        "<0.684301 0.684301 0.468268 2.51241e-05 0.684302 0.684301>",
        "<0.684301 0.684301 0.468268 2.50931e-05 0.684302 0.684301>"
    ));
}
