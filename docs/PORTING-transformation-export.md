# Port — PROJ strings of transformations and concatenated operations

Sources, PROJ 9.9.0 (`src/iso19111/`):

- `operation/singleoperation.cpp`: `SingleOperation::exportToPROJStringGeneric` (line ~3294), `setupPROJGeodeticSourceCRS`, `setupPROJGeodeticTargetCRS`, `extractGeographicCRSIfGeographicCRSOrEquivalent`, `exportToPROJStringChangeVerticalUnit`
- `operation/transformation.cpp`: `Transformation::_exportToPROJString` (line 1917)
- `operation/concatenatedoperation.cpp`: `ConcatenatedOperation::_exportToPROJString` (line 919)
- `operation/singleoperation.cpp`: `InverseCoordinateOperation::_exportToPROJString` (line 5415)
- `io.cpp`: `PROJStringFormatter` (`startInversion`, `stopInversion`, `addParam`, `toString`), `IPROJStringExportable::exportToPROJString`
- `crs.cpp`: `GeographicCRS::_exportToPROJString`, `GeodeticCRS::_exportToPROJString`, `addAngularUnitConvertAndAxisSwap`, `addGeocentricUnitConversionIntoPROJString`, `ProjectedCRS::addUnitConvertAndAxisSwap`
- `datum.cpp`: `Ellipsoid::_exportToPROJString`, `PrimeMeridian::_exportToPROJString`, `lookForProjWellKnownEllps`, `getPROJStringWellKnownName`
- `apps/projinfo_lib.cpp`: `outputObject` (the `-o PROJ` and `-q` paths)

## What is written

`Transformation::export_to_proj_string(ProjLines)` and `ConcatenatedOperation::export_to_proj_string(ProjLines)`. `ProjLines::Multi` is the default layout of `projinfo`; `ProjLines::Single` is `projinfo --single-line`.

| Method (EPSG code) | PROJ step(s) |
|---|---|
| Geocentric translations (1031, 9603, 1035), position vector (1033, 9606, 1037), coordinate frame (1032, 9607, 1038, 1149), full matrix (1132, 1133, 1140), time-dependent PV (1053–1055) and CF (1056–1058) | `helmert`, wrapped in `cart`/`push v_3` and `pop v_3` for a geographic end, with `convention` |
| Affine parametric (9624), 3D affine (name), similarity (9621) | `affine` |
| Cartesian grid offsets (9656) | `affine` with `xoff`/`yoff`, and the projected unit/axis step of each end |
| Longitude rotation (9601) | `longlat` with `+pm`, inverted unless the negated offset is a well-known meridian |
| Axis order reversal (9843, 9844) | `axisswap order=2,1`, and `unitconvert` when the two axis units differ |
| Height/depth reversal (1068) | `axisswap order=1,2,-3` |
| Geographic/geocentric (9602) | `longlat` or `cart` on each end, inverted on the source |
| Change of vertical unit (1069) | `unitconvert z_in z_out`, or `affine s33` for another factor |
| Inverted operation (`Transformation::inverse`) | the forward pipeline between `startInversion`/`stopInversion` |
| Concatenated step with `reverse` direction | the step between `startInversion`/`stopInversion` |

## API mapping

| PROJ | Rust |
|---|---|
| `IPROJStringExportable::exportToPROJString` | `Transformation::export_to_proj_string(lines)`, `ConcatenatedOperation::export_to_proj_string(lines)` |
| `PROJStringFormatter` | `pipeline::PipelineFormatter` (crate-private) |
| `PROJStringFormatter::Convention` | not modelled, PROJ-5 only |
| `setMultiLine` / `setCoordinateOperationOptimizations` | `ProjLines` / `set_optimize_units` |
| `Transformation::_exportToPROJString` | `Transformation::export_steps` (crate-private), `export_forward` |
| `SingleOperation::exportToPROJStringGeneric` | `export_forward` and the per-method functions in `transform_export.rs` |
| `setupPROJGeodeticSourceCRS` / `TargetCRS` | `setup_source` / `setup_target` |
| `extractGeographicCRSIfGeographicCRSOrEquivalent` | `extract_geographic` |
| `GeographicCRS::_exportToPROJString` (non-CRS) | `geographic_export` |
| `GeodeticCRS::_exportToPROJString` (geocentric) | `geodetic_export` |
| `addAngularUnitConvertAndAxisSwap` | `angular_unit_and_axis_swap` |
| `addAxisSwap` | `axis_swap` |
| `ProjectedCRS::addUnitConvertAndAxisSwap` | `projected_unit_and_axis_swap` |
| `addGeocentricUnitConversionIntoPROJString` | `geocentric_unit_steps` |
| `Ellipsoid::_exportToPROJString` | `projstring::ellipsoid_params` |
| `PrimeMeridian::_exportToPROJString` | `projstring::prime_meridian_params`, `prime_meridian_radians_params` |
| `PrimeMeridian::getPROJStringWellKnownName` | `projstring::well_known_pm_name` |
| `lookForProjWellKnownEllps` | `projstring::well_known_ellps`, over `ellipsoid::PJ_ELLPS` |
| `UnitOfMeasure::exportToPROJString` | `projstring::proj_unit_name` (`projections::linear_proj_unit`, `angular_proj_unit`) |
| `formatToString` / `%.15g` | `pipeline::proj_number`, `pipeline::g15` |
| `InverseCoordinateOperation::_exportToPROJString` | `Transformation::inverted` |
| `ConcatenatedOperation::_exportToPROJString` | `ConcatenatedOperation::export_steps` |
| `projinfo` `operation_proj` / `outputObject` | `src/bin/projinfo.rs` `operation_proj`, `operation_bodies` |

## Tests mapping

`tests/transform_export.rs`. The expected text of each test is `projinfo -k operation EPSG:<code> -o PROJ -q` from PROJ 9.9.0, run once and pasted in.

| Test | EPSG code | Method |
|---|---|---|
| `geog2d_geocentric_translations_9603` | 1055 | geocentric translations, geog2D |
| `position_vector_geog2d_9606_with_geog_source` | 1074 | position vector, geog2D, flattened `+a +b` ellipsoid |
| `coordinate_frame_geog2d_9607_push_v3` | 1024 | coordinate frame, geog2D |
| `coordinate_frame_geog2d_9607_second` | 1056 | coordinate frame, geog2D (second case) |
| `geocentric_translations_1031` | 7817 | geocentric translations, geocentric |
| `position_vector_geocentric_1033` | 6936 | position vector, geocentric |
| `coordinate_frame_geocentric_1032_is_noop` | 7666 | coordinate frame whose parameters are all zero: `+proj=noop` |
| `time_dependent_position_vector_1053` | 5900 | time-dependent position vector, geocentric |
| `time_dependent_coordinate_frame_1056` | 6276 | time-dependent coordinate frame, geocentric |
| `full_matrix_geog2d_1133` | 10676 | full matrix (`+exact`), geog2D |
| `full_matrix_geog3d_1140` | 10646 | full matrix, geog3D (`unitconvert` with `z`) |
| `coordinate_frame_geog3d_to_compound_1149` | 10905 | coordinate frame, geog3D to compound |
| `similarity_9621` | 3929 | similarity, `affine` |
| `affine_9624` | 10087 | affine parametric |
| `cartesian_grid_offsets_9656` | 6724 | Cartesian grid offsets, engineering CRS to projected |
| `concatenated_helmert_with_pm` | 3966 | concatenated: longitude rotation then Helmert |
| `molodensky_badekas_step_is_unimplemented` | 5192 | concatenated with a Molodensky-Badekas step: `Unimplemented` |

`projinfo` yaml: `test_projinfo.yaml#105` (`-s "Christmas Island Grid 1985" -t "GDA94 / MGA zone 48" -o PROJ -q`) now matches.

## Deliberate differences

- **Unimplemented methods are errors.** A method this crate does not write (grid shifts, Molodensky and Molodensky-Badekas, geographic and vertical offsets, deformation models, seismic bin grids, NZ deformation model, point motion) returns `FactoryError::Format("Unimplemented <name>")`, the message PROJ uses. `projinfo` then prints no PROJ string for that object. PROJ prints a warning and the string of the step it has.
- **Conversions in a concatenated operation are errors.** A `Conversion` step (for example geographic to geocentric) needs the CRS-aware export of `Conversion::_exportToPROJString`, which this crate does not have for every method. The error says so.
- **Coordinate epochs are not written.** PROJ writes `set v_4=<year>` before and after the Helmert when the source or target coordinate epoch is set. The crate does not store coordinate epochs on `Transformation`, so the `set` steps are missing.
- **Some pipeline rewrites are not ported** (`src/iso19111/pipeline.rs` header): the grid push/pop rewriting around `hgridshift`/`vgridshift`, the Krovak `axis=swu` merge, and the `set` epoch merge. A pipeline that skips one is the same transform with more steps.
- **Quoting.** PROJ quotes a parameter value that has a space. This writer never writes a value with a space: names come from `ellps` and `pm` tables and from numbers.
- **Derived ellipsoid parameters.** `ellipsoid_params` writes `+rf` only when the database declared an inverse flattening, and `+b` otherwise, as `Ellipsoid::_exportToPROJString` does. The name lookup uses the computed `rf` or `b`, as `lookForProjWellKnownEllps` does.
- **Equivalence in longitude rotation.** `ellipsoids_equivalent` compares `a` and the inverse flattening, both to `1e-10` relative, names aside. PROJ's `Ellipsoid::_isEquivalentTo` (EQUIVALENT) also compares the semi-minor axis when both ellipsoids have one, and is lax for `Clarke 1880 mod.`. The crate's `Ellipsoid::is_equivalent_to` is the strict one, names included, so it is not used. A longitude rotation between two ellipsoids that differ only in how they are declared may be rejected here and accepted by PROJ.
- **Prime meridian name.** The name is found by longitude, as PROJ does, for every prime meridian. Before this lot, `paris` was written by name or by EPSG code 8903 only.

## Changes outside the new module

- `src/iso19111/pipeline.rs` (new): `ProjLines`, `PipelineFormatter`, `proj_number`, `g15`.
- `src/iso19111/transform_export.rs` (new): the export of `Transformation` and `ConcatenatedOperation`.
- `src/iso19111/projstring.rs`: `ellipsoid_proj` and `prime_meridian_proj` are built on `ellipsoid_params` and `prime_meridian_params`, which return key/value pairs and use `proj_number`. `well_known_ellps` now searches the whole PROJ table. The helpers the new module needs are `pub(crate)`.
- `src/ellipsoid.rs`: `PJ_ELLPS`, the PROJ `pj_ellps` table, moved from `src/engine.rs` (`named_ellipsoid` now reads it). Same rows, same order.
- `src/iso19111/wkt.rs`: `format_g` writes the exponent as C does (`e-05`, `e+15`), not Rust's `e-5`. The fix reaches WKT output too; no test depended on the old form.
- `src/projections/conversions.rs`: `angular_proj_unit`, the angular counterpart of `linear_proj_unit`.
- `src/bin/projinfo.rs`: `operation_proj` exports transformations and concatenated operations; `-q` on candidates prints only the first operation, without headers; candidates print the PROJ string only when `PROJ` is among the requested formats, and the WKT2:2019 string only when it is.
- `src/iso19111/buildop.rs`: its private `proj_number` is the one in `pipeline.rs`; `names_match` is `pub(super)`.

## Open against the yaml suite

The export is not what blocks most open cases. Counts from `docs/UNPORTED-CLI.md` for this lot:

- `projinfo`: 60 open before, 59 after, out of 111 (`test_projinfo.yaml#105`). Measured on `main` at `a748dec`, the base of this lot after the rebase.
- `cs2cs`: 45 open before, 45 after, out of 143. `cs2cs` does not reach the operation factory: `Proj::new_known_crs` picks its datum step from the Helmert table of `proj.db` (`Crs::datum_bridge`), so an exported operation is never used by it. Routing `cs2cs` through `CoordinateOperationFactory::create_operations` is a separate lot, with its own risk to the cases that pass today.
- `projinfo` candidate lists are the main gap behind the rest: for instance `-s EPSG:5613 -t EPSG:4977` gives 0 candidates against 2 in PROJ, and `-s "WGS 84 (G1150) + EGM2008 height" -t "WGS 84 (G1674) + EGM2008 height" -q` gives 0 candidates where PROJ prints a pipeline. The factory does not split a compound CRS into its components, and `-s EPSG:4326 -t EPSG:4204` gives 0 because it does not list the inverse of a registered operation (the registered direction is `EPSG:4204 → EPSG:4326`).
- `test_projinfo.yaml#86`, `#38`, `#39`, `#27`, `#26`, `#52`–`#57`, `#58`–`#64` also need grid shifts or deformation models (`vgridshift`, `hgridshift`, `defmodel`, `tinshift`), which this crate does not write.

## Quirks copied on purpose

- `ProjectedCRS::addUnitConvertAndAxisSwap` swaps only when the axis order is not east-north, and writes `2,1` for a UPS north/south pair named northing/easting, as PROJ does.
- A transform whose parameters are all zero leaves no step, and prints `+proj=noop`.
- `unitconvert` is dropped when it does nothing only after a Helmert step has been written (`coordOperationOptimizations`), as PROJ does.
- A `Longitude rotation` offset that is not a well-known meridian is written inverted (`+inv +proj=longlat +pm=<offset>`), and a well-known one is written forward with its name.
- `pm` in degrees is `%.15g` of the longitude over `π/180`, after the one-tenth rounding of `formatToString`.

## Verification

Commands run on branch `lot-transform-export`:

- `cargo test --test transform_export`: 17 passed, against `projinfo -k operation` from PROJ 9.9.0.
- `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`: passes; no previously passing case fails.
- `PATH=… cargo test --all-features --no-fail-fast`: all 22 test binaries pass.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt --check`: clean.

No bench case is added. `src/bin/bench.rs` times point transforms; an operation export is not a per-point operation, so a bench case here would measure something else.
