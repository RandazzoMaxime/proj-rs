# Port — datum operations

Sources, PROJ 9.9.0:

- `src/transformations/molodensky.cpp` (`+proj=molodensky`)
- `src/transformations/helmert.cpp` (`+proj=helmert`, `+proj=molobadekas`, `+exact`)
- `src/projections/merc.cpp` (`PJ_PROJECTION(webmerc)`)
- `src/init.cpp` and `src/fwd.cpp` (`+lon_wrap`)

No grid is read. A `+proj=hgridshift` / `+nadgrids` string is still an unknown operation.

## What was copied

| PROJ | PROJ-RS |
|---|---|
| `calc_standard_params` / `calc_abridged_params` | `Molodensky::deltas` |
| `build_rot_matrix` + `helmert_forward_3d` / `helmert_reverse_3d` | `rotation_matrix`, `spatial_kernel` |
| `helmert_forward` / `helmert_reverse` (the `theta` path) | `planar_kernel` |
| `update_parameters` | `EpochShift::observed_dt` applied on every point |
| `PJ_PROJECTION(webmerc)` | `WebMerc` kernel, `k0 = 1`, spherical formulas |
| `fwd_finalize` radian arm | `wrap_longitude` |

Molodensky and Molodensky-Badekas are angular and cartesian steps. Web Mercator is a classic kernel: it returns units of the semi-major axis and the frame multiplies by `a`. `P->k0` is read by `merc_s_forward`, so the kernel owns scale, and that scale is the constant 1 `webmerc` writes over `+k`.

## Quirks copied on purpose

- The rotation matrix is built in the coordinate-frame sign, then transposed when `convention=position_vector`. `+exact` uses the sine/cosine Euler matrix. Without `+exact` the matrix is the linear one (`cos = 1`, `sin = angle`, products dropped). The inverse multiplies by the transpose, including for that linear matrix. It is not a real inverse.
- 3-D `+s` is parts per million (`1 + s·10⁻⁶`). On the 4-parameter path (`+theta`) `+s` is a raw scale, default 1, and `+s=0` is rejected. `+theta` is arcseconds. The 2-D forward reads the epoch translation (`xyz_0`), so `+dx/+dy/+dz` do not move a 4-parameter Helmert. `+dtheta` and `+ds` do.
- `convention` is required only when a 3-D rotation or rotation rate is non-zero. `+proj=molobadekas` leaves `no_rotation` at 0, so it always requires `convention`.
- `+towgs84` overwrites `+x/+y/+z/+rx/+ry/+rz/+s`, then an explicit `+s` (and the 4-parameter default scale) can overwrite the scale again. With a non-zero rotation, `towgs84` plus `convention=coordinate_frame` is illegal. `+transpose` is illegal.
- `+s <= -1e6` is illegal on Helmert. Time is `P(t) = P(epoch) + rate·(t − epoch)`. A non-finite time stands in for PROJ `HUGE_VAL` and means the epoch. The coordinate's time is copied through unchanged.
- Cartesian Helmert output is multiplied by `1/to_meter` on the way out and the inverse multiplies the input by `to_meter` first. `+x_0` is not added on that cartesian path. The 4-parameter path is projected, so false offsets and `vto_meter` apply. Default metres make both a no-op.
- Molodensky's inverse recomputes the same deltas at the destination and subtracts them. It is not an iterative inverse. `fwd_prepare` subtracts `lon_0` before the deltas; the forward result stays in that reduced longitude; the inverse adds `lon_0` back.
- `RM` / `RN` keep the `es == 0`, `phi == 0` and `|phi| == π/2` branches. A zero denominator is outside the domain.
- Web Mercator ignores flattening and `+lat_ts`. It still scales by the ellipsoid `a`. `+k` / `+k_0` are accepted as global keys and then ignored, because `webmerc` assigns `P->k0 = 1`.
- `+lon_wrap` is degrees (a PROJ `r` parameter). It is rejected when the centre is NaN or `|centre| >= 10` turns. Forward radian output becomes `centre + adjlon(lam − centre)`. The inverse does not wrap. Longlat and Molodensky apply it; a projected forward (Mercator, Helmert) does not.

## Oracles

`tests/gie/more_builtins.gie` (Molodensky, Helmert including `+exact`, time rates, `+theta`, Molodensky-Badekas, `+lon_wrap`) and `tests/gie/GDA.gie`. Web Mercator is the EPSG Guidance Note block in `tests/gie/4D-API_cs2cs-style.gie` (`+proj=webmerc +ellps=WGS84`). The `init=epsg` pipelines in that file stay unsupported.

## Check

```bash
cargo test --offline --lib datum_ops
cargo test --offline --test gie_run
cargo clippy --offline --all-targets --all-features -- -D warnings
```

`cargo test --offline --test gie_run` reported `failed=0`, `builtins.gie` `failed=0 skipped=0`, total passed `6323`.
