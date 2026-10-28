//! Progress benchmark: one case per ported operation, one JSON line on stdout.
//!
//! Metric: nanoseconds per point. Each case is the median of 3 trials, each
//! trial the median of 11 passes over a million points. Pin the process
//! (`taskset -c 0`) before comparing runs. `bench/record.sh` appends the line
//! to `bench/history.jsonl` with the commit and the machine.
//!
//! A new lot adds its cases. Never rename a case: the history is keyed by name.

use proj_rs::{
    bench_definition, Ellipsoid, Helmert, HelmertConvention, HelmertParams, Hemisphere, Lcc,
    Mercator, Operation, Projected, Tmerc, UtmZone, Xyzt, PORTED,
};
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const GIE_PASSES: &str = include_str!("../../tests/gie/passing.txt");
/// Catalogue transforms use a short grid. The historical cases stay at [`N`].
const CATALOGUE_N: usize = 128;

const N: usize = 1_000_000;
const REPEATS: usize = 11;
const TRIALS: usize = 3;

/// `N` points drawn uniformly in a lon/lat box, degrees.
fn grid(seed: u64, lon: (f64, f64), lat: (f64, f64)) -> (Vec<f64>, Vec<f64>) {
    let mut state = seed;
    let mut unit = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "the top 53 bits of the LCG are an exact integer in f64"
        )]
        let u = ((state >> 11) as f64) / 9_007_199_254_740_992.0;
        u
    };
    (0..N)
        .map(|_| {
            (
                lon.0 + unit() * (lon.1 - lon.0),
                lat.0 + unit() * (lat.1 - lat.0),
            )
        })
        .unzip()
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn time_stable(count: usize, mut run: impl FnMut()) -> f64 {
    let mut trials = [0.0; TRIALS];
    for trial in &mut trials {
        run();
        let mut samples = [0.0; REPEATS];
        for sample in &mut samples {
            let start = Instant::now();
            run();
            #[expect(
                clippy::cast_precision_loss,
                clippy::as_conversions,
                reason = "the point count is at most 1_000_000 and fits in f64"
            )]
            let n = count as f64;
            *sample = start.elapsed().as_secs_f64() * 1e9 / n;
        }
        *trial = median(&mut samples);
    }
    median(&mut trials)
}

/// Times `op` on every point. Its result is summed so the call is not elided.
fn per_point<T: Copy>(points: &[T], op: impl Fn(T) -> f64) -> f64 {
    time_stable(points.len(), || {
        black_box(points.iter().map(|&p| op(black_box(p))).sum::<f64>());
    })
}

type Batch<'a> = &'a dyn Fn(&[f64], &[f64], &mut [f64], &mut [f64]);
type Forward<'a> = &'a dyn Fn((f64, f64)) -> Option<Projected>;

/// Forward and inverse cases of one projection on one grid.
fn scalar_pair(
    cases: &mut Vec<(String, f64)>,
    name: &str,
    points: &[(f64, f64)],
    forward: Forward<'_>,
    inverse: &dyn Fn(Projected) -> Option<f64>,
) {
    let xy: Vec<Projected> = points.iter().filter_map(|&p| forward(p)).collect();
    assert_eq!(xy.len(), N, "every {name} bench point is inside the domain");
    let fwd = per_point(points, |p| forward(p).map_or(0.0, |q| q.east));
    cases.push((format!("{name}_forward"), fwd));
    cases.push((
        format!("{name}_inverse"),
        per_point(&xy, |p| inverse(p).unwrap_or(0.0)),
    ));
}

#[expect(
    clippy::print_stdout,
    reason = "the benchmark's product is this JSON record on stdout"
)]
fn main() {
    let zone = UtmZone::try_new(31).expect("zone 31");
    let utm = Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm");
    let l93 = Lcc::lambert93();
    let web = Mercator::web();
    let helmert = Helmert::try_new(HelmertParams {
        translation_x: -168.0,
        translation_y: -60.0,
        translation_z: 320.0,
        rotation_x_arcsec: 0.1,
        rotation_y_arcsec: -0.2,
        rotation_z_arcsec: 0.3,
        scale_ppm: 1.5,
        convention: HelmertConvention::PositionVector,
    })
    .expect("helmert");

    let utm_center = grid(1, (2.0, 4.0), (43.0, 50.0));
    let utm_edge = grid(2, (0.05, 0.55), (43.0, 50.0));
    let france = grid(3, (-4.5, 8.0), (41.5, 51.0));
    let world = grid(4, (-180.0, 180.0), (-85.0, 85.0));
    let zip = |g: &(Vec<f64>, Vec<f64>)| -> Vec<(f64, f64)> {
        g.0.iter().copied().zip(g.1.iter().copied()).collect()
    };

    let mut cases = Vec::new();
    // Lots 2, 3, 6: scalar paths.
    scalar_pair(
        &mut cases,
        "utm31",
        &zip(&utm_center),
        &|(lon, lat)| utm.forward(lon, lat).ok(),
        &|p| utm.inverse(p).ok().map(|g| g.lat_deg),
    );
    scalar_pair(
        &mut cases,
        "lambert93",
        &zip(&france),
        &|(lon, lat)| l93.forward(lon, lat).ok(),
        &|p| l93.inverse(p).ok().map(|g| g.lat_deg),
    );
    scalar_pair(
        &mut cases,
        "webmerc",
        &zip(&world),
        &|(lon, lat)| web.forward(lon, lat).ok(),
        &|p| web.inverse(p).ok().map(|g| g.lat_deg),
    );

    // Lots 4, 5: batch paths (AVX2 on Linux glibc, scalar loop elsewhere).
    let (mut east, mut north) = (vec![0.0; N], vec![0.0; N]);
    let batches: [(&str, Batch<'_>, _); 3] = [
        (
            "utm31_batch_center",
            &|a, b, c, d| utm.forward_batch(a, b, c, d),
            &utm_center,
        ),
        (
            "utm31_batch_edge",
            &|a, b, c, d| utm.forward_batch(a, b, c, d),
            &utm_edge,
        ),
        (
            "lambert93_batch",
            &|a, b, c, d| l93.forward_batch(a, b, c, d),
            &france,
        ),
    ];
    for (name, batch, (lon, lat)) in batches {
        let ns = time_stable(lon.len(), || {
            batch(black_box(lon), black_box(lat), &mut east, &mut north);
        });
        black_box((&east, &north));
        cases.push((name.to_owned(), ns));
    }

    // Lot 7: Helmert on geocentric coordinates around France.
    let xyz: Vec<(f64, f64, f64)> = zip(&france)
        .iter()
        .map(|&(lon, lat)| (4_200_000.0 + lon * 1e3, 170_000.0, 4_780_000.0 + lat * 1e3))
        .collect();
    cases.push((
        "helmert7_apply".to_owned(),
        per_point(&xyz, |(x, y, z)| helmert.apply(x, y, z).0),
    ));

    cases.extend(catalogue_cases());
    let ratios = libproj_ratios(&[
        ("utm31_forward", "+proj=utm +zone=31 +ellps=WGS84", &utm_center),
        (
            "lambert93_forward",
            "+proj=lcc +lat_0=46.5 +lat_1=49 +lat_2=44 +lon_0=3 +x_0=700000 +y_0=6600000 +ellps=GRS80",
            &france,
        ),
        (
            "webmerc_forward",
            "+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +k=1",
            &world,
        ),
    ]);

    let mut rows = String::new();
    for (name, ns) in &cases {
        let sep = if rows.is_empty() { "" } else { "," };
        write!(
            rows,
            "{sep}{{\"name\":\"{name}\",\"ns_per_point\":{ns:.6}}}"
        )
        .expect("String write");
    }
    let batch_path = proj_rs::batch_path();
    let simd = batch_path == "avx2";
    let passes = GIE_PASSES.trim();
    let mut ratio_rows = String::new();
    for (name, proj_ns) in &ratios {
        let ours = cases
            .iter()
            .find(|(case, _)| case == name)
            .map(|(_, ns)| *ns)
            .expect(name);
        let ratio = ours / proj_ns;
        let sep = if ratio_rows.is_empty() { "" } else { "," };
        write!(
            ratio_rows,
            "{sep}{{\"name\":\"{name}\",\"libproj_ratio\":{ratio:.6},\"libproj_ns\":{proj_ns:.6}}}"
        )
        .expect("String write");
    }
    println!(
        "{{\"impl\":\"proj-rs\",\"version\":\"{}\",\"simd_build\":{simd},\"batch_path\":\"{batch_path}\",\"passes\":{passes},\"n\":{N},\"libproj\":[{ratio_rows}],\"cases\":[{rows}]}}",
        proj_rs::VERSION
    );
}

fn catalogue_cases() -> Vec<(String, f64)> {
    let (lon, lat) = grid(5, (2.0, 4.0), (43.0, 48.0));
    let lon = &lon[..CATALOGUE_N];
    let lat = &lat[..CATALOGUE_N];
    PORTED
        .iter()
        .map(|name| {
            let definition = bench_definition(name);
            let operation = Operation::create(&definition).expect(name);
            let points: Vec<Xyzt> = lon
                .iter()
                .zip(lat)
                .map(|(&lon_deg, &lat_deg)| {
                    if operation.angular_input(false) {
                        Xyzt::xy(lon_deg.to_radians(), lat_deg.to_radians())
                    } else {
                        Xyzt::xy(lon_deg, lat_deg)
                    }
                })
                .collect();
            let ns = per_point(&points, |coord| {
                operation.forward(coord).map_or(0.0, |out| out.x)
            });
            (format!("op_{name}"), ns)
        })
        .collect()
}

type ProjGrid<'a> = (&'a str, &'a str, &'a (Vec<f64>, Vec<f64>));

fn libproj_ratios(grids: &[ProjGrid<'_>]) -> Vec<(String, f64)> {
    #[cfg(not(link_libproj))]
    {
        let _ = grids;
        Vec::new()
    }
    #[cfg(link_libproj)]
    {
        libproj_ratios_linked(grids)
    }
}

/// `PJ_COORD` as four `f64`. `proj_trans` reads `x` and `y` for a classic projection.
#[cfg(link_libproj)]
#[repr(C)]
#[derive(Clone, Copy)]
struct PjCoord {
    x: f64,
    y: f64,
    z: f64,
    t: f64,
}

#[cfg(link_libproj)]
#[link(name = "proj")]
unsafe extern "C" {
    fn proj_context_create() -> *mut std::ffi::c_void;
    fn proj_context_destroy(ctx: *mut std::ffi::c_void);
    fn proj_create(
        ctx: *mut std::ffi::c_void,
        definition: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
    fn proj_destroy(pj: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn proj_trans(pj: *mut std::ffi::c_void, direction: i32, coord: PjCoord) -> PjCoord;
}

#[cfg(link_libproj)]
fn libproj_ratios_linked(grids: &[ProjGrid<'_>]) -> Vec<(String, f64)> {
    use std::ffi::CString;
    // SAFETY: OSGeo4W proj.dll creates one context. It is destroyed before return.
    let ctx = unsafe { proj_context_create() };
    assert!(!ctx.is_null(), "proj_context_create");
    let mut ratios = Vec::new();
    for (name, definition, (lon, lat)) in grids {
        let text = CString::new(*definition).expect("definition");
        // SAFETY: `text` is a nul-terminated definition. `ctx` is the context above.
        let pj = unsafe { proj_create(ctx, text.as_ptr()) };
        assert!(!pj.is_null(), "proj_create {definition}");
        if *name == "utm31_forward" {
            // SAFETY: `pj` is the UTM zone 31 projection just created.
            let out = unsafe {
                proj_trans(
                    pj,
                    1,
                    PjCoord {
                        x: 3.0_f64.to_radians(),
                        y: 0.0,
                        z: 0.0,
                        t: 0.0,
                    },
                )
            };
            assert!(
                (out.x - 500_000.0).abs() < 1.0,
                "libproj UTM easting {}, ABI or definition is wrong",
                out.x
            );
        }
        let points: Vec<PjCoord> = lon
            .iter()
            .zip(lat)
            .map(|(&lon_deg, &lat_deg)| PjCoord {
                x: lon_deg.to_radians(),
                y: lat_deg.to_radians(),
                z: 0.0,
                t: 0.0,
            })
            .collect();
        let ns = per_point(&points, |coord| {
            // SAFETY: `pj` is live and `coord` is a 32-byte `PJ_COORD`.
            let out = unsafe { proj_trans(pj, 1, coord) };
            out.x
        });
        // SAFETY: `pj` was created in this loop and is not used again.
        unsafe { proj_destroy(pj) };
        ratios.push(((*name).to_owned(), ns));
    }
    // SAFETY: `ctx` was created at the start of this function.
    unsafe { proj_context_destroy(ctx) };
    ratios
}
