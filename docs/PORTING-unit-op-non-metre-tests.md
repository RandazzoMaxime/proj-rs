# Port: compound CRS with a non-metre height, and the IGNF `init=` pair (lot `unit-op-non-metre-tests`)

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp`. This lot adds two tests and no code: both
already pass on `main`, they only pin behavior the earlier lots produced.

| PROJ `TEST()` | Rust test | What it pins |
|---|---|---|
| `compoundCRS_to_proj_string_with_non_metre_height` | `operation_compound_to_proj_string_with_non_metre_height` | `EPSG:6318+5703` to a PROJ string geographic CRS with `+vunits=us-ft`: the first operation writes the vertical unit change once, in the last `unitconvert` |
| `geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context` | `operation_geog_crs_to_geog_crs_init_ignf_to_init_ignf_context` | `+init=IGNF:NTFG` to `+init=IGNF:RGF93G` under the PROJ 4 init rules (`InitRules::Proj4`): two operations, the NTF to RGF93 grid shift first |

Expected names and strings are copied from the C++ (`tests/unit_op_non_metre_height_export.rs`).

## Not in this lot

Two more `TEST()`s of the same family were written and **fail** on `main`; they stay open and in the
ledger:

- `compoundCRS_to_PROJJSON_with_non_metre_height`: the source is a compound with a `GEOIDMODEL`
  `us_noaa_g2012bu0.tif` vertical CRS in US survey feet, the target a projected 3D CRS in feet. The
  port picks `us_noaa_g2018u0.tif` (the registered NAVD88 candidate) where PROJ uses the grid of the
  geoid model. In PROJ the vertical operation comes from `createOperationsGeogToVertFromGeoid`
  (coordinateoperationfactory.cpp:4629-4871), whose `useTransf` joins a height-unit change when the
  geographic CRS has a non-metre height axis (4635-4691). The port's `geoid_vertical` returns nothing
  for a geographic CRS with a non-metre height; a first attempt to port that branch was not verified.
- `createOperation_ossfuzz_18587`: `create_crs_from_proj_string` refuses
  `type=crs proj=pipeline step proj=merc vunits=m nadgrids=@x proj="\nproj=pipeline step\n"`
  (`unsupported +proj=pipeline for a CRS`). PROJ reads `type` with `getParamValue`, which looks at the
  global parameters first (io.cpp:12754-12758); the port reads only the step's.

## Commands run

On a Windows workstation, without the reference `cs2cs`: `cargo fmt --check`,
`cargo clippy -p proj-rs --all-targets --all-features -- -D warnings` (clean),
`cargo test -p proj-rs --test unit_op_non_metre_height_export` (2 passed). The tests were written by a
port agent (Claude Haiku 5.5) that could not run them.
