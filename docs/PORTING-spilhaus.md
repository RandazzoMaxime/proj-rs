# Port — Spilhaus

Source: PROJ 9.9.0 `src/projections/spilhaus.cpp`. The square is not a separate
operation here. It is `ADAMS_WS2` from `src/projections/adams.cpp`
(`adams_forward`, `adams_inverse`, `ell_int_5`), inverted by
`pj_generic_inverse_2d` in `src/generic_inverse.cpp`. Conformal latitude is
`pj_conformal_lat` / `pj_conformal_lat_inverse` in `src/latitudes.cpp`, and the
inverse calls `pj_sinhpsi2tanphi` from `src/phi2.cpp`.

`+proj=adams_ws2` is not registered. `tests/gie/spilhaus.gie` still contains
one such operation; those two checks stay skipped.

## Quirks copied on purpose

- Omitted parameters use the PROJ defaults, in degrees:
  `lon_0 = 66.94970198`, `lat_0 = -49.56371678`, `azi = 40.17823482`,
  `rot = 45`. The classic frame treats a missing `+lon_0` as 0 and subtracts
  that before the kernel. The kernel therefore subtracts the default central
  meridian itself, and `adjlon`s unless `+over`, only when `+lon_0` is absent.
  The inverse adds that same angle back. An explicit `+lon_0` is left to the
  wrapper. `+lat_0` is not subtracted from the input; it only orients the
  oblique sphere.
- `lambda_0 = atan2(tan(azi), -sin(χ0))` and
  `beta = π + atan2(-sin(azi), -tan(χ0))` keep signed zeros.
  `atan2(+0, -0) = π` and `atan2(-0, -0) = -π`. Collapsing those to
  `atan2(0, 0)` moves `+azi=0 +lat_0=0 +rot=0 +lon_0=0` off Adams WS2.
  The forward longitude uses raw `atan2`. The inverse uses `aatan2`, which
  returns 0 when both arguments are below `1e-50`.
- The Adams longitude is folded with `while` loops of `±2π`, not `adjlon`.
  `π` stays `π` and `-π` stays `-π`.
- `cos α = sqrt(1 - sin²α)` uses `sqrt`. A tiny negative under the radical
  stays NaN, as in C. It is not `asqrt`.
- `ell_int_5` is the even Chebyshev series with the coefficients in
  `adams.cpp` (`C0 = 2.19174570831038` and the seven `C[]` terms). It is not
  a full elliptic integral. The 45° rotation uses the literal `RSQRT2`
  (`0.7071067811865476`).
- The Adams inverse is Newton–Raphson, 15 iterations, finite-difference step
  `1e-6` (sign opposite the coordinate), Jacobian reused once the residual is
  inside `1e-6`, correction clamped to `±0.3` rad, longitude and latitude
  clamped to `±π` and `±π/2`, tolerance `1e-10` in semi-major-axis units.
  The initial guess divides by `2.62181347` and `2.62205760`, the unit-sphere
  images of `(0°, 90°)` and `(180°, 0°)` recorded in the C++ comments.
  Failure after 15 iterations is outside the domain. PROJ still returns the
  last guess and sets errno, which the caller also rejects.
- `k0` is applied inside the kernel, multiplied by the conformal scale
  `m(φ0) / cos(χ0)` (`pj_msfn` over `cos` of the conformal latitude of
  `φ0`). The classic wrapper multiplies by `a` only.
- Conformal latitude is the closed form
  `atan(sinh(asinh(tan φ) - e atanh(e sin φ)))`, not the order-6 Karney
  series. The inverse is `atan(pj_sinhpsi2tanphi(tan χ, e))`. On the sphere
  (`e = 0`) both are the identity. If the five Newton steps in
  `sinhpsi2tanphi` do not converge, this crate returns an error. PROJ returns
  the last `τ` and sets errno; the transform is rejected either way.
- `aasin` / `aacos` clamp on `[1, 1 + 1e-14]` and reject beyond that.
  PROJ returns the clamped angle and sets errno, which also rejects the point.
- Defaults are converted with `f64::to_radians` (`π/180`), the same conversion
  `Params::radians` uses, so writing the defaults out explicitly matches
  omitting them. PROJ multiplies by `DEG_TO_RAD` (`.017453292519943296`).
  The two constants differ by about `2e-19` per degree, well under the 1 mm
  gie tolerance. `spilhaus.gie` was run with the module registered:
  `passed=131 failed=0 skipped=2`.
