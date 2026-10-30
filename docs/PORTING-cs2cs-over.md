# Port — `+over` (longitude not wrapped) for `merc` and null grids

Source: PROJ 9.9.0 `src/fwd.cpp` and `src/inv.cpp` (the `over` flag skips
`adjlon` on the longitude), `src/transformations/gridshift.cpp`
(`isNullGrid`: the input is returned unchanged), and the `merc` kernel in
`src/projections/merc.cpp`. The Rust side is `Mercator` in `src/merc.rs`,
`build_merc` in `src/engine.rs`, and `Horizontal::apply` in
`src/projections/gridshift/mod.rs`.

## Observed behaviour (cs2cs 9.9.0)

| Command | Output |
|---|---|
| `cs2cs +proj=longlat +over +datum=WGS84 +to proj=merc … +over`, input `-181 49` | `-20148827.8335825 6274861.3940066` |
| same, target `+nadgrids=@null +over` | `-20148827.8335825 6274861.3940066` |
| same, target `+nadgrids=@null` without `+over` | `19926188.8519960 6274861.3940066` |
| `+init=epsg:4326 +over +to +init=epsg:3857 +over`, input `-181 49` | `-20148827.8335825 6274861.3940066` |

So `+over` on the target decides whether the output longitude is kept. A
`+over` on the source alone is not enough: without `+over` on the target, the
output is `+179`.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `pj_fwd`: `lp.lam -= lam0`; `if (!over) adjlon` | `Mercator::forward_rad_over`, which skips `adjlon(adjlon(λ) − λ0)` and computes `λ − λ0` |
| `pj_inv`: `if (!over) adjlon(lam + lam0)` | `Mercator::inverse_over`, which skips the `adjlon` |
| `pj_fwd` for `nadgrids` where the grid is null (`isNullGrid`) | `Horizontal::apply` returns the input before `geographic_in` when every set is null |

`build_merc` picks the `_over` pair when `frame.over` is set. The default pair
is unchanged.

## Test mapping

| PROJ test | Rust test |
|---|---|
| `test/cli/test_cs2cs_various.yaml` (`+init=epsg:4326 +over +to +init=epsg:3857 +over`) | `tests/cli_yaml`, `test_cs2cs_various.yaml#91` (now passes) |
| `test/cli/test_cs2cs_various.yaml` (`+datum=WGS84` source, `+nadgrids=@null +over` target) | `tests/cli_yaml`, `test_cs2cs_various.yaml#92` (now passes) |

## Deliberate differences

- **Only `merc` honours `+over` in its kernel.** Other native projections
  still reduce the longitude inside their own kernels, so `+over` on them is
  not yet exact. This lot does not touch them.
- **`inverse_over` returns degrees as `Mercator::inverse` does.** The
  longitude is no longer folded into `[-180, 180]`. The engine turns it back
  into radians with `to_radians`.

## Preserved quirks

- A null grid still adds nothing to the coordinate. Only the longitude
  wrap, which the null shift used to do through `geographic_in`, is gone.
  The non-null grid path keeps `geographic_in`, which is what PROJ does for
  `|lon| > π` on a real grid.

## Verification

- `cargo test --test cli_yaml` with `PROJ_CS2CS_REQUIRED=1`, against the
  PROJ 9.9.0 `cs2cs` binary. The suite passes; open cs2cs cases fall from 45
  to 43 (the two `+over` cases above).
- `cargo test --test gie_run` passes (ratchet unchanged).
- `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check` are clean.
