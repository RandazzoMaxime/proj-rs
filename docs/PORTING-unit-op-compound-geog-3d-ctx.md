# Port: compound CRS to geographic 3D CRS, with a database context (lot `unit-op-compound-geog-3d-ctx`)

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp` and
`src/iso19111/operation/{coordinateoperationfactory,concatenatedoperation,conversion,singleoperation}.cpp`.

## Tests closed (5)

| PROJ `TEST()` | Rust test | File |
|---|---|---|
| `compoundCRS_to_geogCRS_3D_context` (5 blocks: names, counts, PROJ strings, remarks) | `operation_compound_to_geog_crs_3d_context` | `tests/unit_op_compound_geog3d.rs` |
| `compoundCRS_of_projCRS_to_geogCRS_3D_context` (16 candidates, the NADCON5 chain) | `operation_compound_of_proj_crs_to_geog_crs_3d_context` | `tests/unit_op_compound_geog_3d_ctx.rs` |
| `WGS84_G1762_to_compoundCRS_with_bound_vertCRS` | `operation_wgs84_g1762_to_compound_crs_with_bound_vert_crs` | same |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` | `operation_compound_with_bound_geog_crs_and_geoid_to_geod_crs_nad2011_ctxt` | same |
| `ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids` | `operation_etrs89_3d_to_proj_string_with_geoidgrids_nadgrids` | same |

Expected names, counts and strings are the C++ ones. `geog3DCRS_to_geog2DCRS_plus_vertCRS_context` is
inside `#if no_longer_work_since_epsg_13_001` (test_operationfactory.cpp:1053-1097): PROJ 9.9.0 does not
compile it, nothing to port.

## What was different, and the C++ it follows

Each cause was found by comparing the candidate list and the steps of the port with PROJ 9.9.0
`projinfo` on the same pair (and, for the strings, with the steps PROJ gives to its formatter before
the simplification: a build of the same sources with a debug print in `PROJStringFormatter::toString`).

1. **Grid names of the intermediate operations** (`factory.rs`, `datum_based_intermediate_operations`,
   `crs_codes_intermediate_operations`). The operations chained through an intermediate CRS were read
   with the official grid names where the context says PROJ's (`usePROJAlternativeGridNames`,
   factory.cpp:7007). `EPSG:2225+6360` to `EPSG:4269` wrote `cnhpgn.las` and the NADCON5 `.b` files
   instead of `us_noaa_cnhpgn.tif` and `us_noaa_nadcon5_nad83_harn_nad83_fbn_conus.tif`.
2. **Bound CRS to bound CRS** (`bound_ops.rs`, `bound_to_bound`). The two operations through a shared hub
   were joined by a plain concatenation. PROJ calls `createComputeMetadata({opFirst, opLast})`
   (coordinateoperationfactory.cpp:6228-6240), which flattens the steps and removes consecutive
   inverse ones: the round trip through the hub disappears.
3. **Conversion of a compound with a bound horizontal part** (`conversion_export.rs`). The first
   component of a compound stands for its base when it is bound (conversion.cpp:3896-3900); the port
   kept the bound CRS, and the export of the projection failed (`this operation is not a projection
   of the source CRS`).
4. **`setCRSs` of the factory** (`compound.rs`, `with_endpoints`). It is
   `CoordinateOperation::setCRSsUpdateInverse` (singleoperation.cpp:240-272): on a concatenated
   operation the source also goes to its first step and the target to its last one. Without it, the
   `z` terms of the first angular-unit step were lost (`+xy_in=deg +z_in=m +xy_out=rad +z_out=m`).
5. **Own extent of a compound CRS** (`factory.rs`, `crs_extent`). `getExtentPossiblySynthetized`
   (oputils.cpp:502-525) takes the extent of the CRS itself first; the port always intersected the
   components, so `EPSG:5500` lost its registered extent and the strict filter dropped the geoid
   candidates.
6. **`getRemarks`** (`compound.rs`, `get_remarks`; coordinateoperationfactory.cpp:2668-2713). The remarks
   of the operations of a horizontal/vertical PROJ-based operation are joined, each as
   `For <name> (<authority>:<code>): <remarks>`, the `Inverse of` and `INVERSE(...)` /
   `DERIVED_FROM(...)` prefixes dropped; with two cancelling horizontal operations only the first one
   and the vertical one count (`opsForRemarks`).

## Callers at risk

`with_endpoints` (4 callers in `compound.rs`), `crs_extent` (every sort and filter), `bound_to_bound`,
the intermediate-operation reads. The full suite was run (below): no test changes.

## Still open (not in this lot)

- `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound`: the first candidate still differs
  from PROJ's. PROJ's route goes through `createOperationsCompoundToGeog` on the compound whose
  horizontal part is bound, then through the registry operations of the hub; the port still uses an
  approximation (`bound_compound_to_geog3d`, "PROJ's consistency filter between several pairs is not
  ported"). Porting that function whole is the next step; an attempt in an earlier lot reopened eight
  tests and was reverted.
- `compoundCRS_to_compoundCRS_context`: the registry vertical candidate with the VERTCON grid is not
  built.
- The two other open rows of the string-export family
  (`boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`, `nadgrids_with_pm`, WKT round
  trip) are in `docs/PORTING-unit-op-string-exports.md`.

## Commands run

On a Windows workstation, without the reference `cs2cs` on the PATH:

- `cargo fmt --check`, `cargo clippy -p proj-rs --all-targets --all-features -- -D warnings`: clean.
- `cargo test -p proj-rs --test unit_op_compound_geog3d --test unit_op_compound_geog_3d_ctx`: 1 and 4 passed.
- `cargo test -p proj-rs --all-features --no-fail-fast`: 1704 passed, 11 failed, 28 ignored. The same 11
  tests fail on `main` on that machine (its `PROJ_DATA` points at an OSGeo4W install with real grids;
  cause not confirmed by a second run); no test fails on this branch only. CI is the reference run.
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`, then without `UPDATE_DOCS`.

The tests were written by a port agent (Claude Haiku 5.5) that could not run them; the causes 1 to 6 were
found and fixed by the coordinator session.
