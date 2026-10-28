# Port — Guyou

Source: PROJ 9.9.0, `src/projections/adams.cpp`, entry `PJ_PROJECTION(guyou)`
and the shared helpers it calls (`adams_forward` in mode `GUYOU`, `ell_int_5`,
`aacos` / `aasin` from `src/aasincos.cpp`). The same file also implements
`peirce_q`, `adams_hemi`, `adams_ws1` and `adams_ws2`. Those are not this port.

`P->es = 0` in `pj_adams_setup`. The formulas are spherical and the classic
wrapper still multiplies by the caller's semi-major axis. `k0` is not read.
The PROJ header is `Misc Sph No inv`: inverse is `TransformError::NoInverse`.

## Quirks copied on purpose

- Pole shortcut, before the series: if `||φ| − π/2| < 1e-9`, return
  `(0, ±1.85407)`. The literal is the truncated constant in `adams.cpp`, not
  `ell_int_5(±π/2)` and not the longer Peirce value `1.8540746773013719` in
  the same file. `+R=1` at the poles expects exactly `±1.85407`.
- Longitude gate is `(|λ| − 1e-9) > π/2`, checked before the pole shortcut.
  A pole whose longitude is outside the hemisphere is outside the domain.
- `RSQRT2 = 0.7071067811865475244008443620` and `M_2_PI = 0.63661977236758134308`
  are the PROJ literals.
- `a` and `b` go through `aacos`. An argument whose absolute value exceeds
  `1.00000000000001` is outside the domain. Inside that tolerance, `aacos`
  clamps (`π` if negative, `0` otherwise) and the forward continues.
- Then, matching `std::min` / `std::max` (strict `<`, so a NaN cosine yields 0):
  `m = ±aasin(sqrt(1 + min(0, cos(a+b))))` with the sign of `λ`,
  `n = ±aasin(sqrt(|1 − max(0, cos(a−b))|))` with the sign of `φ`.
  The square roots are `sqrt`, not `asqrt`.
- `ell_int_5` is Evenden's even Chebyshev series for the incomplete elliptic
  integral of the first kind at `k² = 0.5` (comment in the C++: about `1e-7`,
  not a higher-order elliptic integral). Clenshaw adds `0.5 * C0`, then the
  sum is multiplied by the argument. Coefficients are the decimal literals
  from `adams.cpp`. The vendored `guyou.gie` expectations were generated from
  this series.
