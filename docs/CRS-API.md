# CRS API

ISO 19111 objects live in `src/iso19111` and are re-exported from the crate
root. The names follow PROJ 9.9 `osgeo::proj`: `GeographicCRS`, `ProjectedCRS`,
`BoundCRS`, `CompoundCRS`, `DatumEnsemble`, `AuthorityFactory`,
`CoordinateOperationFactory`, and `WktFormatter`.

`DatabaseContext::create` opens the embedded `data/proj.db`. It checks
`DATABASE.LAYOUT.VERSION` the way `DatabaseContext::create` does in
`src/iso19111/factory.cpp` (major 1, minor at least 7). `AuthorityFactory`
runs the same `SELECT`s as PROJ for units, prime meridians, ellipsoids,
extents, datums, datum ensembles, coordinate systems, geographic, geocentric,
projected, vertical, compound, engineering, and derived projected CRS, and for
Helmert, grid, other, and concatenated operations.

EPSG:4326 is the datum ensemble `World Geodetic System 1984 ensemble`. Its
area of use is the PROJ hot patch to extent 1262 / scope 1183, not the verbose
EPSG v10 extent. A row with `text_definition` is an error. It is not parsed
into a coordinate.

`CoordinateOperationFactory::create_operations` reads
`coordinate_operation_view` the way
`AuthorityFactory::createFromCoordinateReferenceSystemCodes` does: deprecated
rows dropped, ordered by the pseudo-area of the extent (the SQLite function
`pseudo_area_from_swne`) and then by accuracy. When the target is a projected
CRS and the source is its base, the deriving conversion is returned first.
Pivots are used only for pairs passed to
`CoordinateOperationContext::set_intermediate_crs`.

## WKT

`WktConvention` is `Wkt2_2015`, `Wkt2_2015Simplified`, `Wkt2_2019`,
`Wkt2_2019Simplified`, `Wkt1Gdal`, and `Wkt1Esri`.

- 2019 geographic CRS use `GEOGCRS`. 2015 uses `GEODCRS`.
- Simplified forms omit Greenwich, a metre ellipsoid unit, per-axis units when
  they match, and every `ID` except the top object.
- A parent with an `ID` suppresses nested identifiers, except `METHOD` and
  `PARAMETER` in full WKT2.
- WKT1 GDAL writes `AUTHORITY` and omits axes unless
  `OutputAxisRule::Yes` is set, or the projected axes are east then north.
- WKT1 Esri is one line, with no authority nodes. `WGS 84` becomes
  `GCS_WGS_1984`. Pass `WktFormatter::with_database` to read Esri aliases.
- Strict WKT1 rejects a geographic 3D CRS. A projected 3D CRS is WKT2:2019 only.

`Crs::export_to_wkt` is the export entry point.
