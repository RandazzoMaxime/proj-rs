# Port — 3D CRS chain for topocentric sources

Source: PROJ 9.9.0 `src/iso19111/io.cpp`, where `+proj=topocentric +type=crs`
is built by `buildProjectedCRS` over a geocentric base (`isTopocentricCRS`),
and `src/conversions/topocentric.cpp` for the conversion itself. The C code
keeps the 3D coordinate through the whole chain. The port kept only x and y
in `Proj::convert` and `Crs::to_geodetic`, so a topocentric source was rejected
and height was dropped on the way through. This lot carries the third axis.

## Diagnosis

The earlier diagnosis held, with one correction.

- `crs.rs::from_proj` rejected `+proj=topocentric … +type=crs`, because it is
  neither geographic, projected, nor geocentric. Confirmed.
- `Body::Between` in `proj.rs` was 2D: `to_geodetic(x, y)` and
  `map_f64` returned `(x, y)`. Confirmed.
- The correction: the 2D path was not only a problem for topocentric. A
  geocentric source (`EPSG:4978`) also lost its Z, because `to_geodetic` built
  `Xyzt::xy(x, y)`. The cs2cs facade then copied the input height back out,
  so a geocentric output received the wrong third value. `#131` and `#133`
  were this same bug.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `buildProjectedCRS` with `isTopocentricCRS` (base `buildGeocentricCRS`, conversion `topocentric`, CS `topocentric East/North/Up`) | `crs.rs::from_topocentric`, kind `Kind::Topocentric` |
| `buildGeocentricCRS` (`+proj=cart` step on the ellipsoid) | `+step +proj=cart` in the pipeline header |
| `+proj=topocentric` conversion (`topocentric_fwd/inv`) | `+step +proj=topocentric` in the pipeline, origin parameters on that step |
| `+type=crs` flag | recognised by `is_topocentric_crs`, removed from the operation string |
| `pj_fwd4d` / `pj_inv4d` 3D wrapper | `Proj::map_xyz` carries `z` through `Crs::to_geodetic`, the datum step, and `Crs::geodetic_out` |
| 3D projected CRS output (`proj_trans` with z) | `Proj::convert_xyz(x, y, z) -> (x, y, z)`, new |
| cs2cs facade call | `Engine::Facade` calls `Proj::convert_xyz` (was `convert((x, y))` plus a copy of `z`) |

`from_topocentric` builds the operation as
`+proj=pipeline <ellipsoid> +step +proj=cart +step +proj=topocentric <origin>`.
The pipeline header passes the ellipsoid keys (`ellps`, `datum`, `a`, `b`,
`rf`, `f`, `es`, `e`, `R`) to both steps. The origin keys (`X_0`, `Y_0`,
`Z_0`, `lon_0`, `lat_0`, `h_0`) stay on the topocentric step. Putting the
origin in the header made the `cart` step reject `+X_0` as an unknown
parameter, so the split is required.

The forward direction of a topocentric `Crs` is geodetic to east, north, up.
`to_geodetic` uses the inverse and `geodetic_out` the forward, the same
convention as `Kind::Geocentric`.

## Test mapping

| Reference | Rust test |
|---|---|
| `test/cli/test_cs2cs_various.yaml`, "+proj=topocentric … +type=crs +to EPSG:4978 -d 3 -E" | `tests/cli_yaml`, `test_cs2cs_various.yaml#142` (now passes) |
| `test/cli/test_cs2cs_various.yaml`, "Test cs2cs geocentric -> geographic using BETA2007.gsb grid" | `tests/cli_yaml`, `test_cs2cs_various.yaml#131` (now passes) |
| `test/cli/test_cs2cs_various.yaml`, "Test cs2cs geocentric -> geographic, normally using BETA2007.gsb grid but missing" | `tests/cli_yaml`, `test_cs2cs_various.yaml#133` (now passes) |
| values from `cs2cs 9.9.0` at `0 0 0` and `100 -50 20` | `tests/proj_unit.rs` `topocentric_crs_to_geocentric_matches_cs2cs_in_3d` |
| values from `cs2cs 9.9.0` at `1000 2000 30`, and the inverse direction | `tests/proj_unit.rs` `geocentric_to_topocentric_round_trips_in_3d` |

## Deliberate differences

- **Topocentric source only with `+type=crs`.** Without `+type=crs`,
  `+proj=topocentric` is a conversion, not a CRS, and `Crs::parse` still
  rejects it as before.
- **Datum of a PROJ-string topocentric CRS is not read.** `from_topocentric`
  sets no `datum_auth`/`datum_code`, as every PROJ-string CRS does in
  `from_proj`. So no datum shift is applied between `+datum=WGS84` and a
  different datum on the other side. Matching the test case needs none,
  because the target is `EPSG:4978` on WGS 84.
- **Height goes through the datum step.** `Crs::to_geodetic` now passes the
  input height to the Helmert pipeline, where it used to pass 0. PROJ does the
  same. On the cases checked (`EPSG:4267` target, `+datum=WGS84` source, height
  100) the height is unchanged.
- **`Proj::convert_xyz` is new public API.** It is the 3D form of
  `Proj::convert`. It exists because cs2cs is a separate binary and needs a
  public call that carries `z`. The 2D methods are unchanged.

## Preserved quirks

- The cs2cs facade keeps the output axis order of the existing path. The
  `Engine::Facade` variant reports `lat_first == false`, so `+to EPSG:4326`
  prints longitude first, while PROJ prints latitude first. That was true
  before this lot, and this lot does not change it.
- A `+lat_0 +lon_0 +h_0` topocentric origin is not compared. The reference
  `cs2cs 9.9.0` fails on it with `pipeline: Mismatched units between step 1 and 2`.
  It is not part of `#142`.

## Verification

- `cargo test --test cli_yaml` with the PROJ 9.9.0 binaries on `PATH` and
  `PROJ_CS2CS_REQUIRED=1`: cs2cs open cases go from 48 to 45. `#131`, `#133`,
  and `#142` now pass. No row was added.
- `cargo test --test proj_unit topocentric`: the two new tests pass.
- Manual comparison with `cs2cs 9.9.0` at `0 0 0`, `100 -50 20`, and
  `1000 2000 30`, with `-d 3 -E`: all agree to the printed digits.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt`: no change outside the touched files.
- `cargo test --all-features --no-fail-fast`: all 21 test binaries pass.
- Bench: no case added. `src/bin/bench.rs` measures ns per point for ported
  operations, and this lot adds no operation. `Operation` for `topocentric` is
  unchanged.
