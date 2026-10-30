# Port — PROJ `test/unit/test_operationfactory.cpp`, geographic pairs with an EPSG context

Sources, PROJ 9.9.0:

- `test/unit/test_operationfactory.cpp`: the tests. This lot takes the tests that build an `AuthorityFactory` context and call `createOperations` between registered geographic CRS, and checks names, EPSG codes, counts, and PROJ strings.
- `src/iso19111/operation/coordinateoperationfactory.cpp`: `createOperations` and `createOperationsGeodToGeod` (celestial body check, line ~5246).

## Tests ported (9)

Each test is one function in `tests/test_operationfactory.rs`, named `operation_<C++ name>` in snake case. The C++ test names have `geogCRS` in them, so the Rust names use `geog_crs`. Each function copies the assertions of its `TEST()`, with the same names, counts, and PROJ strings.

| C++ TEST | Rust test |
|---|---|
| `geogCRS_to_geogCRS_context_default` | `operation_geog_crs_to_geog_crs_context_default` |
| `geogCRS_to_geogCRS_context_filter_accuracy` | `operation_geog_crs_to_geog_crs_context_filter_accuracy` |
| `geogCRS_to_geogCRS_context_filter_bbox` | `operation_geog_crs_to_geog_crs_context_filter_bbox` |
| `geogCRS_to_geogCRS_context_incompatible_area` | `operation_geog_crs_to_geog_crs_context_incompatible_area` |
| `geogCRS_to_geogCRS_longitude_rotation_context` | `operation_geog_crs_to_geog_crs_longitude_rotation_context` |
| `geogCRS_to_geogCRS_context_EPSG_4240_Indian1975_to_EPSG_4326` | `operation_geog_crs_to_geog_crs_context_epsg_4240_indian1975_to_epsg_4326` |
| `geogCRS_to_geogCRS_context_invalid_EPSG_ID` | `operation_geog_crs_to_geog_crs_context_invalid_epsg_id` |
| `geogCRS_to_geogCRS_context_WGS84_G1674_to_WGS84_G1762` | `operation_geog_crs_to_geog_crs_context_wgs84_g1674_to_wgs84_g1762` |
| `geogCRS_to_geogCRS_context_deprecated` | `operation_geog_crs_to_geog_crs_context_deprecated` |

All nine pass on the factory as it was on `origin/main`. No code under `src/` changed in this lot. The tests were written first, and they confirmed the existing factory behavior.

## API added

None.

## Behavior fixed

None.

## Deliberate differences

None. The Rust tests use `ProjLines::Single` for the PROJ strings of transformations and concatenated operations, which is the single-line text PROJ's `exportToPROJString` gives with the default formatter.

## Still open, and why

These 265 names stay in `docs/UNPORTED-UNIT.md`. The causes below are checked by running the factory or by reading the code (`grep` in `src/`). The first group was run against a Rust test that failed, and was then removed from the test file, so it is not in the crate. The rest of the section has not been examined in this lot.

### Failed when tested (count and text differ from PROJ)

1. **`geocentricCRS_to_geogCRS_same_datum_context`** (EPSG:4326 to EPSG:4978): 0 candidates, PROJ gives 1. `create_candidates` builds no geographic-to-geocentric conversion for two CRS on the same datum. The test also calls `CoordinateOperation::inverse`, which is `pub(crate)`.
2. **`geogCRS_to_geogCRS_context_helmert_geocentric_3D`** (EPSG:4348 to EPSG:7842): 1 candidate, but it is the composite `Inverse of Conversion from GDA94 to GDA94 (geocentric) + GDA94 to GDA2020 (1) + Conversion from GDA2020 to GDA2020`. PROJ gives the bare Helmert. The factory routes the geocentric pair through the geographic CRS. The test also needs the public `inverse`.
3. **`geogCRS_to_geogCRS_context_helmert_geog3D_crs`** (EPSG:4939 to EPSG:7843): 1 candidate, as PROJ. The PROJ string wraps the registered 2D operation in `+proj=push +v_3` and `+proj=pop +v_3`, and has no `+z_in=m +z_out=m` on the unit conversions. PROJ builds the operation on the 3D CRS, with no push/pop. The cause is in how the candidate is built for 3D endpoints (not checked further).
4. **`geogCRS_to_geogCRS_context_helmert_geog3D_to_geocentirc`** (EPSG:4939 to EPSG:7842): the same push/pop and unit problem as item 3, and an extra `+proj=cart +ellps=GRS80` step after the `pop`, because the geocentric target goes through a geocentric conversion.
5. **`geogCRS_to_geogCRS_context_match_by_name`**: a NAD27 CRS built with the name of EPSG:4267 and no identifier gives 1 candidate. EPSG:4267 itself gives 79. `create_candidates` looks up the database only by the CRS identifier (`authority_code("EPSG")` or the first identifier). PROJ identifies the CRS first.
6. **`geogCRS_to_geogCRS_context_datum_ensemble`** (ensemble WGS 84 CRS without identifier to EPSG:4258): 1 candidate, `Ballpark geographic offset from ETRS89 to unknown`. PROJ gives `ETRS89 to WGS 84 (1)`. Same cause as item 5.
7. **`geogCRS_to_geogCRS_context_NAD27_to_WGS84`**: 79 candidates, as PROJ, but `list[0]` is `NAD27 to WGS 84 (3)`, not `(33)`. The test sets `IGNORE_GRID_AVAILABILITY`, and the context has no setter for it (`grep` finds no grid-availability setting in `src/`). With the default, the ranking puts `(33)`, whose grid is not on disk here, after `(3)`.
8. **`geogCRS_to_geogCRS_context_ED50_to_WGS72_no_NTF_intermediate`** (EPSG:4230 to EPSG:4322, partial intersection): 1 candidate, the ballpark. PROJ gives at least 2 registered operations. Not investigated further.
9. **`geogCRS_to_geogCRS_context_concatenated_operation_Egypt1907_to_WGS84`** (EPSG:4229 to EPSG:4326): 4 candidates, PROJ gives 3. The port lists `Egypt 1907 to WGS 84 (2)`, `(1)`, the ballpark, and `(4)`. Not investigated further.
10. **`geogCRS_to_geogCRS_geographic_offset_context`** (EPSG:4120 to EPSG:4121): 2 candidates, PROJ gives 1. The port keeps `Ballpark geographic offset from Greek to GGRS87` next to `Greek to GGRS87 (1)`. PROJ returns the registered one only. Not investigated further.

### Blocked by a missing API, or a missing PROJ feature

- **`createOperation(src, dst)` with no authority factory** (`geogCRS_to_geogCRS`, `_noop`, `_longitude_rotation`, `_3D`, and the other `createOperation` tests in the file). The Rust `CoordinateOperationContext::create` needs a `DatabaseContext`, and no `create_operation` exists in `src/` (grep). For EPSG:4807 to EPSG:4326, the database-backed result has a Helmert (`NTF (Paris) to WGS 84 (1)`), and PROJ's no-database result (expected text) does not.
- **Grid availability and alternative grid names** (`geogCRS_to_geogCRS_context_inverse_needed`, `_ntv1_ntv2_ctable2`, `_NAD27_to_WGS84_G1762`, `_lonlat_vs_latlon_crs`, `_concatenated_operation`, `_same_grid_name`, `_CH1903_to_CH1903plus_context`): `setGridAvailabilityUse` and `setUsePROJAlternativeGridNames` are not in the Rust context (grep).
- **Celestial body check** (`geogCRS_to_geogCRS_context_incompatible_celestial_body`, `_incompatible_celestial_body_but_same_radius`, `geogCRS_to_geogCRS_incompatible_unknown_celestial_body`): PROJ raises `UnsupportedOperationException` in `createOperationsGeodToGeod` when the two ellipsoids are on different bodies, or have semi-major axes that differ by more than the tolerance. The port has no such check and returns the ballpark instead. For EPSG:4326 to IAU_2015:51200 (Ananke), it gives `Ballpark geographic offset from WGS 84 to Ananke (2015) - Sphere / Ocentric`. The check would change the candidate list that `cs2cs` and `projinfo` also use, so it is left for the selection work.
- **`geogCRS_to_geogCRS_compatible_unknown_celestial_body`** also reads `DatabaseContext::getQueryCounter`, which the Rust database has no equivalent for.

### Not examined in this lot

The rest of the 265, which are on vertical, compound, bound, projected, derived, point motion, and geocentric pairs with a ballpark, and the `createOperations` pivot tests. They are not checked against the factory.

## Not changed on purpose

- The factory selection (`create_candidates`, the sort, the ballpark rule) is not touched. It is shared with `cs2cs`, `projinfo`, and the `cs2cs` selection work, and each change there moves CLI output.
- The grids are not touched (other agents).
- `docs/UNPORTED-CLI.md` is rewritten by the `cli_yaml` harness during the test run. It is restored with `git checkout` and not committed.
- `CoordinateOperation::inverse` stays `pub(crate)`. Making it public is needed by items 1 and 2, and the change belongs with the fix for those tests.

## Verification

Commands run on branch `lot-unit-opfactory`:

- `cargo test --all-features --test test_operationfactory` with `PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`: 9 passed, 0 failed.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `cargo fmt --check`: clean.
- `cargo test --all-features --no-fail-fast` with the same environment: exit 0, no `FAILED` or `panicked` line. Each binary reports `0 failed`: `proj_rs` lib 96 passed, 1 ignored (`src/projections/nkg.rs`, on `origin/main`, needs the network); `cli_yaml` 1; `test_crs` 105; `test_io` 84; `test_operation` 62; `test_operationfactory` 9; `test_c_api` 50; `test_factory` 58; `transform_export` 17.
- The `cli_yaml` run did not change `docs/UNPORTED-CLI.md`.
