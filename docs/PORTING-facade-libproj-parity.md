# Port: the georust facade follows libproj's `proj_create_crs_to_crs`

Status: **landed, one gap open**. `Proj::new_known_crs`, `Proj::new` of a pair, and
`create_crs_to_crs_from_pj` now choose the candidates and transform as PROJ 9.9.0 does with
`proj_create_crs_to_crs` (no options, so the default warning policy) and `proj_trans`, with the
georust axis order. On the 40 pairs of the table below, both grid modes and both directions,
156 of 160 cells equal the libproj georust path. The other pair (`EPSG:4326+5773` to
`EPSG:4326+5782`) is a creation error here, listed under open items. The facade has no datum-bridge
fallback any more: a pair the selection does not write is a creation error.

Reference oracle: a C program (`scratchpad/facade-parity/oracle/georust_oracle.c`) that calls
`proj_create_crs_to_crs`, `proj_normalize_for_visualization`, `proj_pj_info` and `proj_trans` of
the PROJ 9.9.0 shared library (`scratchpad/reftest/build/lib/libproj.so.25.9.9.0`), exactly the
three calls georust `proj` 0.31 makes. Two data sets: `proj.db` alone with the network off
("nogrid"), and the test grids of `data/tests` on the search path ("grid").

## What PROJ does, and where each part lives

| PROJ 9.9.0 | Rust |
|---|---|
| `proj_create_crs_to_crs`, no options: warning default, `ONLY_BEST` unset (`crs_to_crs.cpp:494-504`, `filemanager.cpp:2027`) | `select::facade_candidates`, the warning policy: the registry sorts `KNOWN_AVAILABLE` (`crs_to_crs.cpp:570-576`) |
| one candidate, instantiable: returned at once (`crs_to_crs.cpp:617-647`) | `facade_candidates`, first branch; no rerun, and the ballpark is allowed |
| the candidates prepared as `pj_create_prepared_operations` (`crs_to_crs.cpp:362-466`, `add_coord_op_to_list` 103-161): source and target boxes | `Preparer::prepare` in `select.rs`, one call per candidate, so each keeps its operation |
| the rerun with `DISCARD_OPERATION_IF_MISSING_GRID` when no candidate is both instantiable and free of a ballpark step; the first list's non-ballpark candidates, then the rerun's (`crs_to_crs.cpp:668-751`) | `facade_candidates`, the `discarded` block |
| more than one candidate: `alternativeCoordinateOperations`, chosen per point (`crs_to_crs.cpp:776-795`, `trans.cpp:233-346`) | `Route::Alternatives` in `proj_iso.rs`, `Alternatives::transform` in `alternatives.rs` |
| `pj_get_suggested_operation`, forward: the source box; inverse: the target box and `dstIsLonLatDegree`, `dstIsLatLonDegree`, a geocentric target through its geographic position (`trans.cpp:47-176`) | `Alternatives::suggest`, `covers`, `box_point`, one code path for both directions |
| retries, then the first candidate that names no grid (`trans.cpp:248-342`) | `Alternatives::transform`, the same loop for both directions |
| `proj_normalize_for_visualization` switches only the CRS that `mustAxisOrderBeSwitchedForVisualization` names (`crs.cpp:869-947`) | `proj_iso::must_switch_for_visualization`; before this lot every CRS with a north or south first axis was switched (EPSG:3413) |
| `proj_pj_info` of several candidates: the only instantiable one, else `unavailable until proj_trans is called` (`info.cpp:165-191`) | `proj_iso::reported_candidate`, read by `Proj::def` |
| a same-datum geographic pair: the null offset (`createOperationsGeogToGeog`) | `proj_iso::null_offset_pipeline`, unchanged |
| a coordinate epoch on each candidate (`c_api.cpp:8580-8660`, `proj_create_crs_to_crs_from_pj`) | `proj_iso::iso_pipeline_at` writes `proj=set` into each candidate (`Item::definition`) |
| `isPROJInstantiable`: a missing grid whose name starts with `@` is optional (`singleoperation.cpp:347-354`) | `Preparer::optional_grids_serve`, set by the facade only; `cs2cs` is unchanged |

Not ported, on purpose or for now: `iCurCoordOp` (the candidate `proj_trans` used last changes what
`proj_pj_info` reports afterwards; the facade reports the list before any point), the area of
interest of the context (`cs2cs` passes none, the facade passes none), the network-enabled branch
(see open item 8).

## Changes of behaviour, and the oracle evidence

Each row is a test expectation the facade had, that libproj 9.9.0 does not have. The new value is
the oracle's (georust path). No other expectation changed.

| Test | Before | After | Oracle evidence (PROJ 9.9.0, georust path) |
|---|---|---|---|
| `tests/proj_facade.rs` `database_crs_edges_and_grid_pairs`, `EPSG:4267` to `EPSG:4269` | `expect_err`: the NADCON grid is missing and the ballpark was refused | creates; `(-74, 40)` gives `(-73.999573598275, 40.000008481659)` | nogrid: `DEF unavailable until proj_trans is called`, `OK -73.999573598275 40.000008481659`. The rerun gives the WGS 84 pivot (`crs_to_crs.cpp:668-735` names this pair, line 688-690) |
| `tests/proj_meta_facade.rs` `compound_target_creates_and_refuses_a_forward_conversion` (renamed `compound_target_takes_a_zero_height_as_libproj_does`) | `convert` refused with `ProjError::Conversion` | `(2, 49)` gives `(2, 49, 0)` | nogrid and grid: `DEF proj=noop ellps=GRS80`, `OK 2.000000000000 49.000000000000 0.000000000000` for `EPSG:4326` to `EPSG:4326+3855` |
| `tests/proj_meta_facade.rs` `compound_target_refusal_is_not_taken_by_the_inverse` (renamed `compound_target_inverse_keeps_the_horizontal_part`) | name only | the same values | unchanged |
| `tests/proj_facade.rs` `wkt_and_projjson_build_a_geographic_or_projected_crs`: the PROJJSON input | the input had no `coordinate_system`, and the facade read it through its datum bridge | the input carries its axes; the expected values are unchanged | `proj_create` of the old JSON: `proj_create: Missing "coordinate_system" key` (PROJ 9.9.0 refuses it). The complete JSON: `DEF proj=pipeline ... proj=utm zone=31`, `OK 500000.000000000990 0 0` for `(3, 0)`, the expected `500000` |

Not an expectation change, a fix with its own test: `tests/facade_libproj_parity.rs`
`polar_stereographic_3413_keeps_easting_first` (`EPSG:4326` to `EPSG:3413` at `(-45, 75)`:
`(0, -1633879.497470785)`; the facade gave `(-1633879.497, 0)`), and `ups_north_32661_is_switched_for_visualization`.

New characterization tests, values from the oracle: `tests/facade_libproj_parity.rs`
`wgs84_to_nad27_without_grid_chooses_per_point`, `nad27_to_wgs84_inverse_without_grid_chooses_per_point`,
`a_ballpark_only_pair_is_a_transformation` (Xian 1980 is a zero offset; KKJ `EPSG:2393`);
`tests/facade_libproj_grids.rs` `nad27_to_nad83_takes_the_grid_that_covers_each_point` (the
NADCON grid that covers each point, with the test grids on the search path).

## Oracle table

Facade against the georust path of PROJ 9.9.0, 40 pairs with 1 to 6 points each (points in the
georust order: longitude then latitude, or easting then northing). `equal` is a difference below
1e-6 (metres or degrees; the largest is 7e-10 on the projected pairs, the rounding of the 12
printed decimals). The inverse rows take the oracle's forward output as input, and compare the
inverse of both.

| case | nogrid fwd | nogrid inv | grid fwd | grid inv |
|---|---|---|---|---|
| nad27-nad83 | equal | equal | equal | equal |
| nad83-nad27 | equal | equal | equal | equal |
| wgs84-nad27 | equal | equal | equal | equal |
| wgs84-nad83 | equal | equal | equal | equal |
| nad83-wgs84 | equal | equal | equal | equal |
| wgs84-lambert93 | equal | equal | equal | equal |
| wgs84-utm31 | equal | equal | equal | equal |
| wgs84-laea3035 | equal | equal | equal | equal |
| wgs84-albers5070 | equal | equal | equal | equal |
| wgs84-ca6ft | equal | equal | equal | equal |
| wgs84-webmerc | equal | equal | equal | equal |
| wgs84-mercator | equal | equal | equal | equal |
| ed50-wgs84 | equal | equal | equal | equal |
| wgs84-osgb36 | equal | equal | equal | equal |
| wgs84-ntm-nl | equal | equal | equal | equal |
| rd-wgs84 | equal | equal | equal | equal |
| wgs84-lv95 | equal | equal | equal | equal |
| wgs84-belgian | equal | equal | equal | equal |
| wgs84-nztm | equal | equal | equal | equal |
| nzgd49-wgs84 | equal | equal | equal | equal |
| etrs89-wgs84 | equal | equal | equal | equal |
| wgs84-etrs89-utm | equal | equal | equal | equal |
| wgs84-utm50n | equal | equal | equal | equal |
| wgs84-xian | equal | equal | equal | equal |
| xian-wgs84 | equal | equal | equal | equal |
| wgs84-polarstereo | equal | equal | equal | equal |
| wgs84-geocentric | equal | equal | equal | equal |
| wgs84-3d-4979 | equal | equal | equal | equal |
| same-datum-4326-4258 | equal | equal | equal | equal |
| same-datum-4326-4326 | equal | equal | equal | equal |
| nad83-feet | equal | equal | equal | equal |
| ft-to-m | equal | equal | equal | equal |
| compound-4326-to-3855 | equal | equal | equal | equal |
| compound-3855-to-4326 | equal | equal | equal | equal |
| compound-5773-5782 | creation differs | creation differs | creation differs | creation differs |
| wgs84-egm96-compound | equal | equal | equal | equal |
| proj-nad27-wgs84-string | equal | equal | equal | equal |
| proj-utm-string | equal | equal | equal | equal |
| ballpark-only | equal | equal | equal | equal |
| oblique-merc-nad83 | equal | equal | equal | equal |

Summary: 156 of 160 cells equal (the largest difference among them is 1.9e-09). The
4 differing cells are one pair (`compound-5773-5782`), in both modes and both directions, all
of them a creation error on our side.

The datum bridge of the former facade (`Body::Between`, `Crs::datum_bridge`, the Helmert selection
of `crs.rs`) is removed: it was the path for a pair the selection did not write. The one pair that
used it in the tests (`test_cs2cs_various.yaml` #129, `+geoidgrids=@...`) is now a pipeline of the
selection (optional grid rule above). The facade keeps `Proj::new` of a single CRS as a check of
its definition (`crs::check_crs`), not as a CRS object.

## Open items

1. **The factory's discard rerun for a compound pair (blocks the missing-grid creation).** When the
   only candidate of a pair needs a grid that is not on disk, PROJ's warning policy reruns the
   search without the grid operations. For `EPSG:4326+5773` to `EPSG:4326+5782` the rerun list
   holds the ballpark vertical transformation (`projinfo --grid-check discard_missing`:
   `Transformation from EGM96 height to Alicante height (ballpark vertical transformation)`).
   Our rerun returns no candidate for this pair (the facade sees the one missing-grid candidate
   alone), so the facade cannot build PROJ's list, and a missing-grid single candidate keeps the
   creation error. This is the factory's work (candidates, lot `#139`), not the facade's.
   Trying the missing-grid route (`Route::Missing`, every point fails) before the optional-grid
   rule regressed `test_cs2cs_various.yaml` #129 (`+geoidgrids=@i_dont_exist.tif` to `EPSG:4979`,
   expected `49dN 2dE 0.000`): the optional grid counted as missing printed `* * inf`. The
   optional-grid rule fixed that case, and the route was then not kept, for the reason above.
2. **`EPSG:4326+5773` to `EPSG:4326+5782`.** libproj creates it: `def()` is `proj=noop ellps=GRS80`,
   and the points are the identity (oracle, both modes and both directions). The facade refuses it
   at creation, because its only candidate needs a grid. Same cause as 1.
3. **`Proj::def` after a transformation.** libproj's `proj_pj_info` reports the candidate that the
   last `proj_trans` used (`iCurCoordOp`, `info.cpp:168-171`). The facade reports the list before
   any point. Not in any test of the facade.
4. **The inverse of a single missing-grid candidate.** Not reached: the same creation error as 1.
5. **`cs2cs` path (1) is not this lot.** `select_alternatives` (`select.rs`, used by `cs2cs`)
   returns a single missing-grid candidate at once (`op_count == 1`), where `proj_create_crs_to_crs`
   reruns the search for it (`crs_to_crs.cpp:648-651`). `facade_candidates` does the rerun. The
   `cs2cs` owner decides whether `cs2cs` follows; the facade and `cs2cs` differ for such a pair today.
6. **The PROJJSON reader accepts a projected CRS without `coordinate_system`.** libproj refuses it.
   The import is outside this lot (`src/iso19111/json_read.rs`); the facade test now carries the
   axes, as libproj requires.
7. **Pivot candidates for the discard rerun in `cs2cs`** are not changed (cs2cs is not this lot).
8. **`PROJ_NETWORK` on.** libproj does not rerun the search when the network is enabled
   (`mayNeedToReRunWithDiscardMissing` is false, `crs_to_crs.cpp:612-615`, so the first list
   stands). The facade reruns in both cases. The oracle and the tests run with the network off.

## Tests added

`tests/facade_libproj_parity.rs`: `polar_stereographic_3413_keeps_easting_first`,
`ups_north_32661_is_switched_for_visualization`, `wgs84_to_nad27_without_grid_chooses_per_point`,
`nad27_to_wgs84_inverse_without_grid_chooses_per_point`, `a_ballpark_only_pair_is_a_transformation`,
`an_optional_missing_geoid_grid_is_an_identity_step`. `tests/facade_libproj_grids.rs`:
`nad27_to_nad83_takes_the_grid_that_covers_each_point`. Expected values: the oracle, not the code.

## Commands

```text
# Oracle (C, georust path): built against the PROJ 9.9.0 shared library
gcc -O1 -I<reftest>/src/src -L<reftest>/build/lib -Wl,-rpath,<reftest>/build/lib \
    georust_oracle.c -o georust_oracle -lproj -lm
PROJ_DATA=<oracle-db> PROJ_NETWORK=OFF georust_oracle EPSG:4267 EPSG:4269 < points.txt   # nogrid
PROJ_DATA=<reftest>/build/data:<repo>/data/tests PROJ_NETWORK=OFF georust_oracle ...      # grid

# Ours: the facade probe (a scratch crate on the worktree) against the same points, and the table
python3 compare.py both > compare.txt
python3 make_table.py compare.txt

# Tests
cargo test --all-features --test facade_libproj_parity --test facade_libproj_grids \
    --test proj_facade --test proj_meta_facade --test georust_parity --test agree_cs2cs
cargo test --test cli_yaml            # the cs2cs ledger: unchanged
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<oracle-bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

Totals, the full suite on `fa77029` (the code is the same as the final commit of this lot; the only
later change is this sheet): 152 test binaries, 1595 passed, 0 failed, 34 ignored, exit 0.
`cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.
