# Port — lot C, bound CRS cases (`lotc-bound3`)

PROJ 9.9.0 sources read for this lot:

- `test/unit/test_operationfactory.cpp`: `boundCRS_to_compoundCRS` (line 9609),
  `boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS` (9641),
  `compoundCRS_of_bound_horizCRS_and_bound_vertCRS_to_geogCRS` (9168),
  `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric` (11095).
- `src/iso19111/operation/coordinateoperationfactory.cpp`: `createOperations` (3579),
  `createOperationsFromProj4Ext` (3840), `createOperationsFromSphericalPlanetocentric` (5411),
  `createOperationsFromBoundOfSphericalPlanetocentric` (5468), `createOperationsDerivedTo` (5523),
  `createOperationsBoundToGeog` (5578), `createOperationsBoundToCompound` (7851),
  `createOperationsCompoundToGeog` (6454, read in part: the `+geoidgrids` branch, the vertical
  transforms with `skipHorizontalTransformation`, and the interpolation CRS branch).
- `src/iso19111/crs.cpp`: `GeodeticCRS::isSphericalPlanetocentric` (1937), `GeographicCRS::_exportToPROJString` (3426).
- `src/iso19111/operation/conversion.cpp`: `Conversion::createGeographicGeocentricLatitude` (2632).

## Result

| Case | Status |
|---|---|
| `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric` | ported, passes (`tests/unit_opfac_core_c_bound3.rs`) |
| `boundCRS_to_compoundCRS` | not ported: see below |
| `boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS` | not ported: see below |
| `compoundCRS_of_bound_horizCRS_and_bound_vertCRS_to_geogCRS` | not ported: see below |

The case that passes is the one whose missing logic was the spherical planetocentric path, not
the bound CRS path. The three bound cases share one missing piece, the compound-to-geographic
path of `compound.rs`, which another lot is working on.

## Ported

### `createOperation_ellipsoidal_ocentric_to_projected_of_ellipsoidal_ocentric`

The source is IAU_2015 `19902`, a `GEODCRS` on a spherical planetocentric CS. The target is
`19912`, an equirectangular projection whose base is `19902`. Before this lot, the Rust port
returned the bare `+proj=eqc` projection.

Changes:

- `factory.rs`, `create_candidates`: a spherical planetocentric source takes
  `createOperationsFromSphericalPlanetocentric`, and a spherical planetocentric target takes the
  inverse of the reverse call (`applyInverse(createOperations(target, source))`). Both run after
  the PROJ-string extension and before the bound and registry logic.
- `geodetic_ops.rs`: `is_spherical_planetocentric` (`isSphericalPlanetocentric`),
  `spherical_planetocentric_operations`. The geographic intermediate is named
  `"<name> (geographic)"` with the datum of the source and a lat/lon CS in degrees. The first
  leg is the geographic/geocentric conversion, built by `geographic_geocentric_conversion`.
  When the target is a projected CRS over the source, the second leg is the deriving conversion
  from the intermediate. See "Deviations".
- `equiv.rs`, `geodetic_equivalent`: a spherical CRS had no equivalence arm, so every comparison
  was `false`. It now compares name (strict criterion only), datum or ensemble, and axes, as the
  geocentric arm does. Needed to recognise the base of `19912` as the source.
- `transform_export.rs`: `Geodetic::Spherical` carries its CRS. Its export writes `geoc`, the
  ellipsoid, the prime meridian, and the angular unit and axis order (`geodetic_export`). A
  spherical end is accepted as the geocentric side of `geographic_geocentric`. A geographic end
  with a zero prime meridian writes only its angular unit and axis order, not `longlat`
  (`geographic_geocentric_end`).
- `projstring.rs`, `projection_steps`: a spherical planetocentric base takes the geographic source
  as its base for the ellipsoid (`+a`, `+b`).

Both directions match the C++ expectations exactly:

```
+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +inv +proj=geoc +a=2440530 +b=2438260
  +step +proj=eqc +lat_ts=0 +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 +a=2440530 +b=2438260
```

and the inverse.

## Not ported

### `boundCRS_to_compoundCRS`, `boundCRS_to_compoundCRS_with_hubCRS_same_as_compound_geographicCRS`

Both need `createOperationsBoundToCompound` and the compound-to-geographic path in
`createOperationsCompoundToGeog` for a compound of two bound CRS. A bound-to-compound port was
written during this lot and reverted, because its output dropped the vertical grid shift.

The output that PROJ gives for the compound `[BOUND(GRS80, WGS84, bar.gsb), BOUND(VERT, HUB=GRS80, bar.gtx)]`
to `WGS 84 3D` (checked against `projinfo`):

```
+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad
  +step +proj=vgridshift +grids=@bar.gtx +multiplier=1
  +step +proj=hgridshift +grids=@bar.gsb
  +step +proj=unitconvert +xy_in=rad +z_in=m +xy_out=deg +z_out=m +step +proj=axisswap +order=2,1
```

The vertical grid shift comes first and is interpolated on the horizontal CRS (`HUB=GRS80`).
The Rust `bound_compound_to_geog3d` (`compound.rs`) does not reproduce this:

- The vertical operations are computed with `create_candidates(vertical, target)`, and that
  returns nothing for a bound vertical with a 2D hub. PROJ's `skipHorizontalTransformation` branch
  of `createOperationsBoundToGeog` returns the vertical operation to the hub instead. This branch
  was ported in an experiment (a `skip_horizontal` flag on `CoordinateOperationContext`, and the
  branch in `bound_to_geog`). With that flag set for the vertical part, the vertical grid shift
  appears, but the interpolation branch then pairs it with a ballpark, and the horizontal
  `hgridshift` is lost.
- The interpolation branch builds the second leg with `create_candidates(interp3D, target)`
  on a plain geographic CRS, which cannot find the bound `hgridshift` (GRS80 to WGS 84). PROJ
  appears to build that leg from the bound transformation of the horizontal component (the
  `+geoidgrids` branch of `createOperationsCompoundToGeog`). This is inferred from PROJ's output,
  not traced step by step.

The missing C++ logic is therefore: the `+geoidgrids` branch of `createOperationsCompoundToGeog`
(`coordinateoperationfactory.cpp`, line 6616 of 9.9.0, in `createOperationsCompoundToGeog`), and the
`skipHorizontalTransformation` branch of `createOperationsBoundToGeog` (line 5861). Both live in the compound code.
The `createOperationsBoundToCompound` three-branch logic is the part of this lot's scope that
was written and reverted.

### `compoundCRS_of_bound_horizCRS_and_bound_vertCRS_to_geogCRS`

Expected: the CH1903 horizontal bound to a 3D WGS 84 with `push`/`pop` around a
`vgridshift us_nga_egm96_15.tif`. It needs the same compound-to-geographic path as above, and
the `Geographic3D to GravityRelatedHeight (EGM)` method (9661) with a `PARAMETERFILE`, whose
export was not checked in this lot. The export is not the first blocker: the compound path
returns the horizontal part alone.

## Deviations

- **Spherical branch before the registry.** `createOperationsFromDatabase` runs first in PROJ
  when an authority is set. The Rust branch runs before the registry lookup, so a registry
  operation between two spherical planetocentric CRS is not returned first. No test covers it.
- **Method name.** PROJ's `createGeographicGeocentricLatitude` uses the method name of
  `PROJ_WKT2_NAME_METHOD_GEOGRAPHIC_GEOCENTRIC_LATITUDE`. The Rust operation is the EPSG:9602
  transformation (`geographic_geocentric_conversion`), which the PROJ-string export reads. The
  name in WKT2 differs from PROJ; the PROJ string does not.
- **Intermediate domain.** The domains of the source are not copied to the intermediate CRS
  (`addDomains` in PROJ).
- **Base ellipsoid.** The projection export takes the ellipsoid from the geographic source, not
  from the spherical base. The two are the same ellipsoid for `19902`.
- **`longlat` omission.** The geographic end of a geographic/geocentric conversion writes no
  `longlat` step when its prime meridian is zero. This matches the expected PROJ string. The C++
  export setting that does it (`omitProjLongLatIfPossible` for the conversion) was not read.
- **Spherical equivalence.** The new arm compares axes by `cs_axes_match` (name, direction, unit)
  and the datum by `frame_equivalent`, as the geocentric arm does. It was not checked against
  `SphericalCS::_isEquivalentTo` in PROJ.

## Commands and results

```
cargo fmt --check                                              # clean
cargo clippy --all-targets --all-features -- -D warnings       # clean
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
  # 63 test result lines: 1373 passed, 0 failed, 26 ignored (exit 0)
cargo test --all-features --test unit_opfac_core_c_bound3     # 1 passed
```

Oracle checks, with `PROJ_DATA` set to the PROJ 9.9.0 share directory of the reference build:

- `projinfo IAU_2015:19902` exports `GEODCRS` on a spherical planetocentric CS, and the PROJ string
  export fails ("not supported on spherical planetocentric coordinate systems"), which is why the
  case is taken through the operation.
- `projinfo` of `+proj=longlat +ellps=GRS80 +nadgrids=@bar.gsb +geoidgrids=@bar.gtx +geoid_crs=horizontal_crs`
  to `EPSG:4979` gives the pipeline quoted under "Not ported".
