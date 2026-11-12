# Port: operation factory, vertical and bound cases (status sheet, 0 of 5 closed)

Source: PROJ 9.9.0, `test/unit/test_operationfactory.cpp` (the tag `9.9.0`, read from
`target/proj-reference/src/test/unit/`). The five `TEST()` names of this lot are in
`docs/UNPORTED-UNIT.md`. Expected names and PROJ strings are copied from that file, or from
`cs2cs`/`projinfo` 9.9.0 built in `target/proj-reference/build/bin` when the C++ test is not run by PROJ.

This lot closes no unit test. It writes the five characterization tests, leaves them `#[ignore]`d with
the cause of each difference, and changes no line of `src/`. No line is removed from
`docs/UNPORTED-UNIT.md`, so the generated docs do not change.

## Result per test

| PROJ `TEST()` | Rust test | Status | Cause |
|---|---|---|---|
| `geog3DCRS_to_geog2DCRS_plus_vertCRS_context` | `operation_geog3d_to_geog2d_plus_vert_crs_context` (`tests/unit_op_vertical_bound_context.rs`) | open | The C++ test is under `#if no_longer_work_since_epsg_13_001` (line 1053), so PROJ 9.9.0 does not run it. The first candidate matches (PROJ string and name, `Inverse of ETRS89-CZE [2007] to Baltic 1957 height (2)`). The second has the PROJ string, but its name is `Inverse of 'Inverse of '...''` here, where `projinfo` 9.9.0 has `'ETRS89-SVK [SKTRF09] to Baltic 1957 height (from ...)'`. The factory builds that candidate as the inverse of an already inverted operation. |
| `boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not` | `operation_bound_crs_to_geog_hub_and_target_same_but_base_not` | open | The Rust chain is one `unitconvert +z_in=us-ft +z_out=m`. PROJ 9.9.0 (`projinfo`, same WKT) gives `+proj=unitconvert +xy_in=deg +z_in=us-ft +xy_out=deg +z_out=m`: its concatenated operation (`Transformation from NAD83 (Ellipsoid (US Feet)) to NAD83 + Transformation from NAD83 to WGS84`) keeps the horizontal degree identity. The Rust factory does not build that concatenation. |
| `nadgrids_with_pm` | `operation_nadgrids_with_pm_to_epsg_4326`, `..._to_epsg_4258` (pass), `operation_nadgrids_with_pm_wkt2_round_trip` (open) | open | The two first blocks pass. The WKT2 round trip of the bound CRS adds `+step +proj=longlat +ellps=intl +pm=lisbon` before the grid. The in-memory operation is named `... + unknown (with Greenwich prime meridian) to WGS84 ...` and has the prime meridian change; the imported one loses it, so the exporter writes the longlat step. |
| `compoundCRS_to_compoundCRS_with_vertical_transform` | `operation_compound_to_compound_with_vertical_transform` | open | The first candidate has no `vgridshift`: the bound vertical part (ODN height to the anonymous vertical CRS, VERTCON `bla.gtx`) is dropped when its hub is the target vertical CRS. The approx-tmerc assertions (`setUseApproxTMerc(true)`) are not written: `CoordinateOperation` has no approx export (only `Conversion::export_to_proj_string_approx_tmerc`). |
| `compoundCRS_to_compoundCRS_context` | `operation_compound_to_compound_context` (`tests/unit_op_compound_to_compound_2.rs`, already `#[ignore]`d in the previous lot) | open | The first candidate's name and PROJ string are right. The WKT2 round trip is not: after import the steps are reordered (`unitconvert +z_in=us-ft` first, then `vgridshift`, `axisswap`, `hgridshift`). Re-checked in this lot. |

## Function map (Rust, where the causes are)

| PROJ 9.9.0 | Rust |
|---|---|
| `createOperationsCompoundToCompound` (vertical pass) | `compound.rs`, `compound_to_compound`, `compound_vertical_pass` |
| `createOperationsCompoundToGeog` (bound horizontal part) | `compound.rs`, `compound_to_geog3d`, `bound_compound_to_geog3d` (line 2402) |
| `Conversion::_exportToPROJString`, projection part | `conversion_export.rs`, `export_projection` (141) |
| Change of vertical unit, `exportToPROJStringChangeVerticalUnit` | `transform_export.rs`, `change_vertical_unit` (1798), `export_vertical_unit_factor` (1808) |
| Transformation export dispatch | `transform_export.rs`, `export_forward` (256) |

## Deliberate differences and not ported

- The disabled C++ test (`#if no_longer_work_since_epsg_13_001`) keeps its PROJ strings, but its inverse names
  are not used as written: `projinfo` 9.9.0 with the same database prints `ETRS89-CZE [2007]` and `ETRS89-SVK [SKTRF09]`.
- The approx-tmerc assertions of `compoundCRS_to_compoundCRS_with_vertical_transform` need an operation-level
  approx export that does not exist in this crate. Not added in this lot.

## Next steps

1. Prime meridian: after WKT import of a bound CRS whose base has a non-Greenwich meridian, keep the
   prime meridian change operation (`nadgrids_with_pm` round trip). Trace the factory with the base
   geographic CRS of the imported bound.
2. Vertical part of a compound when the bound hub is the target vertical (`..._with_vertical_transform`).
3. Concatenation of a null geographic offset with a vertical unit change, keeping the degree identity
   (`boundCRS_to_geogCRS_...`).
4. Round trip step order (`compoundCRS_to_compoundCRS_context`), in WKT import of a concatenated operation.

## Commands run

- `cargo test --test unit_op_nadgrids_with_pm` (2 passed, 1 ignored); `cargo test --test unit_op_vertical_bound_context`
  (all three ignored; with `-- --include-ignored` the failures are the ones listed above).
- `cargo test --test unit_op_compound_to_compound_2 operation_compound_to_compound_context -- --include-ignored`
  (fails at the WKT2 round trip, as before).
- Reference: `PROJ_DATA=target/proj-reference/build/data target/proj-reference/build/bin/projinfo -s ... -t ... -o PROJ --spatial-test intersects`
  for the ETRS89 to Baltic pair, and for the WKT of the bound CRS (`-t EPSG:4979`).
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
