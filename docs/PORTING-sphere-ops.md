# Port — Equal Earth and Bertin 1953

Sources: PROJ 9.9.0 `src/projections/eqearth.cpp` and
`src/projections/bertin1953.cpp`.

Equal Earth also calls `pj_authalic_lat_q` and `pj_authalic_lat_inverse` from
`src/latitudes.cpp`. Bertin's internal `adjlon` is `src/adjlon.cpp`.
`PJ_TORAD` and `M_TWOPI` are the macros in `src/proj_internal.h`.

Both kernels return coordinates in units of the semi-major axis. The classic
wrapper multiplies by `a`. Neither forward reads `P->k0`, so `+k` / `+k_0`
are accepted and not applied. Neither projection has a parameter of its own.
An unknown parameter is rejected at creation.

## Equal Earth

`+proj=eqearth`. Forward and inverse are `eqearth_e_forward` and
`eqearth_e_inverse`. The same function handles the sphere and the ellipsoid.

### Quirks copied on purpose

- `A1`..`A4` are the decimal literals `1.340264`, `-0.081106`, `0.000893`,
  `0.003796`. `M` is `sqrt(3.0) / 2.0`, as the macro expands it.
- `MAX_Y` is the literal `1.3173627591574`, not the forward image of 90°.
- On the ellipsoid (`P->es != 0`) the forward sine is
  `pj_authalic_lat_q(sin φ) / qp`, then clamped when its absolute value is
  strictly greater than 1. It is not `pj_authalic_lat`. The `|n| < 0.01`
  series is not used on the way forward, including for WGS 84.
- `qp = pj_authalic_lat_q(1, P)` and `rqda = sqrt(0.5 * qp)` only when
  `es != 0`. Otherwise `rqda` stays `1` and the sine is `sin φ`.
  `pj_authalic_lat_q` still has its own cutoff `e >= 1e-7`; below that it
  returns `2 sin φ`. A zero denominator in `q` returns infinity, and the
  clamp then stores ±1.
- The inverse divides by `rqda`, then clamps `y` into `[-MAX_Y, MAX_Y]`
  without setting an error. Newton–Raphson runs at most 12 steps and stops
  when `|f / f'| < 1e-11`. Running out of steps is outside the domain.
  PROJ returns `{0, 0}` and sets the errno; `pj_inv` discards that pair.
- A clamped ordinate is the pole. The C expression `asin(sin(yc) / M)` is
  left a few ulps under 1, and that inverse sine is about 0.7 m short of
  90°. PROJ still accepts it: `tolerance 1cm` in the gie file is one token,
  and PROJ's parser only scales a unit that is its own column, so the
  tolerance stays 1 m. This crate's runner treats `1cm` as 1 cm. After a
  clamp, latitude is therefore `±π/2` (then the authalic inverse, which
  keeps the pole). Longitude still uses the Newton ordinate. An ordinate
  that is exactly `±MAX_Y` is not clamped, matching the strict `>` / `<`.
- Longitude is `M * x * (A1 + 3 A2 y² + y⁶ (7 A3 + 9 A4 y²)) / cos yc`.
  Latitude is `asin(sin(yc) / M)`. Only then, and only when `es != 0`,
  `pj_authalic_lat_inverse` runs. That inverse does use the authalic series
  when `|n| < 0.01`, and Newton otherwise. The port calls the shared
  `authalic_inverse`, which is that function.
- `+lon_0`, false offsets, and the ellipsoid axes are the usual frame.
  `+k_0` does not scale the result.

## Bertin 1953

`+proj=bertin1953`. Forward only (`bertin1953_s_forward`). `P->inv` is never
set, so the inverse is `NoInverse`.

### Quirks copied on purpose

- Setup assigns `P->es = 0` and leaves `a` (or `+R`) alone. The formulas are
  spherical. `+ellps=WGS84` therefore matches `+R=6378137`. The frame's
  ellipsoid is replaced with that sphere so `+geoc` sees `1 − e² = 1`, as
  PROJ does after the assignment.
- Setup assigns `P->lam0 = 0` and `P->phi0 = PJ_TORAD(-42.)` after the
  parameters have been read. `+lon_0` is ignored. `+lat_0` is not subtracted
  from the latitude. `phi0` is only the rotation used below.
- The rotation stores `cos(phi0)`, `sin(phi0)`, `cos_delta_gamma = 1`, and
  `sin_delta_gamma = 0`. The zeros and ones are still multiplied through.
- The forward adds `PJ_TORAD(-16.5)` to longitude, rotates, then calls
  `adjlon`. That `adjlon` is the C function: a value with absolute value
  less than `π + 1e-12` is returned unchanged. The crate helper stops at
  `≤ π`, so this call does not use it. `M_TWOPI` is the header literal.
- The pre-projection warp runs only when `λ + φ < -1.4`. The slope uses
  `1.6`, `/ 8`, and `0.8 * d * sin(φ + π/2)`. That sine is not rewritten as
  `cos φ`.
- The Hammer step is `(w = 1.68, 2)`: `sqrt(2 / (1 + cos φ cos(λ/2)))`,
  `x = 1.68 * d * cos φ * sin(λ/2)`, `y = d * sin φ`.
- The post-projection correction is two separate tests, `y < 0` and
  `y > 0`, not an `else`. `y == 0` does neither. The northern factor is
  `1 + d / 1.5 * x * x` with C's left-associative multiply and divide.
  `d = (1 - cos(λ φ)) / 12`. `1.4`, `12`, and `1.68` are the locals in the
  C function.
- `+k_0` is not applied. False easting and northing still are.
