# Port — `opfactory-core-b`

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the last 114 names of the `core` class in `docs/UNIT-TRIAGE.md` (from `compoundCRS_with_boundVerticalCRS_to_geogCRS` through `createOperation_ETRS89_to_Amersfoort`). Niche names that sit in the C++ file between those are a different lot.

Rust tests are in `tests/unit_opfactory_core_b.rs`. Each function is `operation_` plus the C++ name in snake case. Expected PROJ strings are copied from the C++ `TEST()`.

## Tests ported (6)

| C++ TEST | Rust test |
|---|---|
| `compoundCRS_with_boundVerticalCRS_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_to_geog_crs` |
| `compoundCRS_with_boundGeogCRS_to_geogCRS` | `operation_compound_crs_with_bound_geog_crs_to_geog_crs` |
| `compoundCRS_with_boundGeogCRS_and_boundVerticalCRS_to_geogCRS` | `operation_compound_crs_with_bound_geog_crs_and_bound_vertical_crs_to_geog_crs` |
| `compoundCRS_with_boundVerticalCRS_from_geoidgrids_with_m_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_m_to_geog_crs` |
| `compoundCRS_with_boundVerticalCRS_from_geoidgrids_with_ftus_to_geogCRS` | `operation_compound_crs_with_bound_vertical_crs_from_geoidgrids_with_ftus_to_geog_crs` |
| `vertCRS_to_vertCRS` | `operation_vert_crs_to_vert_crs` |

`vertCRS_to_vertCRS` builds `+vunits` CRS with `create_crs_from_proj_string`. `create_from_user_input` only treats a string as PROJ when it contains `proj=`.

## Behavior added

- A bound CRS to a geographic CRS, and a compound of a bound component to a geographic 3D CRS, export as one pipeline (`src/iso19111/bound_ops.rs`, `compound.rs`, `ProjBased` on `ConcatenatedOperation`).
- `vertical_to_vertical`: same datum and scale `-1` is method 1068 (`+proj=axisswap +order=1,2,-3`). Any other pair is method 1069. A negative factor that is not a pure reversal exports as `+proj=affine +s33=...`, including `-3.28083333333333` for metre height to US-survey-foot depth.
- `CoordinateOperation::inverse` is public. The vertical test checks the inverse of metres to feet.
- WKT1 `VERT_DATUM` `EXTENSION["PROJ4_GRIDS", ...]` becomes a bound vertical CRS (`GravityRelatedHeight to Geographic3D`), except the two historical NAVD88 grid lists, which stay plain vertical CRS. The hub is the compound horizontal geographic CRS promoted to 3D when its prime meridian is Greenwich and its axis is degrees; otherwise it is EPSG:4979.
- `COMPD_CS` of a geographic CRS and a `VERT_DATUM` type `2002` uses `CompoundCRS::create_lax` and becomes one geographic 3D CRS.

## Still open

These stay in `docs/UNPORTED-UNIT.md`.

- **`+over` / `+lon_wrap` PROJ-string fallback** (`createOperation_fallback_to_proj4_strings`, `_bound_of_geog`, `_regular_with_datum_to_projliteral`, `_proj_NAD83_to_projliteral`, `_geog_NAD83_to_projliteral`, `_regular_with_nadgrids_to_projliteral`, `_projliteral_to_projliteral`, `_regular_to_projliteral_with_towgs84`, `_with_axis_inverted_projCRS`): the CRS has no PROJ4 extension, so `+over` and `+lon_wrap` are dropped. `+over` on a projected step survives the UTM rewrite and the parse fails with `unsupported +proj=tmerc for a CRS`. The ISO path then emits a Helmert or `+proj=noop` instead of the literal pipeline.
- **`createOperation_on_crs_with_bound_crs_and_wktext`**: unknown keys (`ignored1`, `ignored2`) fail the projected parse. The expected string is the ISO bound pipeline (`hgridshift`), which also needs bound-to-bound.
- **`compoundCRS_with_boundProjCRS_and_boundVerticalCRS_to_geogCRS`**: the inverse UTM step has no `+pm=paris`, and a separate `+inv +proj=longlat +ellps=clrk80ign +pm=paris` remains. PROJ attaches the prime meridian to the projection step and omits that longlat.
- **WKT1 `PROJ4_GRIDS` compounds** (`compoundCRS_with_boundProjCRS_with_ftus_and_boundVerticalCRS_to_geogCRS`, `compoundCRS_with_boundVerticalCRS_from_grids_to_geogCRS_with_ftus_ctxt`, `compoundCRS_with_boundGeogCRS_boundVerticalCRS_from_grids_to_boundGeogCRS_with_ftus_ctxt`, `compoundCRS_with_boundVerticalCRS_from_grids_to_boundGeogCRS_with_ftus_ctxt`, `compoundCRS_with_bound_of_projected_and_bound_of_vertical_to_geog3D`): import now yields `Compound[Geog2, Bound(Vert→Geog3)]` and, for an ellipsoidal-height `COMPD_CS`, a geographic 3D CRS. `create_operations` is still empty. The hub is metres and the target height is US survey foot, and there is no same-datum geographic height-unit change to chain after the geoid transformation.
- **`compoundCRS_to_compoundCRS` and the later compound-to-compound names** (context, Helmert, geoid grids, two vertical transformations, issues 2232, 2720, 3152, 3191, 3328, 4550, GDA94/AHD, OSGB36, WGS 84/EGM96, WGS 84/EGM2008, extent none): `compound_to_compound` only keeps a vertical operation when the geographic parts are equivalent and the vertical names differ. Same vertical CRS, a geoid model, an interpolation CRS, and differing geographic parts are not built. `compoundCRS_to_compoundCRS` also needs the horizontal projected-to-projected operation retargeted onto the compounds.
- **`vertCRS_to_vertCRS_context`, `_New_Zealand_context`**: EPSG vertical operations are looked up, and `setGridAvailabilityUse` does not exist, so the first candidate is not forced to the grid PROJ names.
- **`vertCRS_to_vertCRS_pivot_context`**: no chain of a registered vertical transformation with a height-depth reversal.
- **`projCRS_3D_to_geogCRS_3D` and `compoundCRS_to_geogCRS_3D`**: a projected or compound CRS whose only vertical difference is the unit is not the non-ballpark pipeline PROJ writes (the compound case is a ballpark that still has to be the inverse Mercator plus one `z` unit conversion).
- **EPSG compound/geographic 3D context, IGNF:LAMB1, NAD83 to EPSG:6414, canonical bound CRS, epochs, velocity grids, local orthographic EPSG:10622, ITRF/ETRS89, time-dependent Helmert, deformation model, ETRF2000/ETRS89 to Amersfoort**: not reached. Several need point-motion operations, a datum-pivot iteration, or `IGNORE_GRID_AVAILABILITY`, which the context does not have.
- **`isPROJInstantiable`**: no method that checks a grid file on disk and rejects an empty method.

## Verification

- `cargo test --test unit_opfactory_core_b operation_vert_crs_to_vert_crs -- --exact`: passed.
- The five bound-compound tests above passed in the same tree before the WKT1 `PROJ4_GRIDS` import (`cargo test --test unit_opfactory_core_b`: 5 passed, 16 failed). Those 16 were removed from the test file so a red test is not committed. They do not use `PROJ4_GRIDS` or `COMPD_CS` type `2002`.
- A throwaway probe (not committed) then showed the grids `COMPD_CS` importing as `Compound[Geog2, Bound(Vert→Geog3)]` and the ellipsoidal-height target as `Geog3`, with no operation.
- After that probe, `cc` stopped linking: `xcode-select` reports no developer tools (`/var/db/xcode_select_link` points at a missing `Xcode.app`). `cargo test`, `cargo clippy`, and `cargo test --test docs_in_sync` could not be run again. `cargo fmt` was run on the files this lot edited.
