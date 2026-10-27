//! Accord direct avec `cs2cs`. Le binaire PROJ recalcule les points.

use proj_rs::{Cone, Ellipsoid, Hemisphere, Lcc, Tmerc, UtmZone};
use std::io::Write;
use std::process::{Command, Stdio};

fn cs2cs(from: &str, to: &str, lon: f64, lat: f64) -> (f64, f64) {
    let mut child = Command::new("cs2cs")
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
    let zone = UtmZone::try_new(31).expect("zone");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let (east, north) = cs2cs(
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
        "+proj=latlong +ellps=GRS80",
        "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
        2.2945,
        48.858_222,
    );
    let xy = lcc.forward(2.2945, 48.858_222).expect("forward");
    assert!((xy.east - east).abs() < 1e-6, "{} vs {east}", xy.east);
    assert!((xy.north - north).abs() < 1e-6, "{} vs {north}", xy.north);
}
