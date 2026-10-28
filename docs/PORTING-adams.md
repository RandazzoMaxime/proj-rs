# Port — Adams hemisphere and world in a square

Source: PROJ 9.9.0 `src/projections/adams.cpp` (`adams_forward`, `ell_int_5`,
`adams_inverse`, `pj_adams_setup` for `ADAMS_HEMI`, `ADAMS_WS1`, `ADAMS_WS2`).

The same C++ file also builds Guyou and Peirce quincuncial. Those two are not
registered. The inverse for world-in-a-square II calls
`src/generic_inverse.cpp` (`pj_generic_inverse_2d`). `aasin` / `aacos` follow
`src/aasincos.cpp`. `M_2_PI` is the macro in `src/proj_internal.h`.

`pj_adams_setup` sets `P->es = 0`. The formulas are spherical and still use
the caller's semi-major axis. The classic wrapper multiplies by `a`. Nothing
in the forward reads `P->k0`, so `+k_0` is not applied.

## Quirks copied on purpose

- `ell_int_5` is the even Chebyshev series of `F(φ, k² = 1/2)`, not a call to
  an elliptic-integral library. The seven coefficients and `C0 = 2.19174570831038`
  are the PROJ literals. Clenshaw walks that array from the first entry to the
  last, then `phi * (y * d1 - d2 + 0.5 * C0)`.
- `y` inside that series uses `M_2_PI = 0.63661977236758134308`, not `2/π`.
- `RSQRT2` is the truncated literal `0.7071067811865475244008443620`.
- The 45° rotation for `adams_hemi` and `adams_ws2` is
  `x' = RSQRT2 * (x - y)`, `y' = RSQRT2 * (x_old + y)`. `adams_ws1` is not rotated.
- `m` uses `sqrt(1 + min(0, cos(a + b)))` and `n` uses
  `sqrt(fabs(1 - max(0, cos(a - b))))`, with C++ `std::min` / `std::max`
  (`b < a`, not `f64::min`). `sqrt` is the libm square root, not PROJ `asqrt`.
- `aasin` / `aacos` clamp only when `|v| >= 1`. They set the domain flag only
  when `|v| > 1.00000000000001` (`ONE_TOL`). The clamped angle is still fed to
  the rest of the formula. On the public forward that flag is
  `OutsideDomain`, which is what `pj_fwd` does with the errno.
- `adams_hemi` rejects `(|λ| - 1e-9) > π/2` before the series. It has no
  inverse (`NoInverse`). `adams_ws1` has no inverse either. Neither has a
  projection-specific parameter.
- `adams_ws2` inverse is Newton on the forward, 15 steps, tolerance `1e-10`
  in units of the semi-major axis. The initial guess is the PROJ magic scales
  `2.62181347` (pole) and `2.62205760` (equator at λ = 180°), clamped to
  `[-1, 1]` before scaling by `π/2` and `π`. `|φ| >= π/2` forces `λ = 0`.
  The finite-difference step is `±1e-6` toward zero. The correction is
  clamped to `±0.3` rad, then `λ` to `±π` and `φ` to `±π/2`. The Jacobian is
  reused once both residuals are at most `1e-6`. A zero determinant leaves
  the previous inverse Jacobian, which starts at zero.
- `pj_generic_inverse_2d` calls `P->fwd` directly, so an `aasin` / `aacos`
  domain flag raised on any trial point is sticky. Convergence does not clear
  it; `pj_inv` still fails the transform. The port does the same.
