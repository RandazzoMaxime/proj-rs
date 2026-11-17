//! The bits `convert_array`, `project_array` and `forward_slice` produce, fixed before the
//! conversion went in place (`docs/PORTING-memory.md`). Moving the results straight into the
//! slice must not change a bit, on any pair, direction, element type or backend.

// The digests are glibc's on x86-64: another libm, or another batch path, gives other bits.
#![cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]

use proj_rs::{Backend, Context, Crs, Point, Proj, Transformer};

fn france(count: usize) -> Vec<(f64, f64)> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut unit = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the top 53 bits of the generator are exact in f64"
        )]
        let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    };
    (0..count)
        .map(|_| (-4.0 + unit() * 12.0, 42.0 + unit() * 9.0))
        .collect()
}

/// FNV-1a over the bits of every value.
fn digest(values: impl IntoIterator<Item = f64>) -> u64 {
    values
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, value| {
            value.to_bits().to_le_bytes().iter().fold(hash, |h, byte| {
                (h ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            })
        })
}

fn flat(points: &[(f64, f64)]) -> impl Iterator<Item = f64> + '_ {
    points.iter().flat_map(|&(x, y)| [x, y])
}

fn digests() -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for (from, to) in [
        ("EPSG:4326", "EPSG:2154"),
        ("EPSG:4326", "EPSG:32631"),
        ("EPSG:4326", "EPSG:3857"),
        ("EPSG:4326", "EPSG:3035"),
        ("EPSG:4267", "EPSG:4326"),
    ] {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        let mut points = france(9_001);
        proj.convert_array(&mut points).expect("convert_array");
        out.push((format!("convert {from} {to}"), digest(flat(&points))));
        if from == "EPSG:4326" {
            proj.project_array(&mut points, true).expect("inverse");
            out.push((format!("inverse {from} {to}"), digest(flat(&points))));
        }
    }
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("pair");
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the test narrows degrees to the f32 it feeds"
    )]
    let mut single: Vec<(f32, f32)> = france(1_001)
        .into_iter()
        .map(|(x, y)| (x as f32, y as f32))
        .collect();
    proj.convert_array(&mut single).expect("f32");
    out.push((
        "f32".to_owned(),
        digest(
            single
                .iter()
                .flat_map(|&(x, y)| [f64::from(x), f64::from(y)]),
        ),
    ));
    let utm = Proj::new("+proj=utm +zone=31 +ellps=WGS84").expect("utm");
    let mut radians: Vec<(f64, f64)> = france(4_100)
        .into_iter()
        .map(|(x, y)| (x.to_radians(), y.to_radians()))
        .collect();
    utm.convert_array(&mut radians).expect("utm");
    out.push(("utm".to_owned(), digest(flat(&radians))));

    let ctx = Context::default();
    let crs = |text: &str| Crs::from_user_input(&ctx, text).expect("crs");
    for (from, to) in [("EPSG:4326", "EPSG:2154"), ("EPSG:4267", "EPSG:4269")] {
        for backend in [Backend::Auto, Backend::Scalar] {
            let t = Transformer::builder(&crs(from), &crs(to))
                .backend(backend)
                .build()
                .expect("transformer");
            let mut points: Vec<Point> = france(5_000)
                .into_iter()
                .map(|(lon, lat)| Point::new(lat, lon))
                .collect();
            t.forward_slice(&mut points).expect("forward_slice");
            out.push((
                format!("forward_slice {from} {to} {backend:?}"),
                digest(points.iter().flat_map(|p| [p.x, p.y, p.z])),
            ));
            t.inverse_slice(&mut points).expect("inverse_slice");
            out.push((
                format!("inverse_slice {from} {to} {backend:?}"),
                digest(points.iter().flat_map(|p| [p.x, p.y, p.z])),
            ));
        }
    }
    out
}

#[test]
fn the_results_keep_their_bits() {
    let got = digests();
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        for (name, hash) in &got {
            eprintln!("    (\"{name}\", {hash:#018x}),");
        }
    }
    assert_eq!(got.len(), GOLDEN.len());
    for ((name, hash), (want_name, want)) in got.iter().zip(GOLDEN) {
        assert_eq!(name, want_name);
        assert_eq!(*hash, *want, "{name}");
    }
}

const GOLDEN: &[(&str, u64)] = &[
    ("convert EPSG:4326 EPSG:2154", 0x3c07f6ca54a9bbaf),
    ("inverse EPSG:4326 EPSG:2154", 0xca230f8a43e2b98a),
    ("convert EPSG:4326 EPSG:32631", 0xbbb1a711a1e210dc),
    ("inverse EPSG:4326 EPSG:32631", 0x93a48215125ccef0),
    ("convert EPSG:4326 EPSG:3857", 0xa2b720cebc1a118a),
    ("inverse EPSG:4326 EPSG:3857", 0xb521ea30aa15682f),
    ("convert EPSG:4326 EPSG:3035", 0xf7e2edc8d238100f),
    ("inverse EPSG:4326 EPSG:3035", 0x12531ae399462e84),
    ("convert EPSG:4267 EPSG:4326", 0xd2b331a2dea7f399),
    ("f32", 0xf2cc08bb7eaeef8f),
    ("utm", 0xc13a460b6754fada),
    ("forward_slice EPSG:4326 EPSG:2154 Auto", 0x4adcb8b7bb73fb3a),
    ("inverse_slice EPSG:4326 EPSG:2154 Auto", 0x4d93f3d7f7daee62),
    (
        "forward_slice EPSG:4326 EPSG:2154 Scalar",
        0x2234745ba99384ce,
    ),
    (
        "inverse_slice EPSG:4326 EPSG:2154 Scalar",
        0x185bfe2a6198172f,
    ),
    ("forward_slice EPSG:4267 EPSG:4269 Auto", 0xd6cbc3679d9266fc),
    ("inverse_slice EPSG:4267 EPSG:4269 Auto", 0xd6cbc3679d9266fc),
    (
        "forward_slice EPSG:4267 EPSG:4269 Scalar",
        0xd6cbc3679d9266fc,
    ),
    (
        "inverse_slice EPSG:4267 EPSG:4269 Scalar",
        0xd6cbc3679d9266fc,
    ),
];
