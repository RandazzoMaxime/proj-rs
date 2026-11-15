//! `proj -b` rend les octets de PROJ 9.9 au bit près, en sortie binaire et en entrée binaire.
//!
//! Les attendus sont les mots de 64 bits natifs de `proj -b` de la référence
//! (`target/proj-reference/build/bin/proj`) sur les entrées `FWD` (directes) et `INV` (inverses).
//! Régénérer une ligne : `proj -b [-I] ARGS`, entrée = les doubles natifs de `FWD` ou `INV`,
//! sortie lue en mots de 64 bits natifs.

use std::io::Write;
use std::process::{Command, Stdio};

/// Points directs : longitude et latitude en radians.
const FWD: [(f64, f64); 6] = [
    (0.05, 0.9),
    (-1.2, 0.3),
    (0.2, -0.7),
    (0.3, 0.0),
    (0.45, 1.4),
    (-0.9, -0.35),
];

/// Points inverses : x et y en mètres. Le dernier sort du domaine d'`aea` (PROJ rend NaN).
const INV: [(f64, f64); 6] = [
    (500_000.0, 4_500_000.0),
    (250_000.0, 100_000.0),
    (-100_000.0, 2_000_000.0),
    (700_000.0, 6_600_000.0),
    (1_200_000.0, -900_000.0),
    (-3_600_000.0, 8_160_000.0),
];

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    inverse: bool,
    expected: &'static [u64],
}

const CASES: &[Case] = &[
    Case {
        name: "merc_ell fwd",
        args: &["+proj=merc", "+ellps=GRS80"],
        inverse: false,
        expected: &[
            0x411376eb66666667,
            0x415983cf772c2aac,
            0xc15d326119999999,
            0x413d73c964fec1cd,
            0x413376eb66666667,
            0xc15284367dcd363b,
            0x413d326119999999,
            0x0000000000000000,
            0x4145e5c8d3333333,
            0x416dd26815900a6d,
            0xc155e5c8d3333333,
            0xc141474bde81e222,
        ],
    },
    Case {
        name: "merc_ell inv",
        args: &["+proj=merc", "+ellps=GRS80"],
        inverse: true,
        expected: &[
            0x3fb4118ce3fa1eb2,
            0x3fe5030401835586,
            0x3fa4118ce3fa1eb2,
            0x3f902990d728252b,
            0xbf900e0a4ffb4bc2,
            0x3fd3df4b8f1b9001,
            0x3fbc18920bf7c493,
            0x3fec8582a559558a,
            0x3fc8150f77f8f1a3,
            0xbfc21f2c6a98620c,
            0xbfe20fcb99fab53a,
            0x3ff07f37e466ca8e,
        ],
    },
    Case {
        name: "merc_sph fwd",
        args: &["+proj=merc", "+R=6371000"],
        inverse: false,
        expected: &[
            0x4113715800000000,
            0x41599d2c0a16d8c6,
            0xc15d2a0400000000,
            0x413d9c97ebc0f8a1,
            0x4133715800000000,
            0xc15299c3f163f4c5,
            0x413d2a0400000000,
            0x0000000000000000,
            0x4145df8300000000,
            0x416dde6e3d002212,
            0xc155df8300000000,
            0xc1415eeaf9a8dafd,
        ],
    },
    Case {
        name: "merc_sph inv",
        args: &["+proj=merc", "+R=6371000"],
        inverse: true,
        expected: &[
            0x3fb4174e3c083278,
            0x3fe4ed95aa599712,
            0x3fa4174e3c083278,
            0x3f901279bd014ac9,
            0xbf9012a4fcd35b93,
            0x3fd3c4da4189a63b,
            0x3fbc20a0ba71e042,
            0x3fec7099dfb66f6b,
            0x3fc81bf77b3d095d,
            0xbfc205a7d368c794,
            0xbfe214f99c6dc706,
            0x3ff0761c2e2079df,
        ],
    },
    Case {
        name: "tmerc fwd",
        args: &[
            "+proj=tmerc",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
        ],
        inverse: false,
        expected: &[
            0x411df210f3c323fc,
            0x4155cae8177843ed,
            0xc161708760121d45,
            0x41530d8f09558bab,
            0x4132a30a29b0d1a8,
            0xc1511138625210a3,
            0x413ff8f9b2cda7c2,
            0x0000000000000000,
            0x412c201a10ce4d12,
            0x416125490913e318,
            0xc156a95f9275ceb2,
            0xc14b47f86319f688,
        ],
    },
    Case {
        name: "tmerc inv",
        args: &[
            "+proj=tmerc",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
        ],
        inverse: true,
        expected: &[
            0x3faacee9f37bebd6,
            0x3fe6b42713e70d92,
            0x3f8aeffd5c860174,
            0x3f902830b11c7cb1,
            0xbfa7c6054667f704,
            0x3fd41cd18059f598,
            0x3fbd301d3bcb3de8,
            0x3ff09ce81b130ce7,
            0x3fc4de386216cd77,
            0xbfc2143602a57c6f,
            0xbff2062f4f970b72,
            0x3fed2aa1365516e6,
        ],
    },
    Case {
        name: "utm31n fwd",
        args: &["+proj=utm", "+zone=31", "+ellps=WGS84"],
        inverse: false,
        expected: &[
            0x411df210f3c32a60,
            0x4155cae8177a4ad4,
            0xc161708760120683,
            0x41530d8f09537919,
            0x4132a30a29b07eae,
            0xc15111386253ede6,
            0x413ff8f9b2cd9e64,
            0x0000000000000000,
            0x412c201a10cd6a93,
            0x416125490914b166,
            0xc156a95f92753e5b,
            0xc14b47f8631b0756,
        ],
    },
    Case {
        name: "utm31n inv",
        args: &["+proj=utm", "+zone=31", "+ellps=WGS84"],
        inverse: true,
        expected: &[
            0x3faacee9f37bebc0,
            0x3fe6b42713e492a2,
            0x3f8aeffd5c8600b8,
            0x3f902830b11a3355,
            0xbfa7c60546679f86,
            0x3fd41cd18057430e,
            0x3fbd301d3bc9da44,
            0x3ff09ce81b11b994,
            0x3fc4de386216c95f,
            0xbfc2143602a2f763,
            0xbff2062f4f96113b,
            0x3fed2aa13653ae79,
        ],
    },
    Case {
        name: "utm31s fwd",
        args: &["+proj=utm", "+zone=31", "+south", "+ellps=WGS84"],
        inverse: false,
        expected: &[
            0x411df210f3c32a60,
            0x416df8440bbd256a,
            0xc161708760120683,
            0x416c999784a9bc8c,
            0x4132a30a29b07eae,
            0x415514679dac121a,
            0x413ff8f9b2cd9e64,
            0x416312d000000000,
            0x412c201a10cd6a93,
            0x41721c0c848a58b3,
            0xc156a95f92753e5b,
            0x415881a3ce727c55,
        ],
    },
    Case {
        name: "utm31s inv",
        args: &["+proj=utm", "+zone=31", "+south", "+ellps=WGS84"],
        inverse: true,
        expected: &[
            0x3faacee9f37bebc0,
            0xbfebbb30ab74fb82,
            0xbff25231a5d12240,
            0xbff876195705e99d,
            0xbfcf45a04fd4942c,
            0xbff3ebc0a9d90a70,
            0x3fb6bc6fc922bcb0,
            0xbfe127b47301d52a,
            0x4004405e1e584ae8,
            0xbff6484809703e47,
            0xbfe23f427919d8fd,
            0xbfce721e475313c7,
        ],
    },
    Case {
        name: "tmerc_approx fwd",
        args: &[
            "+proj=tmerc",
            "+approx",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
        ],
        inverse: false,
        expected: &[
            0x411df210f3c323fc,
            0x4155cae8177843ee,
            0xc161bb980a47cf94,
            0x41526c66cf672fdf,
            0x4132a30a2a339aba,
            0xc151113862546844,
            0x413ff8f969ca9df4,
            0x0000000000000000,
            0x412c201a1a2f33d2,
            0x41612549091b55f9,
            0xc156b3d996ffcccd,
            0xc14b39f6c165f719,
        ],
    },
    Case {
        name: "tmerc_approx inv",
        args: &[
            "+proj=tmerc",
            "+approx",
            "+ellps=GRS80",
            "+lon_0=3",
            "+k=0.9996",
            "+x_0=500000",
        ],
        inverse: true,
        expected: &[
            0x3faacee9f37bebd6,
            0x3fe6b42713e70d92,
            0x3f8aeffd5c84e64c,
            0x3f902830b11c7ca9,
            0xbfa7c605467f9268,
            0x3fd41cd18059720e,
            0x3fbd301d3bca4956,
            0x3ff09ce81b12da86,
            0x3fc4de386229ae04,
            0xbfc2143602a54b7d,
            0xbffb474215d640d0,
            0x4019291c03de3932,
        ],
    },
    Case {
        name: "lcc fwd",
        args: &[
            "+proj=lcc",
            "+lat_1=49",
            "+lat_2=44",
            "+lat_0=46.5",
            "+lon_0=3",
            "+ellps=GRS80",
        ],
        inverse: false,
        expected: &[
            0xc0c25e171fd8c32d,
            0x4121335b0b809a82,
            0xc15c649e0f6c99b1,
            0x410f08d79e246561,
            0x4140a80edaf5ccc7,
            0xc16b2bd5a4e9574f,
            0x41400710ce250999,
            0xc1550443aa338e3c,
            0x41213c01b505ee36,
            0x414faf38e91f47d8,
            0xc1627ced8818c2b0,
            0xc1559b261686bb96,
        ],
    },
    Case {
        name: "lcc inv",
        args: &[
            "+proj=lcc",
            "+lat_1=49",
            "+lat_2=44",
            "+lat_0=46.5",
            "+lon_0=3",
            "+ellps=GRS80",
        ],
        inverse: true,
        expected: &[
            0x3fdec7deec43d5ff,
            0x3ff70a55e257d792,
            0x3fbc34900b02d51d,
            0x3fea726d0f282cab,
            0x3f92d39a9ac0099a,
            0x3ff1ea6ad327dd97,
            0x4009065f48a3d8f3,
            0x3ff83b046a76f8c7,
            0x3fd26b65d2d63586,
            0x3fe4f1942cc9d9fb,
            0xc006bb507e04248d,
            0x3ff1a57ee5f0f862,
        ],
    },
    Case {
        name: "aea fwd",
        args: &[
            "+proj=aea",
            "+lat_1=29.5",
            "+lat_2=45.5",
            "+lat_0=23",
            "+lon_0=-96",
            "+ellps=GRS80",
        ],
        inverse: false,
        expected: &[
            0x41563ebeb1b15158,
            0x4158d3ec3a58e67c,
            0x4146c47872dca66f,
            0xc1079b3dc7894737,
            0x416a983254bd1bbd,
            0x4149aa60421c9cf1,
            0x4165c48c19f74df0,
            0x41547fefd7b1245d,
            0x414f4d32235781cf,
            0x41609bcb429bac19,
            0x4158217ecaf826fd,
            0xc143d5aa50b9568c,
        ],
    },
    Case {
        name: "aea inv",
        args: &[
            "+proj=aea",
            "+lat_1=29.5",
            "+lat_2=45.5",
            "+lat_0=23",
            "+lon_0=-96",
            "+ellps=GRS80",
        ],
        inverse: true,
        expected: &[
            0xbff85ef8b8ac9cb4,
            0x3ff1f7f8cd26fe41,
            0xbffa2226719a9fbc,
            0x3fdaaf51f7ddc133,
            0xbffb2497ca266462,
            0x3fe6e68e3957c893,
            0x7ff0000000000000,
            0x7ff0000000000000,
            0xbff7e11fa4dafc94,
            0x3fcf2195d29a7b07,
            0x4006141ac798162c,
            0x7ff8000000000000,
        ],
    },
];

fn run_proj(args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_proj"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn proj");
    child
        .stdin
        .take()
        .expect("proj stdin")
        .write_all(input)
        .expect("write proj stdin");
    let output = child.wait_with_output().expect("wait for proj");
    assert!(output.status.success(), "proj: {:?}", output.status);
    output.stdout
}

#[test]
fn proj_binary_output_matches_proj_bit_for_bit() {
    for case in CASES {
        let points: &[(f64, f64)] = if case.inverse { &INV } else { &FWD };
        let input: Vec<u8> = points
            .iter()
            .flat_map(|&(u, v)| u.to_ne_bytes().into_iter().chain(v.to_ne_bytes()))
            .collect();
        let mut args = vec!["-b"];
        if case.inverse {
            args.push("-I");
        }
        args.extend_from_slice(case.args);
        let bits: Vec<u64> = run_proj(&args, &input)
            .chunks_exact(8)
            .map(|word| u64::from_ne_bytes(word.try_into().expect("8-byte word")))
            .collect();
        assert_eq!(bits, case.expected, "{}", case.name);
    }
}
