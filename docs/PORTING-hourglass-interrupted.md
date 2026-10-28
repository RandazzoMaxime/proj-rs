# Port — hourglass and interrupted

Two unrelated PROJ 9.9.0 entry points, grouped because the oracles are
`tests/gie/hourglass.gie` and `tests/gie/interrupted.gie`.

## hourglass

Source: `src/projections/hourglass.cpp`, `PJ_PROJECTION(hourglass)`,
`hourglass_forward`, `hourglass_inverse`.

The kernel returns units of the semi-major axis. The classic wrapper
multiplies by `a`. `k0` is not read. `+lat_0` is accepted as a global key
and does not enter the formula. Default ellipsoid is the crate's GRS80, so
the vendored expects are ellipsoidal, not spherical.

`+n` and `+s` default to 1. A present value `<= 0` (including a bare flag,
which PROJ reads as 0) is `IllegalValue`. `+outinvalid` defaults to 1. A
bare `+outinvalid` is 0, which disables the inverse longitude gate.

### Quirks copied on purpose

- Ellipsoid branch is `e != 0`, not the authalic `1e-7` cutoff. `L` is
  `sqrt(π/2 * (n+1)/n * (1 + (1-e²)/e * atanh(e)))`. `qp` is
  `pj_authalic_lat_q(1, P)` (`authalic_q`), which still returns `2 sin φ`
  when `e < 1e-7`. Those two expressions are not forced to agree.
- Sphere (`e == 0`): `L = sqrt(π * (n+1)/n)`, `qp = 2`, no authalic
  latitude. Geographic latitude is used as `β`.
- Authalic forward and inverse are the same helpers as Lambert azimuthal
  (`authalic_forward` / `authalic_inverse`): series when `|n| < 0.01`,
  otherwise `asin(q/qp)` and the Newton inverse.
- Powers stay in PROJ's left-associative order. Forward uses
  `pow(L, n-1)` and `pow(L, (1-n)/n)`. The inverse stores
  `pow(L, (n-1)/n)` and `pow(L, 1-n)`. Those exponents are not the same.
- Inverse `|y| <= 1e-8` (units of `a`, about 64 m on GRS80) returns
  `(0, 0)` and is not an error. That is the origin singularity.
- `sin β > 1` is outside the domain. There is no clamp to 1.
- When `outinvalid != 0`, `|λ| > (2*outinvalid - 1) * π + 1e-7` is outside
  the domain. The threshold is evaluated in `f64`, so a huge integer does
  not wrap the way a C `int` multiply would. The gate runs after `λ` and
  `φ` are computed; this crate reports the domain error and drops the
  coordinate, which is what the gie runner does with `proj_errno`.

## interrupted

Source: `src/projections/interrupted.cpp`, `PJ_PROJECTION(interrupted)`.
The base factories are `pj_sinu` (`gn_sinu.cpp`), `pj_tmerc`, `pj_cass`,
and `pj_poly`. Those four are already ported and are called, not copied.

`+base` must be `sinu`, `tmerc`, `cass`, or `poly`. Missing or anything
else is `IllegalValue("base")`, matching `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`.

The base is a new projection. It receives the caller's ellipsoid and `k0`
only. `lon_0`, `lat_0`, false offsets, `pm`, `geoc`, and linear units stay
on the outer classic wrapper. `sinu`, `cass`, and `poly` do not read `k0`
(confirmed in `gn_sinu.cpp`, `cass.cpp`, `poly.cpp`). `tmerc` does, so the
base is built with `frame.k0`. The default transverse Mercator algorithm
is Poder/Engsager, as in PROJ 9.9 `proj.ini`. A sphere uses the existing
spherical `+proj=tmerc` path.

The base steps return metres. This kernel divides by `a` (the transverse
Mercator path already multiplied by `a` inside `Tmerc::forward_rad`) and
the classic wrapper multiplies by `a` once. Gore abscissae are therefore
in units of the semi-major axis, same as `P->fwd`.

### Quirks copied on purpose

- Omitted `+gores` is the single value `3`, which then expands to three
  equal gores of weight 1. One entry is a count: `static_cast<size_t>`
  truncates a non-negative value toward zero (`+gores=5.9` is five gores).
  A count above 1_000_000 is rejected instead of allocating. Zero gores
  after that expansion, or an empty list, is `IllegalValue("gores")`.
- A comma list is weights, not counts. Empty pieces (`1,,2`) are skipped.
  A non-numeric piece is illegal. The total weight must be finite and
  non-zero.
- Each gore's east edge in longitude and in `x` is stored. The last
  longitude border then gains `+1` radian and the last `x` border gains
  `+100` units of `a`. That padding is only for choosing a gore. The
  inverse of the last gore still uses `end_lam = π`, and the first gore
  starts at `-π`. A hit on the shared edge belongs to the western gore
  (`<=`).
- If no border contains the value, both centres stay 0 and the inverse
  range check is `λ > 0 || λ < 0`, so only a zero longitude would pass.
- `Tmerc::forward_rad` and its inverse reduce longitude with `adjlon`.
  PROJ's raw `fwd` does not. Gore-relative longitudes for a positive
  weight that does not exceed the total are inside `(-π, π]`, where
  `adjlon` is the identity.
- A base forward that fails while measuring a gore edge (transverse
  Mercator near 180° for a single gore) aborts creation. PROJ would keep
  the non-finite width and the shared `proj_errno`.
