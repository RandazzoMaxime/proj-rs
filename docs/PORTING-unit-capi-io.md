# Unit lot capi-io: porting sheet

Lot `capi-io`, branch `unit-fix-capi-io`, base `origin/main`. One sheet for the whole lot;
one section per commit.

## Function map

| PROJ (9.9.0) | Rust | Status |
|---|---|---|
| `c_api.cpp` `proj_get_suggested_operation` (single operation returns 0) | `src/iso19111/suggest.rs` `suggested_forward` | ported |
| `trans.cpp` `pj_get_suggested_operation` (`spatialCriterionOK`, best choice) | `src/iso19111/suggest.rs` `covers`, `suggested_forward` | ported |
| `crs_to_crs.cpp` `pj_create_prepared_operations`, `add_coord_op_to_list` | `src/iso19111/suggest.rs` `prepare` | ported for the source box; target box not read (see deviations) |
| `crs_to_crs.cpp` `reproject_bbox` | `src/iso19111/select.rs` `reproject_box` (now `pub(super)`) | reused |
| `crs_to_crs.cpp` `create_operation_to_geog_crs` | `src/iso19111/select.rs` `lon_lat_base`, `geodetic_to_crs` | prime meridian now zero, as PROJ |
| `crs_to_crs.cpp` `create_operation_geocentric_crs_to_geog_crs` | `src/iso19111/select.rs` `geocentric_to_lon_lat` (now `pub(super)`) | reused |

## Commit 1: suggestion for projected and geocentric sources

Cause: `suggest.rs` accepted only a geographic source. PROJ reads the candidate boxes in the
coordinates of the source (`reproject_bbox` through the operation from the geodetic base of the
source), and converts a geocentric point to longitude and latitude first.

Changes:

- `suggest.rs`: `SourceKind` (Geographic, Projected, Geocentric). The geographic path keeps the
  previous behaviour, including the degree-order fallback. A single operation is returned
  before any source check, as `proj_get_suggested_operation` does.
- `select.rs` `lon_lat_base`: the geodetic base is an unnamed datum on the ellipsoid of the CRS
  with the Greenwich prime meridian, as `create_operation_to_geog_crs` sets it (`crs_to_crs.cpp`,
  `proj_create_geographic_crs(... "Reference prime meridian", 0 ...)`). Without it the boxes of
  a non-Greenwich source (NTF Paris) are shifted by the meridian. The cs2cs alternatives path
  uses the same base; `cargo test --test cli_yaml` passes with no reopened case.
- `factory.rs`: `suggested_operation` keeps its signature and delegates to the new
  `suggested_operation_xyz(operations, source, (x, y, z))`, which takes the height that a
  geocentric source needs.

Tests (`tests/unit_capi_operations_suggest.rs`):

| test | result |
|---|---|
| `CApi.proj_create_operations_prime_meridian_non_greenwich` | closed |
| `CApi.proj_get_suggested_operation_with_operations_without_area_of_use` | closed |

Deviations:

- The target box is not read. PROJ also drops a candidate whose target box cannot be
  reprojected (`minxDst <= maxxDst`). The suggestion here keeps it. No test in this lot needs it.
- A projected or geocentric source opens the default `proj.db` for each call, as PROJ opens its
  default context inside `create_operation_to_geog_crs`. The cost per call is not measured.
- A source with no geodetic base, or whose base operation cannot be built, gives `Ok(None)`,
  as PROJ returns -1 when it prepares no candidate.

Commands run: `cargo test --test unit_capi_operations_suggest`, `cargo test --test cli_yaml`
(with the reference PROJ 9.9.0 `bin` on `PATH` and `PROJ_DATA` the oracle database),
`cargo test --all-features --no-fail-fast` (1668 passed, 0 failed, 31 ignored), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
`UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`.

## Open: `CApi.proj_create_crs_to_crs_from_custom_compound_crs_with_NAD83_2011_and_geoidgrid_ref_against_NAD83_2011_to_WGS84_G1762`

Status: not closed. The brief said "Proj::new rejects the pipeline". On `origin/main` this is no
longer true: the pipeline builds.

What was checked:

- The PROJ string of the compound (`best_pipeline` for the WKT2 compound, `EPSG:7665`) equals the
  string `projinfo -o PROJ` gives for the same compound, parameter for parameter, including
  `+dx`, `+drx`, `+ds`, `+t_epoch=1997`. The export side is correct.
- The parser reads the time-dependent Helmert. The engine applies it with the point's time.
- The reference transform at t=2010 (`cct -d 12` on the same pipeline, with the time as the
  fourth column) gives `35.000003665013 -118.000014142114`, which matches the test's expected
  values within 1e-9.

Cause of the failure: the facade has no 4-D entry point. `Proj::convert_xyz` passes `t = 0`
(`src/proj.rs` `map_xyz`), and the test needs `t = 2010`. This is a missing public API, not a
parser or export error.

Next step (public API, not in this lot): a 4-D forward conversion on `Proj`, for example
`pub fn convert_xyzt(&self, x: f64, y: f64, z: f64, t: f64) -> Result<(f64, f64, f64, f64),
ProjError>`, mapping to `Xyzt { x, y, z, t }` in `map_xyz`. The draft test is kept out of the
tree (`scratchpad/geoid_both_blocked.rs`) until that exists.

Rejected change: giving no-time points `t = +inf` (cs2cs behaviour). `tests/proj_meta_facade.rs`
`an_epoch_moves_itrf2014_to_itrf2008` pins the `proj_trans_generic` behaviour that a null time
array is zero (`trans.cpp`, "a null pointer ... treated as a fully populated array of zeroes").
The C API and the facade use zero. The `cs2cs` CLI uses `HUGE_VAL` for a missing time, a
separate existing difference. The change was reverted.

## Commit 3: `io.projparse_init` (closed)

Cause: `create_from_user_input` sent `init=` texts to the authority-code lookup, which returned
`NotFound { authority: "init=epsg" }`. PROJ routes them to the PROJ string parser, and reads
`init=epsg:` and `init=IGNF:` only under the PROJ 4 rules.

Changes:

- `json.rs`: `create_from_user_input` is `create_from_user_input_with_init_rules(text, db,
  InitRules::Iso19111)`. A text with `init=` and no `proj=` step goes to `crs_from_init_text`.
  Under `InitRules::Iso19111` it is refused with PROJ's message (`io.cpp` 12548-12558). Under
  `InitRules::Proj4` the CRS of `EPSG:` (or `IGNF:`) is rebuilt as in `io.cpp` 12571-12619:
  longitude and latitude in degrees (with the height axis when there are three), or easting and
  northing for a projected CRS (not for Transverse Mercator South Orientated). The `title=` name
  and the `over` flag are applied to the geographic and projected results only.
- `model.rs` `Identified.over`: PROJ's `OVER` property of a CRS (`crs.cpp` 128-135).
  `projstring.rs` writes `+over` after the datum, before `+no_defs +type=crs` (`crs.cpp` 3469).
- `mod.rs`: the public names `InitRules` (an enum, since PROJ's `usePROJ4InitRules` is a bool
  and no bool may be in a public signature) and `create_from_user_input_with_init_rules`.

Tests (`tests/unit_io_projparse_init.rs`): `io.projparse_init` closed. The draft called the
two-argument form for the four compatibility blocks; the test now calls the rules form with
`InitRules::Proj4`, and adds PROJ's default-mode refusal. Expected values are unchanged. The
WKT of the test uses `WktConvention::Wkt2_2015`, the default of PROJ's `WKTFormatter::create()`
(`Convention::WKT2`); the draft's 2019 form writes `GEOGCRS` for the ensemble datum of
EPSG:4326.

Deviations and not ported:

- A `init=` with parameters beside `over`, `no_defs`, `wktext` and `type=crs` is an error
  (`Unsupported`), not PROJ's expansion through `exportToPROJString` (`io.cpp` 12581-12600).
- `init=` names that are not EPSG or IGNF codes (PROJ's init files, for example `nad27:`) are an
  error: the init file expansion is not ported.
- The domains are copied; PROJ sets the domain of validity from `getExtent(crs)`, which is the
  first domain.

The original requirement, kept for reference:

The test needs, in compatibility mode (PROJ `usePROJ4InitRules = true`):

- routing of `init=`, `+init=` and `title=` texts to the PROJ string parser in
  `create_from_user_input` (`io.cpp` `createFromUserInput`, lines 7800-7830);
- `init=epsg:` and `init=IGNF:` expanded through the EPSG CRS, with the longitude-latitude
  degree axes for a geographic CRS and easting-northing for a projected one, unless the
  conversion is Transverse Mercator South Orientated (`io.cpp` 12514-12600);
- the `title=` name and the `OVER` property (`crs.cpp` 128-135, `crs.cpp` 3469 `+over` export);
- the error `init=epsg:/init=IGNF: syntax not supported in non-PROJ4 emulation mode` in default
  mode (`io.cpp` 12548-12558), which is what `create_from_user_input(text, db)` must keep giving.


## APIs needed by the tests not in this lot

- `CApi.proj_create_vertical_crs_ex`, `_with_geog_crs`, `_implied_accuracy`: a vertical CRS
  with a `GEOID_MODEL` transformation and the `ACCURACY=` option. Needs a public
  `GeoidModel` constructor (`src/iso19111/model.rs`, around line 3110) and a public vertical CRS
  constructor that takes the geoid model and the accuracy.
- `CApi.proj_normalize_for_visualization_with_alternatives` and `_reverse`: needs a public
  `degree_input` and `degree_output` on `Proj` (`proj_iso.rs:393` is crate-private).
- `CApi.proj_coordoperation_create_inverse` with `MULTILINE`, `INDENTATION_WIDTH`,
  `MAX_LINE_LENGTH`: needs an inverse of a PROJ-string operation on `Proj`, and a multi-line PROJ
  string writer with those options (`ProjLines` has only `Multi` and `Single`).
- `CApi.proj_coordoperation_get_grid_used_fullname_caching`: needs a grid accessor on `Proj`
  (grid names and URLs of a `proj=vgridshift` operation).
- `io.createFromUserInput`: about 14 calls pass a null database, but
  `create_from_user_input` always takes `&DatabaseContext`. Needs an optional database
  parameter and the URN forms (`urn:ogc:def:crs,crs:...,cs:PROJ::ENh`), about 550 lines.

## Not ported

- `crs.boundCRS_crs_link`: checks `shared_ptr::use_count` and weak expiry, with no Rust
  equivalent.
- `factory.AuthorityFactory_createDynamicVerticalDatum`: inside `#ifdef
  no_more_dynamic_vertical_datum` (`test_factory.cpp:321`), not compiled in PROJ 9.9.0. A dead
  ledger entry.
