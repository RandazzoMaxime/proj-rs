//! The whole of Karney's `geodesic.c` against what PROJ 9.9.0 computes with it
//! (`tools/gen-geodesic-full.c`, `tests/geodesic/full.txt`): the inverse and the direct problems
//! with every output (distance, azimuths, reduced length, scales, area), the line through two
//! points, and the polygon calls (`geod_polygon_*`, `geod_polygonarea`).
//!
//! The expected values are the C library's. The two differ only where the math library does: the
//! tolerance is the one of `geodesic_reference.rs`, in the unit of each quantity.

use proj_rs::geodesic::{
    AreaRange, Geodesic, Orientation, Polygon, PolygonKind, GEOD_ARCMODE, GEOD_AREA, GEOD_AZIMUTH,
    GEOD_DISTANCE, GEOD_DISTANCE_IN, GEOD_GEODESICSCALE, GEOD_LATITUDE, GEOD_LONGITUDE,
    GEOD_REDUCEDLENGTH,
};

/// Semi-major axis and flattening of the ellipsoids of the fixture.
fn ellipsoid(name: &str) -> (f64, f64) {
    match name {
        "wgs84" => (6_378_137.0, 1.0 / 298.257_223_563),
        "grs80" => (6_378_137.0, 1.0 / 298.257_222_101),
        "bessel" => (6_377_397.155, 1.0 / 299.152_812_8),
        "sphere" => (6_371_000.0, 0.0),
        "flat20" => (6_378_137.0, 0.05),
        "prolate" => (6_378_137.0, -0.003_352_810_664_747_48),
        "unit" => (1.0, 1.0 / 297.0),
        other => panic!("unknown ellipsoid {other}"),
    }
}

/// The inputs, then the outputs after the `|`.
fn split(line: &str) -> (Vec<&str>, Vec<f64>) {
    let (inputs, outputs) = line.split_once('|').expect("a bar");
    (
        inputs.split_whitespace().collect(),
        outputs
            .split_whitespace()
            .map(|word| word.parse().expect("a number"))
            .collect(),
    )
}

fn number(word: &str) -> f64 {
    word.parse().expect("a number")
}

/// The gap of two angles in degrees, modulo a turn.
fn angle_gap(x: f64, y: f64) -> f64 {
    ((x - y + 180.0).rem_euclid(360.0) - 180.0).abs()
}

/// What each quantity may differ by: the unit of the quantity at the size of the ellipsoid.
struct Scale {
    a: f64,
    /// The distance of the line, which sets how much an azimuth may move.
    distance: f64,
}

impl Scale {
    /// Lengths: a micrometre on the Earth.
    fn length(&self) -> f64 {
        self.a * 2e-12
    }

    /// Areas: the square of a length.
    fn area(&self) -> f64 {
        self.a * self.a * 2e-12
    }

    /// An azimuth of a line `distance` long moves by `precision / distance`.
    fn azimuth(&self) -> f64 {
        1e-9 + 5e-7 * self.a / self.distance.max(self.a * 1e-9)
    }
}

struct Failures(Vec<String>);

impl Failures {
    fn number(&mut self, line: &str, what: &str, got: f64, want: f64, tolerance: f64) {
        let gap = if want.is_nan() && got.is_nan() {
            0.0
        } else {
            (got - want).abs()
        };
        if gap.is_nan() || gap > tolerance {
            self.0.push(format!(
                "{line}\n  {what}: {got} against {want}, gap {gap:e}"
            ));
        }
    }

    fn angle(&mut self, line: &str, what: &str, got: f64, want: f64, tolerance: f64) {
        let gap = angle_gap(got, want);
        if gap.is_nan() || gap > tolerance {
            self.0.push(format!(
                "{line}\n  {what}: {got} against {want}, gap {gap:e}"
            ));
        }
    }

    fn finish(self, kind: &str) {
        assert!(
            self.0.is_empty(),
            "{kind}: {} failures:\n{}",
            self.0.len(),
            self.0.join("\n")
        );
    }
}

const ALL: u32 = GEOD_DISTANCE | GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE | GEOD_AREA;

#[test]
fn inverse_with_every_output_matches_proj() {
    let mut failures = Failures(Vec::new());
    for line in include_str!("geodesic/full.txt")
        .lines()
        .filter(|line| line.starts_with("I "))
    {
        let (inputs, want) = split(line);
        let (a, f) = ellipsoid(inputs[1]);
        let [lat1, lon1, lat2, lon2] = [2, 3, 4, 5].map(|i| number(inputs[i]));
        let got = Geodesic::new(a, f).gen_inverse(lat1, lon1, lat2, lon2, ALL);
        let scale = Scale {
            a,
            distance: want[1],
        };
        // a12, s12, azi1, azi2, m12, M12, M21, S12
        failures.angle(line, "a12", got.a12, want[0], scale.azimuth());
        failures.number(line, "s12", got.s12, want[1], scale.length());
        failures.angle(line, "azi1", got.azi1, want[2], scale.azimuth());
        failures.angle(line, "azi2", got.azi2, want[3], scale.azimuth());
        failures.number(line, "m12", got.m12, want[4], scale.length());
        failures.number(line, "M12", got.big_m12, want[5], 1e-9);
        failures.number(line, "M21", got.big_m21, want[6], 1e-9);
        failures.number(line, "S12", got.big_s12, want[7], scale.area());
    }
    failures.finish("inverse");
}

#[test]
fn direct_with_every_output_matches_proj() {
    let mut failures = Failures(Vec::new());
    for line in include_str!("geodesic/full.txt")
        .lines()
        .filter(|line| line.starts_with("D "))
    {
        let (inputs, want) = split(line);
        let (a, f) = ellipsoid(inputs[1]);
        let [lat1, lon1, azi1] = [2, 3, 4].map(|i| number(inputs[i]));
        let flags: u32 = inputs[5].parse().expect("flags");
        let s12_a12 = number(inputs[6]);
        let got = Geodesic::new(a, f).gen_direct(
            lat1,
            lon1,
            azi1,
            flags,
            s12_a12,
            ALL | GEOD_LATITUDE | GEOD_LONGITUDE | GEOD_AZIMUTH,
        );
        let scale = Scale {
            a,
            distance: if flags & GEOD_ARCMODE != 0 {
                want[4]
            } else {
                s12_a12
            },
        };
        // a12, lat2, lon2, azi2, s12, m12, M12, M21, S12
        failures.number(line, "a12", got.a12, want[0], 1e-9);
        failures.angle(line, "lat2", got.lat2, want[1], 1e-9);
        // An unrolled longitude is compared as it is, the others modulo a turn.
        if flags & proj_rs::geodesic::GEOD_LONG_UNROLL != 0 {
            failures.number(line, "lon2", got.lon2, want[2], 1e-8);
        } else {
            failures.angle(line, "lon2", got.lon2, want[2], 1e-8);
        }
        failures.angle(line, "azi2", got.azi2, want[3], scale.azimuth());
        failures.number(line, "s12", got.s12, want[4], scale.length());
        failures.number(line, "m12", got.m12, want[5], scale.length());
        failures.number(line, "M12", got.big_m12, want[6], 1e-9);
        failures.number(line, "M21", got.big_m21, want[7], 1e-9);
        failures.number(line, "S12", got.big_s12, want[8], scale.area());
    }
    failures.finish("direct");
}

#[test]
fn the_line_through_two_points_matches_proj() {
    let mut failures = Failures(Vec::new());
    let caps = GEOD_LATITUDE
        | GEOD_LONGITUDE
        | GEOD_AZIMUTH
        | GEOD_DISTANCE
        | GEOD_REDUCEDLENGTH
        | GEOD_GEODESICSCALE
        | GEOD_AREA
        | GEOD_DISTANCE_IN;
    for line in include_str!("geodesic/full.txt")
        .lines()
        .filter(|line| line.starts_with("L "))
    {
        let (inputs, want) = split(line);
        let (a, f) = ellipsoid(inputs[1]);
        let [lat1, lon1, lat2, lon2] = [2, 3, 4, 5].map(|i| number(inputs[i]));
        let earth = Geodesic::new(a, f);
        let l = earth.inverse_line(lat1, lon1, lat2, lon2, caps);
        let scale = Scale {
            a,
            distance: want[2],
        };
        // azi1, a13, s13, then three positions of nine numbers each.
        failures.angle(line, "azi1", l.azimuth(), want[0], scale.azimuth());
        failures.number(line, "a13", l.arc(), want[1], 1e-9);
        failures.number(line, "s13", l.distance(), want[2], scale.length());
        for i in 1..=3_u32 {
            let at = l.distance() * f64::from(i) / 4.0;
            let p = l.gen_position(0, at, caps);
            let index = usize::try_from(i).expect("a small index");
            let w = &want[3 + 9 * (index - 1)..];
            let tag = |what: &str| format!("{what} at {i}/4");
            failures.number(line, &tag("a12"), p.a12, w[0], 1e-9);
            failures.angle(line, &tag("lat2"), p.lat2, w[1], 1e-9);
            failures.angle(line, &tag("lon2"), p.lon2, w[2], 1e-8);
            failures.angle(line, &tag("azi2"), p.azi2, w[3], scale.azimuth());
            failures.number(line, &tag("s12"), p.s12, w[4], scale.length());
            failures.number(line, &tag("m12"), p.m12, w[5], scale.length());
            failures.number(line, &tag("M12"), p.big_m12, w[6], 1e-9);
            failures.number(line, &tag("M21"), p.big_m21, w[7], 1e-9);
            failures.number(line, &tag("S12"), p.big_s12, w[8], scale.area());
        }
    }
    failures.finish("line");
}

#[test]
fn polygons_match_proj() {
    let mut failures = Failures(Vec::new());
    for line in include_str!("geodesic/full.txt")
        .lines()
        .filter(|line| line.starts_with("P "))
    {
        let (inputs, want) = split(line);
        let (a, f) = ellipsoid(inputs[1]);
        let n: usize = inputs[2].parse().expect("a count");
        let vertices: Vec<(f64, f64)> = (0..n)
            .map(|i| (number(inputs[3 + 2 * i]), number(inputs[4 + 2 * i])))
            .collect();
        let rest = &inputs[3 + 2 * n..];
        let [extra_lat, extra_lon, eazi, edist] = [0, 1, 2, 3].map(|i| number(rest[i]));
        let earth = Geodesic::new(a, f);
        let scale = Scale { a, distance: a };
        // The length of a perimeter adds the lengths of its edges: a few ulps each.
        let length = scale.length() * f64::from(u32::try_from(n).expect("a count"));
        let area = scale.area() * f64::from(u32::try_from(n).expect("a count"));

        let mut polygon = Polygon::new(PolygonKind::Polygon);
        for &(lat, lon) in &vertices {
            polygon.add_point(&earth, lat, lon);
        }
        let mut w = want.iter().copied();
        for orientation in [Orientation::CounterClockwise, Orientation::Clockwise] {
            for range in [AreaRange::Unsigned, AreaRange::Signed] {
                let totals = polygon.compute(&earth, orientation, range);
                failures.number(
                    line,
                    "area",
                    totals.area.expect("an area"),
                    w.next().expect("w"),
                    area,
                );
                failures.number(
                    line,
                    "perimeter",
                    totals.perimeter,
                    w.next().expect("w"),
                    length,
                );
            }
        }
        // The calls of C are made with (reverse, sign) = (0, 1) for the point and (1, 0) for the edge.
        let test = polygon.test_point(
            &earth,
            extra_lat,
            extra_lon,
            Orientation::CounterClockwise,
            AreaRange::Signed,
        );
        failures.number(
            line,
            "testpoint area",
            test.area.expect("an area"),
            w.next().expect("w"),
            area,
        );
        failures.number(
            line,
            "testpoint perimeter",
            test.perimeter,
            w.next().expect("w"),
            length,
        );
        let test = polygon.test_edge(
            &earth,
            eazi,
            edist,
            Orientation::Clockwise,
            AreaRange::Unsigned,
        );
        failures.number(
            line,
            "testedge area",
            test.area.expect("an area"),
            w.next().expect("w"),
            area,
        );
        failures.number(
            line,
            "testedge perimeter",
            test.perimeter,
            w.next().expect("w"),
            length,
        );
        polygon.add_edge(&earth, eazi, edist);
        let totals = polygon.compute(&earth, Orientation::CounterClockwise, AreaRange::Signed);
        failures.number(
            line,
            "addedge area",
            totals.area.expect("an area"),
            w.next().expect("w"),
            area,
        );
        failures.number(
            line,
            "addedge perimeter",
            totals.perimeter,
            w.next().expect("w"),
            length,
        );

        let mut polyline = Polygon::new(PolygonKind::Polyline);
        for &(lat, lon) in &vertices {
            polyline.add_point(&earth, lat, lon);
        }
        let totals = polyline.compute(&earth, Orientation::CounterClockwise, AreaRange::Signed);
        assert!(totals.area.is_none(), "a polyline has no area");
        failures.number(
            line,
            "polyline length",
            totals.perimeter,
            w.next().expect("w"),
            length,
        );

        let (all_area, all_perimeter) = earth.polygon_area(&vertices);
        failures.number(
            line,
            "polygonarea area",
            all_area,
            w.next().expect("w"),
            area,
        );
        failures.number(
            line,
            "polygonarea perimeter",
            all_perimeter,
            w.next().expect("w"),
            length,
        );
    }
    failures.finish("polygon");
}

#[test]
fn a_polygon_with_no_point_or_one_has_no_area() {
    let earth = Geodesic::wgs84();
    let mut polygon = Polygon::new(PolygonKind::Polygon);
    let totals = polygon.compute(&earth, Orientation::CounterClockwise, AreaRange::Signed);
    assert_eq!(
        (totals.num, totals.area, totals.perimeter),
        (0, Some(0.0), 0.0)
    );
    polygon.add_point(&earth, 10.0, 20.0);
    let totals = polygon.compute(&earth, Orientation::CounterClockwise, AreaRange::Signed);
    assert_eq!(
        (totals.num, totals.area, totals.perimeter),
        (1, Some(0.0), 0.0)
    );
    // An edge needs a starting point, and a test edge without one is NaN.
    let empty = Polygon::new(PolygonKind::Polygon);
    let test = empty.test_edge(
        &earth,
        0.0,
        1000.0,
        Orientation::CounterClockwise,
        AreaRange::Signed,
    );
    assert_eq!(test.num, 0);
    assert!(test.perimeter.is_nan());
}

#[test]
fn the_area_of_a_known_polygon() {
    // The one-degree cell at the equator of the WGS 84: 12308778361.469 m^2 (GeographicLib's
    // test of `geod_polygonarea`, whose reference is the C library's own).
    let earth = Geodesic::wgs84();
    let (area, perimeter) = earth.polygon_area(&[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]);
    assert!((area.abs() - 12_308_778_361.469).abs() < 1.0, "{area}");
    assert!((perimeter - 443_770.917).abs() < 1.0, "{perimeter}");
}
