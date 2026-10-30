# Port notes — PROJ unit and CLI tests

PROJ 9.9.0 sources: `test/unit` and `test/cli`. The Rust tests call `Operation` and `Proj`. They do not shell out to a libproj install, and they do not download the PROJ CDN (`PROJ_NETWORK` is forced off for the CLI runner).

## CLI

`tests/cli_yaml.rs` reads the vendored copies of `test_cct.yaml`, `test_cs2cs_various.yaml`, `test_projinfo.yaml`, and `test_proj.yaml` and runs `cct` and `cs2cs`. Comparison is numeric, line by line. A geographic line that PROJ prints as latitude then longitude is accepted when this `cs2cs` printed longitude then latitude, which is the order `Proj::new_known_crs` uses.

A case is not executed, and is listed in `docs/UNPORTED-CLI.md`, when:

- the executable is `proj`, `geod`, `gie`, or `projsync` (no such binary);
- it needs a flag, an input file, DMS input, a third axis, `+pm` / `+geoc`, an authority other than `EPSG:`, grad CRS `EPSG:4807`, stderr text, or a non-zero status.

`test_proj.yaml` is entirely in that list.

## Unit tests ported as Rust tests

| PROJ file | Rust |
|---|---|
| `pj_phi2_test.cpp` | `projmath::tests::pj_phi2_basic` |
| `proj_angular_io_test.cpp` | `angular_units_*` in `tests/proj_unit.rs` |
| `proj_context_test.cpp` | `proj_context_*`. `+init=` expands an EPSG code. A dictionary file finder and a CA-bundle path are not a public API; those cases check the error and that network stays off. |
| `proj_errno_string_test.cpp` | `proj_errno_string_no_error_and_known_failures`. This crate has no integer errno. Success is `Ok`. A bad string and an unknown `+proj` are create errors, not a coordinate. |
| `test_grids.cpp` | Null grids are an identity shift. The NTv2-subset GeoTIFF points match `gridshift.gie`. Pixel iterators and TIFF metadata are not on `Operation`. |
| `test_defmodel.cpp` | Invalid JSON is a create error. Time-function scales from the C++ evaluator are checked as degree shifts through `+proj=defmodel` on `simple_model_degree_3d_grid.tif`. |
| `test_tinshift.cpp` | The triangle cases (horizontal, vertical, both, rejected fallback, empty nearest-side) go through `+proj=tinshift`. |
| `test_polyhedral.cpp` | Round-trip, origin, rejected net, known nets, and `x_0`/`y_0`. |
| `test_tmerczoned.cpp` | `EPSG:32600` and `EPSG:32700` for all 60 zones. |
| `test_misc.cpp` | `ProjBuilder::lib_info` is 9.9.0, and the `PROJ_AT_LEAST_VERSION` inequalities. |
| `test_util.cpp` | `NameFactory` is the ISO 19111 name layer, which is not built. `"foo"` and `"bar/foo"` are create errors, not coordinates. |

From `test_c_api.cpp`, the cases that build a projection or pipeline this crate already had:

- `proj_create` — unknown text and `EPSG:i_do_not_exist` fail; `EPSG:4326` builds. The geodesic distance inside that test is not on `Operation`.
- `check_coord_op_obj_can_be_used_with_proj_trans` and `proj_create_projections` — UTM zone 31 at 3°E is easting 500000; zone 0 is rejected; zone 1 builds. The generated `proj_create_conversion_*` constructors are not a `+proj` string.
- `proj_normalize_for_visualization` and `proj_create_crs_to_crs_from_pj` — the degree pipeline into UTM zone 31, and `Proj::create_crs_to_crs_from_pj`. A non-empty option such as `ACCURACY=0.05` is rejected.
- `proj_trans_bounds_densify_0`, `proj_trans_bounds_densify_100`, and `proj_trans_bounds_normalized` — Lambert azimuthal equal area via `Proj::transform_bounds`. This API is already longitude, latitude, so the normalized case is the same box.
- `proj_trans_bounds_3d_geog3D_to_geocentric` — the eight corners through `+proj=cart`.
- `proj_coordoperation_requires_per_coordinate_input_time` — a Helmert rate uses the epoch, a plain Helmert does not, deformation with `+t_epoch` and a NaN time fails, `+dt` does not report a missing time, and defmodel requires a finite time.
- `proj_trans_bounds_*` (17 of the 22 `TEST()`s; the other five are blocked, see `docs/PORTING-c-api-unit.md`) — `Proj::transform_bounds_in` and `Proj::transform_bounds_3d`, the box walk of `trans_bounds.cpp`. The authority-order cases are read in the crate's longitude, latitude order; the expected numbers are the same.

Every other `TEST()` name in `test_crs.cpp`, `test_factory.cpp`, `test_io.cpp`, `test_operation.cpp`, `test_operationfactory.cpp`, and `test_c_api.cpp` is in `docs/UNPORTED-UNIT.md`. Those tests assert WKT, PROJJSON, database identity, or ISO 19111 object structure. They do not become a coordinate, and this crate does not write WKT.

## Small library changes the tests needed

- `+proj=tmerczoned` (`src/projections/tmerczoned.cpp`). Method EPSG:9824, including initial longitude (8830) and zone width (8831). The inner transverse Mercator keeps a zero origin latitude, as `pj_tmerc` does on a fresh `PJ`.
- A degree parameter is emitted as the database value. Multiplying by the stored π/180 factor and converting back turned −180° into −179.99999999999983°, which put the antimeridian in zone 61.
- `+init=epsg:CODE` is expanded before the `+proj` name is required, so an init string is not reported as an empty `+proj`.

## Run

```
cargo test --offline --test proj_unit --test cli_yaml --lib pj_phi2_basic
```
