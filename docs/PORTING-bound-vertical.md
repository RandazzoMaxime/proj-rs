# Port — bound CRS to WGS 84 and vertical CRS PROJ strings

Sources, PROJ 9.9.0:

- `src/iso19111/crs.cpp`: `CRS::createBoundCRSToWGS84IfPossible` (line ~516), `CRS::extractGeodeticCRSRaw` (line 252), `VerticalCRS::_exportToPROJString` (line 3722), `BoundCRS::_exportToPROJString` (line 6175), `BoundCRS::getVDatumPROJ4GRIDS` (line 6049), `BoundCRS::isTOWGS84Compatible` (line 6025), `GeodeticCRS::addDatumInfoToPROJString` (line 2538), `GeographicCRS::_exportToPROJString` (line 3426)
- `src/iso19111/operation/transformation.cpp`: `Transformation::getTOWGS84Parameters` (line 194)
- `src/iso19111/operation/singleoperation.cpp`: `Transformation::isGeographic3DToGravityRelatedHeight` (line 2206), `getHeightToGeographic3DFilename` (line 2283), `SingleOperation::gridsNeeded` (line 1756)
- `src/iso19111/factory.cpp`: `DatabaseContext::lookForGridInfo` (line 3398), `DatabaseContext::getAllowedAuthorities` (line 3814)
- `src/iso19111/metadata.cpp`: `Identifier::canonicalizeName` (line 1285), `Identifier::isEquivalentName` (line 1383)
- `src/iso19111/factory.cpp`: `AuthorityFactory::createObjectsFromNameEx` (line 9363), used by `projinfo`'s name search
- `src/apps/projinfo_lib.cpp`: `buildObject` (line 333, the `-k crs` name search), `outputObject` (line 580, the PROJ string calls `createBoundCRSToWGS84IfPossible` for every CRS)

## What is written

| Rust | PROJ |
|---|---|
| `Crs::create_bound_crs_to_wgs84_if_possible(db)` | `CRS::createBoundCRSToWGS84IfPossible` |
| `Transformation::towgs84_lenient()`, `towgs84_strict()` | `getTOWGS84Parameters(false)`, `getTOWGS84Parameters(true)` |
| `Transformation::height_to_geographic_3d_filename()` | `getHeightToGeographic3DFilename` |
| `DatabaseContext::grid_is_known(name)` | `lookForGridInfo` with `considerKnownGridsAsAvailable` |
| `DatabaseContext::proj_grid_name(name)` | the `grid_alternatives` rename of `substitutePROJAlternativeGridNames` |
| `equivalent_name_bigger_differences(a, b)` | `Identifier::isEquivalentName(a, b)`, the two-argument form |
| `Crs::Vertical` and `Crs::Bound` in `Crs::export_to_proj_string` | `VerticalCRS::_exportToPROJString`, `BoundCRS::_exportToPROJString` |
| `projinfo`: `proj_text` binds the CRS before exporting | `outputObject` |

Helmert methods read by `towgs84_*`: coordinate frame (1032, 1132, 9607, 1133, 1140, 1038, 1149, and any 7-parameter `Coordinate Frame` name), position vector (1033, 9606, 1037), geocentric translations (1031, 9603, 1035). The rotation signs follow PROJ: inverted for coordinate frame only.

## Tests mapping

No PROJ unit test is ported for these functions. The behaviour is checked by the CLI cases of `tests/cli_yaml/test_projinfo.yaml` and by the Rust tests below:

| yaml case | Rust test |
|---|---|
| `#110` `projinfo -k crs EGM2008` | `bound::tests::vertical_crs_binds_to_its_known_geoid_grid` (and the CLI case) |
| `#24` `projinfo -o PROJ EPSG:25832` | `bound::tests::projected_crs_on_etrs89_carries_the_helmert_towgs84` (and the CLI case) |
| `#109` `projinfo EGM2008` (keeps passing) | CLI case |
| WGS 84 itself is not bound | `bound::tests::wgs84_itself_is_not_bound` |
| `EGM2008 height` ≡ `EGM2008`, `WGS 84 + EGM2008 height` ≢ `EGM2008` | `equiv::bigger_differences_tests` |

## Deliberate differences

- **No spatial filter.** PROJ's `createOperations` gets the extent of the CRS (`extentResolved`). The Rust `create_operations` does not filter by extent. So the candidate count is taken over every operation the database holds for the pair. Cases where PROJ's extent leaves one candidate and the Rust list holds several do not bind. `#83` (MGI 1901, several Helmert operations) stays open for this reason.
- **Compound CRS.** PROJ binds each component of a compound CRS. Not ported: a compound CRS is returned as it is.
- **Concatenated candidates.** PROJ also accepts a concatenated operation whose second step is a Helmert (NTF, EPSG:4807). Not ported.
- **Geocentric CRS.** PROJ binds a geocentric CRS to EPSG:4978. Not ported: returned as it is.
- **Horizontal nadgrids.** `BoundCRS::getHDatumPROJ4GRIDS` (`+nadgrids=`) is not written. A candidate bound from `create_bound_crs_to_wgs84_if_possible` is always a Helmert, so the branch is unreachable from there. A user-built bound CRS over a grid transformation would lose its `+nadgrids=`.
- **Ballpark.** The database list has no ballpark operation (the Rust factory creates none), so `hasBallparkTransformation` is always false. The `Ballpark geo` name check of the horizontal branch is kept.
- **Grid availability.** `lookForGridInfo` also tests the local file through `gridpath::resolve` and does not start a download. PROJ can test through the network when it is enabled.
- **Grid renames.** The Rust `create_operations` keeps the official grid name. The bound creation applies the `grid_alternatives` rename to the candidate only (`with_proj_grid_names`), as PROJ does for the candidates of `createOperations`. The transformation export (`transform_export.rs`) is not changed.
- **Authority.** PROJ walks the authorities of `getAllowedAuthorities`. Here the authority is the code space of the first identifier, or any authority when there is none.
- **`--3d`, `--normalize-axis-order`** are not part of this lot. `#28` (`--3d`) still fails.

## Quirks kept

- The vertical geoid grid is read from the transformation, not from the vertical CRS. `+geoid_crs=WGS84` is written whenever the hub is named `WGS 84`, as PROJ does.
- A horizontal bound writes `+towgs84` after `+ellps` and the prime meridian, and drops the datum token (`+datum=`) that would otherwise be written, as `addDatumInfoToPROJString` does.
- `equivalent_name` (the strict form) is unchanged: the factory's exact-match path still uses it. Only the `projinfo` name search uses the bigger-differences form.

## Verification

Run on this branch, rebased on `origin/main` at `eaaf929`:

- `cargo fmt --check`: clean.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --all-features --no-fail-fast` with `PATH=…/proj-ref/bin`, `PROJ_CS2CS_REQUIRED=1`, `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`: all test binaries `ok`, exit 0.
- `cargo test --test cli_yaml` on this branch: `ok`, no regression. Open cases on `origin/main`: 59 projinfo, 43 cs2cs. On this branch: 57 projinfo, 43 cs2cs. The two projinfo cases that close are `#110` and `#24`, and no case opens.

`docs/UNPORTED-CLI.md` is rewritten by the harness and is not part of this lot.
