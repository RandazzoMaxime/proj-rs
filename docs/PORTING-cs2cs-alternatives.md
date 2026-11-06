# cs2cs: alternative operations and the per-point choice

Port of the alternatives of `proj_create_crs_to_crs` (PROJ 9.9.0) and of their use in `proj_trans`.
When the candidates of a pair are two or more, `cs2cs` no longer uses only the first one. Each point
takes the candidate PROJ would take for it, and falls back to the next one when the candidate fails.

Code:

- `src/iso19111/alternatives.rs`: the per-point choice (`Alternatives::forward`) and the policy
  (`BestPolicy`: `Warn` = no option, `Error` = `--only-best`, `Discard` = `--only-best=no`).
- `src/iso19111/select.rs`: `select_alternatives` builds the candidates (`Selection::Single` or
  `Selection::Several`), with the boxes of their areas of use in the source coordinates.
- `src/iso19111/geodetic_ops.rs`: `geographic_geocentric_ballpark`, the ballpark candidates between a
  geographic and a geocentric CRS on different datums.
- `src/iso19111/suggest.rs`: `priority_operation` and `area_of_use` made shared with the above.
- `src/bin/cs2cs.rs`: `build_engine` (path (1)) uses `select_alternatives`, `Engine::Alternatives`,
  `Engine::Unavailable`, `single_engine`, `policy_of`.

## PROJ file:line map

| PROJ 9.9.0 | This port |
|---|---|
| `src/trans.cpp:47-176` `pj_get_suggested_operation`: spatial test (77-147), choice (155-172) | `Alternatives::covers`, `Alternatives::suggest` |
| `src/trans.cpp:233-346` `proj_trans` with alternatives: retry (248-303), `errorIf` return (289-295), `warnIf` state (296-297), fallback on a grid-free candidate (305-345) | `Alternatives::forward` |
| `src/trans.cpp:53-64` `normalizeLongitude` | `normalize_longitude` (alternatives.rs) |
| `src/crs_to_crs.cpp:54-100` `reproject_bbox` (20 samples per side, world not reprojected) | `reproject_box` (select.rs) |
| `src/crs_to_crs.cpp:103-161` `add_coord_op_to_list` (box, antimeridian split in `pj_create_prepared_operations`, drop when a box cannot be reprojected) | `Preparer::prepare`, `reproject_box` |
| `src/crs_to_crs.cpp:174-224` `create_operation_to_geog_crs` (longitude and latitude CRS on the datum) | `lon_lat_base` (select.rs) |
| `src/crs_to_crs.cpp:268-317` `create_operation_geocentric_crs_to_geog_crs` (exactly one operation) | `geocentric_to_lon_lat` |
| `src/crs_to_crs.cpp:362-466` `pj_create_prepared_operations` | `select_alternatives` and `Preparer` |
| `src/crs_to_crs.cpp:521-531` `ONLY_BEST` option, `warnIf` / `errorIf` | `BestPolicy`, `policy_of` (cs2cs.rs) |
| `src/crs_to_crs.cpp:572-576` grid availability `KNOWN_AVAILABLE` unless `DISCARD` | `select_alternatives`, `availability` |
| `src/crs_to_crs.cpp:597-611` flags of the first operation (`skipNonInstantiable = warnIf`) | `Alternatives::new` (`warn`), `forward` |
| `src/crs_to_crs.cpp:617-647` one operation returned without preparation | `select_alternatives`, `Selection::Single` |
| `src/crs_to_crs.cpp:685-773` rerun with DISCARD when no instantiable non-ballpark candidate | `select_alternatives` (rerun branch) |
| `src/crs_to_crs.cpp:776-795` alternatives set on the returned object | `Selection::Several` |
| `src/coord_operation.cpp:49-54` `isInstantiable` | `Item::instantiable` (exportable and grids on disk) |
| `src/coord_operation.cpp:77-136` `PJCoordOperation`: `isOffshore`, `isUnknownAreaName`, `isPriorityOp`, `srcIsLonLatDegree` | `Item` fields, `priority_operation`, `degree_order` |
| `src/iso19111/operation/coordinateoperationfactory.cpp:5366-5387` geographic/geocentric through the geographic CRS of the geocentric one (PROJ 9.2+) | `geographic_geocentric_ballpark` (geodetic_ops.rs) |
| `src/proj_internal.h:804` `warnIfBestTransformationNotAvailableDefault = true` | `BestPolicy::Warn` for no option |
| `src/apps/cs2cs.cpp:974-991` option strings `ONLY_BEST=YES` / `NO` | `policy_of` |

## Behaviour

- Default (no `--only-best`): PROJ's default is the warn policy. The candidates are sorted with
  `KNOWN_AVAILABLE`, so a missing grid does not move a candidate down. A point is served by the first
  candidate whose area covers it. When that candidate fails, it is excluded and the next one is tried,
  with the instantiable ones only from then on. No message is printed (PROJ logs it at debug level).
- `--only-best` (`Error`): the candidate PROJ would choose for the point is used, or the point fails
  with PROJ's missing-grid message (printed on stderr, one line per point). No retry.
- `--only-best=no` (`Discard`): candidates whose grids are missing are not kept.
- A single candidate keeps the single-pipeline path. Under the default policy its missing grid fails
  each point silently (`Engine::Unavailable`), as PROJ does with the warn policy.

## Deviations from PROJ

1. Prime meridian. The longitude and latitude CRS of `lon_lat_base` keeps the prime meridian of the
   CRS. PROJ builds it with a zero prime meridian, which adds a longitude rotation to the boxes of a
   CRS with another meridian. Not ported: no test case has such a CRS.
2. Only geodetic and projected CRS are prepared. Any other kind (compound, bound, derived) keeps the
   previous path (`select_alternatives` returns an error, and `build_engine` falls back).
3. Ballpark candidates of a geographic and a geocentric CRS on different datums
   (`geographic_geocentric_ballpark`) are added only when the registry gave candidates. The registry
   candidates of such a pair are the ones the port already built.
4. Intermediate CRS search. PROJ's `findsOpsInRegistryWithIntermediate` runs when the direct
   candidates have too small areas (`hasTooSmallAreas`, `coordinateoperationfactory.cpp:3897-3960`,
   called at 4474). Not ported: the pivot candidates (for example `PD/83 to ETRS89 (1) + ETRS89 to WGS
   84 (1)`) are missing. See `test_cs2cs_various.yaml#132` and `#133` below.
5. Domain of a concatenated operation. The datum-pivot candidates now take the intersection of the
   areas of their legs (`pivot_domain`, `getExtent` of PROJ, `oputils.cpp:529`), as of lot
   `lonlat-source-axis` (see `docs/PORTING-lonlat-source-axis.md`). Other concatenated operations
   keep an unknown domain: `PD/83 to WGS 84 (1) + Conversion ...` still gets no area where PROJ gives
   `Germany - Thuringen`, so its box is the world. The result is the same for the cases checked,
   but not in general.
6. Destination box. PROJ uses the destination box only to drop a candidate whose box cannot be
   reprojected, and for the inverse direction. This port does the same gating for a projected target
   and does not box the destination (cs2cs is forward only).
7. A candidate without a PROJ string is kept, and fails each point (PROJ keeps a `PJ` with no
   function). Its box and its accuracy are used for the choice, as in PROJ.
8. The per-point choice reads the input as given. A geocentric source is converted with the single
   operation to longitude and latitude (`geocentric_to_lon_lat`), as PROJ does.

## Test cases (`tests/cli_yaml/test_cs2cs_various.yaml`, PROJ 9.9.0 values)

Closed by this lot (the ratchet, `cargo test --test cli_yaml`):

- `#122` (`--only-best NAD27 NAD83`, grid missing): the point fails with the stderr message of
  `NAD27 to NAD83 (7)`.
- `#134` (`EPSG:4746 EPSG:4978`, `49 2 0`, outside the BETA2007 grid): the ballpark candidate.
- `#135` (`EPSG:4978 EPSG:4746`, outside the grid): the ballpark candidate, reversed.

Changed, still open (the got values move towards PROJ's, but the candidate list is missing):

- `#132` (`EPSG:4746 EPSG:4978`, `PROJ_DATA` with no grid): PROJ takes the candidate
  `PD/83 to ETRS89 (1) + ETRS89 to WGS 84 (1)`, which comes from the intermediate search (deviation 4).
  The port's ballpark gives `4003345.54 705897.83 4898352.56`, the value of a zero offset on WGS 84.
- `#133` (reverse, grid missing): the same candidate, reversed. The port gives
  `50d29'55.695"N 9d59'55.692"E -0.002`.

Closed by lot `lonlat-source-axis` (`docs/PORTING-lonlat-source-axis.md`):

- `#124` (`+proj=latlong +datum=WGS84 +to +proj=utm +zone=10 +datum=NAD27`, grid of the first
  candidate missing): a PROJ-string pair now goes through the ISO selection, and its candidates have
  alternatives. The axis change of a longitude-first source was missing from the datum-pivot
  candidates, which is what the previous text called the export fault. Pairs with `+approx`, `+over`
  or `+lon_wrap` keep the side-pair path, as their flag is not in the ISO candidates.

## Ratchet and checks

- Baseline `cargo test --test cli_yaml` before any change: passes, 22 open cs2cs rows.
- After the change: passes, no previously passing case fails. Open cs2cs rows: 19 of 22 (`#122`,
  `#134`, `#135` closed; `#132`, `#133` changed).
- `tests/alternatives_select.rs`: the outside-the-grid point of `#134`, and the `ONLY_BEST` failure of
  `#110` without retry.

## Commands

```bash
# Reference PROJ 9.9.0 tools, built from the tag 9.9.0 sources, for candidate traces:
cmake -S <proj-9.9.0> -B <build> -DBUILD_TESTING=OFF -DENABLE_CURL=OFF -DENABLE_TIFF=OFF \
  -DBUILD_PROJSYNC=OFF -DEXE_SQLITE3=<shim> -DCMAKE_BUILD_TYPE=Release
cmake --build <build> --target cs2cs projinfo
# PROJ's choice for a point (stderr shows the candidate used, PROJ_DEBUG=3):
echo "49 2 0" | PROJ_DEBUG=3 PROJ_DATA=<data>:<data/tests> <build>/bin/cs2cs EPSG:4746 EPSG:4978
# This port:
echo "49 2 0" | PROJ_DATA=<data>:<data/tests> target/debug/cs2cs EPSG:4746 EPSG:4978
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_CS2CS_REQUIRED=1 cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync
cargo test --test docs_in_sync
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

The reference build needs a `sqlite3` binary to generate `proj.db`. A shim that runs the SQL script
through Python's `sqlite3` module was passed with `-DEXE_SQLITE3`. The `proj.db` the port reads is the
one in `data/`.
