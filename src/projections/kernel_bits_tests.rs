//! The vector kernels of the projections give the same bits on every architecture.
//!
//! AVX2 and NEON round the same way (IEEE 754, fused multiply-add), and the kernels only combine
//! the operations of `V4`: one digest per kernel, measured on the AVX2 path, must be the NEON one.
//! The points lie well inside the domain and their number is a multiple of four, so that no group
//! goes to the scalar path, which calls the C library of the platform. Oblique `laea` is left out:
//! its set-up uses the `sin`/`atan` of that library, and Apple silicon gave other constants.

use super::{geographic_points, Way};
use crate::engine::{Coord, Operation};

/// FNV-1a over the bits of the three coordinates of every point.
fn digest(points: &[Coord]) -> u64 {
    points
        .iter()
        .flat_map(|point| [point.x, point.y, point.z])
        .fold(0xcbf2_9ce4_8422_2325, |hash, value| {
            value.to_bits().to_le_bytes().iter().fold(hash, |h, byte| {
                (h ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            })
        })
}

fn batch(op: &Operation, points: &[Coord], way: Way) -> Vec<Coord> {
    let mut out = points.to_vec();
    let mut failed = vec![None; points.len()];
    match way {
        Way::Forward => op.forward_batch(&mut out, &mut failed),
        Way::Inverse => op.inverse_batch(&mut out, &mut failed),
    }
    assert!(failed.iter().all(Option::is_none), "no point is refused");
    out
}

/// A name, a definition, and the box of its points in degrees.
type Case = (&'static str, &'static str, (f64, f64), (f64, f64));

const CASES: [Case; 8] = [
    (
        "laea equatorial",
        "+proj=laea +lat_0=0 +lon_0=20 +ellps=WGS84",
        (10.0, 30.0),
        (-15.0, 15.0),
    ),
    (
        "somerc",
        "+proj=somerc +lat_0=46.9524055555556 +lon_0=7.43958333333333 +k_0=1 +x_0=2600000 \
         +y_0=1200000 +ellps=bessel",
        (6.0, 10.0),
        (46.0, 48.0),
    ),
    (
        "sterea",
        "+proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 +k=0.9999079 \
         +x_0=155000 +y_0=463000 +ellps=bessel",
        (3.4, 7.2),
        (50.8, 53.5),
    ),
    (
        "sterea sphere",
        "+proj=sterea +lat_0=46 +lon_0=25 +k=0.99975 +x_0=500000 +y_0=500000 +R=6371000",
        (22.0, 28.0),
        (44.0, 48.0),
    ),
    (
        "cart",
        "+proj=cart +ellps=GRS80",
        (-30.0, 40.0),
        (30.0, 60.0),
    ),
    (
        "cart geocentric",
        "+proj=cart +ellps=bessel +geoc",
        (-30.0, 40.0),
        (30.0, 60.0),
    ),
    (
        "helmert",
        "+proj=helmert +x=482.5 +y=-130.6 +z=564.6 +rx=-1.042 +ry=-0.214 +rz=-0.631 +s=8.15 \
         +convention=coordinate_frame",
        (-30.0, 40.0),
        (30.0, 60.0),
    ),
    (
        "helmert position vector",
        "+proj=helmert +x=-28 +y=121 +z=77 +rx=0.3 +ry=0.2 +rz=0.1 +s=2 \
         +convention=position_vector",
        (-30.0, 40.0),
        (30.0, 60.0),
    ),
];

/// The digests of the forward and of the inverse batch of each case, measured on the AVX2 path.
const EXPECTED: [(u64, u64); 8] = [
    (0xf53b_1763_ed35_0af9, 0xf1aa_08d3_43aa_8524),
    (0x6879_caf3_8323_dcf0, 0x82d0_1771_44a6_5444),
    (0xecb4_72dd_3573_0605, 0x26bf_2b77_9a08_4d33),
    (0x0c17_8f41_1e13_7328, 0x6932_1726_94fa_a9f9),
    (0xbb7e_b916_0e3a_f29d, 0xf034_6baf_d397_1fc2),
    (0x55b0_0f69_dd82_2fd5, 0xb54a_408f_dace_5a04),
    (0xbd54_bf7e_fdcb_73bc, 0x95ee_c8ff_f16d_f11c),
    (0x45e4_a231_cb94_d06b, 0x85fa_6c48_b6a4_aff7),
];

#[test]
fn kernels_keep_their_bits_on_every_architecture() {
    if !crate::simd::has_lanes() {
        return;
    }
    let cartesian = Operation::create("+proj=cart +ellps=GRS80").expect("cart");
    let mut measured = Vec::new();
    for (seed, (name, definition, lon, lat)) in (2..).zip(CASES) {
        let op = Operation::create(definition).expect(name);
        let mut geographic = geographic_points(4_000, lon, lat, seed);
        geographic.truncate(4_000);
        // Helmert works on geocentric coordinates: the points go through `cart` first.
        let input = if name.starts_with("helmert") {
            batch(&cartesian, &geographic, Way::Forward)
        } else {
            geographic
        };
        let forward = batch(&op, &input, Way::Forward);
        let inverse = batch(&op, &forward, Way::Inverse);
        measured.push((digest(&forward), digest(&inverse)));
    }
    for ((name, ..), got) in CASES.iter().zip(&measured) {
        eprintln!("DIGEST {name} ({:#018x}, {:#018x}),", got.0, got.1);
    }
    for ((name, ..), (got, want)) in CASES.iter().zip(measured.iter().zip(EXPECTED)) {
        assert_eq!(*got, want, "{name}");
    }
}
