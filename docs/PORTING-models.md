# Port — deformation models

Sources, PROJ 9.9.0:

| Operation | File |
|---|---|
| `+proj=deformation` | `src/transformations/deformation.cpp` |
| `+proj=defmodel` | `src/transformations/defmodel.cpp`, `defmodel.hpp`, `defmodel_impl.hpp` |
| `+proj=tinshift` | `src/transformations/tinshift.cpp`, `tinshift_json_impl.hpp`, `tinshift_gpkg.cpp` |

Grid bytes follow `src/grids.cpp` (`GTiffDataset::nextGrid`, `CTable2Grid::open`, `GTXVerticalShiftGrid::open`, `pj_hgrid_value`, `pj_vgrid_value`, `pj_bilinear_interpolation_three_samples`). Float-predictor strips follow libtiff `fpAcc`, applied per scanline.

## What was copied

- Deformation reads east/north/up in millimetres per year, divides by 1000, and rotates ENU into geocentric with the Nørbech formulas. `+dt` wins over the coordinate time; otherwise `dt = t - t_epoch`. A non-finite time in that mode is `TransformError::MissingTime`.
- The reverse iteration is the PROJ loop: ten steps, tolerance `1e-8` on the horizontal residual, `out += dif` (the z residual uses a minus), then `z` is replaced by the first sample's up shift.
- `+t_obs` is rejected with a missing-argument error, which is the errno PROJ sets for that deprecated key. `+dt` and `+t_epoch` together are illegal. Both horizontal and vertical grids are required unless `+grids` is set.
- CTable2 is little-endian. `pj_hgrid_value` calls the interpolator with `compensateNTConvention = false`, so the first float stays east. The north edge is `(height - 1) * resX`, not `resY`, as in `CTable2Grid::open`.
- GTX is big-endian. Row 0 is the southern row when `ystep` is positive. Nodata is the PROJ test (`|v| > 1000` or `-88.8888`).
- Defmodel time functions are constant, velocity, step, reverse step, piecewise, and exponential. ISO-8601 instants become decimal years with leap days and without leap seconds.
- Geographic models use radians at the step boundary (`PJ_IO_UNITS_RADIANS`). Projected models stay in metres. Degree grid samples are converted to radians before they are added. Metre offsets on a geographic CRS use `DeltaEastingNorthingToLongLat` or the geocentric path.
- Longitude is wrapped into the model extent before the grid lookup. An additive model then adds the offset to the original longitude. The geocentric method overwrites longitude with the converted result.
- Geocentric bilinear builds the cell the way `GridEx::getBilinearGeocentric` does, including the small-angle sine and cosine when `resx < 1°`. The inverse is ten iterations, `1e-12` rad horizontal and `1e-3` m vertical.
- Tinshift barycentric coordinates match `FindTriangle`: `lambda1` and `lambda2` may be `1e-10` outside `[0, 1]`, and `lambda3` must be `≥ 0`. A vertical column is stored as `target_z - source_z` (or `offset_z`). Fallback `nearest_side` and `nearest_centroid` scan every triangle.
- A `.gpkg` is loaded through the embedded SQLite into that same vertex layout. Source `x,y` come from the GeoPackage point. Triangle indices are `vertices.fid`.

## Errors

A missing `+file`, `+model`, or grid argument is `CreateError::MissingArgument`. A file that exists but is not a model is `CreateError::IllegalValue`. A file that cannot be found is `CreateError::Unsupported`, so the gie runner skips the case instead of emitting a coordinate. `alaska` and `egm96_15.gtx` are not in the PROJ 9.9.0 tree and are not vendored. The small JSON, GeoTIFF, CTable2, GTX, and GeoPackage files named by the gie cases (and the grids those JSON files name) are under `data/tests/`. Search order is the given path, `data/` under the crate, then `PROJ_DATA` and `PROJ_LIB`.

`proj.ini` is not vendored. If `PROJ_DATA` contains it, `+model=proj.ini` and `+file=proj.ini` fail as invalid files, which is what those gie lines expect.
