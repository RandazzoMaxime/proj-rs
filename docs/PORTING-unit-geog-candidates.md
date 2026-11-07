# Port — geographic candidates between geographic and projected CRS (lot `geog-candidates`)

Source: PROJ 9.9.0, tag `9.9.0` of OSGeo/PROJ. The PROJ lines cited below are from
`src/iso19111/operation/coordinateoperationfactory.cpp`, `operation/singleoperation.cpp`,
`operation/concatenatedoperation.cpp`, `datum.cpp` and `common.cpp`, and from
`test/unit/test_operationfactory.cpp`.

The lot is about the candidate list between two geodetic CRS: which operations are kept, in which
order, and what each exports to. It is run as one cause per commit. Each section says what was
wrong, what PROJ does, and which tests the commit closes.

## Method and reference

The reference is PROJ 9.9.0 `projinfo` and a traced build of the same tag (`PROJ_DEBUG=3`). For the
candidate of a pair, the PROJ JSON (`projinfo -o PROJJSON`) gives the source, target and
interpolation CRS, and the PROJ string of each step. The expected values are the ones of the five
`TEST()` bodies of `test_operationfactory.cpp`, copied into `tests/unit_op_geog_candidates.rs`.

## Function map

| PROJ 9.9.0 | Rust (`src/iso19111`) |
|---|---|
| `createOperationsFromDatabase`, steps 2 to 4 (coordinateoperationfactory.cpp:4247-4582) | `factory.rs`, `create_candidates_at`: the datum pivot, the CRS-codes search and the intermediate block |
| `createOperationsWithDatumPivot` (3167) | `factory.rs`, `datum_pivot_operations` |
| `isSameGeodeticDatum` with the database (2056-2066), `GeodeticReferenceFrame::hasEquivalentNameToUsingAlias` (datum.cpp:1602) | `equiv.rs`, `same_geodetic_datum_with_database` |
| `createOperationsGeogToGeog`, `sameDatum` (2961), `createBallparkGeographicOffset` (2298) | `geodetic_ops.rs`, `geographic_offset`, through `BallparkRule::ByDatum { same_datum }` |
| `createOperationsGeodToGeod`, the ballpark appended after the registry (3670-3673) | `factory.rs`, the geodetic ballpark block of `create_candidates_at` |
| `createOperationsEnsembleCRSToOtherGeodCRS`, interpolation CRS of a member operation (4047-4170) | `factory.rs`, `ensemble_to_other_geodetic`, `interpolation_of`, `set_interpolation_of` |
| `FilterResults::sort`, `SortFunction` (913-1181, 1444-1552), `gridsNeeded` (singleoperation.cpp:1756) | `select.rs`, `sort_candidates`, `characteristics`, `precedes` |
| `setUsePROJAlternativeGridNames` (`usePROJNames_`, default true) | `factory.rs`, `CoordinateOperationContext::set_alternative_grid_names`, `registry_grid_names`, `sort_grid_names` |

## Commit 1 — the intermediate CRS search runs under `IntermediateCRSUse::ALWAYS` (closes 3 tests)

### What was wrong

With `IntermediateCRSUse::ALWAYS`, PROJ reads the registered CRS between the two geodetic CRS
(`findsOpsInRegistryWithIntermediate`) whatever the direct and datum-pivot results are
(coordinateoperationfactory.cpp:4449-4457). The port read them only when the direct search and the
datum pivot were both empty. Under ALWAYS the registered pivots were therefore missing: for
`IGNF:LAMB1` to `EPSG:4326`, the second operation (the Helmert through WGS 84 of `NTF (Paris)`) was
absent, and for `NTF (Paris)` to UTM 31N the list had one operation where PROJ keeps four.

### What is now ported

- Step 3 (the CRS-codes search) runs under ALWAYS whatever the other results, and under
  `IF_NO_DIRECT` only when there is no direct operation and no datum pivot, as before.
- Step 2 (the datum pivot) is still tried only when the direct search is empty
  (coordinateoperationfactory.cpp:4404).
- Step 4 (the intermediate block) runs after a non-empty direct search when its operations are
  replaced (`useOnlyReplacedOperations`, 4467-4473). The port reaches it now, as PROJ does.

With the pivots in the list, the ballpark that PROJ appends last is dropped by
`removeSyntheticNullTransforms`: an operation without grid contains the area of interest.

### Tests

| Rust test | PROJ `TEST()` | Status |
|---|---|---|
| `operation_geog_crs_to_geog_crs_ch1903_to_ch1903plus_context` | `geogCRS_to_geogCRS_CH1903_to_CH1903plus_context` | closed, commit 1 |
| `operation_geog_crs_different_from_base_crs_to_proj_crs_context_compatible_area` | `geogCRS_different_from_baseCRS_to_projCRS_context_compatible_area` | closed, commit 1 |
| `operation_ignf_lamb1_to_epsg_4326` | `IGNF_LAMB1_TO_EPSG_4326` | closed, commit 1 |

The accuracy of the pivot operation is read with `accuracy()`, which is 1.0. PROJ's
`coordinateOperationAccuracies()[0]->value()` is the text `"1"`. See the deviations below.

## Commit 2 — the same datum is decided with the database (closes 1 test)

### What was wrong

PROJ's `isSameGeodeticDatum` takes the database (coordinateoperationfactory.cpp:2056-2066). The
datum of a geodetic CRS is `datumNonNull(db)`: an ensemble is read as its registry datum
(`DatumEnsemble::asDatum`, datum.cpp:1943), and two datum names are equivalent when one is an alias
of the official name of the identifier of a datum (`hasEquivalentNameToUsingAlias`, datum.cpp:1602).
The port compared the datum names as written, with no database. A 3D source whose datum is `WGS84`
was therefore not the datum of `EPSG:4326`, and the port took the datum pivot and a
"Ballpark geographic offset", where PROJ gives a null offset named after the source CRS.

### What is now ported

- `equiv::same_geodetic_datum_with_database` compares the two datums with the `EQUIVALENT`
  criterion, the database aliases (`frame_names_equivalent_by_db`, which already served the CRS
  comparison), the tolerant ellipsoid and the prime meridian (`isEquivalentToNoExactTypeCheck`,
  datum.cpp:1572).
- The factory uses it for the `sameGeodeticDatum` gates of `createOperationsFromDatabase`
  (4402-4404, 4449, 4468, 4524), for the ballpark of `createOperationsGeodToGeod`, and for the
  datum pivot.
- `BallparkRule::ByDatum` carries the decision: `{ same_datum }`. `geographic_offset` names the null
  offset from the source CRS, as `createBallparkGeographicOffset` does.

### Not ported

The name-only `same_geodetic_datum` remains in the callers listed under the deviations.

### Tests

| Rust test | PROJ `TEST()` | Status |
|---|---|---|
| `operation_geog_crs_3d_source_datum_name_is_alias_to_geog_crs` | `geogCRS_3D_source_datum_name_is_alias_to_geogCRS` | closed, commit 2 |

## Commit 3 — the interpolation CRS of a member operation is the ensemble CRS (no test closed alone)

### What was wrong

In `createOperationsEnsembleCRSToOtherGeodCRS`, PROJ replaces the interpolation CRS of a member
operation that is the member CRS by the ensemble CRS (coordinateoperationfactory.cpp:4161-4170,
`setInterpolationCRS`). The port kept the member CRS. For the RGF93 grid operations between NTF and
ETRS89 (EPSG:4258, an ensemble), the interpolation CRS was then neither the source nor the target
of the operation, and their PROJ strings were refused ("interpolation CRS should be the source or
target CRS"). They were dropped from the candidates.

### What is now ported

The replacement, in the branch of `ensemble_to_other_geodetic` where the member is the source's
nature. The three RGF93 operations (EPSG:9327, 9888, 9889) export with `grid_ref=output_crs`, and the
candidate list of NTF to ETRS89 is the one PROJ gives: EPSG:9888, the inverse of EPSG:15959, and
EPSG:1651.

## Commit 4 — the candidates are sorted on the grid names the context sets (closes 1 test)

### What was wrong

`SortFunction` reads the grids of each candidate with `gridsNeeded(dbContext)`
(coordinateoperationfactory.cpp:1480-1498), under the names the operation carries. These are the
PROJ names when the registered operations are read with `usePROJAlternativeGridNames`, which is
PROJ's default, and the database names when the caller turns it off
(`setUsePROJAlternativeGridNames(false)`, `findOpsInRegistryDirect`, 1927-1931). The port always
looked up the PROJ names. With the names turned off, the grids known to the database were not told
apart, and the order of `inverse_needed` differed from PROJ's.

### What is now ported

- `CoordinateOperationContext` keeps the grid names the caller sets, if any. Unset, the candidates
  keep the database names, and they are sorted on the PROJ names, as before. Set to `Official`,
  they are sorted on the database names. Set to `Proj`, both use the PROJ names.
- `select::characteristics` takes the mode, and reads the grids as the sort of PROJ does.

### Deviation: the default

PROJ's default is `usePROJNames_ = true`. The port keeps `Official` as the default for the
candidates, because `Proj` by default changes them: a NADCON-derived operation of
`Inverse of NAD83(NSRS2007) to NAD83(2011) (1)` is then read with a method the export does not write
("Unimplemented"), and `unit_op_compound_vertical_units` fails
(`operation_compound_ft_to_geog_ft`, `operation_compound_m_to_geog_ft`). Not kept. It needs the
export of that method first.

### Tests

| Rust test | PROJ `TEST()` | Status |
|---|---|---|
| `operation_geog_crs_to_geog_crs_context_inverse_needed` | `geogCRS_to_geogCRS_context_inverse_needed` | closed, commit 4 (its three blocks: the names set to `Official`, the default, and the reverse pair) |

## Tests of the lot

File `tests/unit_op_geog_candidates.rs` (PROJ 9.9.0 expected strings, copied from
`test/unit/test_operationfactory.cpp`). Five tests, all passing. The five ledger lines are removed
from `docs/UNPORTED-UNIT.md`, and the counts regenerated (`UPDATE_DOCS=1 cargo test --test
docs_in_sync --test progress_svg`).

## Deviations and open points

- `CoordinateOperation::accuracy_text()` returns the text the database wrote, for a transformation
  only. A computed concatenated operation has the accuracy `1.0`, which PROJ writes as `"1"`. The
  pivot test checks `accuracy() == Some(1.0)`. Exposing the computed text needs a field on
  `ConcatenatedOperation`, not done in this lot.
- The default grid names of the candidates stay `Official` (commit 4, the deviation above).
- The name-only `same_geodetic_datum` remains in: `geodetic_ops` (`geographic_geocentric_ballpark`,
  the spherical planetocentric path, the forced geographic/geocentric case), `compound.rs` (four
  callers), `bound_ops.rs` (two), and the recursion checks of `factory.rs`
  (`intermediate_through_candidates`, `pivot_allowed`, the ensemble step). They may differ from PROJ on an
  alias datum name. Open.
- The members of an ensemble that carry no identifier are not ported (existing note on
  `ensemble_to_other_geodetic`).

## Commands run

- After each commit: `cargo test --test unit_op_*`, `unit_capi_*`, `unit_crs_*` (82 binaries, no
  fail-fast) and `cargo test --all-features --test cli_yaml` under `ulimit -s 1024`, with the
  reference `cs2cs` 9.9.0 on `PATH`, `PROJ_DATA` set to the reference database, `PROJ_CS2CS_REQUIRED=1`,
  `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`. `docs/UNPORTED-CLI.md` not reopened.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- Final bar, at the head of the lot (commit 4 and this sheet): `cargo test --all-features --no-fail-fast`
  with the reference environment: 1671 passed, 0 failed, 31 ignored (173 `test result:` lines).
  `cli_yaml` passes under `ulimit -s 1024`. `cargo fmt --check` and clippy with `-D warnings`: clean.
  `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` regenerated, then both pass.
