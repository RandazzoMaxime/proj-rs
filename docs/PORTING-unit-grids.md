# Porting sheet: grid reading API (`grids.hpp`) and the unit-grids lot

PROJ 9.9.0 sources: `src/grids.hpp`, `src/grids.cpp`,
`test/unit/test_grids.cpp`. The audit with every `TEST()` of the lot is in
`docs/AUDIT-unit-grids.md`.

## Map

The public API is `proj_rs::grids`. It sits on the engine that already reads
grids in `src/projections/gridshift/`, and adds no reader of its own.

| PROJ | Rust |
|---|---|
| `VerticalShiftGridSet::open(ctx, name)` | `grids::VerticalShiftGridSet::open(name)` |
| `HorizontalShiftGridSet::open(ctx, name)` | `grids::HorizontalShiftGridSet::open(name)` |
| `GenericShiftGridSet::open(ctx, name)` | `grids::GenericShiftGridSet::open(name)` |
| `gridSet->name()`, `format()` | `name()`, `format()` (`"gtiff"`, `"gtx"`, `"ntv2"`, `"ctable2"`, `"null"`) |
| `gridSet->grids()` | `grids()`: top-level grids; the null set has one null grid |
| `gridSet->gridAt(lon, lat)` | `grid_at(lon, lat)`: the finest grid containing the point, radians |
| `gridSet->reopen(ctx)` | `reopen()`: re-reads the file |
| `grid->width()`, `height()` | `width()`, `height()` |
| `grid->extentAndRes()` | `extent()` returning `ExtentAndRes { is_geographic, west, south, east, north, res_x, res_y }` |
| `grid->isNullGrid()`, `hasChanged()` | `is_null_grid()`, `has_changed()` (always `false`) |
| `valueAt(x, y, sample, out)` | `value_at(x, y, sample) -> Option<f32>` |
| `valueAt(x, y, compensate, lon, lat)` | `horizontal_shift(x, y) -> Option<(f32, f32)>` (lon, lat in radians) |
| `valuesAt(...nodataFound)` | `values_at(...) -> Option<Nodata>` |
| `isNodata(val, multiplier)` | `is_nodata(value, multiplier)` |
| `unit(sample)`, `description(sample)` | `unit(sample)`, `description(sample)` (`String`, empty when absent) |
| `metadataItem(key)`, `metadataItem(key, sample)` | `metadata_item(key)`, `metadata_item_for_sample(key, sample)` (`Option<&str>`) |
| `samplesPerPixel()` | `samples_per_pixel()` |

Pixel coordinates count columns from the west edge and rows from the south
edge, as in `valueAt`. Longitudes and latitudes are radians.

The layering: `src/projections/gridshift/public.rs` is a child module of
`gridshift`, so it reaches the private `GridSet` and `Node` without widening
them. `projections/mod.rs` re-exports the types, and `src/lib.rs` exposes them
as `proj_rs::grids`.

## Deliberate differences

- **No context argument.** A set owns its bytes and resolves names through the
  crate's search path. `reassign_context` therefore has no counterpart.
- **`reopen` re-reads.** PROJ's `reopen(ctx)` rebinds the file to a context.
  The Rust `reopen()` opens the file again under the same name.
- **`bool` outputs.** `valueAt` and `valuesAt` report failure or nodata with
  `Option`/`Nodata` rather than a `bool` out-parameter.
- **Two reads at open.** `format()` is identified from the magic bytes, so the
  file is read once for the format and once for the parse. The sniff could be
  folded into the open functions; it was not, to keep the readers untouched.
- **Metadata lookup.** `metadata_item` returns `None` where C++ returns an
  empty string. `unit` and `description` keep the empty string.
- **Null grid.** The null grid is 3 by 3 over the world, has no samples, and
  `value_at` returns zero for any index, as `NullGenericShiftGrid::valueAt` does.

## Not ported

- **`compensateNTConvention`.** C++ `valueAt(x, y, compensate, lon, lat)` for
  NTv1, NTv2 and CTable2 negates the longitude shift when the flag is set
  (`grids.cpp`, `NTv2Grid::valueAt`). `horizontal_shift` does not take the flag.
  No test in `test_grids.cpp` uses it, and it is a sign convention of the
  transform caller. Cause: the flag is not wired through the shared horizontal
  node reader, so exposing it would need a change in `h_corner`.
- **`reassign_context`.** See above.

## Tests

- `tests/unit_grids_null_gtx.rs`: the null grids, the GTX vertical grid, `reopen`.
- `tests/unit_grids_gtiff.rs`: GeoTIFF pixels, value blocks, metadata, the
  tiled file, subgrids, and the projected GeoTIFF.

Expected values are PROJ 9.9.0's, from `test_grids.cpp`. The `f32` literals
are the `static_cast<float>` results of the C++ expressions, computed exactly
(`0.069_813_17_f32` is `static_cast<float>(14400/3600/180*PI)`).

## Commands

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --all-features --test unit_grids_null_gtx --test unit_grids_gtiff
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```

The fixtures are in `data/tests/`, reached through `common::use_fixture_search_path`,
as the C++ tests reach `tests/...` through `PROJ_DATA`.
