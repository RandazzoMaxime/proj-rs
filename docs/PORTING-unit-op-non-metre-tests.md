# Port: compound CRS with a non-metre height, the IGNF `init=` pair, a fuzzer string

Source: PROJ 9.9.0 `test/unit/test_operationfactory.cpp`, `src/iso19111/io.cpp`,
`src/iso19111/operation/coordinateoperationfactory.cpp`. Tests: `tests/unit_op_non_metre_height_export.rs`
(expected values copied from the C++).

| PROJ `TEST()` | Rust test | State |
|---|---|---|
| `compoundCRS_to_proj_string_with_non_metre_height` | `operation_compound_to_proj_string_with_non_metre_height` | passed on `main` |
| `geogCRS_to_geogCRS_init_IGNF_to_init_IGNF_context` | `operation_geog_crs_to_geog_crs_init_ignf_to_init_ignf_context` | passed on `main` |
| `createOperation_ossfuzz_18587` | `operation_create_operation_ossfuzz_18587` | closed here |
| `compoundCRS_to_PROJJSON_with_non_metre_height` | `operation_compound_to_projjson_with_non_metre_height` | closed here |

## `createOperation_ossfuzz_18587`

`type=crs proj=pipeline step proj=merc vunits=m nadgrids=@x proj="\nproj=pipeline step\n"` must be read
as a CRS. PROJ 9.9.0 `projinfo` reads it as a Mercator CRS with a `nadgrids` bound; the quoted value of
the second `proj` is a parameter of the one step. Two differences:

1. **Quotes.** `PROJStringSyntaxParser` splits with `pj_trim_argv` (io.cpp:10190-10203): blanks, newlines
   included, separate the words except inside double quotes, which are dropped. `parse_pipeline` split on
   every blank, so the quoted `step` opened a second, empty step.
2. **`type`.** `getParamValue(step, "type")` for a single step reads the step, then the global parameters
   (io.cpp:12754-12758); the port read the step only.

The test checks what the C++ one does: the string is accepted as a CRS, and `create_operation` returns.

## `compoundCRS_to_PROJJSON_with_non_metre_height`

The source is a compound CRS read from PROJJSON, with a vertical CRS in US survey feet and a
`geoid_model` `PROJ us_noaa_g2012bu0.tif`; the target is a projected 3D CRS whose base CRS PROJJSON reads
as a `GeodeticCRS::Geographic`. PROJ builds the vertical operation from the model
(`createOperationsGeogToVertFromGeoid`, coordinateoperationfactory.cpp:4629-4871) and gives two
candidates, the first with the grid `us_noaa_g2012bu0.tif`. The port gave twelve, the first with the
registered NAVD88 grid `us_noaa_g2018u0.tif`.

Two causes, both needed (each alone leaves the test failing):

1. `geoid_vertical` matched `Crs::Geographic` only, so the base CRS PROJJSON reads, held as
   `Crs::Geodetic(GeodeticCRS::Geographic(..))`, skipped the geoid models and fell through to the
   registry. It now reads the geographic CRS with `geographic_of`, as the other callers do.
2. The base CRS of the target has its height in US survey feet, and `geoid_vertical` returned nothing for
   a geographic CRS with a non-metre height axis. PROJ's `useTransf` (4641-4672) takes the operations of
   the geographic CRS with a metre height, and joins each to the geographic CRS of the operation with
   the height axis of the request, by a change of height unit (`createOperationsGeogToGeog`).
   `geoid_vertical_non_metre_geographic` does that with `geographic_height_change`.

After the change the port gives the two candidates PROJ gives, the first made of the vertical
conversion, the model transformation, then `Transformation from NAD83(2011) to NAD83(2011)`.

## Commands run

On a Windows workstation, without the reference `cs2cs`: `cargo fmt --check`,
`cargo clippy -p proj-rs --all-targets --all-features -- -D warnings` (clean),
`cargo test -p proj-rs --test unit_op_non_metre_height_export` (4 passed), the full suite
(`cargo test -p proj-rs --all-features --no-fail-fast`), `UPDATE_DOCS=1 cargo test --test docs_in_sync
--test progress_svg`, then without. The JSON strings of the PROJJSON test are the C++ literals of
`test_operationfactory.cpp:10299-10521` and `10534-10680`, un-escaped.
