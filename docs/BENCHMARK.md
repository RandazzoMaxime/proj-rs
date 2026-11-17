# Benchmark: what is compared with libproj

`src/bin/bench.rs` (the `proj-rs-bench` binary) times PROJ-RS and PROJ 9.9.0's
`libproj` in the same process, on the same points, one after the other. The
numbers in the README come from one line of
[`bench/history.jsonl`](../bench/history.jsonl).

## The ten real cases

Five CRS pairs, each timed two ways. The pair is a named CRS pair, so the work
includes the CRS lookup done once at creation, the axis order, and the choice of
operation among the candidates for the point.

| Pair | Source points |
|---|---|
| `EPSG:4326` to `EPSG:2154` | 200,000 points, lon/lat degrees, uniform in -4.5..8.0, 41.5..51.0 |
| `EPSG:4326` to `EPSG:32631` | the same points |
| `EPSG:4326` to `EPSG:3857` | the same points |
| `EPSG:2154` to `EPSG:32631` | the same points, in EPSG:2154 metres (from the first pair) |
| `EPSG:4267` to `EPSG:4326` | the same points, read as NAD27 lon/lat degrees |

| Call | PROJ-RS | libproj |
|---|---|---|
| Creation (not timed) | `Proj::new_known_crs(from, to, None)` | `proj_create_crs_to_crs(ctx, from, to, NULL)`, then `proj_normalize_for_visualization` |
| One point at a time | `Proj::convert((x, y))` for each point | `proj_trans(pj, PJ_FWD, coord)` for each point |
| A slice | `Proj::convert_array(&mut [(x, y)])` | `proj_trans_generic` on the same interleaved `(x, y)` buffer, strides of 16 bytes, no z, no t |

Details that make the two sides do the same work:

- Axis order: `new_known_crs` is georust's `Proj::new_known_crs`, which takes and
  gives longitude, latitude (easting, northing). `proj_normalize_for_visualization`
  gives libproj the same order.
- The inputs are the same `f64` values on both sides. libproj reads degrees for a
  geographic CRS made by `proj_create_crs_to_crs`, as the facade does.
- The slice case restores the input with `copy_from_slice` before each pass, on
  both sides, and that copy is inside the timed region on both sides.
- The two sides must agree. After timing a pair, the bench converts the points
  with PROJ-RS and compares them to libproj's output. It stops if the largest
  coordinate difference exceeds 1e-6 degrees or 1 cm. It prints the difference on
  stderr. On the recorded run it is below 1e-8.
- No grid and no network on either side. For `EPSG:4267` to `EPSG:4326` the NAD27
  grid-based candidates are unavailable, so both fall back to the same
  grid-free operation, PROJ's "Ballpark geographic offset from NAD27 to WGS 84"
  (`projinfo -s EPSG:4267 -t EPSG:4326` lists it alone): units and axis order, no datum
  shift. The outputs are identical. proj-rs folds the steps that only permute or scale the
  axes into one (`docs/PORTING-affine-plan.md`); this pair shows the largest ratio.
- Both read the same `proj.db`: `PROJ_DATA` points to the repository's `data/`
  (that is PROJ 9.9's database) for the PROJ-RS side and for libproj.
- libproj is a Release build of PROJ 9.9.0 (`tools/build-reference.sh`, linked
  with `PROJ_RS_LIBPROJ_DIR`). Its logging is switched off during the timing.

`proj_trans_generic` calls `proj_trans` for each point. It has no vector kernel,
so on a slice the difference is PROJ-RS's batch kernels (AVX2 where the CPU has
it) against libproj's scalar loop. That is what a user of each public API gets.
The batch path of the run is in the line: `batch_path`.

## The three bare projections

`utm31_forward`, `lambert93_forward` and `webmerc_forward` time one projection
from a `+proj=` string: PROJ-RS's `Tmerc`, `Lcc` and `Mercator`, and libproj's
`proj_create` with the equivalent string and `proj_trans`, over a million points.
They have no CRS lookup and no operation selection.

## Statistic

Each case is the median of 3 trials, each trial the median of 11 passes
(`time_stable`). `BENCH_ROUNDS=5` runs the whole bench five times and keeps, for
each case and for each side, the smallest value of the five
(`tools/bench/min_of_runs.py`). The ratio in the history line is PROJ-RS's time
over libproj's, so a ratio below 1 means PROJ-RS is faster. The README quotes
libproj's time over PROJ-RS's, so that "2× faster" reads as 2.

The machine is a shared VM and is not quiet: libproj's own time for one case moves from one
line to the next (`EPSG:4326` to `EPSG:2154`, one point at a time: 87 to 108 ns over the four
lines that have it). Read the ratio of one line, not the nanoseconds of two lines.

The README average is a geometric mean of the ten ratios. The README also gives
the slowest and the fastest case, and the mean of the one-point and the slice
cases apart.

## Rerun

```bash
tools/build-reference.sh   # once: PROJ 9.9.0 in target/proj-reference/build/lib
export PROJ_RS_LIBPROJ_DIR=$PWD/target/proj-reference/build/lib
BENCH_MACHINE=my-box BENCH_ROUNDS=5 taskset -c 2 sh bench/record.sh
```

The script appends a line to `bench/history.jsonl`, draws
`bench/trend-<machine>-{light,dark}.svg` and rewrites the "How fast" and "Over
time" blocks of the README (`tools/bench/how_fast.py`, `tools/bench/trend_svg.py`).
Compare only lines with the same `machine`.

## Size and memory

`bench/footprint.sh` measures what the same conversion, `EPSG:4326` to `EPSG:2154`, costs on
disk and in memory with proj-rs and with PROJ 9.9.0. It writes
[`bench/footprint.json`](../bench/footprint.json), draws `bench/{size,memory}-{light,dark}.svg`
and rewrites the `bench-footprint` block of the README (`tools/bench/footprint_measure.py`,
`tools/bench/footprint_svg.py`). The README shows only numbers from that file. Linux x86-64
only. It is not called by `bench/record.sh`: it builds PROJ a second time, the Rust programs and
the C library, which takes minutes, and its numbers do not move with the speed work.

The programs are in `bench/footprint/`: `rs/` (a crate of its own, out of the workspace, so the
normal build and the CI never compile it) and `c/convert.c`. They take one mode each:

| Mode | Work |
|---|---|
| `convert` | create the transform, convert one point (the program whose size is measured) |
| `empty` | start and exit |
| `create` | create the `EPSG:4326` to `EPSG:2154` transform |
| `points` | create, then 1,000,000 points one at a time (`convert` / `proj_trans`) |
| `array` | create, then 1,000,000 points in one array (`convert_array` / `proj_trans_generic`) |
| `multi` | `EPSG:4267` to `EPSG:4326` (several candidate operations), then 1,000,000 points one at a time |

The points are those of the speed bench (same generator, same box), made on the fly for
`points` and `multi` on both sides. The C program is `proj_create_crs_to_crs` plus
`proj_normalize_for_visualization`, as in the speed bench, with logging off. Each run prints
`ok= fail= sum=` and the script stops if the sums of PROJ and proj-rs differ by more than 1e-6
relative: both sides did the same work.

### Size

Five installs, in bytes on disk after `strip --strip-unneeded`:

1. the C program with PROJ 9.9.0 built as in the speed comparison (`tools/build-reference.sh`:
   TIFF and curl off);
2. the C program with PROJ 9.9.0 built with TIFF and curl (`PROJ_REF_TIFF_CURL=ON`), the way
   distributions build it: it reads `.tif` grids and downloads them, which proj-rs also does;
3. the same C program linked to the `libproj` of `capi/` (`capi/tests/dist.sh --release`);
4. the Rust program with proj-rs and its default features (no TLS library, no HTTPS download);
5. the same Rust program with the `network` feature (HTTPS grid download, TLS from the system:
   OpenSSL on Linux), the counterpart of the PROJ with curl.

**Counting rule for the dependencies.** A program counts with every shared library that `ldd`
lists for it, transitively, except the C library and the unwinder that any Linux system has:
`libc`, `libm`, `libdl`, `libpthread`, `librt`, `libutil`, `libgcc_s` and the loader. The rule
is the same on both sides. A library counts with its real file, not the symlink. `proj.db` counts
in full: as a file for PROJ, and as the bytes inside the binary for proj-rs (`include_bytes!`;
the script checks that the bytes are there unchanged). The Rust programs have SQLite built in
(`rusqlite` `bundled`); on the PROJ side `libsqlite3` is a dependency and counts. The libraries
come from this machine's distribution, so their sizes depend on it.

Not measured: the conda-forge `proj` 9.9.0 package with its dependencies, as the CI installs
it. The measurement runs without micromamba or network access to conda-forge. Its closest
stand-in is the second install, a PROJ with TIFF and curl on the system's own libraries.

### Memory

Peak resident memory (`VmHWM`, read by each program from `/proc/self/status` just before it
exits), the median of 5 runs, one scenario per process. An external `getrusage` is not used:
on Linux a child that execs keeps its parent's peak as a floor. It includes the loader, the
mapped libraries and, for `array`, the 15.3 MiB buffer of points that both sides allocate.
The heap is the peak `mem_heap_B` of one `valgrind --tool=massif` run, when valgrind is
installed. `PROJ_DATA` points at the PROJ install for the C programs; proj-rs reads its built-in
database. PROJ's numbers are those of the benchmark build (the first install above); the other
builds are in the README table.

### Rerun

```bash
BENCH_MACHINE=my-box sh bench/footprint.sh
```

Needs cmake, a C and C++ compiler, the libtiff, libcurl and SQLite development headers, `strip`,
`ldd`, and valgrind for the heap column. `PROJ_REF` (default `target/proj-reference`) is the
directory of `tools/build-reference.sh`, built when missing; the other builds go in
`target/footprint` (`FOOTPRINT_OUT`), with `FOOTPRINT_JOBS` jobs (default 2). Compare only files
with the same `machine`.
