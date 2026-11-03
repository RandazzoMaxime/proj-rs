# Port — niche CRS kinds (`niche-other`)

Source: PROJ 9.9.0 `test/unit/test_crs.cpp`, `test/unit/test_c_api.cpp`,
`test/unit/test_factory.cpp`, and `test/unit/test_operation.cpp`, the `niche`
rows in `docs/UNIT-TRIAGE.md`. Behaviour comes from `src/iso19111/crs.cpp`
(derived, temporal, parametric, and engineering CRS), `src/iso19111/io.cpp`
(`buildDatum` datum-name suffix, WKT2 keyword split, identifier suppression),
and `src/iso19111/operation/conversion.cpp` (affine parametric and pole
rotation `ob_tran` export).

`test_io.cpp` and `test_operationfactory.cpp` niche rows belong to other lots
and are not ported here.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `TemporalCRS`, `ParametricCRS`, `DerivedGeographicCRS` and the other derived kinds | `TemporalCRS`, `ParametricCRS`, `DerivedCRS` (`src/iso19111/derived_crs.rs`) |
| `DerivedProjectedCRS` | `DerivedProjectedCRS` (already present; WKT2:2019 export and `promote_to_3d`) |
| `exportToWKT` of those CRS | `src/iso19111/wkt/derived.rs` and `WktFormatter` |
| `UnitOfMeasure::PARAMETRIC` / `TIMEUNIT` factor 0 | `UnitKind::Parametric`, `parametric_unit`; a time unit of factor 0 omits the factor |
| `DerivedVerticalCRS` simple unit change as WKT1 `VERT_CS` | method codes 1069, 1104, 1068 |
| `EngineeringCRS` equivalent to an unknown datum | `Equivalence::Equivalent` in `src/iso19111/equiv.rs` |
| `proj_create_engineering_crs` | `EngineeringCRS::create` with an empty datum name (WKT1 omits the datum) |
| `proj_crs_is_derived` | `Crs::is_derived` |
| `proj_get_celestial_body_list_from_database` | `AuthorityFactory::get_celestial_body_list` |
| `proj_create_conversion_affine_parametric` / 3D | `Conversion::create_by_name` plus `ParameterValue::measure` |
| `Conversion::_exportToPROJString` affine and pole rotation | `Conversion::affine_parametric_proj_string`, `Conversion::pole_rotation_proj_string` |
| `proj_create_derived_projected_crs`, `proj_crs_add_horizontal_derived_conversion` | `DerivedProjectedCRS::create`, `Crs::add_horizontal_derived_conversion` |
| `proj_create_derived_geographic_crs` (GRIB and netCDF CF) | `DerivedCRS::create` on a database geographic CRS, then `Crs::export_to_proj_string` |
| `buildDatum` ` using nadgrids=` / ` using towgs84=` | `named_datum` in `src/iso19111/proj_crs.rs` |
| `normalizeForVisualization` of a derived projected CRS | `DerivedProjectedCRS::normalize_for_visualization` |

Rust test names keep the C++ `TEST()` name: `crs.derivedGeographicCRS_basic`
is `crs_derived_geographic_crs_basic`. `3D` in a C++ name is `3d`, because
clippy rejects `3D` in an identifier. Tests live in `tests/unit_niche_other.rs`.

## Test mapping

Factory (2) and operation (7) were committed before the CRS and C API work.
Their Rust names are `factory_authority_factory_create_coordinate_operation_point_motion_operation`,
`factory_authority_factory_iau_2015`, and the seven `operation_*` functions in
the same file.

CRS, 29 of 32: every `crs.*` niche name except the three left open below.
C API, 9 of 14:

| PROJ `TEST()` | Rust test |
|---|---|
| `proj_create_engineering_crs` | `c_api_proj_create_engineering_crs` |
| `proj_get_celestial_body_list_from_database` | `c_api_proj_get_celestial_body_list_from_database` |
| `proj_crs_is_derived` | `c_api_proj_crs_is_derived` |
| `proj_create_linear_affine_parametric_conversion` | `c_api_proj_create_linear_affine_parametric_conversion` |
| `proj_create_linear_3D_affine_parametric_conversion` | `c_api_proj_create_linear_3d_affine_parametric_conversion` |
| `proj_create_derived_projected_crs` | `c_api_proj_create_derived_projected_crs` |
| `proj_crs_add_horizontal_derived_conversion` | `c_api_proj_crs_add_horizontal_derived_conversion` |
| `proj_create_derived_geographic_crs` | `c_api_proj_create_derived_geographic_crs` |
| `proj_create_derived_geographic_crs_netcdf_cf` | `c_api_proj_create_derived_geographic_crs_netcdf_cf` |

Database `EPSG:4326` and `EPSG:32631` are the WGS 84 ensemble. Object-graph
`GeographicCRS::epsg_4326()` stays the EPSG:6326 datum realization, which is
what the `test_crs.cpp` object-graph tests expect.

## Deliberate differences

- A C call that returns null for a wrong object type is not represented. The
  Rust functions take the typed value.
- `isEquivalentTo` against an unrelated type is not ported. A shallow clone is
  `is_equivalent_to` of the clone.
- `UnitOfMeasure::unity()` still has no authority code. The 3D affine scales
  that must print EPSG:9201 use `with_identifier`.
- `DerivedGeographicCRS` exports a PROJ string only for the GRIB and netCDF CF
  pole rotations. A `PROJ ob_tran o_proj=longlat` method and the operation
  pipeline around it are not written.

## Preserved quirks

- A parent identifier suppresses descendant identifiers, except `METHOD` and
  `PARAMETER`. Geographic and geodetic derived bases open with an identifier
  whenever they have one, and the base code is written only with the 2019
  keywords.
- A parameter that carries an EPSG code suppresses its unit identifier. A
  parameter with an empty code prints the unit identifier.
- Engineering equivalence treats an empty datum name and `"Unknown engineering
  datum"` as the same, and treats Cartesian axes named X/Y with unspecified
  direction as easting/northing in the same order. The CRS name is not
  required to match. Strict equivalence is unchanged.
- An unknown ellipsoid datum is named `Unknown based on {ellipsoid} ellipsoid`,
  and `+nadgrids` or else `+towgs84` appends ` using nadgrids=` or
  ` using towgs84=` when the datum was not named by `+datum=`.

## Left open

| `TEST()` | Reason |
|---|---|
| `crs.derivedGeographicCRS_to_PROJ` | WKT import of a derived geodetic CRS is refused (`derived geodetic CRS is not parsed in this build`; the parser is the test_io lot), and `createOperation` does not build the axisswap / unitconvert / `ob_tran` pipeline. |
| `crs.derivedGeographicCRS_with_affine_transform_to_PROJ` | Same WKT import refusal. `createOperation` from the base to the derived geographic CRS does not export `+proj=affine`. |
| `crs.normalizeForVisualization_derivedprojected_operation` | The operation factory does not emit `utm` + `unimplemented` + metre-to-foot `unitconvert` + the two axisswaps, and `normalizeForVisualization` is not applied to that operation. |
| `CApi.proj_create_crs_to_crs_coordinate_metadata_in_src` | `ITRF2014@2025.0` is not read as a CRS at an epoch, and `Proj::create_crs_to_crs_from_pj` refuses an epoch. |
| `CApi.proj_create_crs_to_crs_coordinate_metadata_in_target` | Same refusal, with the epoch on the target. |
| `CApi.proj_crs_promote_to_3D_on_coordinate_metadata` | `ITRF2014@2010.0` is not parsed, and `promote_to_3d` does not keep an epoch or the dynamic identifier 7912. |
| `CApi.proj_trans_bounds_world_geodetic_to_spilhaus` | `Proj::new_known_crs` does not read `OGC:CRS84` or `ESRI:54099` (also recorded in `docs/PORTING-c-api-unit.md`). |
| `CApi.proj_crs_has_point_motion_operation` | There is no `getPointMotionOperationsFor`. Point motion is only a flag stored while a grid transformation is built. |

## Verification

`cargo test --offline --test unit_niche_other`: 47 passed.
`cargo clippy --offline --all-targets --all-features -- -D warnings`: clean.
`cargo test --offline --test docs_in_sync` fails on the generated counts in
`README.md` and `docs/UNIT-TRIAGE.md`. Those files are not edited here.
The linker is a local zig wrapper because the Command Line Tools are absent.
