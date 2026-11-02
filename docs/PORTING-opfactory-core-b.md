# Port — `opfactory-core-b`

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the last 114 names of the `core` class in `docs/UNIT-TRIAGE.md` (from `compoundCRS_with_boundVerticalCRS_to_geogCRS` through `createOperation_ETRS89_to_Amersfoort`). Niche names that sit in the C++ file between those are a different lot.

Rust tests are in `tests/unit_opfactory_core_b.rs`. Each function is `operation_` plus the C++ name in snake case. Expected PROJ strings are copied from the C++ `TEST()`.

## Tests ported (11)

| C++ TEST | Rust test |
|---|---|
| `compoundCRS_with_boundVerticalCRS_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_to_geog_crs` |
| `compoundCRS_with_boundGeogCRS_to_geogCRS` | `operation_compound_crs_with_bound_geog_crs_to_geog_crs` |
| `compoundCRS_with_boundGeogCRS_and_boundVerticalCRS_to_geogCRS` | `operation_compound_crs_with_bound_geog_crs_and_bound_vertical_crs_to_geog_crs` |
| `compoundCRS_with_boundVerticalCRS_from_geoidgrids_with_m_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_m_to_geog_crs` |
| `compoundCRS_with_boundVerticalCRS_from_geoidgrids_with_ftus_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_ftus_to_geog_crs` |
| `compoundCRS_with_boundVerticalCRS_from_grids_to_geogCRS_with_ftus_ctxt` | `operation_compound_crs_with_bound_vertical_crs_from_grids_to_geog_crs_with_ftus_ctxt` |
| `compoundCRS_with_boundGeogCRS_boundVerticalCRS_from_grids_to_boundGeogCRS_with_ftus_ctxt` | `operation_compound_crs_with_bound_geog_crs_bound_vertical_crs_from_grids_to_bound_geog_crs_with_ftus_ctxt` |
| `compoundCRS_with_boundVerticalCRS_from_grids_to_boundGeogCRS_with_ftus_ctxt` | `operation_compound_crs_with_bound_vertical_crs_from_grids_to_bound_geog_crs_with_ftus_ctxt` |
| `vertCRS_to_vertCRS` | `operation_vert_crs_to_vert_crs` |
| `projCRS_3D_to_geogCRS_3D` | `operation_proj_crs_3_d_to_geog_crs_3_d` |
| `compoundCRS_to_geogCRS_3D` | `operation_compound_crs_to_geog_crs_3_d` |

`vertCRS_to_vertCRS` builds `+vunits` CRS with `create_crs_from_proj_string`. `create_from_user_input` only treats a string as PROJ when it contains `proj=`.

## Behavior added

- A bound CRS to a geographic CRS, and a compound of a bound component to a geographic 3D CRS, export as one pipeline (`src/iso19111/bound_ops.rs`, `compound.rs`, `ProjBased` on `ConcatenatedOperation`).
- `vertical_to_vertical`: same datum and scale `-1` is method 1068 (`+proj=axisswap +order=1,2,-3`). Any other pair is method 1069. A negative factor that is not a pure reversal exports as `+proj=affine +s33=...`, including `-3.28083333333333` for metre height to US-survey-foot depth.
- `CoordinateOperation::inverse` is public. The vertical test checks the inverse of metres to feet.
- WKT1 `VERT_DATUM` `EXTENSION["PROJ4_GRIDS", ...]` becomes a bound vertical CRS (`GravityRelatedHeight to Geographic3D`), except the two historical NAVD88 grid lists, which stay plain vertical CRS. The hub is the compound horizontal geographic CRS promoted to 3D when its prime meridian is Greenwich and its axis is degrees; otherwise it is EPSG:4979.
- `COMPD_CS` of a geographic CRS and a `VERT_DATUM` type `2002` uses `CompoundCRS::create_lax` and becomes one geographic 3D CRS. The same with a bound geographic CRS becomes a bound CRS whose source is that 3D geographic CRS.
- Two geographic CRS of the same datum that differ only by height unit (a 2D CRS counts as metres) get method 1069, `Transformation::create_from_code`. A lat/lon reversal is left to the unported proj-based path.
- A compound CRS to a bound CRS uses the base of the bound, then the inverse. PROJ's earlier bound-to-compound cases are not ported.
- A bound CRS whose base datum already matches the geographic target uses that base. When that list is empty, the null geographic offset fills it.
- A 3D projected CRS exports its height unit in the projection pipeline: the geographic step converts that height into metres, and the projected step converts metres into the projected height unit. `projCRS_3D_to_geogCRS_3D` is that inverse pipeline plus the same-datum geographic height change (method 1069). It is not a ballpark.
- A compound of a projected CRS and a vertical CRS, on the same datum as a 3D geographic target and with no registered vertical operation, is the inverse projection plus the ballpark vertical change. The projection is exported from an east-north copy of its base, which is the pipeline left after PROJ cancels the north-east axis swap.

## Still open

These stay in `docs/UNPORTED-UNIT.md`.

- **`+over` / `+lon_wrap` PROJ-string fallback** (`createOperation_fallback_to_proj4_strings`, `_bound_of_geog`, `_regular_with_datum_to_projliteral`, `_proj_NAD83_to_projliteral`, `_geog_NAD83_to_projliteral`, `_regular_with_nadgrids_to_projliteral`, `_projliteral_to_projliteral`, `_regular_to_projliteral_with_towgs84`, `_with_axis_inverted_projCRS`): the CRS has no PROJ4 extension, so `+over` and `+lon_wrap` are dropped. `+over` on a projected step survives the UTM rewrite and the parse fails with `unsupported +proj=tmerc for a CRS`. The ISO path then emits a Helmert or `+proj=noop` instead of the literal pipeline.
- **`createOperation_on_crs_with_bound_crs_and_wktext`**: unknown keys (`ignored1`, `ignored2`) fail the projected parse. The expected string is the ISO bound pipeline (`hgridshift`), which also needs bound-to-bound.
- **`compoundCRS_with_boundProjCRS_and_boundVerticalCRS_to_geogCRS`**: the inverse UTM step has no `+pm=paris`, and a separate `+inv +proj=longlat +ellps=clrk80ign +pm=paris` remains. PROJ attaches the prime meridian to the projection step and omits that longlat.
- **WKT1 `PROJ4_GRIDS` with a projected horizontal CRS** (`compoundCRS_with_boundProjCRS_with_ftus_and_boundVerticalCRS_to_geogCRS`, `compoundCRS_with_bound_of_projected_and_bound_of_vertical_to_geog3D`): the geographic NAD83 cases are ported. These still need the prime meridian on the projected step, and `create_lax` does not promote a projected CRS with an ellipsoidal-height vertical CRS to a 3D projected CRS.
- **`compoundCRS_to_compoundCRS` and the later compound-to-compound names** (context, Helmert, geoid grids, two vertical transformations, issues 2232, 2720, 3152, 3191, 3328, 4550, GDA94/AHD, OSGB36, WGS 84/EGM96, WGS 84/EGM2008, extent none): `compound_to_compound` only keeps a vertical operation when the geographic parts are equivalent and the vertical names differ. Same vertical CRS, a geoid model, an interpolation CRS, and differing geographic parts are not built. `compoundCRS_to_compoundCRS` also needs the horizontal projected-to-projected operation retargeted onto the compounds.
- **`vertCRS_to_vertCRS_context`, `_New_Zealand_context`**: EPSG vertical operations are looked up, and `setGridAvailabilityUse` does not exist, so the first candidate is not forced to the grid PROJ names.
- **`vertCRS_to_vertCRS_pivot_context`**: no chain of a registered vertical transformation with a height-depth reversal.
- **`compoundCRS_to_geogCRS_3D_context` and the later compound-to-geographic-3D names**: the no-database ballpark above does not cover an EPSG compound, a 3D Helmert, or `KNOWN_AVAILABLE` grid ranking.
- **EPSG compound/geographic 3D context, IGNF:LAMB1, NAD83 to EPSG:6414, canonical bound CRS, epochs, velocity grids, local orthographic EPSG:10622, ITRF/ETRS89, time-dependent Helmert, deformation model, ETRF2000/ETRS89 to Amersfoort**: not reached. Several need point-motion operations, a datum-pivot iteration, or `IGNORE_GRID_AVAILABILITY`, which the context does not have.
- **`isPROJInstantiable`**: no method that checks a grid file on disk and rejects an empty method.

## Verification

- `cargo test --test unit_opfactory_core_b`: 11 passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --test docs_in_sync`: fails on the generated counts in `README.md` and `docs/UNIT-TRIAGE.md`. Those files are left for the orchestrator.
- `cargo fmt` on the files this lot edited.
