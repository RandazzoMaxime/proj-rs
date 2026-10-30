# Port — operation selection for `cs2cs` (`proj_create_crs_to_crs`)

Source: PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp`
(`PrecomputedOpCharacteristics` 864–907, `SortFunction` 913–1170,
`FilterResults` 1211–1440, `findOpsInRegistryDirect` 1889–1966,
`createOperations` 4340–4480), `src/crs_to_crs.cpp` (the `cs2cs` path:
ballpark flag, `proj_create_operation_factory_context`), and the context
defaults in `c_api.cpp` (setters at 8214–8320).

The Rust side is `best_pipeline` in `src/iso19111/select.rs`, called from
`build_engine` in `src/bin/cs2cs.rs` (`select_pipeline`). The old path
(`CrsPipeline`, `Proj::new_known_crs`) is kept as the fallback.

## Behaviour of the new path

For a pair with no `+proj=` side and no `--area`/`--bbox`:

1. `create_from_user_input` reads both CRS (codes, names, compounds, WKT).
2. `CoordinateOperationFactory::create_operations` lists the registered
   operations, with the default context (`StrictContainment`, `Smallest`).
3. Each candidate gets the characteristics `SortFunction` reads, and the list
   is ordered by `precedes`, which follows `SortFunction::compare` in PROJ.
4. The first candidate is exported as a PROJ pipeline with
   `export_to_proj_string`, and `Operation::create` builds it.

If any step fails (no candidate, export or build error, unsupported grid), the
caller falls through to the previous path, so a pair that passed before cannot
lose its result to a failed selection.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `proj_create_crs_to_crs` → `createOperations` | `best_pipeline`, using `CoordinateOperationFactory::create_operations` |
| `PrecomputedOpCharacteristics::area_` (pseudo-area of the op extent ∩ CRS extents, `extent1`/`extent2` branches of `sort()`) | `pseudo_area_of` |
| `isPROJExportable_` (`exportToPROJString` succeeds) | `characteristics` → `export` returns `Ok` |
| `projStepCount_` (`+step` count of the exported text, `xyzgridshift` excluded) | `proj_steps` (zero when the text has `+proj=xyzgridshift`) |
| `stepCount_` (`getStepCount`) | `step_count` |
| `accuracy_` (`getAccuracy`, conversions count as 0) | `accuracy` |
| `hasGrids_`, `gridsAvailable_`, `gridsKnown_` | `grid_names`; `gridpath::available`; `DatabaseContext::grid_is_known` |
| `isApprox_` (`hasBallparkTransformation`) | `approx` (name starts with `Ballpark `) |
| `hasBallparkVertical_`, `isNullTransformation_` | `ballpark_vertical`, `is_null_transformation` |
| `hasXYZGridshift_` | `xyz_gridshift` |
| `is_ETRS89_XXX_to_ETRS89_YYY_`, `uses_ETRF2000_`, `uses_ETRS89_to_ETRS89_XXX_generic_` | `etrs_pair`, `uses_etrf2000`, `uses_etrs89_generic` |
| `SortFunction::compare` | `precedes` (same branch order, including the `NTF (Paris)` and ballpark-offset name rules) |
| `std::sort` | `sort`, an insertion sort on `precedes` |
| `getExtent(op)` | `operation_extent` (now `pub(crate)` in `factory.rs`) |
| `crs_to_crs` target axis order | `lat_or_northing_first`, from the first axis direction of the target CRS |

## Deliberate differences

- **No area of interest.** `cs2cs` passes none, and `--area`/`--bbox` keep the
  old path. The `extent1`/`extent2` branch of PROJ's `sort()` is the one ported.
- **No pivot search.** `findsOpsInRegistryWithIntermediate`,
  `createOperationsWithDatumPivot` and `createOperationsGeodToGeod` are not
  ported. A pair that needs a pivot through `proj.db` returns no candidate, and
  the old path answers it.
- **Filters not ported:** `removeSyntheticNullTransforms`,
  `removeUninterestingOps`, `removeDuplicateOps`, and the
  `hasOpThatContainsAreaOfInterestAndNoGrid` shortcut.
- **Grid availability** is the file presence on the search path
  (`gridpath::available`). PROJ reads `gridsNeeded` with the `available` flag;
  the two agree for the grids in the test environment, but the database
  `gridsKnown` test is approximated by `grid_is_known`.
- **Ballpark** is detected by name prefix, because the port does not store
  PROJ's `hasBallparkTransformation_` flag.
- **Ties** keep the registry order, where PROJ's `std::sort` leaves them
  unspecified. `precedes` is not guaranteed to be a strict weak order (the
  ballpark-offset rule can disagree with the name-length rule), and `sort_by`
  may panic on such a comparator, so the sort is an insertion sort.

## Preserved quirks

- The ETRF2000 hub rule in `SortFunction` tests `!uses_ETRF2000` twice, as
  PROJ does; the first clause is therefore never true. It is ported as written.
- A `Conversion` candidate is the deriving conversion, exported by
  `operation_proj_string`, which writes the pipeline for a north-east geographic
  source. Its accuracy is 0.

## Fix found by the cs2cs yaml run

`operation_proj_string` and `geocentric_operation` only wrote
`+proj=unitconvert +xy_in=deg`. A grad source (EPSG:4807) got no conversion and
gave `*\t*`. The unit step now comes from `angular_to_radians`, which covers
`degree` and `grad`. Degree output is unchanged.

## Test mapping

| Case | Result |
|---|---|
| `test_cs2cs_various.yaml#8` (`EPSG:4807 EPSG:27572`, grad input) | passes (it regressed before the grad fix, and is back to the reference output `760724.02 3457334.86 0.00`) |
| `test_cs2cs_various.yaml#99` (`Pulkovo 1942` → `WGS 84`, antimeridian) | now passes |
| `test_cs2cs_various.yaml#112`, `#113` (`--only-best` `EPSG:4326` ↔ `GDA94`) | now pass |

## Remaining cs2cs cases (40 open, was 43)

Each row gives the reason as measured against `cs2cs` 9.9.0.

| Cases | Cause |
|---|---|
| `#136`–`#138` (similarity, `EPSG:23031` → `EPSG:25831`) | PROJ selects `ED50 / UTM zone 31N to ETRS89 / UTM zone 31N (1)` (5166, a similarity). Its extent (Catalonia) does not contain the CRS extents, and `filter_and_sort` drops it, which leaves no candidate. PROJ keeps it through `findOpsInRegistryDirect` and the `resFindDirectNonEmptyBeforeFiltering` path (4345–4480), not ported. |
| `#114`, `#115` (`--only-best` `EPSG:4326` ↔ `GDA2020`) | Observed: `best_pipeline("EPSG:4326", "GDA2020")` returns no candidate, while `best_pipeline` on `EPSG:4326` and `EPSG:7844` gives 2 candidates (the `proj.db` rows for 4326 → 7844 are 9691, 9690 and the inverse of 8450). The old path then runs and prints `4881228d19'8.549"W`. Not confirmed yet which CRS the name `GDA2020` resolves to. |
| `#95`, `#96` (`ITRF96` → `ITRF2014`) | The names resolve to the geocentric CRS (4917, 7789). The reference uses the geographic 3D CRS (7907, 7912), with lat-first output. Name resolution, not the selection itself. |
| `#100` (`EPSG:2636` → `WGS 84`) | Selection gives a height change of −6.772 where the reference gives 0. Not analysed. |
| `#130`, `#132`, `#134`, `#135`, `#110`, `#116`–`#122`, `#124`–`#128`, `#123` | Grids (`BETA2007.gsb`, `fr_ign_*`, `us_nga_egm08`, `DMA ConusW`), and the `--only-best` fallback when a grid is missing. Not a selection step. |
| `#140`, `#141` (`--s_epoch`, `--t_epoch`) | Time-dependent Helmert and epochs. `cs2cs` ignores `--s_epoch`/`--t_epoch` (`_selection` in `src/bin/cs2cs.rs`). |
| `#139`, `#143` (compounds `EPSG:3912+EPSG:5779`, `EPSG:4559+5757`) | `create_from_user_input` does not resolve these compounds, and the old path cannot either. |
| `#86`, `#88`, `#102`, `#103` | `IAU_2015:4993` ellipsoid, the `ITRF2000`/`ITRF1993` names, and `WGS 84 + EGM96 height` by name. |
| `#97`, `#98`, `#123` (`--accuracy`, `--no-ballpark`) | Options not implemented in `cs2cs` (`_selection`). |
| `#59`, `#89`, `#90` | `+proj=` on one side: these pairs use the `SidePair` path, which the new selection does not replace. |

## Verification

```
PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml
```

- Before (`origin/main` at `e8e3e1c`): 43 cs2cs rows in `docs/UNPORTED-CLI.md`.
- After: 40 cs2cs rows. `#99`, `#112`, `#113` leave the list. No row for another
  app changes, and no previously passing case fails.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt`: applied.
- Full suite: see the pull request description.
