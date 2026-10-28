# Port — unitconvert, noop, push, pop, set

Sources, PROJ 9.9.0:

- `src/conversions/unitconvert.cpp` (`forward_4d`, `reverse_4d`, `PJ_CONVERSION(unitconvert)`)
- `src/conversions/noop.cpp`
- `src/conversions/set.cpp`
- `src/pipeline.cpp` (`push`, `pop`, `setup_pushpop`, `Pipeline::stack[4]`)
- `src/units.cpp` (`pj_units`, then `pj_angular_units`)
- `src/fwd.cpp` `fwd_finalize` and `src/inv.cpp` `inv_finalize` for the parts unitconvert does not skip

These are not classic kernels. Nothing reads `P->k0`, and the result is not scaled by the semi-major axis. `+ellps` still selects the ellipsoid `gie` uses when an angular miss is turned into metres.

## Quirks copied on purpose

- Linear names are searched before angular names. Factors are the `units.cpp` literals, not a recomputed `π`. `deg` is `0.017453292519943296` (`DEG_TO_RAD`). `grad` is `0.015707963267948967` (`M_PI / 200` truncated). Survey units are the same divisions as `100 / 3937.0` and `6336000 / 3937.0`.
- An unknown name falls through to `pj_atof`. A scale of `0` or a scale whose reciprocal is `0` (`1e400`, which is infinite) is `IllegalValue` at creation. `+xy_in` with no text is the empty string, and that is `0`.
- `xy_in` / `xy_out` (and the `z` pair) must both be linear or both angular when both are named. A numeric scale leaves the family unknown, so it does not trip that check. `m` to `rad`, and `rad` to `m` on `z`, fail at creation.
- Only the normalized names `Radian` and `Degree` set the side's unit. `grad` and every linear unit stay `PJ_IO_UNITS_WHATEVER`. Degree input is `IoKind::Degrees`: `gie` does not convert it. Radian output is `IoKind::Angular`, so `gie` converts the expected degrees back. That is why `xy_in=deg xy_out=rad` of `(1, 1, 1, 1)` still expects `(1, 1, 1, 1)`.
- `skip_fwd_prepare` and `skip_inv_prepare` are set. The degree/radian scale is the factor itself. Forward radian output still runs the radian arm of `fwd_finalize`: `z = vfr_meter * (z + z0)`. Inverse output in radians still runs `inv_finalize`: add `pm` and `lam0`, then `adjlon` unless `+over`. Forward never subtracted `lam0`, because prepare was skipped. `+lon_wrap`, `+geoc`, grids, and datum steps in that finalize are not applied; this engine does not run them for a conversion.
- Time pivots through the modified Julian date, in the C++ order: forward applies `t_in` then `t_out`; inverse applies `t_out`'s input function, then `t_in`'s output function. A missing side is left out, so a lone `t_in` emits MJD. `decimalyear` outside `[-10000, 10000]`, including NaN, returns `0`. Leap days are the year-by-year loops. `yyyymmdd` clamps month `0` to `1` and any other out-of-range month to `12`; a day past the month length is clamped; day `0` is kept. Non-finite or huge dates return `0` instead of looping until the year counter overflows. The GPS epoch constant is `44244`.
- `noop` is a 4D identity. Both sides are `PJ_IO_UNITS_WHATEVER`.
- `set` writes `v_1`..`v_4` when the key is present, and the same function is forward and inverse. `+v_1` with no value stores `0` (`pj_atof` of an empty tail). An absent key leaves that axis alone.
- `push` forward pushes the selected axes; its inverse pops. `pop` is that pair swapped. Each axis has its own stack, stored on the pipeline and kept across transforms. A pop of an empty axis leaves the coordinate. The value after `+v_1` is ignored; only presence matters. With no pipeline parent (`+proj=push` alone) both operations are identities, because PROJ returns immediately when `P->parent` is null.
- Pipelines that do not contain a push or pop keep the previous call path. A pipeline that does locks one stack for the whole transform.

## Oracle

`tests/gie/unitconvert.gie`, the `proj=set` cases, and the push/pop cases in `tests/gie/4D-API_cs2cs-style.gie`. Two push/pop pipelines also name `+proj=affine` or `+proj=eqearth`. Those steps are not in this crate, so the whole pipeline stays skipped until they exist. The UTM stack cases do not need them.

Checked while the module was linked: `cargo test --offline --lib conversions` (6 passed) and `cargo clippy --offline --all-targets --all-features -- -D warnings`. The module is not registered in `src/projections/mod.rs`; see `PORT-NOTES-conversions.md`.
