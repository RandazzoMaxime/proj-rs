# Port — lot C vertical and geoid cases

PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, the 12 `operation` cases of the vertical and geoid group. Sources read: `src/iso19111/operation/coordinateoperationfactory.cpp` (9.9.0), fetched from `raw.githubusercontent.com/OSGeo/PROJ/9.9.0` and read locally.

**Result: 0 of 12 cases ported.** No code under `src/` or `tests/` changed, so there is no new test file. Each case is listed below with the first difference found against PROJ.

## C++ functions read

Line numbers are those of `coordinateoperationfactory.cpp` in 9.9.0.

| C++ function | Line | Read |
|---|---|---|
| `createOperations` dispatch, vertical and compound branches | 3630-3800 | yes |
| `createOperationsFromDatabase`, vertical branch (`vertSrc \|\| vertDst`) | 4374-4393 | yes |
| `createOperationsFromDatabaseWithVertCRS` | 5141-5232 | yes |
| `createOperationsGeogToVertWithIntermediateVert` | 4875-4938 | yes |
| `findCandidateVertCRSForDatum` | 4587-4620 | yes |
| `createOperationsVertToVertWithIntermediateVert` | 4952-5005 (head) | partly |
| `createOperationsVertToGeog` | 6070-6110 | yes |
| `createOperationsVertToGeogSynthetized` | 6113-6200 | partly |
| `createOperationsCompoundToGeog` | 6454-7173 | yes |
| `createOperationsCompoundToCompound` | 7214 onward | no |
| `createOperationsGeogToVertFromGeoid` | 4629-4700 | no |
| `createHorizVerticalPROJBased` and the `MyPROJStringExportable*` classes | 2384-2600, 2717-2810 | no |

`createOperationsGeogToVertFromGeoid` is the function the three GEOIDMODEL cases need. It was not read in this session, so no port is attempted for them.

## What the Rust code does today

- `src/iso19111/compound.rs`, `compound_to_geog3d`: the vertical branch of `createOperationsCompoundToGeog`. For a vertical component it calls `create_operations(vertical, target)`. A vertical CRS that is not in the registry for the target gets only `vertical_ballpark` (method 1069, ballpark name).
- `src/iso19111/factory.rs`, `create_candidates`: no `createOperationsFromDatabaseWithVertCRS` hook. No `findCandidateVertCRSForDatum` equivalent: `factory.rs` has `create_geodetic_crs_from_datum` and `create_vertical_crs`, but no query of vertical CRS by datum.
- `CoordinateOperationContext` and `alter_cs_linear_unit`: the altered target (`EPSG:6319` in feet) has no identifiers, so `direct_registry_operations` finds nothing for it.

## Oracle check (projinfo 9.9.0)

Command, same for each source:

```
PROJ_DATA=…/share/proj projinfo -s EPSG:6318+6360 -t EPSG:6319 --spatial-test intersects --grid-check none -o PROJ
```

- `EPSG:6318+6360` (NAD83(2011) + NAVD88 height (ftUS)) to `EPSG:6319`: 3 candidates. Operation 1 is `Conversion from NAVD88 height (ftUS) to NAVD88 height + Inverse of NAD83(2011) to NAVD88 height (3)`, a pipeline with `unitconvert +z_in=us-ft +z_out=m`, `vgridshift +grids=us_noaa_g2018u0.tif`, and the back-conversion.
- `EPSG:6318+6357` (NAD83(2011) + NAVD88 depth): 3 candidates. Operation 1 is `Conversion from NAVD88 depth to NAVD88 height + Inverse of NAD83(2011) to NAVD88 height (3)`, with `axisswap +order=1,2,-3` before `vgridshift`.

The Rust crate, same pairs, same context (`PartialIntersection`, `IgnoreGridAvailability`):

- `EPSG:6318+6360` to `EPSG:6319`: 1 operation, `Transformation from NAVD88 height (ftUS) to NAD83(2011) (ballpark vertical transformation, without ellipsoid height to vertical height correction)`, with no grid.
- `EPSG:6318+6357` to `EPSG:6319`: 1 operation, `Transformation from NAVD88 depth to NAD83(2011) (ballpark …)`, `+proj=affine +s33=-1`.

The metre reference `EPSG:6318+5703` to `EPSG:6319` already gives the 3 grid operations in Rust, through the registry. The gap is therefore only the non-metre vertical CRS and the non-metre target.

## Cases

| C++ TEST | Status | Reason |
|---|---|---|
| `compoundCRS_ftUS_to_geogCRS_ft` | not ported | Rust gives one ballpark operation. PROJ takes `FromDatabaseWithVertCRS` → `GeogToVertWithIntermediateVert` (candidate: NAVD88 height metre, grid via `geog3DToVertTryThroughGeog2D`), then the unit conversions around the grid. Neither is in `create_candidates`. |
| `compoundCRS_ft_to_geogCRS_ft` | not ported | Same as above, from NAVD88 in feet. The expected `res.back()` is `+proj=noop`. Not analysed further. |
| `compoundCRS_m_to_geogCRS_ft` | not ported | The target is a foot CRS with no identifier. The metre vertical `EPSG:5703` to it goes through `FromDatabaseWithVertCRS`, as in the first case. Rust gives a ballpark. |
| `compoundCRS_to_geogCRS_with_height_depth_reversal` | not ported | PROJ gives 3 operations, Rust gives 1 ballpark with `+proj=affine +s33=-1`. Missing: the depth-to-height conversion of the registry candidate (`createOperationsVertToVertWithIntermediateVert`) and the `axisswap +order=1,2,-3` step. |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_height_depth_reversal` | not ported | Same as the depth case, with `NAVD88 depth (ftUS)`. PROJ gives `affine +s33=-0.304800609601219` after `unitconvert`, and 3 operations. Rust gives 1 ballpark. |
| `compoundCRS_to_geogCRS_with_vertical_unit_change_and_complex_horizontal_change` | not ported | The expected name and PROJ string are the metre reference operation with the `unitconvert +z_in=us-ft +z_out=m` step added. Rust's metre reference to `EPSG:7665` gives `+proj=noop` with a ballpark name, not the grid operation. Not verified against the oracle in this session. |
| `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` | not ported | GEOIDMODEL vertical CRS. Needs `createOperationsGeogToVertFromGeoid` (4629), not read. |
| `compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS` | not ported | Same as above (`HT2_2002v70`, EPSG:9985). |
| `compoundCRS_of_vertCRS_with_geoid_model_by_name_and_several_records_to_geogCRS` | not ported | Same as above (`PROJ us_noaa_g2012bp0.tif`, several domain records). The `domainOfValidity` assertion needs the chosen record. |
| `compoundCRS_to_geogCRS_3D_Amersfoort_NAP_height_to_Amersfoort` | not ported | Rust gives 0 operations with ballpark forbidden. PROJ gives `Inverse of RD + Inverse of ETRS89-NLD [AGRS2010] to NAP height (2) using Amersfoort to ETRS89-NLD [AGRS2010] (8)`, which passes through the `nl_nsgi_nlgeo2018.tif` vertical grid. Not built. |
| `compoundCRS_to_proj_string_with_non_metre_height` | not ported | Target is `+proj=longlat +vunits=us-ft`, a PROJ-string CRS with no identifier. Same missing path as the NAVD88 ft cases (`FromDatabaseWithVertCRS`). |
| `compoundCRS_to_PROJJSON_with_non_metre_height` | not ported | Same as the PROJ string case, from a PROJJSON compound with a GEOIDMODEL. Needs both the path above and `createOperationsGeogToVertFromGeoid`. |

## What a port needs (for the next lot)

1. `findCandidateVertCRSForDatum`: a `create_vertical_crs_from_datum` query next to `create_geodetic_crs_from_datum` (`factory.rs`, datum match on `vertical_crs`, not deprecated). The name-based branch via `createObjectsFromName` is also in C++ 4587.
2. `createOperationsGeogToVertWithIntermediateVert` (4875), with the anti-recursion flag `inCreateOperationsGeogToVertWithIntermediateVert`, and the 2D retry `geog3DToVertTryThroughGeog2D` (5169-5203, which uses `findCandidateGeodCRSForDatum`).
3. The hook in `create_candidates`, for a geographic and a vertical CRS, when nothing is found directly: `createOperationsFromDatabase`, 4374-4393 (the 3D target branch, and the reverse order through `applyInverse`). The generic path must not change: the hook only runs for a vertical component.
4. The compound assembly of `createOperationsCompoundToGeog` (6454-7173): `hasOnlyOneOp`, the consistency check `useCompatibleTransformationsForSameSourceTarget`, and the `createHorizVerticalPROJBased` names. The existing `proj_based_horiz_vertical` covers the null-horizontal case only.
5. `createOperationsGeogToVertFromGeoid` (4629) for the GEOIDMODEL cases.

## Step 1 analysis (branch `lotc/vertical3`)

Source: `coordinateoperationfactory.cpp` and `test_operationfactory.cpp` of 9.9.0, saved locally (not in the repo). The sub-functions below were read in full for this step, with the calls each makes.

### C++ call graph

- `createOperations` (3630-3800), vertical and compound branches: `createOperationsFromDatabase` (3652) first, then `createOperationsVertToVert` (vertSrc && vertDst), `createOperationsVertToGeog` (vertSrc && geogDst), `createOperationsCompoundToGeog` (compoundSrc && geogDst). The reverse cases call `applyInverse(createOperations(target, source))`.
- `createOperationsFromDatabase` (4247), vertical branch (4359-4393):
  1. `geogSrc && vertDst` or `geogDst && vertSrc`: `createOperationsGeogToVertFromGeoid` (4266), then `createOperationsVertToGeogSynthetized` (4269).
  2. `findOpsInRegistryDirect` (4346). If empty and a 2D geographic is paired with a vertical CRS: `promoteTo3D` retry (4362-4387).
  3. If still empty: `createOperationsFromDatabaseWithVertCRS` (4390).
- `createOperationsFromDatabaseWithVertCRS` (5141-5232), in order:
  1. `createOperationsGeogToVertWithIntermediateVert` (5157 / 5162), guarded by `inCreateOperationsGeogToVertWithIntermediateVert`.
  2. `geog3DToVertTryThroughGeog2D` (5169-5203): `findCandidateGeodCRSForDatum`, then `findOpsInRegistryDirect` on the 2D candidate.
  3. `createOperationsVertToVertWithIntermediateVert` (5213), guarded by `inCreateOperationsVertToVertWithIntermediateVert`.
  4. `createOperationsGeogToVertWithAlternativeGeog` (5224 / 5229), guarded by `inCreateOperationsGeogToVertWithAlternativeGeog`.
- `createOperationsGeogToVertWithIntermediateVert` (4875-4938): `findCandidateVertCRSForDatum` (4903) on the datum of the target vertical CRS. For each candidate: `createOperations(geog, candidate)` (4908), then `createOperations(candidate, target)` and the first result only (4914). Each registered `opFirst` with identifiers is concatenated with `opsSecond.front()`, or kept as is when the candidate equals the target. The loop stops at the first candidate that gives results.
- `findCandidateVertCRSForDatum` (4587-4624): `createVerticalCRSFromDatum(authName, code)` for each datum identifier. If the datum has no identifier, `createObjectsFromName` with `VERTICAL_REFERENCE_FRAME`, then a recursive call on the single match.
- `createOperationsVertToVertWithIntermediateVert` (4953-5068): strategy 1 pivots on the candidates of the target datum (`createOperations(source, candidate)` then `createOperations(candidate, target).front()`). Strategy 2, only if strategy 1 is empty, pivots on the candidates of the source datum. Each pair is concatenated, and `InvalidOperationEmptyIntersection` is caught.
- `createOperationsGeogToVertFromGeoid` (4629-4871): for each `vertDst->geoidModel()`:
  - model with identifiers: `AuthorityFactory::createCoordinateOperation(code)` from the model's authority;
  - model name starting with `PROJ `: `getProjGeoidTransformation` (`createGravityRelatedHeightToGeographic3D`, with the grid accuracy and extent from `getTransformationsForGridName`);
  - otherwise `authFactory->getTransformationsForGeoid(modelName, usePROJAlternativeGridNames)`.
  Each transformation goes through `useTransf`: optional geographic unit conversion of the source (`createOperationsGeogToGeog`), then the vertical part, and `createOperationsVertToVert` to the target vertical CRS if it differs.
- `createOperationsVertToGeog` (6070-6111): for a vertical CRS without identifiers, `createObjectsFromName` (VERTICAL_CRS), then `createOperations` on the single match. Otherwise `createOperationsVertToGeogSynthetized`.
- `createOperationsCompoundToGeog` (6454-7173): the derived horizontal part (6474), the NAD83(CSRS)v7 epoch hack (6512), the bound source (6578), and then the horizontal/vertical combination.
- `buildCRSIds` (1776-1858): identifiers checked for consistency, else `createObjectsFromName` for a geographic, projected or vertical CRS with no identifier. `findOpsInRegistryDirect` (1889) uses these ids, so an altered CRS (for example `alterCSLinearUnit`) can match by name, not only by id.

### Rust counterparts (`src/iso19111`)

| C++ | Rust | Status |
|---|---|---|
| `createOperations` dispatch, vertical and compound | `factory.rs` `create_candidates` (4873-5027) | Vertical branches: `compound::vertical_to_vertical`, `compound::compound_to_geog3d`, `geographic_height_change`. No `createOperationsVertToGeog`, no `createOperationsFromDatabase` vertical branch. |
| `createOperationsFromDatabase` vertical branch | none | missing |
| `findCandidateVertCRSForDatum` | none. `factory.rs` has `create_geodetic_crs_from_datum` (2437), no vertical counterpart. The C++ `createVerticalCRSFromDatum` is a `vertical_crs` query on `datum_auth_name`, `datum_code`, `deprecated = 0`. | missing |
| `findCandidateGeodCRSForDatum` | `create_geodetic_crs_from_datum` (2437) is close, but it is not used by any vertical path | missing on the vertical path |
| `createOperationsGeogToVertWithIntermediateVert` and its guard | none | missing. The Rust context is `&CoordinateOperationContext`, so the guard needs a field on the factory or a context-local `Cell`. |
| `createOperationsVertToVertWithIntermediateVert` | none | missing |
| `createOperationsFromDatabaseWithVertCRS` | none | missing |
| `createOperationsGeogToVertWithAlternativeGeog` | none | missing |
| `createOperationsGeogToVertFromGeoid` | `get_geoid_models(code)` (`factory.rs` 3410) gives the names of the models of a vertical CRS code. `VerticalCRS::geoid_models()` (model.rs 3099) exists for WKT and PROJJSON CRS. No `getTransformationsForGeoid`, no `getTransformationsForGridName`, no `getProjGeoidTransformation`. The gravity-related height method exists in `bound_import.rs` and `bound.rs`, but not as a builder for this path. | missing |
| `createOperationsVertToGeog` name fallback | none | missing |
| `createOperationsCompoundToGeog` | `compound.rs` `compound_to_geog3d` (31-110): vertical ops from `create_operations(vertical, target)`, then `vertical_ballpark` (method 1069), then `combine`. | Partial. The NAD83(CSRS)v7 hack, derived horizontal part, and bound cases are covered only in part. |

Two points that matter for the ports:

- The C++ sub-calls use the private `createOperations`, which has no filtering. The Rust `compound_to_geog3d` calls `factory.create_operations` (filtered and sorted) at lines 78 and 86. A port of the vertical paths must use `create_candidates` where C++ uses the private call, or the NAVD88 pivots will be filtered too early. This is a difference to check, not changed in step 1.
- `buildCRSIds` matches by name when the CRS has no identifier. The Rust `direct_registry_operations` (factory.rs 5744) uses identifiers only. This is why the altered target `EPSG:6319` in feet gives nothing in Rust.

### Oracle caveat

The test target is `EPSG:6319` with `alterCSLinearUnit(UnitOfMeasure::FOOT)`. `projinfo -t EPSG:6319` is the metre target, so the oracle run in "Oracle check" above is not the test case. For the foot target, the C++ test expectations are the reference, not projinfo.

### Step 1 result

No code change. The missing pieces, in dependency order: (a) vertical CRS by datum query, (b) `getTransformationsForGeoid` and `getTransformationsForGridName`, (c) `createOperationsGeogToVertFromGeoid`, (d) the recursion-guarded `createOperationsGeogToVertWithIntermediateVert`, (e) `createOperationsFromDatabaseWithVertCRS` and the vertical pivot, (f) name fallbacks of `buildCRSIds` for the altered target.

## Step 2: createOperationsGeogToVertFromGeoid (GEOIDMODEL cases)

Read in PROJ 9.9.0: `createOperationsGeogToVertFromGeoid` (4629-4871, with `useTransf` and `getProjGeoidTransformation`), `DatabaseContext::getTransformationsForGridName` (`factory.cpp` 3971), `AuthorityFactory::getTransformationsForGeoid` (`factory.cpp` 10544), and the `verticalTransforms` part of `createOperationsCompoundToGeog` (6698-6930, read in part).

Changes:

- `src/iso19111/factory.rs`:
  - `create_candidates`: one hook, `if registry && found.is_empty()`, calls `compound::geoid_vertical` before `vertical_to_vertical`. It runs only for a pair of a vertical CRS with geoid models and a 3D geographic CRS in metres, and only with a registry (the C++ `authFactory` condition).
  - `registered_operation`, `geoid_model_operations` (the `geoid_model` table by name), `grid_name_operations` (`grid_transformation` by grid name). All three are `pub(super)` helpers.
- `src/iso19111/compound.rs`:
  - `geoid_vertical`: the branches by identifier, by `PROJ <grid>` name, and by registered name. Only operations whose vertical side is the vertical CRS (by name) are kept. The operation goes from the vertical CRS to the geographic CRS, and a geographic source gets its inverse, as `applyInverse` does in C++.
  - `proj_geoid_operation`: `getProjGeoidTransformation`. The grid operations with the datum of the vertical CRS are kept when there are any. The most accurate one gives the accuracy (`toString` written as `format!`) and the domain.
  - `projected_compound_to_geog`: the guard no longer refuses a registered vertical operation. The first registered non-ballpark operation of `create_operations(vertical, target)` replaces `vertical_ballpark`. Without one, the ballpark is used as before.
  - `chain`: the concatenated operation takes the intersection of its steps' domains, as `createComputeMetadata` does.

Oracle: `projinfo` with the WKT of the GEOID12B test and target `EPSG:4957` gives, among its 12 candidates, `vgridshift +grids=us_noaa_g2012bu0.tif`, which confirms the name lookup of `GEOID12B`. The test's own target (`EPSG:4269` promoted to 3D, no identifier) cannot be built with `projinfo`, so the C++ expectations are the reference for these cases.

| C++ TEST | Status | Reason |
|---|---|---|
| `compoundCRS_of_vertCRS_with_geoid_model_by_name_and_several_records_to_geogCRS` | ported | `tests/unit_opfac_core_c_vert.rs`. Name, PROJ string and the `US Virgin Islands` domain match. |
| `compoundCRS_of_vertCRS_with_geoid_model_to_geogCRS` (`GEOID12B`, by name) | not ported | Rust gives `Inverse of SPCS83… + Inverse of NAD83(2011) to NAVD88 height (1)`. PROJ adds the two `Ballpark geographic offset` steps (NAD83 to NAD83(2011) before, NAD83(2011) to NAD83 after). These come from the `geogCRSTmp` branch of `createOperationsCompoundToGeog` (6800-6828) and the horizontal ballparks, not ported. |
| `compoundCRS_of_vertCRS_with_geoid_model_by_id_to_geogCRS` (`HT2_2002v70`, EPSG:9985) | not ported | Rust gives `Ballpark geographic offset from NAD83(CSRS) (geog2D) to NAD83(CSRS) (geog3D)` plus a ballpark vertical. PROJ goes through the intermediate `NAD83(CSRS)v4` (`candidateIntermGeogCRS`, 6848-6918), not ported. |

## Commands and results

Step 2 (branch `lotc/vertical3`):

- `cargo fmt --check`: exit 0. `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.
- `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast`: exit 0, 1373 passed, 0 failed (1372 before, plus `unit_opfac_core_c_vert`: 1).
- `unit_opfactory_core_b::operation_compound_crs_with_bound_geog_crs_to_geog_crs`: green in the full run above.
- The intermediate state with the two unported GEOID12B and HT2_2002v70 cases in the test file, before they were removed from it, gave 1372 passed and 3 failed, all three in the new file: no regression elsewhere.
- The previous attempt's patch (`vert_uncommitted.patch`) is not reused.
