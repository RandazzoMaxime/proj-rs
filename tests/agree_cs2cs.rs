//! Accord direct avec `cs2cs`. Le binaire PROJ recalcule les points.

use proj_rs::{Cone, Ellipsoid, Hemisphere, Lcc, Tmerc, UtmZone};
use std::io::Write;
use std::process::{Command, Stdio};

/// The PROJ `cs2cs` on `PATH`, if there is one. `PROJ_CS2CS_REQUIRED` turns a
/// missing binary into a failure, as the CI does.
#[expect(
    clippy::print_stderr,
    reason = "the skip is reported to the reader of the test output"
)]
fn proj_cs2cs() -> Option<std::path::PathBuf> {
    // `cargo test` puts `target/` on `PATH`. Our own `cs2cs` binary lives
    // there and must not shadow the PROJ install this test compares against.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let name = if cfg!(windows) { "cs2cs.exe" } else { "cs2cs" };
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|dir| {
            if dir.starts_with(&manifest) {
                return None;
            }
            let candidate = dir.join(name);
            candidate.is_file().then_some(candidate)
        })
    });
    if found.is_none() {
        assert!(
            std::env::var_os("PROJ_CS2CS_REQUIRED").is_none(),
            "PROJ_CS2CS_REQUIRED is set but no PROJ cs2cs is on PATH"
        );
        eprintln!("skipped: no PROJ cs2cs on PATH");
    }
    found
}

fn cs2cs(program: &std::path::Path, from: &str, to: &str, lon: f64, lat: f64) -> (f64, f64) {
    let mut child = Command::new(program)
        .args(["-d", "8", from, "+to", to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cs2cs is installed");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(format!("{lon} {lat}\n").as_bytes())
        .expect("write");
    let done = child.wait_with_output().expect("wait");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    let text = String::from_utf8(done.stdout).expect("utf8");
    let mut parts = text.split_whitespace();
    let east = parts.next().expect("east").parse::<f64>().expect("east");
    let north = parts.next().expect("north").parse::<f64>().expect("north");
    (east, north)
}

#[test]
fn eiffel_matches_cs2cs() {
    let Some(program) = proj_cs2cs() else {
        return;
    };
    let zone = UtmZone::try_new(31).expect("zone");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let (east, north) = cs2cs(
        &program,
        "+proj=latlong +ellps=WGS84",
        "+proj=utm +zone=31 +ellps=WGS84",
        2.2945,
        48.858_222,
    );
    let xy = utm.forward(2.2945, 48.858_222).expect("forward");
    assert!((xy.east - east).abs() < 1e-6, "{} vs {east}", xy.east);
    assert!((xy.north - north).abs() < 1e-6, "{} vs {north}", xy.north);

    let lcc = Lcc::try_new(Ellipsoid::grs80(), Cone::lambert93()).expect("l93");
    let (east, north) = cs2cs(
        &program,
        "+proj=latlong +ellps=GRS80",
        "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
        2.2945,
        48.858_222,
    );
    let xy = lcc.forward(2.2945, 48.858_222).expect("forward");
    assert!((xy.east - east).abs() < 1e-6, "{} vs {east}", xy.east);
    assert!((xy.north - north).abs() < 1e-6, "{} vs {north}", xy.north);
}

/// One named CRS pair, with points in the facade's order: longitude then latitude
/// for a geographic CRS, easting then northing for a projected one.
struct Pair {
    from: &'static str,
    to: &'static str,
    points: &'static [(f64, f64)],
    /// The facade is known to disagree with PROJ 9.9.0 on this pair (checked in
    /// CI against the reference `cs2cs`). The test then expects the
    /// disagreement, so that fixing it forces the flag off.
    known_gap: bool,
    /// Our `cs2cs` binary is known to disagree with the reference on this pair.
    cli_gap: bool,
}

const PARIS: &[(f64, f64)] = &[
    (2.2945, 48.8582),
    (2.35, 48.85),
    (-1.55, 47.2),
    (5.37, 43.3),
];

const PAIRS: &[Pair] = &[
    Pair {
        from: "EPSG:4326",
        to: "EPSG:2154",
        points: PARIS,
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4326",
        to: "EPSG:32631",
        points: PARIS,
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4326",
        to: "EPSG:3857",
        points: PARIS,
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4326",
        to: "EPSG:3035",
        points: PARIS,
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4326",
        to: "EPSG:3395",
        points: PARIS,
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:2154",
        to: "EPSG:32631",
        points: &[(652_000.0, 6_862_000.0), (700_000.0, 6_600_000.0)],
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4258",
        to: "EPSG:25832",
        points: &[(9.0, 48.0), (11.5, 53.5), (7.2, 50.1)],
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:4326",
        to: "EPSG:5070",
        points: &[(-100.0, 40.0), (-84.4, 33.7), (-122.3, 47.6)],
        known_gap: false,
        cli_gap: false,
    },
    // Same datum, only the linear unit changes: US survey foot to metre.
    Pair {
        from: "EPSG:2230",
        to: "EPSG:26946",
        points: &[
            (4_760_096.421_921, 3_744_293.729_449),
            (6_000_000.0, 2_000_000.0),
        ],
        known_gap: false,
        cli_gap: false,
    },
    Pair {
        from: "EPSG:26946",
        to: "EPSG:2230",
        points: &[(1_450_880.29, 1_141_263.01)],
        known_gap: false,
        cli_gap: false,
    },
    // The datum shift the facade picks is not the one PROJ picks (about 1 m).
    Pair {
        from: "EPSG:4326",
        to: "EPSG:2230",
        points: &[(-115.797_615, 37.264_797_8)],
        known_gap: false,
        cli_gap: false,
    },
];

/// What a `cs2cs` binary answers for one point, in the order the target CRS
/// writes it, or why it gave no numbers.
fn run_cs2cs(
    program: &std::path::Path,
    pair: &Pair,
    point: (f64, f64),
) -> Result<(f64, f64), String> {
    let geographic = matches!(pair.from, "EPSG:4326" | "EPSG:4258");
    // Authority axis order: latitude first for these geographic CRS.
    let (a, b) = if geographic {
        (point.1, point.0)
    } else {
        point
    };
    let mut child = Command::new(program)
        .args(["-d", "8", pair.from, pair.to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start cs2cs: {error}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        // A cs2cs that fails early may close stdin first: its exit code tells.
        let _ = stdin.write_all(format!("{a} {b}\n").as_bytes());
    }
    let done = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !done.status.success() {
        return Err(String::from_utf8_lossy(&done.stderr).trim().to_owned());
    }
    let text = String::from_utf8_lossy(&done.stdout).into_owned();
    let numbers: Vec<f64> = text
        .split_whitespace()
        .map_while(|n| n.parse::<f64>().ok())
        .collect();
    let [first, second, ..] = numbers[..] else {
        return Err(format!("no numbers in `{}`", text.trim()));
    };
    // These target CRS write northing first; the facade answers east, north.
    if matches!(pair.to, "EPSG:3035") {
        Ok((second, first))
    } else {
        Ok((first, second))
    }
}

/// Whether a result must match the reference, or is known not to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    Agree,
    KnownGap,
}

impl Pair {
    fn facade_expect(&self) -> Expect {
        if self.known_gap {
            Expect::KnownGap
        } else {
            Expect::Agree
        }
    }

    fn cli_expect(&self) -> Expect {
        if self.cli_gap {
            Expect::KnownGap
        } else {
            Expect::Agree
        }
    }
}

/// One line per way `got` disagrees with the reference, or none when it should agree.
fn judge(
    what: &str,
    pair: &Pair,
    point: (f64, f64),
    expect: Expect,
    got: Result<(f64, f64), String>,
    want: (f64, f64),
) -> Option<String> {
    let label = format!("{what} {} -> {} at {point:?}", pair.from, pair.to);
    let off = got
        .as_ref()
        .map(|g| (g.0 - want.0).abs().max((g.1 - want.1).abs()));
    match (off, expect) {
        (Ok(off), Expect::Agree) if off < 1e-2 => None,
        (Ok(off), Expect::Agree) => Some(format!("{label}: {got:?} vs PROJ {want:?}, off {off}")),
        (Err(error), Expect::Agree) => Some(format!("{label}: failed ({error}), PROJ {want:?}")),
        (Ok(off), Expect::KnownGap) if off < 1e-2 => {
            Some(format!("{label}: now agrees with PROJ, clear the gap flag"))
        }
        (_, Expect::KnownGap) => None,
    }
}

#[test]
fn named_crs_pairs_match_cs2cs() {
    let Some(program) = proj_cs2cs() else {
        return;
    };
    let ours = std::path::Path::new(env!("CARGO_BIN_EXE_cs2cs"));
    let mut problems = Vec::new();
    for pair in PAIRS {
        let proj = proj_rs::Proj::new_known_crs(pair.from, pair.to, None)
            .unwrap_or_else(|error| panic!("{} -> {}: {error}", pair.from, pair.to));
        for &point in pair.points {
            let want = match run_cs2cs(&program, pair, point) {
                Ok(want) => want,
                Err(error) => {
                    problems.push(format!("reference {} -> {}: {error}", pair.from, pair.to));
                    continue;
                }
            };
            let facade = proj.convert(point).map_err(|error| error.to_string());
            problems.extend(judge(
                "facade",
                pair,
                point,
                pair.facade_expect(),
                facade,
                want,
            ));
            let cli = run_cs2cs(ours, pair, point);
            problems.extend(judge("cs2cs", pair, point, pair.cli_expect(), cli, want));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
