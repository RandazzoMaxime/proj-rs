# Lot capi-vertical-crs-ex: porting sheet

Lot `unit-capi-vertical-crs-ex`, branch `unit-capi-vertical-crs-ex`, base `origin/main`.
Covers the three open `core` tests of `test/unit/test_c_api.cpp` that build a vertical CRS
with a `GEOID_MODEL` and the `ACCURACY=` option.

## PROJ functions read (PROJ 9.9.0)

| File | Lines | What |
|---|---|---|
| `test/unit/test_c_api.cpp` | 5390-5450 | `TEST_F(CApi, proj_create_vertical_crs_ex)` |
| `test/unit/test_c_api.cpp` | 5454-5540 | `TEST_F(CApi, proj_create_vertical_crs_ex_with_geog_crs)` |
| `test/unit/test_c_api.cpp` | 5544-5601 | `TEST_F(CApi, proj_create_vertical_crs_ex_implied_accuracy)` |
| `src/iso19111/c_api.cpp` | 3759-3782, 3816-3865 | `proj_create_vertical_crs`, `proj_create_vertical_crs_ex`: the `GEOID_MODEL` transformation, its interpolation CRS, `ACCURACY=` |
| `src/iso19111/crs.cpp` | 3488, 3568, 3890-3905 | `VerticalCRS::geoidModel()`, `VerticalCRS::create` with `GEOID_MODEL` |
| `src/iso19111/operation/coordinateoperationfactory.cpp` | 4260-4271 | `createOperationsFromDatabase`: geoid operations, then `createOperationsVertToGeogSynthetized` |
| same | 4629-4871 | `createOperationsGeogToVertFromGeoid`, with `useTransf` (4635-4691) |
| same | 4693-4827 | `getProjGeoidTransformation`, `getNameVertCRSMetre` (4697-4717), accuracy and extent (4747-4826) |
| same | 4691-4742 | `vertCRSMetre` |
| same | 5995-6066 | `createOperationsVertToVert` (read in full; the same-datum conversion and the ballpark change) |

Not read in detail: `proj_coordoperation_get_accuracy` and the accuracy of a concatenated
operation in `concatenatedoperation.cpp`. The accuracy assertions rely on the existing Rust
`get_accuracy` and `pivot_concatenation`.

## Function map

| PROJ | Rust | Status |
|---|---|---|
| `proj_create_vertical_crs_ex` (no option string) | `VerticalCRS::create` + `VerticalCRS::with_geoid_model` + `GeoidModel::create`, `with_identifier`, `with_interpolation_crs`, `with_accuracy` (`src/iso19111/model.rs`) | ported; typed parameters, see deviations |
| `GEOID_MODEL` `accuracies` (`ACCURACY=`) | `GeoidModel::accuracy` (`model.rs`) | ported, single value |
| `getAccuracy` of the model, `modelAccuracies` in `getProjGeoidTransformation` (4786-4801, 4826) | `compound.rs` `proj_geoid_operation`: model accuracy used, registry search skipped | ported |
| `getProjGeoidTransformation` extent (4805-4822) | `proj_geoid_operation`: area of use of the best candidate, or of the first when the model has an accuracy | ported without the model's own extent (none in the API) |
| `createOperationsGeogToVertFromGeoid`, metre vertical | `compound.rs` `geoid_vertical`, branch metre (unchanged apart from `model_operations`) | unchanged behaviour |
| `createOperationsGeogToVertFromGeoid`, non-metre vertical, `useTransf` (4645-4691) | `compound.rs` `geoid_vertical_non_metre`, `model_operations`, `concatenate_vertical_steps` | ported; geographic side with non-metre height not ported (see open) |
| `vertCRSMetre` (4731-4742), `getNameVertCRSMetre` (4697-4717) | `compound.rs` `vertical_metre_crs`, `vertical_metre_name` | ported |
| `createOperationsVertToVert` in `useTransf` (4684-4685) | `compound.rs` `vertical_to_vertical` (existing) | reused |
| `ConcatenatedOperation::createComputeMetadata` | `factory.rs` `pivot_concatenation` (existing), source and target set as `concatenate_steps` does | reused |

## Test map

| PROJ `TEST()` | Rust test (`tests/unit_capi_vertical_crs_ex.rs`) | Expected values |
|---|---|---|
| `CApi.proj_create_vertical_crs_ex` | `capi_proj_create_vertical_crs_ex` | name, PROJ string, accuracy 123, copied from the C++ |
| `CApi.proj_create_vertical_crs_ex_with_geog_crs` | `capi_proj_create_vertical_crs_ex_with_geog_crs` | name, PROJ string, PROJJSON round trip, copied from the C++ |
| `CApi.proj_create_vertical_crs_ex_implied_accuracy` | `capi_proj_create_vertical_crs_ex_implied_accuracy` | accuracy 0.15 within 1e-10, area north <= -10, west >= 110 |

## Deliberate deviations

1. **ACCURACY is one `f64`**, not a list of `PositionalAccuracy`. PROJ accepts `ACCURACY=`
   several times and keeps every value; the constructor keeps one. A typed parameter, as the
   brief asks, rather than an option string.
2. **No model extent.** PROJ takes `getExtent(model)` first. `GeoidModel` has no domain in the
   Rust API, so the area of use comes from the registry candidates, as before this lot.
3. **PROJJSON does not carry the model accuracy.** `json.rs` `write_geoid_model` and
   `json_read.rs` `build_geoid_model` are unchanged: a PROJJSON round trip drops `accuracy`.
   The C test's round trip (test 2) has no accuracy, so it is not affected. Open, see below.
4. **WKT import leaves the accuracy unset** (`wkt_import.rs`, both `GeoidModel` literals). I did
   not check whether PROJ keeps an accuracy read from a WKT `GEOIDMODEL`.
5. **Metre vertical keeps the name filter.** The branch for a metre vertical still keeps only the
   operations whose vertical side has the CRS name (`kept`, unchanged). PROJ would also keep a
   registry operation to another vertical CRS and join it with a conversion. The non-metre branch
   does not filter, as PROJ does not.
6. **"Same vertical" test** `targetOp->_isEquivalentTo(vertDst)` is approximated by
   `Crs::is_equivalent_to` on the two vertical CRS.
7. **A conversion that cannot be built drops its operation** (`vertical_to_vertical` returns
   `None`). PROJ asserts there (`assert(!tmp.empty())`).
8. **The concatenation** is `pivot_concatenation` (sums the accuracies, joins the names, domain
   is the joint extent). It is the `createComputeMetadata` the Rust port already uses for pivots.

## What stays open and why

- **Geographic CRS with a non-metre height axis** (`useTransf`, 4649-4667): PROJ adds a unit
  conversion through `createOperationsGeogToGeog`. `geoid_vertical` still returns before
  `geoid_vertical_non_metre` for it. Not needed by the three tests.
- **PROJJSON of the model accuracy** (deviation 3). The writer and the reader need an `accuracy`
  member, and PROJ's exact member name and placement were not read. Next step: read
  `io.cpp` 6752-6781 and `crs.cpp` 3705-3806 before adding it.
- **Test 2, the step "using NAD83(2011) to WGS 84 (1)"** is produced by the existing outer
  factory (compound to geographic), not by `geoid_vertical`. Its correctness is not checked here.
- **Test 3, the 0.15 accuracy and the area of use** rely on `grid_name_operations` returning
  EPSG:5656 / 5657 for `au_ga_AUSGeoid09_V1.01.tif`. Not checked against the database here.
- **`proj_create_vertical_crs` (no `_ex`) and the C ABI** are not touched.

## Existing callers at risk

- `factory.rs` `create_candidates_at` (around 5872-5879) and `compound.rs` `compound_to_geog3d`
  call `geoid_vertical` for a vertical CRS with `GEOIDMODEL` (WKT, PROJJSON or user-built). A
  vertical CRS in a unit other than the metre now gets its geoid operations; before, it got none
  and the registry or synthesized paths answered. The database-built vertical CRS always has an
  empty `geoid_models` (`factory.rs` 3278), so the registry path is not affected.
- `proj_geoid_operation`: the accuracy branch is taken only when a model has an accuracy. No
  existing constructor sets one (the three literals of `GeoidModel` now set `None`), so existing
  callers keep their previous result.
- Existing tests with a `GEOIDMODEL` that the coordinator should look at for a change of result:
  `tests/unit_op_compound_derived.rs`, `tests/unit_op_compound_misc.rs`,
  `tests/unit_op_compound_to_geog_2.rs`, `tests/unit_opfac_core_c.rs`,
  `tests/unit_opfac_core_c_vert.rs`, `tests/unit_io_core_b.rs` (PROJJSON import), and
  `cargo test --test cli_yaml`.

Compilation: `cargo check -p proj-rs --tests` through devhub, job 3820, succeeded with no warning.

Tests written, not run by the author: the coordinator runs them.
