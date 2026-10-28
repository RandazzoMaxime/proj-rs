# Port — spheroid radii and geocentric latitude

Sources: PROJ 9.9.0 `src/ell_set.cpp` (`ellps_spherification`) and
`src/conversions/geoc.cpp` (`pj_geocentric_latitude`).

`R_A`, `R_V`, `R_a`, `R_g`, `R_h`, and `R_C` were already applied in
`src/engine.rs` `spherify`. This port adds `R_lat_a` and `R_lat_g` there, and
`+proj=geoc` in `src/projections/spheroid.rs`. `+proj=geoc` is not registered
yet. See `PORT-NOTES-spheroid.md`.

## Radii

| PROJ key | Radius |
|---|---|
| `R_A` | `a * (1 - es * (1/6 + es * (17/360 + es * 67/3024)))` |
| `R_V` | `a * (1 - es * (1/6 + es * (5/72 + es * 55/1296)))` |
| `R_h` | `2 a b / (a + b)` |
| `R_lat_a=φ` | `(M + N) / 2` at latitude φ |
| `R_lat_g=φ` | `sqrt(M N)` at latitude φ |
| `R_C` | conformal radius at latitude 0, which is `b` |

`M` is the meridional radius and `N` the prime vertical. With
`t = 1 - es sin²φ` that is `a (1 - es + t) / (2 t √t)` and `a √(1 - es) / t`.
The series coefficients are the C++ divisions `1/6`, `17/360`, `67/3024`,
`5/72`, and `55/1296`, not decimal literals.

## `+proj=geoc`

Both sides are radians (`PJ_IO_UNITS_RADIANS`), not a classic kernel. The
wrapper must not multiply by `a`. Forward is `atan((1 - e²) tan φ)`. Inverse
is `atan(tan φ / (1 - e²))`. Longitude, height, and time are only shifted by
the shared frame (`lon_0`, prime meridian, `z_0`, vertical unit).

The same two functions implement the old `+geoc` flag on `longlat`, `cart`,
and classic inverses. `more_builtins.gie` expects `+proj=geoc` and
`+proj=longlat +geoc +inv` to print the same latitude.

## Quirks copied

- Spherification keys are tried in the C array order `R_A`, `R_V`, `R_a`,
  `R_g`, `R_h`, `R_lat_a`, `R_lat_g`, `R_C`. The first one present wins, not
  the first one in the PROJ string. `R_lat_a` therefore beats `R_lat_g`.
- `R_C` ignores `lat_0`. `pj_ellipsoid` runs in `pj_init_ctx` before `phi0` is
  read, so `sin(phi0)` is `sin(0)`. The radius is `a √(1 - e²)`, the
  semi-minor axis. `builtins.gie` `+proj=merc +R_C +ellps=WGS84 +lat_0=45`
  is that equatorial sphere (easting `221892.515…`), not the conformal sphere
  at 45°. Do not "fix" `R_C` to use `lat_0`.
- `R_lat_*` latitude is `proj_dmstor`. `|φ| > 90°` is
  `invalid_op_illegal_arg_value`. PROJ's log line says `lat_g` for both keys.
  A bare `+R_lat_a` with no `=` stores the key text; `dmstor` of that text
  returns 0, so the latitude is the equator.
- `t = 1 - es sin²φ == 0` is rejected. `+a=1E77 +b=1 +R_lat_a=90` never
  reaches that test here: `es` rounds to 1 and `Ellipsoid::try_from_a_es`
  already rejects it. PROJ rejects the same input in
  `pj_calc_ellipsoid_params` because `one_es == 0`, also before the `t == 0`
  branch. Either way the operation is a creation failure, not a coordinate.
- Within `1e-9` rad of either pole, or on a sphere (`e² = 0`), geocentric and
  geographic latitude are the same value. No `tan` call. The cutoff is
  `π/2 - 1e-9`, so `±90°` and `89.99999999999°` are copied.
- `+geoc` on `+proj=geoc` is applied by `fwd_prepare` / `inv_finalize` as well
  as by the kernel, so the two conversions cancel. Forward still subtracts
  `lon_0` and does not add it back (`fwd_finalize` for radians only scales
  `z`). Inverse adds `lon_0` and the prime meridian, then `adjlon` unless
  `+over`.
- `+R` still skips shape. Spherifying that sphere is the identity for every
  radius in this family, which matches PROJ ignoring spherification when `R`
  is set.

## Oracle

`tests/gie/ellipsoid.gie` (`R_lat_a=60`, `R_lat_g=60`, and the `a=1E77`
failure) and the geocentric-latitude block in `tests/gie/more_builtins.gie`.
Tolerance on the Mercator points is 10 nm. `R_C` in `tests/gie/builtins.gie`
must stay `failed=0 skipped=0`.
