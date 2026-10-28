# Port — hgridshift, vgridshift, gridshift

Source: PROJ 9.9.0.

| Operation | File |
|---|---|
| `+proj=hgridshift` | `src/transformations/hgridshift.cpp` |
| `+proj=vgridshift` | `src/transformations/vgridshift.cpp` |
| `+proj=gridshift` | `src/transformations/gridshift.cpp` |
| GeoTIFF and NTv2 readers | `src/grids.cpp` (`GTiffDataset::nextGrid`, `NTv2GridSet::open`, `pj_hgrid_apply`, `pj_vgrid_value`) |

The Rust side is `src/projections/gridshift/`. Strips, tiles, endianness, BigTIFF, deflate and the floating-point predictor are decoded by the `tiff` crate (the same role as libtiff in PROJ). Georeferencing, GDAL metadata and the shift arithmetic are transcribed from the C++.

Grids are the small files already under `PROJ-9.9.0/data/tests`, copied to `tests/gie/tests/`. A name that is not in that tree is `CreateError::Unsupported("grid")` at creation. The gie runner then skips the case. Nothing is downloaded from the PROJ-data CDN.

## Quirks copied on purpose

- `vgridshift` defaults `+multiplier` to **−1**. Forward does `z += grid * multiplier`, so the historical forward path subtracts the grid. `+multiplier=1` is what the GeoTIFF tests pass.
- `hgridshift` longitude is **west-positive in NTv2** (`compensateNTConvention` negates it) and in GeoTIFF only when `positive_value=west` **and** a `longitude_offset` description was found. Channel indices, the radian factor and that east/west flag persist across later IFDs that omit descriptions.
- NTv2 `GS_TYPE` must be `SECONDS`. Rows run south to north; each row is stored east to west and reversed so column 0 is west. Subfiles nest by the 8-byte parent name, spaces included.
- GeoTIFF `PixelIsArea` shifts the tie point by half a pixel before the extent is built. A negative `ModelPixelScale` / `ModelTransformation` Y scale is bottom-up: file row 0 is south, and north/south are swapped. Overviews (`NewSubfileType` other than 0 or `FILETYPE_PAGE`) are ignored after the first IFD. The first IFD must be a real grid.
- Sample format must be int/uint 16 or 32, or float 32 or 64. Byte and any other mix is `IllegalValue` at creation (`test_vgrid_unsupported_byte.tif`, `test_vgrid_invalid_channel_type.tif`).
- GDAL `SCALE` / `OFFSET` are applied per sample unless the raw value equals `GDAL_NODATA`. Nodata corners in a vertical bilinear cell are dropped and the remaining weights renormalized. An all-nodata cell is outside the grid.
- Horizontal bilinear (NTv2 and `hgridshift`) converts each corner with `float(sample * factor)` **before** the weights. `gridshift` interpolates the raw samples in `f64` and only then multiplies longitude/latitude by `π/180/3600` (arc-seconds). Projected easting/northing stays in metres.
- `gridshift` biquadratic inverse does **not** iterate (NOAA NCAT). Bilinear inverse iterates at most 10 times, tolerance `1e-12` radians or metres. A step that walks off the cell keeps the first guess instead of failing.
- `+interpolation` must be `bilinear` or `biquadratic`. A grid narrower than 3 nodes forces bilinear even when the file says biquadratic.
- `TYPE` is required on `gridshift`. `GEOGRAPHIC_3D_OFFSET` is the main grid; if the point misses it and a `HORIZONTAL_OFFSET` sibling exists, that sibling is used and `ELLIPSOIDAL_HEIGHT_OFFSET` (or the other single vertical type) is a second pass. `+no_z_transform` drops the Z sample. `constant_offset` on samples 0 and 1 is added after the forward shift and subtracted before the inverse (S-JTSK).
- Subgrids nest by `grid_name` / `parent_grid_name` when the parent extent contains the child, otherwise by extent. A child with an empty `TYPE` inherits metadata from the first top-level grid of that file.
- Longitudes with `π < |λ| ≤ 10` are wrapped with `adjlon` before a geographic shift, matching `fwd_prepare` for `PJ_IO_UNITS_RADIANS`. The gie runner does not do that wrap itself. `|λ| > 10` rad is `InvalidCoord`.
- `@grid` is optional. A required name that is not on disk is `Unsupported` (the case stays skipped). A file that exists but is not a readable shift grid is `IllegalValue("grids")`, so `expect failure errno invalid_op_…` still passes. Missing `+grids` is `MissingArgument`.
- `tests/test_vgrid.tif` is named by `geotiff_grids.gie` and is **not** in the PROJ 9.9.0 tree. `egm96_15.gtx` and `alaska` are not either. Those cases stay skipped. `+proj=xyzgridshift` is not this family.

## Registration

The module is not wired in. `PORT-NOTES-gridshift.md` has the `mod.rs` and `ported.rs` lines. With those lines applied, `cargo test --offline --test gie_run` printed `failed=0` and `total passed=6411` (the checked-in `passing.txt` is still 6301).
