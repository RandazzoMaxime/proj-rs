//! Progress benchmark: one case per ported operation, one JSON line on stdout.
//!
//! Metric: nanoseconds per point. Each case is the median of 3 trials, each
//! trial the median of 11 passes over a million points. Pin the process
//! (`taskset -c 0`) before comparing runs. `bench/record.sh` appends the line
//! to `bench/history.jsonl` with the commit and the machine.
//!
//! `--billion` times a billion points instead (`bench/billion.sh`).
//!
//! A new lot adds its cases. Never rename a case: the history is keyed by name.

use proj_rs::{
    bench_definition, Ellipsoid, Helmert, HelmertConvention, HelmertParams, Hemisphere, Lcc,
    Mercator, Operation, Proj, Projected, Tmerc, UtmZone, Xyzt, PORTED,
};
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

#[path = "support/bench_billion.rs"]
mod billion;

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
    if std::env::args().nth(1).as_deref() == Some("--billion") {
        let (ours_point, ours_array) = billion::proj_rs();
        let libproj = billion::libproj().map_or_else(String::new, |(point, array)| {
            format!(",\"libproj_convert_s\":{point:.3},\"libproj_convert_array_s\":{array:.3}")
        });
        println!(
            "{{\"points\":{},\"chunk\":{},\"pair\":\"EPSG:4326 to EPSG:2154\",\"convert_s\":{ours_point:.3},\"convert_array_s\":{ours_array:.3}{libproj}}}",
            billion::TOTAL,
            billion::CHUNK
        );
        return;
    }
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

    // Lots 4, 5: batch paths (AVX2 where the CPU has it, otherwise see `batch_path`).
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

    // Batch inverses and Mercator. The projected inputs come from the forward batch.
    type Points = (Vec<f64>, Vec<f64>);
    let project = |batch: Batch<'_>, points: &Points| -> Points {
        let (mut x, mut y) = (vec![0.0; N], vec![0.0; N]);
        batch(&points.0, &points.1, &mut x, &mut y);
        (x, y)
    };
    let utm_xy = project(&|a, b, c, d| utm.forward_batch(a, b, c, d), &utm_center);
    let l93_xy = project(&|a, b, c, d| l93.forward_batch(a, b, c, d), &france);
    let web_xy = project(&|a, b, c, d| web.forward_batch(a, b, c, d), &world);
    let more: [(&str, Batch<'_>, &Points); 4] = [
        (
            "utm31_inverse_batch",
            &|a, b, c, d| utm.inverse_batch(a, b, c, d),
            &utm_xy,
        ),
        (
            "lambert93_inverse_batch",
            &|a, b, c, d| l93.inverse_batch(a, b, c, d),
            &l93_xy,
        ),
        (
            "webmerc_batch",
            &|a, b, c, d| web.forward_batch(a, b, c, d),
            &world,
        ),
        (
            "webmerc_inverse_batch",
            &|a, b, c, d| web.inverse_batch(a, b, c, d),
            &web_xy,
        ),
    ];
    for (name, batch, (first, second)) in more {
        let ns = time_stable(first.len(), || {
            batch(black_box(first), black_box(second), &mut east, &mut north);
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
    cases.extend(facade_cases());
    #[cfg(feature = "gpu")]
    cases.extend(gpu_cases());
    let mut ratios = libproj_ratios(&[
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
    ratios.extend(libproj_facade());

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
    // `passing.txt` holds one line per mode (`offline N`, `network N`); the history keeps the
    // network count, as `bench/record.sh` does.
    let passes = GIE_PASSES
        .lines()
        .find_map(|line| line.strip_prefix("network "))
        .map(str::trim)
        .filter(|count| !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()))
        .unwrap_or("0");
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

/// Points a facade case converts. The facade is slower than a bare projection, so the grid is
/// shorter than [`N`] to keep the run short.
const FACADE_N: usize = 200_000;

/// A list of (x, y) points.
type Vec2 = Vec<(f64, f64)>;

/// The CRS pairs of the facade cases. Each name is a case of the history.
type FacadePair = (&'static str, &'static str, &'static str);
const FACADE_PAIRS: [FacadePair; 5] = [
    ("facade_convert_4326_2154", "EPSG:4326", "EPSG:2154"),
    ("facade_convert_4326_32631", "EPSG:4326", "EPSG:32631"),
    ("facade_convert_4326_3857", "EPSG:4326", "EPSG:3857"),
    ("facade_convert_2154_32631", "EPSG:2154", "EPSG:32631"),
    ("facade_convert_4267_4326", "EPSG:4267", "EPSG:4326"),
];

/// The source points of [`FACADE_PAIRS`]: geographic degrees (lon, lat), and the same points in
/// EPSG:2154 metres. 2154 reads metres, so its inputs come from the 4326 to 2154 pair. The
/// libproj side reads these exact values.
fn facade_inputs() -> (Vec2, Vec2) {
    let (lon, lat) = grid(6, (-4.5, 8.0), (41.5, 51.0));
    let geographic: Vec<(f64, f64)> = lon.into_iter().zip(lat).take(FACADE_N).collect();
    let to_lambert = Proj::new_known_crs("EPSG:4326", "EPSG:2154", None).expect("4326 to 2154");
    let lambert = geographic
        .iter()
        .map(|&p| to_lambert.convert(p).expect("inside France"))
        .collect();
    (geographic, lambert)
}

/// The facade (`Proj`): one point at a time with `convert`, and a slice with `convert_array`.
/// The pairs are named CRS, so the path is the one a user of `new_known_crs` takes: axis order,
/// the ISO 19111 selection, then the operation.
fn facade_cases() -> Vec<(String, f64)> {
    let mut cases = Vec::new();
    let (geographic, lambert) = facade_inputs();
    for (name, from, to) in FACADE_PAIRS {
        let proj = Proj::new_known_crs(from, to, None).expect(name);
        let points = if from == "EPSG:2154" {
            &lambert
        } else {
            &geographic
        };
        let one_by_one = per_point(points, |p| proj.convert(p).map_or(0.0, |q| q.0));
        cases.push((name.to_owned(), one_by_one));
        let mut buffer = points.clone();
        let array = time_stable(points.len(), || {
            buffer.copy_from_slice(black_box(points));
            black_box(proj.convert_array(&mut buffer).map(|done| done.len()).ok());
        });
        cases.push((name.replace("convert", "convert_array"), array));
    }
    cases
}

/// `convert_array_gpu` on the facade pairs the GPU covers: nanoseconds per point end to end, the
/// conversion to and from the device format, the upload, the dispatch and the read-back included.
/// Only when a GPU adapter is there; the adapter and its precision go to stderr, the history line
/// does not carry them. `min_points` is 0 so every size reaches the device.
#[cfg(feature = "gpu")]
#[expect(
    clippy::print_stderr,
    reason = "the adapter goes to stderr, the JSON record owns stdout"
)]
fn gpu_cases() -> Vec<(String, f64)> {
    use proj_rs::gpu::{GpuContext, GpuOptions};
    let gpu = match GpuContext::with_options(GpuOptions::default().with_min_points(0)) {
        Ok(gpu) => gpu,
        Err(error) => {
            eprintln!("gpu cases skipped: {error}");
            return Vec::new();
        }
    };
    eprintln!("gpu cases on {gpu:?}");
    let (geographic, lambert) = facade_inputs();
    let mut cases = Vec::new();
    for (name, from, to) in FACADE_PAIRS {
        let proj = Proj::new_known_crs(from, to, None).expect(name);
        let points = if from == "EPSG:2154" {
            &lambert
        } else {
            &geographic
        };
        let mut buffer = points.clone();
        let covered = proj
            .convert_array_gpu(&mut buffer, &gpu)
            .is_ok_and(|(_, report)| report.cpu_points == 0);
        if !covered {
            continue;
        }
        let ns = time_stable(points.len(), || {
            buffer.copy_from_slice(black_box(points));
            black_box(
                proj.convert_array_gpu(&mut buffer, &gpu)
                    .map(|(done, _)| done.len())
                    .ok(),
            );
        });
        cases.push((name.replace("facade_convert", "gpu_convert_array"), ns));
    }
    cases
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
    fn proj_log_level(ctx: *mut std::ffi::c_void, level: i32) -> i32;
    fn proj_create_crs_to_crs(
        ctx: *mut std::ffi::c_void,
        source: *const std::ffi::c_char,
        target: *const std::ffi::c_char,
        area: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn proj_normalize_for_visualization(
        ctx: *mut std::ffi::c_void,
        obj: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn proj_trans_generic(
        pj: *mut std::ffi::c_void,
        direction: i32,
        x: *mut f64,
        sx: usize,
        nx: usize,
        y: *mut f64,
        sy: usize,
        ny: usize,
        z: *mut f64,
        sz: usize,
        nz: usize,
        t: *mut f64,
        st: usize,
        nt: usize,
    ) -> usize;
}

#[cfg(link_libproj)]
fn libproj_ratios_linked(grids: &[ProjGrid<'_>]) -> Vec<(String, f64)> {
    use std::ffi::CString;
    // SAFETY: libproj creates one context. It is destroyed before return.
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

/// libproj's time for the same facade cases: `proj_create_crs_to_crs` and
/// `proj_normalize_for_visualization` on the pair, then `proj_trans` one point at a time and
/// `proj_trans_generic` on an interleaved buffer, over the points of [`facade_inputs`].
fn libproj_facade() -> Vec<(String, f64)> {
    #[cfg(not(link_libproj))]
    {
        Vec::new()
    }
    #[cfg(link_libproj)]
    {
        libproj_facade_linked()
    }
}

#[cfg(link_libproj)]
fn libproj_facade_linked() -> Vec<(String, f64)> {
    use std::ffi::CString;
    let (geographic, lambert) = facade_inputs();
    // SAFETY: libproj creates one context. It is destroyed before return.
    let ctx = unsafe { proj_context_create() };
    assert!(!ctx.is_null(), "proj_context_create");
    // SAFETY: `ctx` is live. Level 0 is PJ_LOG_NONE: libproj reports each missing grid of the
    // NAD27 candidates on stderr, which says nothing about the timing.
    unsafe { proj_log_level(ctx, 0) };
    let mut times = Vec::new();
    for (name, from, to) in FACADE_PAIRS {
        let (source, target) = (
            CString::new(from).expect("source"),
            CString::new(to).expect("target"),
        );
        // SAFETY: both strings are nul-terminated and `ctx` is live. A null area is allowed.
        let raw = unsafe {
            proj_create_crs_to_crs(ctx, source.as_ptr(), target.as_ptr(), std::ptr::null_mut())
        };
        assert!(!raw.is_null(), "proj_create_crs_to_crs {from} {to}");
        // SAFETY: `raw` is a live transform of `ctx`.
        let pj = unsafe { proj_normalize_for_visualization(ctx, raw) };
        assert!(
            !pj.is_null(),
            "proj_normalize_for_visualization {from} {to}"
        );
        // SAFETY: `raw` is not used again.
        unsafe { proj_destroy(raw) };
        let points = if from == "EPSG:2154" {
            &lambert
        } else {
            &geographic
        };
        let coords: Vec<PjCoord> = points
            .iter()
            .map(|&(x, y)| PjCoord {
                x,
                y,
                z: 0.0,
                t: f64::INFINITY,
            })
            .collect();
        let one_by_one = per_point(&coords, |coord| {
            // SAFETY: `pj` is live and `coord` is a 32-byte `PJ_COORD`.
            unsafe { proj_trans(pj, 1, coord) }.x
        });
        times.push((name.to_owned(), one_by_one));
        let mut buffer = points.clone();
        let array = time_stable(points.len(), || {
            buffer.copy_from_slice(black_box(points));
            let count = buffer.len();
            let x = buffer.as_mut_ptr().cast::<f64>();
            let y = x.wrapping_add(1);
            // SAFETY: `buffer` holds `count` interleaved (x, y) pairs of 16 bytes, so `y` starts
            // one `f64` after `x`. Null z and t with a zero count make libproj use 0 and no time.
            let done = unsafe {
                proj_trans_generic(
                    pj,
                    1,
                    x,
                    16,
                    count,
                    y,
                    16,
                    count,
                    std::ptr::null_mut(),
                    0,
                    0,
                    std::ptr::null_mut(),
                    0,
                    0,
                )
            };
            black_box(done);
        });
        times.push((name.replace("convert", "convert_array"), array));
        check_same_output(name, &buffer, points, (from, to));
        // SAFETY: `pj` was created in this loop and is not used again.
        unsafe { proj_destroy(pj) };
    }
    // SAFETY: `ctx` was created at the start of this function.
    unsafe { proj_context_destroy(ctx) };
    times
}

/// The two sides must do the same work: the facade's image of every point agrees with the
/// libproj image in `libproj_out`, far below any measurement artefact.
#[cfg(link_libproj)]
#[expect(
    clippy::print_stderr,
    reason = "the agreement figures go to stderr, the JSON record owns stdout"
)]
fn check_same_output(
    name: &str,
    libproj_out: &[(f64, f64)],
    points: &[(f64, f64)],
    (from, to): (&str, &str),
) {
    let mut ours = points.to_vec();
    Proj::new_known_crs(from, to, None)
        .expect(name)
        .convert_array(&mut ours)
        .expect(name);
    let worst = ours
        .iter()
        .zip(libproj_out)
        .map(|(a, b)| (a.0 - b.0).abs().max((a.1 - b.1).abs()))
        .fold(0.0_f64, f64::max);
    eprintln!("{name}: largest coordinate difference with libproj {worst:.3e}");
    let degrees = to == "EPSG:4326" || to == "EPSG:4267";
    let tolerance = if degrees { 1e-6 } else { 1e-2 };
    assert!(
        worst <= tolerance,
        "{name}: proj-rs and libproj disagree by {worst}, the two sides do not do the same work"
    );
}
