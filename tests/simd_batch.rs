//! Les conversions par lots (`forward_batch`, `inverse_batch`) de `Tmerc`, `Lcc` et `Mercator`.
//!
//! Deux accords sont vérifiés :
//!
//! - avec `cs2cs` 9.9.0, qui est la référence : les valeurs attendues sont celles que PROJ
//!   calcule, pas celles du code testé ;
//! - avec le chemin scalaire de la crate, point par point, sur des tailles et des positions de
//!   voie qui exercent la queue et le repli scalaire d'un groupe de quatre.
//!
//! Les lots AVX2 s'écartent du scalaire de l'erreur des fonctions mathématiques vectorielles
//! (quelques ULP). Les bornes ci-dessous sont celles que cet écart respecte, avec de la marge.

use proj_rs::{Cone, Ellipsoid, Hemisphere, Lcc, Mercator, Tmerc, UtmZone};
use std::io::Write;
use std::process::{Command, Stdio};

/// Écart toléré entre le lot et le scalaire, en mètres. Mesuré sur 200 000 points par projection :
/// 11 nm au plus ; la borne laisse une marge de dix.
const METRES: f64 = 1e-7;
/// Même écart en degrés. Mesuré : 4,6e-14° au plus (5 nm) ; la borne laisse une marge de vingt.
const DEGREES: f64 = 1e-12;
/// Écart toléré avec `cs2cs`, qui imprime avec `-d 9` décimales de mètre et 11 de degré.
const PROJ_METRES: f64 = 1e-6;
const PROJ_DEGREES: f64 = 1e-9;

/// Quelques ULP relatifs (un ULP vaut 1,1e-16 de la valeur).
const ULPS: f64 = 4e-15;

type Points = (Vec<f64>, Vec<f64>);
/// Un point du scalaire : le résultat, ou `None` quand il refuse le point.
type Scalar = Box<dyn Fn(f64, f64) -> Option<(f64, f64)>>;
/// Un lot : deux tranches d'entrée, deux tranches de sortie.
type Batch = Box<dyn Fn(&[f64], &[f64], &mut [f64], &mut [f64])>;

/// Une projection, avec son chemin scalaire et son chemin par lots.
struct Case {
    name: &'static str,
    /// `(lon, lat)` en degrés vers `(est, nord)`, ou `None` si le scalaire refuse le point.
    forward: Scalar,
    inverse: Scalar,
    forward_batch: Batch,
    inverse_batch: Batch,
    /// Boîte `(lon_min, lon_max, lat_min, lat_max)` où les points sont dans le domaine.
    domain: (f64, f64, f64, f64),
    /// Chaîne PROJ de la même projection, pour `cs2cs`.
    proj: &'static str,
    /// Chaîne PROJ du CRS géographique d'entrée, sur le même ellipsoïde.
    geographic: &'static str,
}

macro_rules! case {
    ($name:literal, $projection:expr, $domain:expr, $proj:literal, $geographic:literal) => {{
        let forward = $projection;
        let inverse = forward.clone();
        let batch_f = forward.clone();
        let batch_i = forward.clone();
        Case {
            name: $name,
            forward: Box::new(move |lon, lat| {
                forward.forward(lon, lat).ok().map(|p| (p.east, p.north))
            }),
            inverse: Box::new(move |east, north| {
                inverse
                    .inverse(proj_rs::Projected { east, north })
                    .ok()
                    .map(|g| (g.lon_deg, g.lat_deg))
            }),
            forward_batch: Box::new(move |a, b, c, d| batch_f.forward_batch(a, b, c, d)),
            inverse_batch: Box::new(move |a, b, c, d| batch_i.inverse_batch(a, b, c, d)),
            domain: $domain,
            proj: $proj,
            geographic: $geographic,
        }
    }};
}

fn cases() -> Vec<Case> {
    let utm = Tmerc::utm(
        UtmZone::try_new(31).expect("zone"),
        Hemisphere::North,
        Ellipsoid::wgs84(),
    )
    .expect("utm");
    let gauss =
        Tmerc::try_new(Ellipsoid::grs80(), 3.0, 40.0, 0.9996, 500_000.0, 100_000.0).expect("tmerc");
    let l93 = Lcc::lambert93();
    let south = Lcc::try_new(
        Ellipsoid::wgs84(),
        Cone {
            lon0_deg: 135.0,
            lat0_deg: -25.0,
            lat1_deg: -18.0,
            lat2_deg: -36.0,
            x0: 0.0,
            y0: 0.0,
            k0: 1.0,
        },
    )
    .expect("lcc south");
    let web = Mercator::web();
    let merc =
        Mercator::try_ellipsoid(Ellipsoid::wgs84(), 10.0, 0.9996, 1000.0, -2000.0).expect("merc");
    vec![
        case!(
            "utm31",
            utm,
            (0.0, 6.0, -80.0, 84.0),
            "+proj=utm +zone=31 +ellps=WGS84",
            "+proj=latlong +ellps=WGS84"
        ),
        case!(
            "tmerc",
            gauss,
            (-10.0, 16.0, 20.0, 60.0),
            "+proj=tmerc +lat_0=40 +lon_0=3 +k=0.9996 +x_0=500000 +y_0=100000 +ellps=GRS80",
            "+proj=latlong +ellps=GRS80"
        ),
        case!(
            "lambert93",
            l93,
            (-5.0, 9.0, 41.0, 52.0),
            "+proj=lcc +lat_1=49 +lat_2=44 +lat_0=46.5 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
            "+proj=latlong +ellps=GRS80"
        ),
        case!(
            "lcc_south",
            south,
            (110.0, 160.0, -45.0, -10.0),
            "+proj=lcc +lat_1=-18 +lat_2=-36 +lat_0=-25 +lon_0=135 +ellps=WGS84",
            "+proj=latlong +ellps=WGS84"
        ),
        case!(
            "webmerc",
            web,
            (-180.0, 180.0, -85.0, 85.0),
            "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1",
            "+proj=latlong +a=6378137 +b=6378137"
        ),
        case!(
            "merc_ellipsoid",
            merc,
            (-170.0, 170.0, -80.0, 80.0),
            "+proj=merc +lon_0=10 +k=0.9996 +x_0=1000 +y_0=-2000 +ellps=WGS84",
            "+proj=latlong +ellps=WGS84"
        ),
    ]
}

/// `count` points uniformes de la boîte, déterministes.
fn sample(seed: u64, count: usize, domain: (f64, f64, f64, f64)) -> Points {
    let mut state = seed;
    let mut unit = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "les 53 bits de poids fort du générateur sont exacts en f64"
        )]
        let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    };
    (0..count)
        .map(|_| {
            (
                domain.0 + unit() * (domain.1 - domain.0),
                domain.2 + unit() * (domain.3 - domain.2),
            )
        })
        .unzip()
}

fn run(batch: &Batch, a: &[f64], b: &[f64]) -> Points {
    let (mut x, mut y) = (vec![f64::NAN; a.len()], vec![f64::NAN; a.len()]);
    batch(a, b, &mut x, &mut y);
    (x, y)
}

/// Le plus grand écart entre le lot et le scalaire au-delà de [`ULPS`], `NaN` contre refus
/// compris.
fn worst_gap(scalar: &Scalar, inputs: (&[f64], &[f64]), outputs: (&[f64], &[f64])) -> f64 {
    let mut worst = 0.0_f64;
    for index in 0..inputs.0.len() {
        let got = (outputs.0[index], outputs.1[index]);
        match scalar(inputs.0[index], inputs.1[index]) {
            Some(want) => {
                assert!(
                    got.0.is_finite() && got.1.is_finite(),
                    "point {index} ({}, {}): lot {got:?}, scalaire {want:?}",
                    inputs.0[index],
                    inputs.1[index]
                );
                // Quelques ULP relatifs sont admis en plus de la borne : près du pôle opposé à un
                // cône, les coordonnées dépassent 1e10 m, où un ULP dépasse la borne absolue.
                let excess =
                    |got: f64, want: f64| ((got - want).abs() - ULPS * want.abs()).max(0.0);
                worst = worst.max(excess(got.0, want.0)).max(excess(got.1, want.1));
            }
            None => assert!(
                got.0.is_nan() && got.1.is_nan(),
                "point {index} ({}, {}): le scalaire refuse, le lot rend {got:?}",
                inputs.0[index],
                inputs.1[index]
            ),
        }
    }
    worst
}

#[test]
fn batch_matches_scalar_on_a_long_run() {
    for case in cases() {
        // 10 003 points : pas un multiple de quatre, la queue est exercée.
        let (lon, lat) = sample(7, 10_003, case.domain);
        let (east, north) = run(&case.forward_batch, &lon, &lat);
        let gap = worst_gap(&case.forward, (&lon, &lat), (&east, &north));
        assert!(gap < METRES, "{} aller: écart {gap:e} m", case.name);

        let (back_lon, back_lat) = run(&case.inverse_batch, &east, &north);
        let gap = worst_gap(&case.inverse, (&east, &north), (&back_lon, &back_lat));
        assert!(gap < DEGREES, "{} retour: écart {gap:e}°", case.name);
    }
}

#[test]
fn every_length_is_converted() {
    for case in cases() {
        for length in 0..=17 {
            let (lon, lat) = sample(3, length, case.domain);
            let (east, north) = run(&case.forward_batch, &lon, &lat);
            let gap = worst_gap(&case.forward, (&lon, &lat), (&east, &north));
            assert!(gap < METRES, "{} longueur {length}: {gap:e} m", case.name);
            let (back_lon, back_lat) = run(&case.inverse_batch, &east, &north);
            let gap = worst_gap(&case.inverse, (&east, &north), (&back_lon, &back_lat));
            assert!(gap < DEGREES, "{} longueur {length}: {gap:e}°", case.name);
        }
    }
}

/// Des valeurs que le noyau vectoriel ne traite pas (non finies, latitude hors de ±90°) : le
/// groupe entier passe par le scalaire, et ses quatre points valent ceux du scalaire au bit près.
#[test]
fn a_group_with_an_unusual_lane_matches_the_scalar_path_exactly() {
    let unusual = [
        (f64::NAN, 45.0),
        (3.0, f64::NAN),
        (f64::INFINITY, 45.0),
        (3.0, f64::NEG_INFINITY),
        (3.0, 91.0),
        (3.0, -90.5),
        (3.0, 1e9),
    ];
    for case in cases() {
        let (lon, lat) = sample(11, 64, case.domain);
        for &(bad_lon, bad_lat) in &unusual {
            for phase in 0..4 {
                // Le point inhabituel prend chaque position de voie d'un groupe.
                let mut lon = lon.clone();
                let mut lat = lat.clone();
                lon[8 + phase] = bad_lon;
                lat[8 + phase] = bad_lat;
                let (east, north) = run(&case.forward_batch, &lon, &lat);
                for index in 8..12 {
                    let want = (case.forward)(lon[index], lat[index]);
                    let got = (east[index], north[index]);
                    match want {
                        Some(w) => assert_eq!(
                            (got.0.to_bits(), got.1.to_bits()),
                            (w.0.to_bits(), w.1.to_bits()),
                            "{} ({bad_lon}, {bad_lat}) voie {phase}, point {index}",
                            case.name
                        ),
                        None => assert!(
                            got.0.is_nan() && got.1.is_nan(),
                            "{} ({bad_lon}, {bad_lat}) voie {phase}, point {index}: {got:?}",
                            case.name
                        ),
                    }
                }
                // Les autres groupes gardent le chemin rapide et restent dans la borne.
                let gap = worst_gap(&case.forward, (&lon, &lat), (&east, &north));
                assert!(gap < METRES, "{} voisins: {gap:e}", case.name);
            }
        }
    }
}

/// Des valeurs que le noyau vectoriel traite ou non selon la projection : pôles, longitudes
/// démesurées, antiméridien. Le lot s'accorde avec le scalaire à la borne, `NaN` compris.
#[test]
fn extreme_points_agree_with_the_scalar_path() {
    let extreme = [
        (3.0, 90.0),
        (3.0, -90.0),
        (1e9, 45.0),
        (-1e9, -45.0),
        (179.999_999, 0.0),
        (-180.0, 0.0),
        (180.0, 89.999_999),
        (0.0, 0.0),
        (3.0, 89.999_999_9),
    ];
    for case in cases() {
        let (lon, lat) = sample(17, 64, case.domain);
        for &(extreme_lon, extreme_lat) in &extreme {
            for phase in 0..4 {
                let mut lon = lon.clone();
                let mut lat = lat.clone();
                lon[8 + phase] = extreme_lon;
                lat[8 + phase] = extreme_lat;
                let (east, north) = run(&case.forward_batch, &lon, &lat);
                let gap = worst_gap(&case.forward, (&lon, &lat), (&east, &north));
                assert!(
                    gap < METRES,
                    "{} ({extreme_lon}, {extreme_lat}) voie {phase}: {gap:e}",
                    case.name
                );
            }
        }
    }
}

#[test]
fn an_unusual_input_of_the_inverse_gives_nan_like_the_scalar_path() {
    let unusual = [
        (f64::NAN, 5_000_000.0),
        (500_000.0, f64::NAN),
        (f64::INFINITY, 0.0),
        (1e18, 1e18),
        (0.0, 0.0),
    ];
    for case in cases() {
        let (lon, lat) = sample(5, 32, case.domain);
        let (east, north) = run(&case.forward_batch, &lon, &lat);
        for &(bad_east, bad_north) in &unusual {
            for phase in 0..4 {
                let mut east = east.clone();
                let mut north = north.clone();
                east[4 + phase] = bad_east;
                north[4 + phase] = bad_north;
                let (out_lon, out_lat) = run(&case.inverse_batch, &east, &north);
                let worst = worst_gap(&case.inverse, (&east, &north), (&out_lon, &out_lat));
                assert!(
                    worst < DEGREES,
                    "{} ({bad_east}, {bad_north}) voie {phase}: {worst:e}",
                    case.name
                );
            }
        }
    }
}

/// Sur un processeur AVX2 le lot prend le chemin vectoriel : sans ce garde, un repli silencieux
/// sur le scalaire ferait passer tous les autres tests sans rien vérifier du noyau vectoriel.
#[cfg(target_arch = "x86_64")]
#[test]
fn the_vector_path_is_taken_on_an_avx2_processor() {
    let expected = if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma")
    {
        "avx2"
    } else {
        "portable"
    };
    assert_eq!(proj_rs::batch_path(), expected);
}

/// Sur aarch64, NEON est toujours là : le lot prend le chemin vectoriel, comme AVX2 sur x86-64.
#[cfg(target_arch = "aarch64")]
#[test]
fn the_vector_path_is_taken_on_an_aarch64_processor() {
    assert_eq!(proj_rs::batch_path(), "neon");
}

#[test]
#[should_panic(expected = "length")]
fn lengths_must_match() {
    let mut east = [0.0; 3];
    let mut north = [0.0; 4];
    Mercator::web().forward_batch(&[0.0; 4], &[0.0; 4], &mut east, &mut north);
}

/// Le `cs2cs` de PROJ, s'il y en a un sur `PATH`. `PROJ_CS2CS_REQUIRED` fait d'une absence un
/// échec, comme la CI.
fn proj_cs2cs() -> Option<std::path::PathBuf> {
    // `cargo test` met `target/` dans `PATH` : notre propre `cs2cs` ne doit pas faire écran.
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
        drop(writeln!(
            std::io::stderr(),
            "skipped: no PROJ cs2cs on PATH"
        ));
    }
    found
}

/// Passe `points` à `cs2cs from +to to` et rend ses deux premières colonnes.
fn cs2cs(
    program: &std::path::Path,
    digits: &str,
    from: &str,
    to: &str,
    a: &[f64],
    b: &[f64],
) -> Points {
    let mut child = Command::new(program)
        .args(["-d", digits, from, "+to", to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cs2cs is installed");
    let input: String = a
        .iter()
        .zip(b)
        .map(|(x, y)| format!("{x:.15} {y:.15}\n"))
        .collect();
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    let done = child.wait_with_output().expect("wait");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    let text = String::from_utf8(done.stdout).expect("utf8");
    let mut first = Vec::new();
    let mut second = Vec::new();
    for line in text.lines() {
        let mut columns = line.split_whitespace();
        first.push(
            columns
                .next()
                .expect("first")
                .parse::<f64>()
                .expect("number"),
        );
        second.push(
            columns
                .next()
                .expect("second")
                .parse::<f64>()
                .expect("number"),
        );
    }
    (first, second)
}

#[test]
fn batches_match_cs2cs() {
    let Some(program) = proj_cs2cs() else {
        return;
    };
    for case in cases() {
        // Avec un chemin du lot (AVX2) et avec le repli : la tranche est assez longue pour que
        // la plupart des groupes passent par le noyau vectoriel.
        let (lon, lat) = sample(13, 2_003, case.domain);
        let (east, north) = run(&case.forward_batch, &lon, &lat);
        let (want_east, want_north) = cs2cs(&program, "9", case.geographic, case.proj, &lon, &lat);
        assert_eq!(want_east.len(), lon.len(), "{}", case.name);
        let gap = (0..lon.len())
            .map(|i| {
                (east[i] - want_east[i])
                    .abs()
                    .max((north[i] - want_north[i]).abs())
            })
            .fold(0.0, f64::max);
        assert!(
            gap < PROJ_METRES,
            "{} aller contre cs2cs: {gap:e} m",
            case.name
        );

        let (back_lon, back_lat) = run(&case.inverse_batch, &east, &north);
        let (want_lon, want_lat) = cs2cs(&program, "11", case.proj, case.geographic, &east, &north);
        let gap = (0..lon.len())
            .map(|i| {
                // La longitude de PROJ est dans [-180, 180] : comparer modulo 360.
                let mut d = (back_lon[i] - want_lon[i]).abs();
                d = d.min((d - 360.0).abs());
                d.max((back_lat[i] - want_lat[i]).abs())
            })
            .fold(0.0, f64::max);
        assert!(
            gap < PROJ_DEGREES,
            "{} retour contre cs2cs: {gap:e}°",
            case.name
        );
    }
}
