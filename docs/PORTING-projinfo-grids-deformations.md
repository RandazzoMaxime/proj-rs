# Port — PROJ strings of grid and deformation operations (projinfo)

Sources, PROJ 9.9.0 (`src/iso19111/operation/singleoperation.cpp`, `exportToPROJStringGeneric`):

- NTv1 and NTv2 horizontal shift, line ~4329: `hgridshift`
- `Geographic3D to GravityRelatedHeight` (method list `isGeographic3DToGravityRelatedHeight`, line ~2206) with the geoid file, line ~4705: `vgridshift`
- VERTCON, line ~4772, and the regular vertical grid methods (`isRegularVerticalGridMethod`, line ~2377), with NRCan's reversed sign
- New Zealand deformation model (method 1079), line ~4868: `defmodel`
- TIN interpolation, JSON (methods 1138 and 1145), line ~4954: `tinshift`
- `Transformation::_exportToPROJString` substitution of PROJ grid names (`substitutePROJAlternativeGridNames`), line ~4322

Not in this lot: `vgridshift` for the height-to-geographic 3D method (`createGravityRelatedHeightToGeographic3D`), GTIFF `gridshift`, CTABLE2, velocity grids, and the coordinate epochs.

## What is written

`CoordinateOperation::export_to_proj_string_with_database(db, lines)` is the export `projinfo` uses for a transformation or a concatenated operation. The grid methods are written only through it. Each grid name the database renames is written under its PROJ name, as PROJ does before the grid steps.

| Method (EPSG) | Parameter (EPSG) | PROJ step |
|---|---|---|
| NTv1 (9614), NTv2 (9615) | Latitude-longitude difference file (8656) | `hgridshift grids=` between the angular steps of each geographic CRS |
| Geog3D to GravityRelatedHeight (1025 … 9635, or the name prefix) | Geoid correction filename (8666) | `inv` `vgridshift multiplier=1` between the geographic steps of the source (no z conversion) and the target; `axisswap order=1,2,-3` when the target vertical axis is down |
| VERTCON (9658) | Vertical offset file (8732) | `vgridshift`, multiplier `1` for a `.tif` grid, `0.001` otherwise |
| Regular vertical grids (1071, 1080, 1084, 1085, 1101, 1129) | Vertical offset (8732) or geoid model difference (1063) | `vgridshift multiplier=1` |
| NRCan (1112, 1126) | as above | `vgridshift multiplier=-1` |
| New Zealand deformation model (1079) | Point motion velocity grid file (1050) | `inv defmodel model=` between the angular steps of the two geographic CRS |
| TIN, Cartesian (1138) | TIN offset file (1064) | `tinshift file=` between the projected unit and axis steps |
| TIN, geographic 2D (1145) | TIN offset file (1064) | `tinshift file=` between the axis swaps, degree axes only |

The geoid branch runs before the regular one: method 1126 is in both lists, and it takes the geoid branch only when the file is an 8666 geoid correction file.

## API mapping

| PROJ | Rust |
|---|---|
| `SingleOperation::exportToPROJStringGeneric`, grid branches | `transform_export.rs`: `grid_shift`, `horizontal_grid_shift`, `geoid_grid_shift`, `vertical_grid_shift`, `deformation_grid_shift`, `tin_shift` |
| `substitutePROJAlternativeGridNames` | `select::proj_grid_names`, used by `CoordinateOperation::export_to_proj_string_with_database` |
| `pushOmitZUnitConversion` | `horizontal_unit_and_axis_swap` |
| `extractGeographicCRSIfGeographicCRSOrEquivalent`, `extractVerticalCRS` | `extract_geographic`, `vertical_of` |
| `projinfo` `outputObject` for a registry operation | `src/bin/projinfo.rs` `operation_proj`, which takes the database |

## Test mapping

`tests/transform_export.rs`, the tests after `molodensky_badekas_step_is_unimplemented`. Each expected text is `projinfo -k operation EPSG:<code> -o PROJ -q --grid-check none` from PROJ 9.9.0, pasted in. The helper is `export_with_database`, which the existing `export` does not replace.

| Test | EPSG operation | Method |
|---|---|---|
| `geoid_1048_5656` | 5656 | Geog3D to GravityRelatedHeight (AUSGeoid v2) |
| `geoid_1048_8451` | 8451 | same, with a wrapped `multiplier` line |
| `geoid_1103_9708` | 9708 | Geog3D to GravityRelatedHeight (EGM) |
| `geoid_1072_9916` | 9916 | Geog3D to GravityRelatedHeight (OSGM15-Ire) |
| `vertical_1084_9312` | 9312 | regular vertical grid (GTX) |
| `vertical_1085_9553` | 9553 | regular vertical grid (ASC) |
| `vertical_1101_9662` | 9662 | regular vertical grid (PL txt) |
| `vertical_1080_9275` | 9275 | regular vertical grid (BEV AT) |
| `vertcon_9658_7969` | 7969 | VERTCON, `.tif` gives multiplier 1 |
| `nrcan_1126_10518` | 10518 | NRCan, multiplier -1 |
| `defmodel_1079_9084` | 9084 | New Zealand deformation model, 3D |
| `hgridshift_9615_1295` | 1295 | NTv2 |
| `hgridshift_9614_1312` | 1312 | NTv1 |
| `tinshift_1138_10703` | 10703 | TIN, projected |
| `tinshift_1145_10854` | 10854 | TIN, geographic 2D |

The 17 tests that were there before pass unchanged.

## Deliberate differences

- **`cs2cs` does not see the grid steps.** `select.rs` still exports through `export_to_proj_string_with`, where the grid methods stay `Unimplemented`. Writing them there changes which candidate `cs2cs --only-best` picks. In the yaml case `test_cs2cs_various.yaml#111`, the geoid candidate `Inverse of WGS 84 to EGM96 height (1)` becomes exportable, its grid `us_nga_egm96_15.tif` is absent in this environment, and the output is `*	* inf` (the reference prints the same error here). The yaml compares only the two horizontal numbers, so the case passed before. That is a regression by the harness, and `cs2cs` is another lot, so the gate is kept. The `cs2cs` lot must decide it.
- **Coordinate epochs are not stored**, so PROJ's `set v_4=` steps around `defmodel` are not written.
- **Error text for `tinshift` 1145.** A source or target that is not a geographic CRS reports the degree-unit message, not PROJ's "only to source GeographicCRS" message. The export fails in both cases, and `projinfo` prints no PROJ string.
- **Geoid branch without the 8666 file** falls through to the regular and deformation branches, as PROJ does, and ends in `Unimplemented` when nothing matches.

## Preserved quirks

- `vgridshift` on a geoid is inverted (`+inv`), and its horizontal steps around it are the inverse of the source's, so the pipeline is `axisswap`, `unitconvert deg→rad`, `inv vgridshift`, `unitconvert rad→deg`, `axisswap`.
- A 3D geographic source writes no `z` unit step inside a geoid `vgridshift`, but writes `z_in=m z_out=m` in `defmodel`, as the references show.
- The VERTCON `.tif` test reads the substituted name, because PROJ substitutes before it reads the file.
- `defmodel` is written once inside an inversion (`inv defmodel`), as PROJ does.

## Open against the yaml suite

`projinfo`: 42 open before this lot, 41 after. The closed id is `test_projinfo.yaml#58`. No row was newly opened (the id lists were compared, not just the counts). Causes, each checked with `-s/-t` probes against the reference:

- **Vertical CRS paths are not ported** (`createOperationsGeogToVert…`, `createOperationsVertToVert`, `createOperationsVertToGeog`). `-s EPSG:4283 -t EPSG:5711` gives 0 candidates against the reference's `DERIVED_FROM(EPSG):5656`, whose registry source is the 3D CRS 4939. Likewise `-s EPSG:5613 -t EPSG:4977` (25), `-s EPSG:5713 -t EPSG:10588` (85, reference `+proj=noop`), and the vertical cases 38, 39, 60–64 and 86, 87.
- **Derived CRS path** (`createOperationsDerivedTo`). `-s KKJ -t ETRS89` (59) gives the geographic Helmert 4123→4258; the reference chooses the projected route `tmerc`, `tinshift`, `inv utm`.
- **Candidate names and ordering.** 27 has the candidate and its PROJ string, but the reference names it `INVERSE(DERIVED_FROM(EPSG))` and adds a ballpark vertical operation. 52–57: the first candidate here is a `defmodel` operation whose pipeline writes `z` steps, where the reference writes none, so the source or the operation chosen differs (the reference's first candidate is `INVERSE(DERIVED_FROM(EPSG)):9090`). Not investigated further. 26 gives 5 candidates against 3.
- **PROJ-string CRS** (35): `+geoidgrids=@foo.gtx` as the source gives 0 candidates.
- **Point motion** (84, velocity grid), not ported, and the datum-pivot cases (65).
- **`-k operation`** (45) is not supported by `projinfo`.

## Verification

Commands, with the PROJ 9.9.0 binaries on `PATH`:

- `cargo test --test transform_export`: 32 passed, the 17 existing and the 15 new.
- `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`: passes. Against a clean `origin/main` (`6e128e3`) the ledger lists 65 rows, here 64; the only row removed is `test_projinfo.yaml#58`, and no row is added.
- `PATH=… cargo test --all-features --no-fail-fast`: 26 test binaries, all `ok`.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean. `cargo fmt`: applied.

No bench case is added: an operation export is not a per-point operation, as in `PORTING-transformation-export.md`.
