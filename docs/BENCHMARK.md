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
  grid-free operation, and the outputs are identical.
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
