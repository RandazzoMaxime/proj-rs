//! The bits of the AVX2 batch path, fixed before its intrinsics moved behind the safe API of
//! `pulp` (`docs/PORTING-simd.md`, "Sans `unsafe`"). The kernels, the order of the operations
//! and the FMA are the same: not one bit may change, on any kernel, in the fast path or in the
//! groups that fall back to the scalar path (non-finite values, poles, out of domain).

// The digests are those of the AVX2 path with glibc on x86-64: the scalar path, or another libm
// for the fallbacks, gives other bits.
#![cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]

use proj_rs::{Ellipsoid, Hemisphere, Lcc, Mercator, Proj, Tmerc, UtmZone};

struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the top 53 bits of the generator are exact in f64"
        )]
        let u = ((self.0 >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + self.unit() * (high - low)
    }
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

/// `count` points in a box. With `odd_every`, every such point is replaced by a value that
/// sends its group to the scalar path (NaN, an infinity, a pole, far out of range).
fn points(
    seed: u64,
    count: usize,
    (lon, lat): ((f64, f64), (f64, f64)),
    odd_every: Option<usize>,
) -> (Vec<f64>, Vec<f64>) {
    let mut rng = Rng(seed);
    let odd = [
        (f64::NAN, 45.0),
        (3.0, f64::NAN),
        (f64::INFINITY, 10.0),
        (2.0, 90.0),
        (2.0, -90.0),
        (179.0, 89.999_999),
        (540.0, 45.0),
        (2.0, 1e6),
    ];
    (0..count)
        .map(|i| {
            if let Some(every) = odd_every.filter(|every| i % every == every - 1) {
                odd[(i / every) % odd.len()]
            } else {
                (rng.range(lon.0, lon.1), rng.range(lat.0, lat.1))
            }
        })
        .unzip()
}

type Batch<'a> = &'a dyn Fn(&[f64], &[f64], &mut [f64], &mut [f64]);

/// A bare projection: its name, forward and inverse batches, and the box of its points.
type BareCase<'a> = (&'a str, Batch<'a>, Batch<'a>, (f64, f64), (f64, f64));

fn batch_digest(batch: Batch<'_>, (a, b): &(Vec<f64>, Vec<f64>)) -> (u64, (Vec<f64>, Vec<f64>)) {
    let (mut x, mut y) = (vec![0.0; a.len()], vec![0.0; a.len()]);
    batch(a, b, &mut x, &mut y);
    (digest(x.iter().chain(&y).copied()), (x, y))
}

fn digests() -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let zone = UtmZone::try_new(31).expect("zone");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let l93 = Lcc::lambert93();
    let web = Mercator::web();
    let cases: [BareCase<'_>; 3] = [
        (
            "utm31",
            &|a, b, c, d| utm.forward_batch(a, b, c, d),
            &|a, b, c, d| utm.inverse_batch(a, b, c, d),
            (-6.0, 12.0),
            (-80.0, 84.0),
        ),
        (
            "lambert93",
            &|a, b, c, d| l93.forward_batch(a, b, c, d),
            &|a, b, c, d| l93.inverse_batch(a, b, c, d),
            (-180.0, 180.0),
            (-89.0, 89.0),
        ),
        (
            "webmerc",
            &|a, b, c, d| web.forward_batch(a, b, c, d),
            &|a, b, c, d| web.inverse_batch(a, b, c, d),
            (-180.0, 180.0),
            (-85.0, 85.0),
        ),
    ];
    for (seed, (name, forward, inverse, lon, lat)) in (1..).zip(cases) {
        let input = points(seed, 40_003, (lon, lat), Some(97));
        let (hash, projected) = batch_digest(forward, &input);
        out.push((format!("{name} forward_batch"), hash));
        let (hash, _) = batch_digest(inverse, &projected);
        out.push((format!("{name} inverse_batch"), hash));
    }

    // The kernels behind the facade: Transverse Mercator, Lambert, Mercator, Web Mercator,
    // Lambert azimuthal, oblique stereographic, Swiss oblique Mercator, and the datum shifts
    // through geocentric coordinates and Helmert (ED50, OSGB 36, Belge 72).
    for (seed, (from, to, lon, lat)) in (11..).zip([
        ("EPSG:4326", "EPSG:2154", (-4.0, 8.0), (42.0, 51.0)),
        ("EPSG:4326", "EPSG:32631", (0.0, 6.0), (40.0, 60.0)),
        ("EPSG:4326", "EPSG:3857", (-170.0, 170.0), (-80.0, 80.0)),
        ("EPSG:4326", "EPSG:3395", (-170.0, 170.0), (-80.0, 80.0)),
        ("EPSG:4326", "EPSG:3035", (-10.0, 30.0), (35.0, 70.0)),
        ("EPSG:4326", "EPSG:28992", (3.4, 7.2), (50.7, 53.6)),
        ("EPSG:4326", "EPSG:2056", (5.9, 10.5), (45.8, 47.8)),
        ("EPSG:4230", "EPSG:4326", (-10.0, 30.0), (35.0, 70.0)),
        ("EPSG:4326", "EPSG:27700", (-8.0, 2.0), (49.9, 60.9)),
        ("EPSG:4326", "EPSG:31370", (2.5, 6.4), (49.5, 51.5)),
        ("EPSG:4326", "EPSG:3034", (-10.0, 30.0), (35.0, 70.0)),
    ]) {
        let proj = Proj::new_known_crs(from, to, None).expect("pair");
        let (lons, lats) = points(seed, 20_003, (lon, lat), None);
        // `new_known_crs` reads (longitude, latitude), as `proj_normalize_for_visualization`.
        let mut pairs: Vec<(f64, f64)> = lons.into_iter().zip(lats).collect();
        let result = proj.convert_array(&mut pairs).map(|done| done.len());
        out.push((
            format!("convert {from} {to} ({})", result.is_ok()),
            digest(pairs.iter().flat_map(|&(x, y)| [x, y])),
        ));
        if result.is_ok() {
            let back = proj.project_array(&mut pairs, true).map(|done| done.len());
            out.push((
                format!("inverse {from} {to} ({})", back.is_ok()),
                digest(pairs.iter().flat_map(|&(x, y)| [x, y])),
            ));
        }
    }
    out
}

#[test]
fn the_batch_path_keeps_its_bits() {
    if proj_rs::batch_path() != "avx2" {
        eprintln!("no AVX2 on this machine: the digests are those of the AVX2 path, skipped");
        return;
    }
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
    ("utm31 forward_batch", 0x6cc8f648235fafcc),
    ("utm31 inverse_batch", 0x96ed9be93e673a36),
    ("lambert93 forward_batch", 0xec85877847d91a65),
    ("lambert93 inverse_batch", 0x0c969db210dfcaff),
    ("webmerc forward_batch", 0x1618cd5e97590bd3),
    ("webmerc inverse_batch", 0x3ea4d05a6a3b973d),
    ("convert EPSG:4326 EPSG:2154 (true)", 0xefc0e5a6ef7fc657),
    ("inverse EPSG:4326 EPSG:2154 (true)", 0x27cd817ae442766b),
    ("convert EPSG:4326 EPSG:32631 (true)", 0xfebb7320d92d4e97),
    ("inverse EPSG:4326 EPSG:32631 (true)", 0xeef6190ed3c5fe55),
    ("convert EPSG:4326 EPSG:3857 (true)", 0x721d38edea7fd9b3),
    ("inverse EPSG:4326 EPSG:3857 (true)", 0x471efcc9a25162c3),
    ("convert EPSG:4326 EPSG:3395 (true)", 0x368779de69086480),
    ("inverse EPSG:4326 EPSG:3395 (true)", 0x87000d34b8f3ad60),
    ("convert EPSG:4326 EPSG:3035 (true)", 0x9a8bd13bbc05e3a9),
    ("inverse EPSG:4326 EPSG:3035 (true)", 0x5e740725f5e4c7f7),
    ("convert EPSG:4326 EPSG:28992 (true)", 0x10fb35cac39bdb39),
    ("inverse EPSG:4326 EPSG:28992 (true)", 0xa29cf99130e6af7b),
    ("convert EPSG:4326 EPSG:2056 (true)", 0xfd7541f136feea90),
    ("inverse EPSG:4326 EPSG:2056 (true)", 0x74d4649108d9e41c),
    ("convert EPSG:4230 EPSG:4326 (true)", 0x89aecd7bedf30162),
    ("inverse EPSG:4230 EPSG:4326 (true)", 0xc50cb0279cc815de),
    ("convert EPSG:4326 EPSG:27700 (true)", 0xf89a644f8e086b91),
    ("inverse EPSG:4326 EPSG:27700 (true)", 0x53804c2a4bdbcba3),
    ("convert EPSG:4326 EPSG:31370 (true)", 0x473d8d98774f9302),
    ("inverse EPSG:4326 EPSG:31370 (true)", 0x3af2b4710b5a3bcf),
    ("convert EPSG:4326 EPSG:3034 (true)", 0x3616f19c615e3a90),
    ("inverse EPSG:4326 EPSG:3034 (true)", 0x794c014b94f80a25),
];
