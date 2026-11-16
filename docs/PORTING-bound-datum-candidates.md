# Named datum against ellipsoid and `+towgs84` (PROJ 9.9.0)

`cs2cs +proj=longlat +datum=NAD83 +to +proj=longlat +ellps=aust_SA +towgs84=-105,-275,-140` and its
kin: a CRS on a named datum (`+datum=NAD83`, `nzgd49`, `potsdam`, `OSGB36`, `ire65`, `GGRS87`,
`hermannskogel`, `carthage`, `NAD27`) against a CRS given by an ellipsoid and `+towgs84`. The
random differential (`docs/FUZZ.md`, « Datum nommé contre ellipsoïde ») found 328 conversions out of threshold and 143 in
status, in 59 `random-cs2cs` invocations (graine 7, `--named-datums`).

## What PROJ does

- `+datum=NAD83`, `NAD27`, `WGS84` is the EPSG frame; any other name of `datumDescs` is an EPSG
  datum with its code and ellipsoid, and **no** `+towgs84` (`io.cpp:11185-11226`, `buildDatum`).
  The shift of `+datum=carthage` is the registry's, not the one of `pj_datums`.
- `+ellps=... +towgs84=...` is a `BoundCRS` (`io.cpp:11761-11799`,
  `buildBoundOrCompoundCRSIfNeeded`).
- The pair therefore goes through `createOperationsBoundToGeog` (`coordinateoperationfactory.cpp:
  5578-5955`) or, with the bound CRS on the target, its inverse. The candidates are the registry
  operations between the datum and WGS 84 chained with the binding, then a last candidate through
  the ballpark.
- `cs2cs` keeps all of them (`crs_to_crs.cpp:362-466`, `pj_create_prepared_operations`) and
  `proj_trans` picks one per point by the area of each (`trans.cpp:47-176`). A point outside every
  registry area takes the ballpark candidate, whose area is the world.

## Causes in the port

1. `bound_ops.rs` `concatenate` built the chain by hand: no flatten, no removal of adjacent
   inverse steps, no domain. PROJ calls `ConcatenatedOperation::createComputeMetadata` there
   (`concatenatedoperation.cpp:674-756`), whose domain is the intersection of the steps' extents
   (`oputils.cpp:529-553`, a step without extent skipped). A chain through `NAD83 to WGS 84 (24)`
   (Maryland) was the world and won the area test everywhere. It now calls `derived::concatenate`,
   the port of that function that the other builders use, and drops a chain whose extents do
   not meet when the context disallows it (`concatenatedoperation.cpp:743-745`).
2. `select.rs` `lon_lat_base` had no case for a bound CRS. `proj_crs_get_geodetic_crs` goes
   through `CRS::extractGeodeticCRSRaw`, which reads the base of a `BoundCRS`
   (`crs.cpp:270-273`, used by `create_operation_to_geog_crs`, `crs_to_crs.cpp:174-224`).
   `select_alternatives` failed with "has no geodetic base", `cs2cs` fell back to the single best
   operation, and a point outside its area gave `* * inf` (Maryland grid) or the registry shift
   instead of the ballpark (Ireland helmert for a point in Latvia).

The two together close the gap. Cause 2 alone fixes the pairs whose candidates already had their
area (a UTM target: `random-cs2cs-0075`, `0103`, `0134`, `0173`) and leaves the 55 others out of threshold:
cause 2 hid cause 1, because the per-point choice never ran for a bound CRS with several
candidates.

## Tests

`tests/bound_crs_candidates.rs`: the area of the candidates of two pairs (`projinfo --summary`
9.9.0), and the per-point choice of two pairs (`cs2cs` 9.9.0, `PROJ_ONLY_BEST_DEFAULT=NO`).

## Deviation

The domain of a bound-CRS chain keeps the scope of the registry step it comes from
(`SCOPE["(null/copy) Approximation ..."]` in WKT2), where PROJ writes `SCOPE["unknown"]`:
`intersect_extents` returns the containing extent whole. Not read by any conversion. Open.
