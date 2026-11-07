# Port — projinfo vertical and geoid candidate lists (lot C)

Scope: the candidate-operation text of `projinfo` for vertical and geoid pairs. Six yaml cases
of `tests/cli_yaml/test_projinfo.yaml` close (#26, #29, #30, #32, #86, #108). Case #84 stays open
(epochs, see below). No `cs2cs` output changes (the `cs2cs` yaml stays at 0 open).

Sources, PROJ 9.9.0 (`scratchpad/grid/proj-git`, tag 9.9.0):

- `src/iso19111/operation/coordinateoperationfactory.cpp`:
  `createHorizVerticalHorizPROJBased` (2795-2880, the extent at 2851-2864),
  `createPROJBased` (2633), `useCompatibleTransformationsForSameSourceTarget` (6338-6387),
  the datum set of the same loop (7080-7084), the candidate loop (7114-7139),
  `getInterpolationGeogCRS` (6392-6450, demotion at 6436-6447),
  `createOperationsVertToVert` ballpark (6029-6037, `sameExtent` at 6029-6032),
  `createOperationsVertToGeogBallpark` (6144-6147), the ballpark geographic offset (2317-2320),
  `cacheHorizToInterpAndInterpToTarget` (6929-6930, 7030-7044)
- `src/iso19111/operation/oputils.cpp`: `getExtent` (464-484, 529-555)
- `src/iso19111/operation/concatenatedoperation.cpp`: `createComputeMetadata` (674-767)
- `src/iso19111/metadata.cpp`: `Extent::intersection` (889-907), `Extent::_isEquivalentTo`
- `src/iso19111/crs.cpp`: `CRS::promoteTo3D` with an axis (1341-1470)
- `src/apps/projinfo_lib.cpp`: `outputOperationSummary` (898-955), `outputOperations` (989-1160:
  the `CoordinateMetadata` branch at 1085-1104 and the `--s_epoch` handling at 1018-1050)

## Causes and fixes (one per commit)

| commit | PROJ | Rust | cases |
|---|---|---|---|
| `e41ecbc` the PROJ-based horizontal-vertical-horizontal operation keeps the extent of its steps | `createHorizVerticalHorizPROJBased` sets `DOMAIN_OF_VALIDITY` to `getExtent(ops, false)` (2851-2864) | `proj_based_horiz_vertical_horiz` (compound.rs) sets `identified.domains` from `joint_extent(&steps)`, the same intersection as `createComputeMetadata` | #26, #29, #30, #108 |
| `6bbea1b` the geographic/geocentric conversion is not a transformation step | `useCompatibleTransformationsForSameSourceTarget` and the datum set loop only take `dynamic_cast<Transformation*>` steps; the geocentric conversion (EPSG:9602) is a `Conversion` there | `is_transformation_step` (compound.rs) excludes method 9602; the port keeps that conversion as a `Transformation` variant | #86 |
| `a8e4b58` the ballpark domain uses extent equivalence | `sameExtent = _isEquivalentTo(...)` (6029-6032, 6144-6147, 2317-2320) | the three `domain = match (crs_extent(a), crs_extent(b))` sites use `ObjectDomain::is_equivalent_to` (description and bounding box, no scope) instead of `==` | #32 |

Each fix was measured with `cargo test --test cli_yaml` before the commit. The ledger
`docs/UNPORTED-CLI.md` lost exactly the rows named above in each commit. No case reopened.

### #86 in detail

The vertical operation `Inverse of WGS 84 (G1150) to EGM2008 height` has interpolation CRS
`WGS 84 (G1150)` (EPSG:9055). The horizontal operation from the interpolation CRS to
`WGS 84 (G1674)` is the geocentric chain through the Helmert (EPSG:1150 to 1674). PROJ pairs the
null offset with that chain. The port rejected the pair in `compatible_transformations` because the
geocentric conversion steps were compared by name. The pipeline then matches PROJ's first candidate
(`vgridshift`, `cart`, `helmert`, `inv cart`).

## Deliberate differences and open items

- **#84 (NAD83(CSRS)v7, epochs): not closed.** `projinfo` parses `--s_epoch` and `--t_epoch`
  and drops them (`src/bin/projinfo.rs`, the skip list). PROJ turns each into a `CoordinateMetadata`
  (projinfo_lib.cpp:1085-1104) and runs `createOperationsGeodToGeod` with the epoch branch
  (coordinateoperationfactory.cpp:3226-3300 and the point motion branch at 4278-4310). The port has
  only `point_motion_between_epochs` (factory.rs), which returns the bare velocity operation, not the
  `Null (2D to 3D) + velocity + Null (3D to 2D)` chain. A trial wiring of the epochs into `projinfo`
  (through `create_operations_between_metadata`, with `CoordinateMetadata` re-exported) gave
  `Canada velocity grid v7` plus a ballpark instead of the PROJ chain. It was reverted: the output
  was still not PROJ's, and the epoch branch is the piece that needs porting.
- **Interpolation CRS identity: not in this lot.** PROJ's `getInterpolationGeogCRS` demotes the
  interpolation CRS with the database, and `promoteTo3D(std::string(), dbContext, axis)` looks up
  the same-name 3D CRS. With both, the interpolation CRS keeps its identifier, the candidate
  lists of the vertical pairs change as PROJ's do (for example `EPSG:4269+5703` to `EPSG:4326+3855`
  gets PROJ's second candidate, `NAD83 to WGS 84 (1)`), and PROJ's `cacheHorizToInterpAndInterpToTarget`
  applies. In the port this made `projinfo` run for more than 400 s (debug build) on that pair,
  against about 28 s for the baseline (the committed branch: 28 s, the same). No yaml case needs
  it, #86 closes without it. The change is
  kept as a patch outside the lot (`scratchpad/projinfo-c/wip-identity-cache.patch`) and as the local
  branch `backup-projinfo-vertical-464f372`, which carries it together with the three fixes. The
  cause of the slowdown is in `datum_based_intermediate_operations` (factory.rs), reached from
  `compound_vertical_pass` for each interpolation candidate; PROJ's cache is not enough on its own.
- **Remarks of PROJ-based operations** (`getRemarks`, coordinateoperationfactory.cpp:2668) are not
  ported. They do not appear in the summary or in `-o PROJ`.
- **Accuracy of the horizontal-vertical-horizontal operation** uses the sum of the steps
  (`accuracy_sum`), where PROJ uses `getAccuracy(opsForAccuracy)` (2872). Not visible in the yaml.
- Pre-existing differences from PROJ, present in the baseline too, not in the yaml and not changed
  by this lot (see the table): `x5`, `x6`, `x7`, `x8`, `x9`, `x12` (summary), `c2` (`-o PROJ`),
  and the `cs2cs` pair `EPSG:4979 -> EPSG:10645`.

## Oracle table

Built from `projinfo`/`cs2cs` of PROJ 9.9.0 (`scratchpad/reftest/build/bin`), of the baseline
`e5957c7` (debug build from `git archive`), and of this branch (debug). Each row runs the pair with
`--summary` and with `-o PROJ` (or the pair's own arguments). "oracle == ours" compares the text.
Source: `scratchpad/projinfo-c/table2.md`, script `scratchpad/projinfo-c/table.py`.

| pair | yaml | summary: oracle == ours | summary: oracle == baseline | -o PROJ: oracle == ours | -o PROJ: oracle == baseline |
|---|---|---|---|---|---|
| 26 NAD83(2011)+NAVD88 -> NAD83(2011) | #26 | yes | no | yes | no |
| 29 WGS 84 -> WGS 84+EGM96 | #29 | yes | no | yes | no |
| 30 WGS 84+EGM96 -> WGS 84 | #30 | yes | no | yes | no |
| 32 UTM 31N -> 4326+3855 (3D) | #32 | yes | no | no | no |
| 86 G1150+EGM2008 -> G1674 (3D) | #86 | yes | no | yes | no |
| 108 EPSG:4979 -> 10645 Saba | #108 | yes | no | yes | no |
| 84 NAD83(CSRS)v7 epochs | #84 | no | no | no | no |
| x1 EPSG:4979 -> EPSG:9707 | - | yes | no | yes | no |
| x2 EPSG:4979 -> EPSG:9518 | - | yes | no | yes | no |
| x3 EPSG:4326 -> EPSG:4326+5773 | - | yes | yes | yes | yes |
| x4 EPSG:4326+3855 -> EPSG:4326 | - | yes | yes | yes | yes |
| x5 EPSG:4979 -> EPSG:3855 | - | no | no | no | no |
| x6 EPSG:4326 -> EPSG:3855 | - | no | no | no | no |
| x7 EPSG:5773 -> EPSG:3855 | - | no | no | no | no |
| x8 EPSG:4269+5703 -> EPSG:4326+3855 | - | no | no | no | no |
| x9 EPSG:4326+5773 -> EPSG:4326+3855 | - | no | no | no | no |
| x10 EPSG:4979 -> EPSG:4326+5773 | - | yes | no | yes | no |
| x11 NAD83(2011)+NAVD88 -> NAD83(2011) | - | yes | yes | yes | yes |
| x12 EPSG:4326+5703 -> EPSG:4326+5773 | - | no | no | no | no |
| c1 EPSG:4326 -> EPSG:4258 | - | yes | yes | yes | yes |
| c2 EPSG:4326 -> EPSG:3857 | - | yes | yes | no | no |

`cs2cs` (the same binaries, the same ten pairs, a fixed set of three input points):

| cs2cs pair | oracle == ours | oracle == baseline | ours == baseline |
|---|---|---|---|
| EPSG:4979 -> EPSG:10645 | no | no | yes |
| EPSG:4326 -> EPSG:9707 | yes | yes | yes |
| EPSG:4979 -> EPSG:9707 | yes | yes | yes |
| EPSG:4326 -> EPSG:4326+5773 | yes | yes | yes |
| EPSG:4326+5773 -> EPSG:4326+3855 | yes | yes | yes |
| EPSG:4326+3855 -> EPSG:4326 | yes | yes | yes |
| EPSG:6349 -> EPSG:6319 | yes | yes | yes |
| EPSG:32631 -> EPSG:4326+3855 | yes | yes | yes |
| EPSG:4269+5703 -> EPSG:4326+3855 | yes | yes | yes |
| EPSG:4326 -> EPSG:4258 | yes | yes | yes |

Reading: the six closed cases match PROJ in summary and in `-o PROJ`; `cs2cs` output is identical to
the baseline on all ten pairs. The rows marked "no" in both baseline and ours are gaps that
predate the lot.

## Commands

```bash
cargo test --test cli_yaml                      # per commit: ledger and regressions
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<PROJ 9.9.0 bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
# oracle comparison (table2.md)
python3 scratchpad/projinfo-c/table.py
```
