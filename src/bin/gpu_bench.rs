//! GPU benchmark: Markdown on stdout, progress on stderr.
//!
//! For each conversion (Web Mercator, UTM 31N, Lambert-93, both ways) and each precision the
//! device offers: the accuracy against the CPU path, the compute alone (data already on the
//! device), and the whole call (`convert_array_gpu`: conversion to the device format, upload,
//! dispatch, read-back). The CPU side is `project_array` on one core and on every core, so the
//! table says how many times the GPU beats the machine it runs on, which is what decides whether
//! to use it. `--quick` for a short run, `--big` up to 16 million points, `--sizes a,b,c`.
//!
//! A software adapter (llvmpipe, WARP) is flagged: its times say nothing about a GPU.

use proj_rs::gpu::{GpuContext, GpuOptions, Precision};
use proj_rs::{Direction, Proj};
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

/// Points the references and the accuracy check use; the larger sizes cycle them.
const BASE_POINTS: usize = 200_000;

#[derive(Clone, Copy)]
struct Case {
    label: &'static str,
    from: &'static str,
    to: &'static str,
    way: Direction,
    /// Longitude and latitude box of the geographic points, degrees.
    area: (f64, f64, f64, f64),
}

const WORLD: (f64, f64, f64, f64) = (-180.0, 180.0, -85.0, 85.0);
const ZONE_31: (f64, f64, f64, f64) = (0.0, 6.0, -80.0, 84.0);
const FRANCE: (f64, f64, f64, f64) = (-5.5, 10.0, 41.0, 51.5);

const CASES: [Case; 6] = [
    Case {
        label: "4326 -> 3857",
        from: "EPSG:4326",
        to: "EPSG:3857",
        way: Direction::Forward,
        area: WORLD,
    },
    Case {
        label: "3857 -> 4326",
        from: "EPSG:4326",
        to: "EPSG:3857",
        way: Direction::Inverse,
        area: WORLD,
    },
    Case {
        label: "4326 -> 32631",
        from: "EPSG:4326",
        to: "EPSG:32631",
        way: Direction::Forward,
        area: ZONE_31,
    },
    Case {
        label: "32631 -> 4326",
        from: "EPSG:4326",
        to: "EPSG:32631",
        way: Direction::Inverse,
        area: ZONE_31,
    },
    Case {
        label: "4326 -> 2154",
        from: "EPSG:4326",
        to: "EPSG:2154",
        way: Direction::Forward,
        area: FRANCE,
    },
    Case {
        label: "2154 -> 4326",
        from: "EPSG:4326",
        to: "EPSG:2154",
        way: Direction::Inverse,
        area: FRANCE,
    },
];

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    f64::from(u32::try_from(*state >> 32).expect("32 bits")) / 4_294_967_296.0
}

fn scatter(count: usize, area: (f64, f64, f64, f64)) -> Vec<(f64, f64)> {
    let mut state = 0x5eed_u64;
    (0..count)
        .map(|_| {
            let lon = area.0 + (area.1 - area.0) * lcg(&mut state);
            let lat = area.2 + (area.3 - area.2) * lcg(&mut state);
            (lon, lat)
        })
        .collect()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "a point count under 2^53 is exact in f64"
)]
fn ns_per_point(seconds: f64, points: usize) -> f64 {
    seconds * 1e9 / points as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

struct Options {
    sizes: Vec<usize>,
    cpu_points: usize,
}

fn parse_args() -> Options {
    let mut options = Options {
        sizes: vec![100_000, 1_000_000, 4_000_000],
        cpu_points: 1_000_000,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => {
                options.sizes = vec![100_000, 1_000_000];
                options.cpu_points = 200_000;
            }
            "--big" => options.sizes = vec![1_000_000, 4_000_000, 16_000_000],
            "--sizes" => {
                options.sizes = args
                    .next()
                    .expect("--sizes takes a comma list")
                    .split(',')
                    .map(|size| size.trim().parse().expect("a point count"))
                    .collect();
            }
            other => panic!("unknown option {other}: --quick --big --sizes a,b,c"),
        }
    }
    options
}

/// The conversion of a case in its direction on the CPU path, in place.
fn cpu(proj: &Proj, points: &mut [(f64, f64)], way: Direction) {
    black_box(
        proj.project_array(points, way == Direction::Inverse)
            .map(|done| done.len())
            .ok(),
    );
}

struct CpuTimes {
    scalar: f64,
    array: f64,
    threads: f64,
    thread_count: usize,
}

fn cpu_times(proj: &Proj, points: &[(f64, f64)], way: Direction) -> CpuTimes {
    let few = &points[..points.len().min(200_000)];
    let start = Instant::now();
    let mut sink = 0.0;
    for &point in few {
        sink += proj
            .project(point, way == Direction::Inverse)
            .map_or(0.0, |out: (f64, f64)| out.0);
    }
    black_box(sink);
    let scalar = ns_per_point(start.elapsed().as_secs_f64(), few.len());
    let mut buffer = points.to_vec();
    let mut best = f64::MAX;
    for _ in 0..3 {
        buffer.copy_from_slice(points);
        let start = Instant::now();
        cpu(proj, &mut buffer, way);
        best = best.min(start.elapsed().as_secs_f64());
    }
    let array = ns_per_point(best, points.len());
    let thread_count = std::thread::available_parallelism().map_or(1, usize::from);
    let part = points.len().div_ceil(thread_count);
    let mut best_threads = f64::MAX;
    for _ in 0..3 {
        // One transform per thread, made before the clock: `Proj` is `Send`, not `Sync`.
        let locals: Vec<Proj> = (0..thread_count).map(|_| proj.clone()).collect();
        let mut buffer = points.to_vec();
        let start = Instant::now();
        std::thread::scope(|scope| {
            for (chunk, local) in buffer.chunks_mut(part).zip(locals) {
                scope.spawn(move || cpu(&local, chunk, way));
            }
        });
        best_threads = best_threads.min(start.elapsed().as_secs_f64());
    }
    CpuTimes {
        scalar,
        array,
        threads: ns_per_point(best_threads, points.len()),
        thread_count,
    }
}

/// The input of a case: geographic points forward, what the forward made for the inverse.
fn inputs(case: &Case, count: usize) -> Vec<(f64, f64)> {
    let geographic = scatter(count, case.area);
    if case.way == Direction::Forward {
        return geographic;
    }
    let proj = Proj::new_known_crs(case.from, case.to, None).expect("pair");
    let mut projected = geographic;
    cpu(&proj, &mut projected, Direction::Forward);
    projected
}

fn cycle(points: &[(f64, f64)], count: usize) -> Vec<(f64, f64)> {
    points.iter().copied().cycle().take(count).collect()
}

fn describe(gpu: &GpuContext) -> String {
    format!(
        "{} ({}), {:?}, {} points per dispatch",
        gpu.adapter_name(),
        gpu.backend(),
        gpu.precision(),
        gpu.chunk_points()
    )
}

#[expect(
    clippy::print_stdout,
    reason = "the benchmark's product is this Markdown on stdout"
)]
#[expect(
    clippy::print_stderr,
    reason = "progress goes to stderr, the Markdown owns stdout"
)]
fn main() {
    let options = parse_args();
    let max_n = options.sizes.iter().copied().max().unwrap_or(0);
    let mut out = String::new();
    writeln!(out, "# proj-rs GPU benchmark\n").expect("write");
    let contexts: Vec<GpuContext> = [Precision::DoubleFloat, Precision::Float64]
        .into_iter()
        .filter_map(|precision| {
            let options = GpuOptions::default()
                .with_precision(precision)
                .with_min_points(0);
            match GpuContext::with_options(options) {
                Ok(gpu) => Some(gpu),
                Err(error) => {
                    writeln!(out, "- no {precision:?} context: {error}").ok();
                    None
                }
            }
        })
        .collect();
    let Some(first) = contexts.first() else {
        writeln!(out, "\nNo GPU context could be made: nothing to measure.").expect("write");
        print!("{out}");
        return;
    };
    let software = first.adapter_name().to_lowercase();
    if [
        "llvmpipe",
        "lavapipe",
        "warp",
        "basic render",
        "swiftshader",
    ]
    .iter()
    .any(|name| software.contains(name))
    {
        writeln!(
            out,
            "\n> **Software adapter**: accuracy is valid, the times say nothing about a GPU.\n"
        )
        .expect("write");
    }
    for gpu in &contexts {
        writeln!(out, "- {}", describe(gpu)).expect("write");
    }
    writeln!(out).expect("write");

    let mut rows = String::new();
    let mut cpu_rows = String::new();
    let mut accuracy_rows = String::new();
    for case in CASES {
        eprintln!("{}", case.label);
        let proj = Proj::new_known_crs(case.from, case.to, None).expect("pair");
        let base = inputs(&case, BASE_POINTS);
        let cpu_input = cycle(&base, options.cpu_points);
        let times = cpu_times(&proj, &cpu_input, case.way);
        writeln!(
            cpu_rows,
            "| {} | {:.1} | {:.1} | {:.1} ({}) |",
            case.label, times.scalar, times.array, times.threads, times.thread_count
        )
        .expect("write");
        let mut reference = base.clone();
        cpu(&proj, &mut reference, case.way);
        let bench = cycle(&base, max_n);
        for gpu in &contexts {
            let precision = gpu.precision();
            // Accuracy against the CPU path, in metres forward and degrees back.
            let mut result = base.clone();
            let covered = proj
                .convert_array_gpu_in(&mut result, case.way, gpu)
                .is_ok_and(|(_, report)| report.cpu_points == 0);
            if !covered {
                writeln!(
                    accuracy_rows,
                    "| {} | {precision:?} | not on the GPU | | |",
                    case.label
                )
                .expect("write");
                continue;
            }
            let worst = result
                .iter()
                .zip(&reference)
                .map(|(got, want)| (got.0 - want.0).abs().max((got.1 - want.1).abs()))
                .fold(0.0_f64, f64::max);
            let unit = if case.way == Direction::Forward {
                "m"
            } else {
                "deg"
            };
            writeln!(
                accuracy_rows,
                "| {} | {precision:?} | {worst:.2e} {unit} | {} | {} |",
                case.label,
                if worst
                    <= if case.way == Direction::Forward {
                        1e-6
                    } else {
                        1e-11
                    }
                {
                    "within the parity rule"
                } else {
                    "OVER the parity rule"
                },
                base.len()
            )
            .expect("write");
            for &size in &options.sizes {
                let on_device = &bench[..size.min(gpu.chunk_points())];
                let first = proj
                    .gpu_dispatch_seconds(on_device, case.way, gpu, 1)
                    .map_or(0.0, |(_, seconds)| seconds);
                let repeats = if first > 0.0 {
                    let wanted = (0.3 / first).clamp(1.0, 20.0);
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        clippy::as_conversions,
                        reason = "a repeat count between 1 and 20"
                    )]
                    let count = wanted as u32;
                    count
                } else {
                    20
                };
                let mut samples: Vec<f64> = (0..5)
                    .filter_map(|_| proj.gpu_dispatch_seconds(on_device, case.way, gpu, repeats))
                    .map(|(_, seconds)| seconds)
                    .collect();
                let compute = if samples.is_empty() {
                    f64::NAN
                } else {
                    ns_per_point(median(&mut samples), on_device.len())
                };
                let mut whole = Vec::new();
                let mut on_gpu = true;
                for _ in 0..3 {
                    let mut buffer = bench[..size].to_vec();
                    let start = Instant::now();
                    let report = proj
                        .convert_array_gpu_in(&mut buffer, case.way, gpu)
                        .map(|(_, report)| report);
                    whole.push(start.elapsed().as_secs_f64());
                    on_gpu &= report.is_ok_and(|report| report.cpu_points == 0);
                    black_box(&buffer);
                }
                let end_to_end = ns_per_point(median(&mut whole), size);
                writeln!(
                    rows,
                    "| {} | {precision:?} | {size} | {compute:.2} | {:.1} | {end_to_end:.2} | {:.2}x | {:.2}x |{}",
                    case.label,
                    1e3 / compute,
                    times.array / end_to_end,
                    times.threads / end_to_end,
                    if on_gpu { "" } else { " ran on the CPU" }
                )
                .expect("write");
            }
        }
    }
    writeln!(out, "## CPU (the same conversion, `project_array`)\n").expect("write");
    writeln!(
        out,
        "| conversion | one point at a time ns/pt | `project_array` ns/pt | `project_array` on every core ns/pt (threads) |\n|---|---:|---:|---:|"
    )
    .expect("write");
    out += &cpu_rows;
    writeln!(out, "\n## Accuracy against the CPU path\n").expect("write");
    writeln!(
        out,
        "| conversion | precision | largest gap | | points |\n|---|---|---:|---|---:|"
    )
    .expect("write");
    out += &accuracy_rows;
    writeln!(out, "\n## Speed\n").expect("write");
    writeln!(
        out,
        "Compute: data already on the device. Whole call: `convert_array_gpu`, conversion to the \
         device format, upload, dispatch, read-back. `vs 1 core` and `vs all cores` are the CPU \
         time over the GPU whole-call time: above 1 the GPU wins.\n"
    )
    .expect("write");
    writeln!(
        out,
        "| conversion | precision | points | compute ns/pt | compute Mpts/s | whole call ns/pt | vs 1 core | vs all cores |\n|---|---|---:|---:|---:|---:|---:|---:|"
    )
    .expect("write");
    out += &rows;
    print!("{out}");
}
