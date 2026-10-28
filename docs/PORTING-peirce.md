# Port — Peirce quincuncial

Source: PROJ 9.9.0 `src/projections/adams.cpp`.

Only `+proj=peirce_q` is ported (`PJ_PROJECTION(peirce_q)`, `pj_adams_setup`
with `PEIRCE_Q`, the `PEIRCE_Q` arm of `adams_forward`, `ell_int_5`,
`peirce_q_square_inverse`, `peirce_q_diamond_inverse`). Guyou, Adams
hemisphere-in-a-square, and Adams world-in-a-square I and II stay in that
file and are not this module.

The kernel returns coordinates in units of the semi-major axis. The classic
wrapper multiplies by `a`. `adams_forward` never reads `P->k0`, so `+k` /
`+k_0` are accepted and not applied.

## Quirks copied on purpose

- `P->es = 0` after the ellipsoid is built. Eccentricity is dropped. `a`
  (or `+R`) is kept. The formulas are spherical either way.
- `ell_int_5` is the even Chebyshev series, Clenshaw from the high coefficient.
  The seven `C[]` values and `C0 = 2.19174570831038` are the decimal literals
  from the C file, not a recomputed elliptic integral.
- `y = phi * M_2_PI` uses PROJ's `M_2_PI` (`0.63661977236758134308` in
  `proj_internal.h`), not a freshly evaluated `2/π`.
- `RSQRT2` is the literal `0.7071067811865475244008443620`.
- The quincuncial shift is `shd = 1.8540746773013719 * 2`, then `/ 2` again
  where the C code divides. It is not stored pre-rounded.
- `aacos` / `aasin` match `aasincos.cpp`: values in `[1, 1.00000000000001]`
  clamp with no error; past that they still return `0`, `π`, or `±π/2` and
  the public forward becomes `OutsideDomain` (PROJ's errno). The Newton
  inverse calls the raw forward, which keeps the clamped angle and continues.
- `std::min(0, cos)` / `std::max(0, cos)` use `(b < a) ? b : a`, not
  `f64::min` / `f64::max` (they differ when an argument is NaN).
- Southern triangles fold only for `square` and `diamond`, and only when
  `phi < 0`. The five longitude tests are separate `if`s, with the same
  endpoints (`-0.75π`, `-0.25π`, `0.25π`, `0.75π`).
- Square rotates by 45° after the fold, using the saved `x`
  (`x' = RSQRT2 * (x - y)`, `y' = RSQRT2 * (x + y)`).
- Horizontal / vertical layouts reflect the southern hemisphere
  (`shd - x` or `shd - y`) and then subtract `shd / 2`. Scroll runs only when
  the matching scroll is not `0` and the shape is the matching rectangle.
  One wrap, not a modulo loop: `>= +threshold` subtracts, `< -threshold`
  adds. The two sides are not symmetric.
- `|scrollx|` or `|scrolly|` greater than 1 is illegal only when that scroll
  belongs to the active shape. `±1` is legal. A scroll on the other shape is
  ignored, including an out-of-range value. Missing `+shape` means `diamond`.
  `+shape` with no value, or any other string, is an error at creation.
- Hemisphere gate is `TOL = 1e-9`. North rejects `phi < -TOL` (the equator
  passes). South rejects `phi > -TOL` (the equator fails).
- Inverse exists only for `square` and `diamond`. The other shapes return
  no inverse, as in PROJ (`P->inv` unset).
- Both inverses are `pj_generic_inverse_2d` with tolerance `1e-10`, at most
  15 iterations, finite-difference step `1e-6` (negative when the coordinate
  is positive), correction clamped to `±0.3`, longitude to `±π`, latitude to
  `±π/2`. Failure to converge is outside the domain.
- Square and diamond initial guesses are the C heuristics, including the two
  unequal constants `2.622057580396` and `2.622057574224`, the diamond gate
  `1.8540746773013719 + 1e-3`, and `lam = ±0.9π`. The projected origin
  `(0, 0)` returns longitude 0, latitude `π/2` and does not enter Newton.
