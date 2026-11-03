# projinfo: text parity for the open yaml cases

Work on the `test_projinfo.yaml` rows that `docs/UNPORTED-CLI.md` listed as
`projinfo`, against PROJ 9.9.0. Each row was run with the yaml runner, then
compared to its expected output. A row was fixed only when its difference was
text. A difference in the candidate set, the chosen pipeline, a grid, or a
name lookup stays open.

## C++ read

- `src/apps/projinfo.cpp`: the `main` that forwards to `projinfo()`.
- `src/apps/projinfo_lib.cpp`:
  - `main_projinfo`: the option loop, `Too many parameters` (`positional_args[1]`),
    `--normalize-axis-order`, `-k operation`, the `-q` checks.
  - `buildObject`: the `urn:ogc:def:coordinateOperation:AUTH::CODE` read of
    `-k operation`, the promotion to 3D, `normalizeForVisualization`.
  - `outputObject`: the deprecation warning, `getNonDeprecated`, the quiet
    flag, and the PROJ string of a coordinate operation.
  - `outputOperationSummary`, `outputOperations`: the candidate summary line.
- `src/iso19111/metadata.cpp`: `Extent`, `GeographicBoundingBox`.
- `src/iso19111/operation/coordinateoperationfactory.cpp`: where the ballpark
  operations take `Extent::WORLD` (`createBallparkGeographicOffset`,
  `createOperationsVertToVert`, `createOperationsVertToGeogSynthetized`).

## Rows fixed

| id | Change |
|---|---|
| `test_projinfo.yaml#10` | `Too many parameters` names the second positional argument, as `positional_args[1]`. |
| `test_projinfo.yaml#22` | A deprecated CRS prints `Alternative non-deprecated CRS:` and its replacements (`Crs::non_deprecated`). The warning is silent with `-q`, as in `outputObject`. |
| `test_projinfo.yaml#33` | `--normalize-axis-order` is applied to the CRS of the candidates and of a single object (`Crs::normalize_for_visualization`). It was ignored. |
| `test_projinfo.yaml#45` | `-k operation AUTH:CODE` reads the operation with `create_coordinate_operation`, and a name is searched as `ObjectType::CoordinateOperation`. The PROJ and WKT2 output of an operation is printed. |

`--3d` is now applied to a single object too (`build_crs`), as `buildObject`
does. It is not enough for `#28` (see below).

The change is in `src/bin/projinfo.rs`. `PORT-NOTES-projinfo.md` corrects the
`Too many parameters` quirk.

## Rows still open

| id | Reason |
|---|---|
| 13, 14 | The candidate path builds grids with `AlternativeGridNames::Official` (`create_candidates`). PROJ uses the PROJ alternative names (`ca_nrc_ntv2_0.tif`, `HORIZONTAL_SHIFT_GTIFF`) and the `DERIVED_FROM(EPSG)` id, which the library does not produce. Changing the candidate path also changes transformations for cs2cs. |
| 23 | A WKT1 `GEOGCS` without `PRIMEM` is a parse error here. PROJ parses it in lax mode with warnings and grammar errors (WKT parser). |
| 25 | Expected 2 candidates, found 0: the inverse vertical grid operation is not found (engine). |
| 26 | Expected 3 candidates, found 5: operation filtering and grid availability differ (engine). |
| 27 | Expected 2 candidates, found 1: the vertical ballpark operation is not produced (engine). |
| 28 | `--3d` now promotes, but the library writes the 3D base `CS` in `BASEGEOGCRS` (`wkt.rs`, a rule for PROJ issue 2122) and keeps the scope of the source CRS, where PROJ prints `SCOPE["unknown"]`. This is a WKT writer and promotion decision in the library, not changed here. |
| 32 | The inverse of the concatenated operation has domain `World` where PROJ has `World.` (domain propagation of concatenated operations, engine). |
| 36 | Expected 19 candidates, found 1 (engine). |
| 37 | Same four candidates, in a different order (sorting, engine). |
| 38, 39 | Expected 1 candidate (AHD height grid operation), found 0 (engine). |
| 52 | The NZGD2000 pipeline matches except the `unitconvert` steps, which carry `z_in=m`/`z_out=m`: the 3D variant of the operation is chosen (engine). |
| 53, 54, 55, 56, 57 | No candidate found. PROJ finds an NZGD2000 `defmodel` and Helmert pipeline (engine). |
| 59 | Our pipeline uses `push`/`cart`/`helmert`; PROJ uses `tmerc` and `tinshift` (engine). |
| 60, 61, 62 | No candidate for the compound names `KKJ + N43 height` and the like. PROJ builds `tmerc` and `tinshift` pipelines (name lookup and engine). |
| 63, 64 | No candidate for `KKJ / Finland ... + N43 height`: compound names are not looked up (name lookup, engine). |
| 65 | Expected 2 time-dependent NKG candidates, found 0 (engine). |
| 68 | Three candidates found, two expected: the ED50 to ETRS89 (16) candidate is extra, under `--grid-check known_available` (engine). |
| 83 | The `towgs84` values of the MGI 1901 transformation differ: a different operation is selected (engine). |
| 84 | Expected a null geographic offset chained with a velocity grid and epochs; found a single null offset (engine). |
| 86 | The `vgridshift us_nga_egm08_25.tif` step is missing, and the `unitconvert` steps carry `z_in`/`z_out` (engine). |
| 87 | No candidate found for `WGS 84 (G1150) + EGM2008 height` to `WGS 84 (G1674) + EGM2008 height` (engine). |
| 88, 89 | One candidate found, two expected: the ETRF2000 and ETRF2014 hub chains are missing (engine). |
| 90 | Expected a concatenated operation through WGS 84 (G1762), found a ballpark offset (engine). |
| 106 | Name search `Christmas Island Grid 1985`: the identity operation is not returned (engine). |
| 108 | Expected one candidate, found 0: the inverse chain is not built (engine). |

## Commands and results

| Command | Result |
|---|---|
| `cargo test --test cli_yaml` | ok. The runner removed the 4 fixed rows from `docs/UNPORTED-CLI.md`. 39 projinfo rows became 35. |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast` | 1370 passed, 2 failed, 26 ignored |

The two failures are generated documents that depend on the ledger, and they
are left to the lead, who regenerates them with `UPDATE_DOCS=1`:

- `docs_in_sync::counts_in_docs_match_the_ledgers`: the counts in `README.md`
  and `ROADMAP.md` still show the 4 rows.
- `progress_svg::progress_chart_is_current`: `docs/progress.svg` is out of date.
