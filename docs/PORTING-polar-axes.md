# Polar axes and the datum of a CRS read back

## Polar axes from a PROJ string

`createFromPROJString` gives a `stere` or `laea` centred on a pole (`|lat_0| = 90`) the axis type
`NORTH_POLE` or `SOUTH_POLE` (`io.cpp:12101-12140`), and `processAxisSwap` makes its axes from it
(`io.cpp:11448-11495`):

| Pole | Easting | Northing |
|---|---|---|
| North | south, along 90°E | south, along 180° |
| South | north, along 90°E | north, along 0° |

proj-rs wrote `east` and `north` for `stere`, and the right directions without their meridians for
`laea`. `projfam::projected_cs` now uses the polar coordinate systems of the model, and `+axis=`
picks its easting and northing among those polar axes, as `processAxisSwap` does.

## The WGS 84 ensemble of a WKT

A WKT2 that PROJ writes for a CRS on the WGS 84 ensemble gives the ensemble no identifier.
`addDatumInfoToPROJString` takes the datum through `DatumEnsemble::asDatum`, which renames the
ensemble `World Geodetic System 1984` (`crs.cpp:2544`, `datum.cpp:1924-1970`), and finds it
equivalent to EPSG:6326, hence `+datum=WGS84`. proj-rs looked for the EPSG code of the ensemble
only and wrote `+ellps=WGS84`. Geographic and geocentric CRS now go through `as_datum` too.

## Krovak in WKT1

`getMappingFromWKT1` finds Krovak (North Orientated) first for the WKT1 name `Krovak`; only two
axes south then west make it Krovak (`io.cpp:4380-4390`). proj-rs did the opposite, so a WKT1
Krovak without `AXIS` became south-west oriented and wrote `+axis=swu`.

## The unit of a parameter in PROJJSON

PROJ's `JSONFormatter` writes no `id` inside an object whose parent, or an ancestor, has one
(`io.cpp:13102-13113`). The unit of a parameter that has an EPSG id writes none, even when the
conversion has no id, as for a CRS read from a PROJ string with `+units=km`.

## Checks

- `tests/polar_axes_and_datum.rs`: the values of `projinfo` 9.9.0 for each case above.
- The 55 polar PROJ strings that libproj exports for the EPSG, ESRI and IGNF CRS give the WKT2 and
  PROJJSON of `projinfo` 9.9.0, and so does their round trip through WKT2.
- `capi/tests/run.sh wkt_round_trip`: the PROJ string of the WKT2:2019 and WKT1 GDAL of every EPSG
  CRS, read back, is libproj's. Coordinate operations are left out: for 541 of them, mostly
  vertical grid offsets (`.asc`, `.gtx` with a multiplier), proj-rs writes no PROJ string yet.
