# cs2cs build paths: which path serves each case

Sheet for the path tracer (`PROJ_RS_TRACE_PATH`, `src/bin/cs2cs.rs`). It lists, for each cs2cs case of `tests/cli_yaml/test_cs2cs_various.yaml` and each pair of `tests/agree_cs2cs.rs`, the path that built the transformation and why the ISO selection (1) did not serve it.

## Paths

| # | Path | Code | Used when |
|---|---|---|---|
| 1 | `iso` | `select_pipeline` / `select_pipeline_filtered` over `best_pipeline_with` | no area, no PROJ string on either side |
| 2 | `side-pair` | `SidePair::open` | both sides are PROJ strings (`+proj=`, `+init=`) |
| 3 | `legacy-epsg` | `CrsPipeline::create` | both sides are EPSG tokens and path (1) failed |
| 4 | `facade` | `Proj::new_known_crs` | every other case: an area, a PROJ string on one side only, or a failed (3) |

The trace is printed once per run on stderr, only when `PROJ_RS_TRACE_PATH` is set. The line for path (1) carries no status; a later path carries `iso=not-tried` or `iso=failed: <error>`.

## Method

Each case ran against `target/debug/cs2cs` built from commit `efa913c` with `PROJ_RS_TRACE_PATH=1` and `PROJ_DATA` set to an empty directory, so no grid file is found. The yaml `env` values were applied, with `$tmpdir` replaced by that directory. The agree pairs ran as `cs2cs -d 8 FROM TO` on the points of `tests/agree_cs2cs.rs`. The paths were recorded from the trace, not inferred from the code. The inventory script is not committed.

## Counts

| Path | yaml cases (of 143) | agree pairs (of 11) |
|---|---:|---:|
| `iso` (1) | 39 | 11 |
| `side-pair` (2) | 90 | 0 |
| `legacy-epsg` (3) | 0 | 0 |
| `facade` (4) | 8 | 0 |
| no engine (argument error or filter) | 6 | 0 |

Pairs that depend on (3): **0**. Pairs that depend on (4): **8** yaml cases (ids 129, 130, 131, 132, 133, 134, 135, 142), and **0** agree pairs. Path (3) never built an engine in this set: every EPSG pair that reached it was served by the ISO path first, and the EPSG pairs that the ISO path could not serve (`EPSG:4746` to `EPSG:4978`) did not pass `CrsPipeline::create` either. That builder's error is not logged, so its cause is not known here.

## Why path (1) did not serve a yaml case

- **PROJ-string side (`not tried`)**: the gate `!projish(source) && !projish(target)` in `build_engine` skips path (1). 90 cases (both sides PROJ strings) are on path (2) for this reason, and 2 (one side a PROJ string) on path (4).
- **ISO error**: path (1) ran and failed. The error text is in the table below.
- **Filter removes every candidate**: `--accuracy` or `--no-ballpark` leave no operation; the cs2cs error is `cause: (null)` (no engine, rc 3, as PROJ).
- **Argument error**: `-W` out of range; no engine, rc 1.

## Per-case table: yaml `test_cs2cs_various.yaml`

| id | path | (1) | why (1) did not serve it | args |
|---|---|---|---|---|
| 1 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=clrk66 +to +proj=latlong +ellps=bessel` |
| 2 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=NAD27 +to +proj=latlong +ellps=bessel` |
| 3 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=bessel +towgs84=5,0,0 +to +proj=latlong +ellps...` |
| 4 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=bessel +towgs84=5,0,0 +to +proj=latlong +ellps...` |
| 5 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +pm=greenwich +to +proj=latlong +datum=W...` |
| 6 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84 +lon_wrap...` |
| 7 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=utm +zone=11 +datum=WGS84 +pm=3 +to +proj=latlong +datum=WGS8...` |
| 8 | iso | built | - | `EPSG:4807 EPSG:27572` |
| 9 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=geocent +datum=WGS84` |
| 10 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=geocent +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 11 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84 +geoc` |
| 12 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +geoc +to +proj=latlong +datum=WGS84` |
| 13 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=stere +lat_0=90 +lon_0=0 +lat_...` |
| 14 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=stere +lat_0=40 +lon_0=10 +dat...` |
| 15 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=WGS84 +to +proj=kav5 +ellps=WGS84 +units=m` |
| 16 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=kav5 +ellps=WGS84 +units=m +to +proj=latlong +ellps=WGS84` |
| 17 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=6377298.556 +rf=300.8017 +to +proj=omerc +a=637729...` |
| 18 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=etmerc +k=0.998 +lon_0=-20 +datum=WGS84 +x_0=10000 +y_0=20000...` |
| 19 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=etmerc +k=0.998 +lon_0=-20 +da...` |
| 20 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=tmerc +approx +k=0.998 +lon_0=-20 +datum=WGS84 +x_0=10000 +y_...` |
| 21 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=tmerc +approx +k=0.998 +lon_0=...` |
| 22 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=robin +datum=WGS84` |
| 23 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=robin +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 24 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=hammer +datum=WGS84` |
| 25 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=hammer +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 26 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=1 +lon_0=0 +to +proj=healpix +R=1 +lon_0=0 -f %.5f` |
| 27 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=5 +to +proj=healpix +R=5 -f %.5f` |
| 28 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=1 +lon_0=0 +ellps=WGS84 +to +proj=healpix +a=1 +lo...` |
| 29 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=healpix...` |
| 30 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=1 +lon_0=0 +ellps=WGS84 +to +proj=healpix +a=1 +lo...` |
| 31 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=healpix...` |
| 32 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=healpix...` |
| 33 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=5 +to +proj=rhealpix +R=5 +north_square=0 +south_s...` |
| 34 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=5 +to +proj=rhealpix +R=5 +north_square=1 +south_s...` |
| 35 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=5 +to +proj=rhealpix +R=5 +north_square=0 +south_s...` |
| 36 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +R=5 +to +proj=rhealpix +R=5 +north_square=1 +south_s...` |
| 37 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=rhealpi...` |
| 38 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=rhealpi...` |
| 39 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=rhealpi...` |
| 40 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=5 +e=0.8 +r_a=4.3220011711888882 +to +proj=rhealpi...` |
| 41 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 42 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 43 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 44 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 45 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 46 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 47 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 48 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=sphere +to +proj=geos +h=35785831.0 +lon_0=0 +...` |
| 49 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=6371008.7714 +b=6371008.7714 +to +proj=natearth +a...` |
| 50 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=6371008.7714 +b=6371008.7714 +to +proj=natearth2 +...` |
| 51 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +a=6371008.7714 +b=6371008.7714 +to +proj=comill +a=6...` |
| 52 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=pconic +units=m +lat_1=20n +la...` |
| 53 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=pconic +units=m +lat_1=20n +lat_2=60n +lon_0=60W +datum=WGS84...` |
| 54 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=laea +lat_0=45 +lon_0=-100 +units=m +datum=WGS84 +to +proj=la...` |
| 55 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=clrk66 +to +proj=calcofi +ellps=clrk66` |
| 56 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=calcofi +ellps=clrk66 +to +proj=longlat +ellps=clrk66` |
| 57 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=ob_tran +o_proj=moll +a=6378137 +es=0 +o_lon_p=0 +o_lat_p=0 +...` |
| 58 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `-I +proj=ob_tran +o_proj=moll +a=6378137 +es=0 +o_lon_p=0 +o_lat_p=...` |
| 59 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +init=epsg:31284 -f %.7f` |
| 60 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=WGS84 +to +proj=omerc +ellps=WGS84 +lon_1=62.5...` |
| 61 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=WGS84 +to +proj=omerc +ellps=WGS84 +datum=WGS8...` |
| 62 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +ellps=WGS84 +to +proj=omerc +ellps=WGS84 +datum=WGS8...` |
| 63 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=qsc +datum=WGS84 -f %.7f` |
| 64 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=qsc +datum=WGS84 +to +proj=latlong +datum=WGS84 -f %.13f` |
| 65 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+init=epsg:4326 +proj=longlat +ellps=WGS84 +datum=WGS84 +towgs84=0,...` |
| 66 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+init=epsg:4326 +to +init=epsg:4326 -f %.13f` |
| 67 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+init=epsg:4326 +to +proj=aeqd +lon_0=130.0 +lat_0=40.0 +a=6378137 ...` |
| 68 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=aeqd +lon_0=130.0 +lat_0=40.0 +a=6378137 +b=6378137 +units=m ...` |
| 69 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +datum=WGS84 +to +proj=utm +zone=32 +datum=carthage -...` |
| 70 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +datum=WGS84 +to +proj=utm +zone=32 +a=6378249.2 +b=6...` |
| 71 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=sch +datum=WGS84 +plat_0=30.0 ...` |
| 72 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=sch +datum=WGS84 +plat_0=30.0 +plon_0=45.0 +phdg_0=-12.0 +nod...` |
| 73 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=utm +zone=35 +datum=WGS84 -f %.6f` |
| 74 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=etmerc +datum=WGS84 +k=0.9996 ...` |
| 75 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=nzmg +lat_0=-41 +lon_0=173 +x_...` |
| 76 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=nzmg +lat_0=-41 +lon_0=173 +x_0=2510000 +y_0=6023150 +ellps=W...` |
| 77 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=patterson +a=6371008.7714 +b=6...` |
| 78 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=patterson +a=6371008.7714 +b=6371008.7714 +units=m +to +proj=...` |
| 79 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=utm +zone=15 +datum=NAD83 +to +proj=merc +a=6378137 +b=637813...` |
| 80 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +a=1 +b=1 +vto_meter=1000 +to +proj=longlat +a=1 +b=1...` |
| 81 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=merc +a=1 +b=1 +vto_meter=1000 +to +proj=longlat +a=1 +b=1 -f...` |
| 82 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +a=1 +b=1 +to +proj=longlat +a=1 +b=1 +vto_meter=1000...` |
| 83 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +a=1 +b=1 +to +proj=merc +a=1 +b=1 +vto_meter=1000 -f...` |
| 84 | iso | built | - | `EPSG:4326 +to EPSG:32631` |
| 85 | iso | built | - | `EPSG:32631 EPSG:4326` |
| 86 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +R=3376200 +to IAU_2015:49935` |
| 87 | iso | built | - | `EPSG:4896 EPSG:7930 -f %.4f -E` |
| 88 | iso | built | - | `ITRF2000 ITRF1993 -f %.7f -E` |
| 89 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +ellps=GRS80 +to +proj=ob_tran +o_proj=longlat +lon_0...` |
| 90 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +ellps=GRS80 +to +proj=ob_tran +o_proj=longlat +lon_0...` |
| 91 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+init=epsg:4326 +over +to +init=epsg:3857 +over -f %.7f` |
| 92 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +over +datum=WGS84 +to proj=merc +a=6378137 +b=637813...` |
| 93 | iso | built | - | `EPSG:4326 EPSG:4326 tmp.txt -E` |
| 94 | iso | built | - | `EPSG:4686 EPSG:6247 -f %.3f` |
| 95 | iso | built | - | `ITRF96 ITRF2014` |
| 96 | iso | built | - | `--authority PROJ ITRF96 ITRF2014` |
| 97 | none | no engine (rc 3) | no engine: cause: (null) | `--accuracy 0.05 EPSG:4326 EPSG:4258` |
| 98 | none | no engine (rc 3) | no engine: cause: (null) | `--no-ballpark EPSG:4267 EPSG:4258` |
| 99 | iso | built | - | `Pulkovo 1942 WGS 84` |
| 100 | iso | built | - | `EPSG:2636 WGS 84` |
| 101 | iso | built | - | `EPSG:4326 NAD83(HARN)` |
| 102 | iso | built | - | `WGS 84 WGS 84 + EGM96 height -d 3` |
| 103 | iso | built | - | `WGS 84 + EGM96 height WGS 84 -d 3` |
| 104 | iso | built | - | `EPSG:4326 EPSG:32631 input_file2_with_utf8_bom.txt -d 3` |
| 105 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `-W0 +proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 106 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `-W8 +proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 107 | none | no engine (rc 1) | no engine: -W argument missing or not in range [0,8] | `-W +proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 108 | none | no engine (rc 1) | no engine: -W argument missing or not in range [0,8] | `-W9 +proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 109 | none | no engine (rc 1) | no engine: -W argument missing or not in range [0,8] | `-W10 +proj=latlong +datum=WGS84 +to +proj=latlong +datum=WGS84` |
| 110 | iso | built | - | `--only-best NTF RGF93 -E` |
| 111 | iso | built | - | `--only-best EPSG:4326+5773 EPSG:4979` |
| 112 | iso | built | - | `--only-best EPSG:4326 GDA94` |
| 113 | iso | built | - | `--only-best GDA94 EPSG:4326` |
| 114 | iso | built | - | `--only-best EPSG:4326 GDA2020` |
| 115 | iso | built | - | `--only-best GDA2020 EPSG:4326` |
| 116 | iso | built | - | `EPSG:4326+3855 EPSG:4979` |
| 117 | iso | built | - | `--only-best=no EPSG:4326+3855 EPSG:4979` |
| 118 | iso | built | - | `--only-best EPSG:4326+3855 EPSG:4979` |
| 119 | iso | built | - | `--only-best=yes EPSG:4326+3855 EPSG:4979` |
| 120 | iso | built | - | `EPSG:4326+3855 EPSG:4979` |
| 121 | iso | built | - | `EPSG:4326+3855 EPSG:4979` |
| 122 | iso | built | - | `--only-best NAD27 NAD83` |
| 123 | none | no engine (rc 3) | no engine: Attempt to use coordinate operation Inverse of WGS 84 to EGM2008 height (1) failed. Grid us_nga_egm08_25.tif i | `--only-best --no-ballpark EPSG:4326+3855 EPSG:4979` |
| 124 | side-pair | not tried | both sides are PROJ strings, so the ISO selection is not tried (`build_engine` gate) | `+proj=latlong +datum=WGS84 +to +proj=utm +zone=10 +datum=NAD27` |
| 125 | iso | built | - | `EPSG:26915 EPSG:26715` |
| 126 | iso | built | - | `EPSG:4326+5773 EPSG:4326+5782` |
| 127 | iso | built | - | `--only-best=no EPSG:4326+5773 EPSG:4326+5782` |
| 128 | iso | built | - | `--only-best EPSG:4326+5773 EPSG:4326+5782` |
| 129 | facade | not tried | one side is a PROJ string, so the ISO selection is not tried (`build_engine` gate) | `+proj=longlat +datum=WGS84 +units=m +geoidgrids=@i_dont_exist.tif +...` |
| 130 | facade | failed | ISO error: `no operation from EPSG:4746 to EPSG:4978` | `EPSG:4746 EPSG:4978` |
| 131 | facade | failed | ISO error: `no operation from EPSG:4978 to EPSG:4746` | `EPSG:4978 EPSG:4746` |
| 132 | facade | failed | ISO error: `no operation from EPSG:4746 to EPSG:4978` | `EPSG:4746 EPSG:4978` |
| 133 | facade | failed | ISO error: `no operation from EPSG:4978 to EPSG:4746` | `EPSG:4978 EPSG:4746` |
| 134 | facade | failed | ISO error: `no operation from EPSG:4746 to EPSG:4978` | `EPSG:4746 EPSG:4978` |
| 135 | facade | failed | ISO error: `no operation from EPSG:4978 to EPSG:4746` | `EPSG:4978 EPSG:4746` |
| 136 | iso | built | - | `-d 3 EPSG:23031 EPSG:25831` |
| 137 | iso | built | - | `-d 3 EPSG:25831 EPSG:23031` |
| 138 | iso | built | - | `-d 3 EPSG:3912 EPSG:3794` |
| 139 | iso | built | - | `-d 3 EPSG:3912+EPSG:5779 EPSG:3794+EPSG:8690` |
| 140 | iso | built | - | `-d 8 --s_epoch 2022 ITRF2014 GDA2020` |
| 141 | iso | built | - | `-d 8 --t_epoch 2022 GDA2020 ITRF2014 -E` |
| 142 | facade | not tried | one side is a PROJ string, so the ISO selection is not tried (`build_engine` gate) | `+proj=topocentric +datum=WGS84 +X_0=-3982059.42 +Y_0=3331314.88 +Z_...` |
| 143 | iso | built | - | `-d 3 EPSG:5488 EPSG:4559+5757` |

## Per-pair table: `tests/agree_cs2cs.rs`

| pair | points | path | (1) |
|---|---:|---|---|
| EPSG:4326 -> EPSG:2154 | 4 | iso | built |
| EPSG:4326 -> EPSG:32631 | 4 | iso | built |
| EPSG:4326 -> EPSG:3857 | 4 | iso | built |
| EPSG:4326 -> EPSG:3035 | 4 | iso | built |
| EPSG:4326 -> EPSG:3395 | 4 | iso | built |
| EPSG:2154 -> EPSG:32631 | 2 | iso | built |
| EPSG:4258 -> EPSG:25832 | 3 | iso | built |
| EPSG:4326 -> EPSG:5070 | 3 | iso | built |
| EPSG:2230 -> EPSG:26946 | 2 | iso | built |
| EPSG:26946 -> EPSG:2230 | 1 | iso | built |
| EPSG:4326 -> EPSG:2230 | 1 | iso | built |

The agree test also calls `Proj::new_known_crs` (path (4) as the facade) on each pair as its facade check. That call runs its own selection and is not traced by `cs2cs`; it is out of this sheet's scope.

## Note on the EPSG:4746 to EPSG:4978 cases

The brief for this lot said this pair falls to the legacy EPSG builder and swaps lat/lon. The trace shows it falls to the facade (path 4), after path (1) fails with `no operation from EPSG:4746 to EPSG:4978`. The legacy builder (3) was tried, since both sides are EPSG tokens, and returned an error that is not logged. Grid name resolution and the ranking in `src/iso19111/select.rs` belong to another lot and are not changed here.

## Targets 124, 125, 139: why they stay open

Commit `efa913c` changed no pair's path, so the three targets keep their numbers. The findings below come from the trace, from `projinfo -o PROJ` candidate lists, and from `cct` runs of each candidate pipeline. PROJ source: `src/iso19111/operation/coordinateoperationfactory.cpp` at tag 9.9.0, read through a text extraction. `src/4D_api.cpp` was not read, because the raw URL returned 404 at that path on tag 9.9.0, so the per-point retry of PROJ is not checked here.

### 124: PROJ-string pair, path (2)

- Path (2) serves it because both sides are PROJ strings: the gate in `build_engine` skips path (1).
- Attempt 1 (done, reverted): removing the gate sends this pair to the ISO path. `projinfo` then lists 79 candidates, including `Inverse of NAD27 to WGS 84 (6)`, a Helmert with `+x=8 +y=-159 +z=-175` and no grid. The first candidate whose grids are found is then used, and it fails at the point, with no alternative to try. The gate removal also reopened yaml cases #6, #20, #21, #69, #79 and #92 (lon_wrap, `+approx` tmerc, `+datum=carthage`, a bare Web Mercator and `+over`), so it is reverted and the ledger is restored.
- Blockers, not fixed here:
  1. The grid search (`src/gridpath.rs`) also searches the vendored fixtures, so a grid PROJ does not have counts as available. That belongs to the other lot.
  2. Engine::Pipe has no list of alternatives, so a point that fails on the first operation is not retried on the next one, as the expected output requires.
  3. ISO import of PROJ strings with non-ISO parameters (`+approx`, `lon_wrap`, `+over`, `+datum=carthage`) would change six passing cases.
- A numeric check of the exported ISO pipelines for this pair failed for every candidate, so it does not identify which candidate PROJ picked. The cause is not diagnosed.

### 125: EPSG:26915 to EPSG:26715, path (1)

- Path (1) already serves it. `projinfo` lists 5 intersecting candidates: 4 grid-based `Inverse of NAD27 to NAD83 (k)` operations, and `Inverse of UTM zone 15N + Ballpark geographic offset from NAD83 to NAD27 + UTM zone 15N`. With the grid files absent, the grid candidates produce no output, and the ballpark candidate gives `569706.33 4268817.68`, which is what we print.
- No candidate reproduces the expected `569720.46 4268813.88`. The ballpark candidate is zero-offset in PROJ too (`createBallparkGeographicOffset`, `coordinateoperationfactory.cpp:2298-2345`), so the difference is not the ballpark itself. PROJ must select or build an operation absent from this list. The datum pivot exists in PROJ only as the fallback when no direct operation is found (`coordinateoperationfactory.cpp:4410-4418`, `createOperationsWithDatumPivot` at line 3167), and this pair has direct candidates, so the pivot may not be the explanation either. Not resolved here.
- The brief named the ours-side candidate as `Inverse of Ballpark geographic offset from NAD27 to NAD83`. The trace and `projinfo` show it is `Inverse of UTM zone 15N + Ballpark geographic offset from NAD83 to NAD27 + UTM zone 15N`, in the same direction as PROJ's name.

### 139: compound CRS, path (1)

- Path (1) serves it. `projinfo` lists 26 candidates. Candidates 1 to 12 are grid-based `MGI 1901 to ETRS89-SVN [D96] (k)` and produce no output with the grids absent. Candidate 13 is `Inverse of Slovene National Grid + Transformation from SVS2000 height to SVS2010 height (ballpark vertical transformation) + Ballpark geographic offset from MGI 1901 to ETRS89-SVN [D96] + Slovene National Grid`, zero offset, and it is the first that works. It gives `477131.509 95651.636 5.000`, which is what we print.
- Candidates 14 to 25 (`... + MGI 1901 to ETRS89-SVN [D96] (k) + ...`) are within 0.1 m of the expected `476763.303 95620.222` (candidate 19: `476763.396 95620.218`), but none is exact.
- The cause is the choice of candidate 13 over the 14 to 25 group. That choice comes from the ranking in `src/iso19111/select.rs`, which this lot does not change. Not fixed.

### Attempt log

| Target | Attempt | Result | Reverted |
|---|---|---|---|
| 124 | Remove the `projish` gate in `build_engine` so the ISO path serves PROJ-string pairs | #124 still open (now `*`), #6, #20, #21, #69, #79, #92 reopened | yes |
| 125 | none: path (1) already serves it, and no candidate reproduces the expected value | open | - |
| 139 | none: the cause is the ranking in `src/iso19111/select.rs`, out of scope | open | - |

The ledger `docs/UNPORTED-CLI.md` is unchanged from `efa913c`, and the three targets stay open.
