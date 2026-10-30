# Porting sheet: `proj_trans_bounds` family (test_c_api.cpp)

PROJ 9.9.0 sources: `src/trans_bounds.cpp` (`proj_trans_bounds`,
`proj_trans_bounds_3D` and their helpers) and `test/unit/test_c_api.cpp`.

This lot takes the `proj_trans_bounds*` `TEST()`s that are open in
`docs/UNPORTED-UNIT.md`. Of the 22 open ones, 17 pass here. The other 5 stay
open, with the causes below.

## API mapping

| PROJ | Rust |
|---|---|
| `proj_trans_bounds(P, PJ_FWD, ...)` | `Proj::transform_bounds` (unchanged signature), or `Proj::transform_bounds_in(Direction::Forward, ...)` |
| `proj_trans_bounds(P, PJ_INV, ...)` | `Proj::transform_bounds_in(Direction::Inverse, ...)` |
| `proj_trans_bounds_3D(P, dir, ...)` | `Proj::transform_bounds_3d(direction, [xmin, ymin, zmin], [xmax, ymax, zmax], densify)` |
| `PJ_FWD` / `PJ_INV` | `Direction::Forward` / `Direction::Inverse` (public enum in `src/proj.rs`) |
| `densify_pts` as `int` | `i32`, range `0..=10000` checked |
| `out_*` pointers, return `1` / `0` | `Result<[F; 4], ProjError>` or `Result<([f64; 3], [f64; 3]), ProjError>` |
| `antimeridian_min/max`, `simple_min/max`, `find_previous_index` | private functions in `src/proj/bounds.rs`, same algorithm |
| `contains_north_pole` / `contains_south_pole` | `pole_in_box` in `src/proj/bounds.rs` |
| `target_crs_lon_lat_order` | `Proj::axis_order` (`src/proj/bounds.rs`) |
| `proj_degree_input/output` | `Proj::degree_output(direction)`; input is the other direction |
| `pj_info.id == "noop"` | `Proj::is_noop` |

The walk is the one of `trans_bounds.cpp`: densify the four edges, transform
them, take the extremes (antimeridian and pole cases handled), then sample the
interior rows when the box does not cross the antimeridian. The 2D call walks
one height plane; the 3D call walks the two planes `zmin` and `zmax`.

## Test mapping

| PROJ `TEST()` | Rust test in `tests/test_c_api.rs` |
|---|---|
| `proj_trans_bounds_antimeridian_xy` | `c_api_proj_trans_bounds_antimeridian_xy` |
| `proj_trans_bounds_antimeridian` | `c_api_proj_trans_bounds_antimeridian` |
| `proj_trans_bounds_beyond_global_bounds` | `c_api_proj_trans_bounds_beyond_global_bounds` |
| `proj_trans_bounds_noop_geographic` | `c_api_proj_trans_bounds_noop_geographic` |
| `proj_trans_bounds__north_pole_xy` | `c_api_proj_trans_bounds_north_pole_xy` |
| `proj_trans_bounds__north_pole` | `c_api_proj_trans_bounds_north_pole` |
| `proj_trans_bounds__south_pole_xy` | `c_api_proj_trans_bounds_south_pole_xy` |
| `proj_trans_bounds__south_pole` | `c_api_proj_trans_bounds_south_pole` |
| `proj_trans_bounds_from_pipeline_lon_lat_to_x_y` | `c_api_proj_trans_bounds_from_pipeline_lon_lat_to_x_y` |
| `proj_trans_bounds_from_pipeline_lat_lon_to_x_y` | `c_api_proj_trans_bounds_from_pipeline_lat_lon_to_x_y` |
| `proj_trans_bounds_from_pipeline_x_y_to_lon_lat` | `c_api_proj_trans_bounds_from_pipeline_x_y_to_lon_lat` |
| `proj_trans_bounds_from_pipeline_x_y_to_lat_lon` | `c_api_proj_trans_bounds_from_pipeline_x_y_to_lat_lon` |
| `proj_trans_bounds_3d_densify_0_geog3D_to_proj2D` | `c_api_proj_trans_bounds_3d_densify_0_geog3d_to_proj2d` |
| `proj_trans_bounds_3d_noop` | `c_api_proj_trans_bounds_3d_noop` |
| `proj_trans_bounds_3d_inverse_of_geog3D_to_geocentric` | `c_api_proj_trans_bounds_3d_inverse_of_geog3d_to_geocentric` |
| `proj_trans_bounds_3d_geocentric_to_geog3D` | `c_api_proj_trans_bounds_3d_geocentric_to_geog3d` |
| `proj_trans_bounds_3d_geocentric_to_geog2D_lon_lat_ordered` | `c_api_proj_trans_bounds_3d_geocentric_to_geog2d_lon_lat_ordered` |

Rust names are lower case because `clippy -D warnings` rejects `3D` in a
function name. The assertions and tolerances are PROJ's, unchanged.

The `proj_trans_bounds_densify_0`, `_densify_100`, `_normalized`, and
`_3d_geog3D_to_geocentric` tests were ported before this lot, in
`tests/proj_unit.rs` and `tests/test_c_api.rs`.

## Blocked: 5 `TEST()`s stay open

These are not written as Rust tests: a test that cannot pass would be red,
and `#[ignore]` is not used. Each cause was checked with
`Proj::new_known_crs` against the crate:

| `TEST()` | Cause |
|---|---|
| `proj_trans_bounds_ignore_inf` | `OGC:CRS84` is not read: `unknown CRS` (only `EPSG:` codes and PROJ strings are). |
| `proj_trans_bounds_ignore_inf_geographic` | The inline `PROJCS[...]` WKT is not read by `Proj::new_known_crs`: `unknown CRS`. |
| `proj_trans_bounds_to_compound_crs` | `EPSG:9707` is a compound CRS, which `proj.db` loading does not build: `EPSG:9707 (compound) is not built`. |
| `proj_trans_bounds_world_geodetic_to_spilhaus` | `OGC:CRS84` and `ESRI:54099` are not read: `unknown CRS`. |
| `proj_trans_bounds_3D_ignore_inf` | `OGC:CRS84` and `ESRI:102036` are not read: `unknown CRS`. |

They need the `OGC:` and `ESRI:` authorities or inline WKT, which are part of
the ISO 19111 CRS work (`src/iso19111/`), not of this lot.

## Deliberate differences

- **Axis order.** A `Proj` built from a CRS pair (`new_known_crs`,
  `create_crs_to_crs_from_pj`) is always longitude, latitude (geographic) and
  easting, northing (projected). The PROJ tests that pass authority order
  (EPSG:4167, 32661, 32761, 4979, 4978 with lat first) therefore pass the same
  numbers in longitude, latitude order. For example, `proj_trans_bounds_antimeridian`
  passes `(160.6, -55.95, -171.2, -25.88)`, where PROJ passes
  `(-55.95, 160.6, -25.88, -171.2)`. The expected values are the same, and
  the output is read back in the same order. A bare PROJ pipeline has no CRS, so
  its order comes from its text, as in PROJ: a trailing `axisswap +order=2,1`
  means latitude first.
- **No error for a point without an image.** As in PROJ, a boundary or interior
  point whose transform fails is infinite and is skipped. The earlier
  `Proj::transform_bounds` returned `Err` on such a point. It now returns the
  PROJ result, which can be infinite when no point has an image.
- **Identity.** A CRS pair with the same name on both sides, or `+proj=noop`,
  returns the input unchanged, as PROJ's `noop` operation does. This matters
  for the exact-equality 3D case.
- **Error text.** The `densify_pts` and latitude messages are PROJ's. They are
  reported as `ProjError::Conversion`.

## Preserved quirks

- **Height planes swap back and forth.** In the planar 3D walk, PROJ swaps the
  output axes back to native order after each height plane, then swaps the
  boundary arrays again on the next plane. A geographic output with latitude
  first can therefore mix its two orders across planes. This port does the same.
  No test covers it, so nothing here depends on it.
- **Crossing flag.** `crossed_antimeridian` is computed after the extremes of
  the plane are merged into the running result, as in PROJ.
- **Starting extremes.** A 2D box starts at `±inf`, a 3D box at `DBL_MAX` and
  `-DBL_MAX`, as PROJ does. With every point failing, a 3D maximum is `+inf`
  (PROJ's `simple_max` returns `HUGE_VAL`), and a 3D minimum is `DBL_MAX`. No test covers the all-failing case.

## Verification

- `cargo test --all-features --test test_c_api trans_bounds`: 17 passed, 0 failed.
- `cargo test --all-features --test proj_unit`: 36 passed (including
  `c_api_laea_bounds`, which goes through the new walk).
- `cargo fmt --check`: clean.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- Full suite (`PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`): exit 0, every binary `test result: ok`.
- The `cli_yaml` harness rewrites `docs/UNPORTED-CLI.md` on every run. It is restored and not committed.
