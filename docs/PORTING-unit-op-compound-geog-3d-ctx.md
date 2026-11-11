# Port: compound CRS to geographic 3D CRS, with a database context (lot `unit-op-compound-geog-3d-ctx`)

Status: **tests written, no code changed.** The tests are not run by the author: the coordinator
runs them. The lot's open cases are recorded below with the reference output that a probe must
confirm. `cargo check -p proj-rs --tests` succeeded on the last commit (devhub job 3937).

Source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp`:

| `TEST()` | Lines | Note |
|---|---|---|
| `compoundCRS_to_geogCRS_3D_context` | 7594-7812 | five blocks |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` | 8105-8155 | 16 candidates |
| `WGS84_G1762_to_compoundCRS_with_bound_vertCRS` | 4165-4191 | at least 53 candidates |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | 5032-5119 | one named candidate |
| `geog3DCRS_to_geog2DCRS_plus_vertCRS_context` | 1053-1097 | inside `#if no_longer_work_since_epsg_13_001`: not compiled by PROJ 9.9.0 |

The factory is `createOperationsCompoundToGeog`, `coordinateoperationfactory.cpp` 6454 onwards
(the function map of `docs/PORTING-unit-compound-derived.md` covers it; its start line was checked).

## The fifth `TEST()`: nothing to port

`geog3DCRS_to_geog2DCRS_plus_vertCRS_context` sits between `#if no_longer_work_since_epsg_13_001`
and `#endif` (test_operationfactory.cpp 1053 and 1097). The 9.9.0 build does not compile it, so
no assertion of it is active. Its body names EPSG:4937 and EPSG:8360, a pair that changed with
EPSG 13.001 (see also `docs/PORTING-opfac-core-2.md`). No Rust test is written for it.

## Tests written (`tests/unit_op_compound_geog3d.rs`, `tests/unit_op_compound_geog_3d_ctx.rs`)

| PROJ `TEST()` | Rust test | Expected from |
|---|---|---|
| `compoundCRS_to_geogCRS_3D_context` (5 blocks) | `unit_op_compound_geog3d::operation_compound_to_geog_crs_3d_context` | C++ 7594-7812 |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` | `unit_op_compound_geog_3d_ctx::operation_compound_of_proj_crs_to_geog_crs_3d_context` | C++ 8105-8155 |
| `WGS84_G1762_to_compoundCRS_with_bound_vertCRS` | `unit_op_compound_geog_3d_ctx::operation_wgs84_g1762_to_compound_crs_with_bound_vert_crs` | C++ 4165-4191 |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | `unit_op_compound_geog_3d_ctx::operation_compound_with_bound_geog_crs_and_geoid_to_geod_crs_nad2011_ctxt` | C++ 5032-5119 |

No test is `#[ignore]`. The earlier file `tests/unit_op_compound_geog3d.rs` held only block 1 (name),
under the name `operation_compound_to_geog_crs_3d_context_name`; it is replaced by the full `TEST()`.
Block 1 keeps its name assertion and adds the PROJ string.

Contexts: the blocks of `compoundCRS_to_geogCRS_3D_context` use the EPSG authority with
`IGNORE_GRID_AVAILABILITY`; blocks 1 and 2 keep the default `STRICT_CONTAINMENT`, blocks 3 to 5 use
`PARTIAL_INTERSECTION` (C++ 7599-7720 and 7723-7810). Block 5 and the two other tests use the
authority `""` (`std::string()` in the C++ file) or the EPSG authority as the C++ does.

PROJ strings: blocks 1 to 5 of `compoundCRS_to_geogCRS_3D_context` are written with the database
(`PROJStringFormatter::create(PROJ_5, db)` in C++, `export_to_proj_string_with_database` here). The
`of_projCRS`, `WGS84_G1762` and bound geoid tests use the formatter without database
(`PROJStringFormatter::create()` in C++, `proj_without_db` here, i.e. `export_to_proj_string`).

## Reference (PROJ 9.9.0), checked by the author

Command forms (`ref.ps1 projinfo`, `REF_GRIDS=none`): `--authority EPSG`, `--grid-check none`, and
`--spatial-test intersects` for the partial criterion. A `--spatial-test` option is absent for the
strict criterion. The grid-check flag stands for `IGNORE_GRID_AVAILABILITY`.

| Test, block | Command | PROJ result |
|---|---|---|
| block 1 | `-s EPSG:7406 -t EPSG:4979` (strict) | 3 candidates; `NAD27 to WGS 84 (79) + Transformation from NGVD29 height (ftUS) to WGS 84 (ballpark ...)` first |
| block 2 | `-s EPSG:5500 -t EPSG:4979` (strict) | 12 candidates; the second is `Inverse of NAD83(NSRS2007) to NAVD88 height (1) + NAD83(NSRS2007) to WGS 84 (1)`, the one the C++ expects |
| block 3 | `-s EPSG:4269+5703 -t EPSG:4979 --spatial-test intersects` | 629 candidates; first is the 5-step `NAD83 to NAD83(HARN) (47) + ...`, as C++ |
| block 4 | `-s EPSG:6318+5703 -t EPSG:4985 --spatial-test intersects` | 183 candidates; first is `Inverse of NAD83(2011) to NAVD88 height (3) + NAD83(2011) to WGS 84 (1) + Inverse of WGS 72 to WGS 84 (2)`, as C++ |
| block 5 | `-s EPSG:6318+5703 -t EPSG:4979 --spatial-test intersects` | 276 candidates; `us_noaa_g2018u0.tif` in 3 and `mx_inegi_ggm10.tif` in 6, as C++ asserts |
| `of_projCRS` | `-s EPSG:2225+6360 -t EPSG:4269 --3d --spatial-test intersects` | **16** candidates, as C++ (`--3d` promotes the target) |
| `WGS84_G1762` | `-s EPSG:7665 -t "+proj=longlat +datum=NAD83 +geoidgrids=@foo.gtx +type=crs"` | **53** candidates; the first PROJ string is the 4-step pipeline of C++ 4185-4190 |
| `boundGeog_geoid` | `-s @<wkt file> -t EPSG:6317 --authority ""` (the WKT in the scratch directory, read as `@/mnt/c/...`) | 6 candidates; the first name is the one the C++ looks for |

The reference counts and names agree with the C++ expectations for every pair. The PROJ strings
were not compared beyond the first candidates listed above (the C++ strings are the expected values).

## Port state (not measured by the author)

The port cannot be run by the author. The earlier sheets of this tree record the state of these
pairs, from the same code and their drafts:

- `docs/PORTING-unit-compound-derived.md`, "Tests": block 1 of `compoundCRS_to_geogCRS_3D_context`
  passes; block 2 is open (`NAD83(NSRS2007) + NAVD88` to `WGS 84`, default criterion: "the top-level
  filter finding"); `compoundCRS_of_projCRS_to_geogCRS_3D_context` is open with 17 candidates instead
  of 16 (two ballpark-pair candidates `NAD83 -> NAD83(2011)` and `-> NAD83(NSRS2007)` extra, and the
  `NAD83 -> NAD83(NSRS2007)` leg set missing); `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt`
  is open because the first leg is a nested concatenation that `compute_metadata` does not flatten
  beyond one level.
- `docs/PORTING-opfac-core-2.md` (older) records the `WGS84_G1762` test as open: the ensemble hub is
  not applied to this compound.

These are hypotheses until the probe below runs on this tree. Both the 17-versus-16 count and the
nested concatenation are candidates for a cause; neither is confirmed.

## Probe requested from the coordinator

`C:\Users\maxim\AppData\Local\Temp\opencode\port-scratch\unit-op-compound-geog-3d-ctx\probe_compound_geog3d.rs`
prints, for each pair above, the candidate count, the first three names, whether the expected name is
present, and the PROJ string of that candidate. Run:

The probe is not in the tree: copy it to `tests/probe_compound_geog3d.rs` in the worktree, run
`cargo test -p proj-rs --test probe_compound_geog3d -- --nocapture --test-threads=1`, and delete the copy.
The new tests run with `cargo test -p proj-rs --test unit_op_compound_geog3d --test unit_op_compound_geog_3d_ctx`.

## Deviations and open items

- No code change. `compound.rs` and `factory.rs` are untouched in this lot.
- `geog3DCRS_to_geog2DCRS_plus_vertCRS_context`: not compiled by PROJ 9.9.0, nothing to port.
- Callers at risk, if the cause is in shared code: `compound_to_geog3d`, `compound_horizontal_vertical`,
  `bound_compound_to_geog3d`, `compute_metadata` (`compound.rs`), and every test of
  `tests/unit_op_compound_to_geog_2.rs`, `tests/unit_op_compound_derived.rs`, `tests/unit_opfactory_core_b.rs`.

Tests written, not run by the author: the coordinator runs them.

## Commands run

- `ref.ps1 projinfo` with the forms of the table above, `-o PROJ` on block 5 and the `of_projCRS` pair.
- `cargo fmt`; `node S:/Helix_Aero/lab/helix-cockpit/tools/devhub.mjs run --kind build -- cargo check -p proj-rs --tests` (job 3937, succeeded).
