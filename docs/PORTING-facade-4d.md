# Porting sheet: a 4-D entry point on the facade (lot `lot-facade-4d`)

Branch `lot-facade-4d`, base `origin/main` (`c57d7d2`).

## What PROJ does

`proj_trans(P, direction, coord)` takes a `PJ_COORD`, whose fourth member `xyzt.t` is the
coordinate time in decimal years (`src/4D_api.cpp`). A time-dependent step (a Helmert with
`+t_epoch`, a deformation model) reads it. `proj_trans_generic` treats a null time array as
zeroes.

## What the port did

`Proj::convert_xyz` called `map_xyz`, which built `Xyzt { x, y, z, t: 0.0 }`. The epoch of a
point could not be given through the facade, so
`CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762`
(`test/unit/test_c_api.cpp:5330-5386`, `in_coord.xyzt.t = 2010`) was open. The pipeline, the
parser and the engine were already right (`docs/PORTING-unit-capi-io.md`).

## Function map

| PROJ 9.9.0 | Rust |
|---|---|
| `proj_trans` with `PJ_COORD.xyzt`, forward | `Proj::convert_xyzt(x, y, z, t) -> Result<(f64, f64, f64, f64), ProjError>` (`src/proj.rs`) |
| the shared body | `Proj::map_xyzt`; `map_xyz` is `map_xyzt` at `t = 0.0` |

## Tests

| PROJ | Rust test | File |
|---|---|---|
| `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762` | `capi_proj_create_crs_to_crs_custom_compound_nad83_2011_to_wgs84_g1762` | `tests/unit_capi_custom_compound_geoid.rs` |
| `test/gie/more_builtins.gie:405-416` (Helmert with `t_epoch=1988.0`, t = 2017 and t = 2018) | `helmert_with_epoch_takes_the_time_of_the_point_2017`, `..._2018` | `tests/facade_convert_xyzt.rs` |
| same block, `t = 0` | `convert_xyz_is_time_zero_and_not_the_epoch_of_the_point` | `tests/facade_convert_xyzt.rs` |

The expected values and tolerances of the `CApi` test are those of `test_c_api.cpp:5383-5385`
(`35.000003665064803`, `-118.00001414221214` at 1e-9, `-32.8110` at 1e-3). Those of the Helmert
tests are the `expect` lines of the gie block, at its tolerance of 0.1 mm.

## Deliberate deviations and what stays open

- `convert_xyz` keeps `t = 0`, as `proj_trans_generic` does for a null time array.
  `tests/proj_meta_facade.rs` `an_epoch_moves_itrf2014_to_itrf2008` pins it. The `cs2cs` CLI uses
  `HUGE_VAL` for a missing time: an existing difference, not in this lot.
- Forward only: the facade has no 3-D inverse entry point to mirror (`project_xyz` does not
  exist), so no `project_xyzt` is added.
- Not in this lot: `Proj::def` after a transformation, slices of 4-D points.

## Callers at risk

`map_xyz` (`convert_xyz` and the 3-D `Transform` paths) now goes through `map_xyzt` with
`t = 0.0`: the value passed to the engine is unchanged.

## Commands run

The tests and the public method were written by a port agent (Claude Haiku 5.5), which did not
run them. The coordinator ran, through devhub:

- `cargo check -p proj-rs --tests`: clean.
- `cargo test -p proj-rs --test facade_convert_xyzt --test unit_capi_custom_compound_geoid --test proj_meta_facade`:
  3, 2 and 10 passed, 0 failed.
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`, then the same without
  `UPDATE_DOCS`: see the commit.
- `cargo fmt --check`: clean.

Not run in this lot: `cargo clippy --all-targets --all-features -- -D warnings` and the full
`cargo test --all-features`. They are run once on the gathered lots.
