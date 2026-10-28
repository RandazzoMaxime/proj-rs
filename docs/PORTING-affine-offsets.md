# Port — affine, geographic offset, vertical offset and slope

Sources: PROJ 9.9.0 `src/transformations/affine.cpp` (`affine`, `geogoffset`)
and `src/transformations/vertoffset.cpp`. Pipeline `+omit_fwd` / `+omit_inv`
follow `src/pipeline.cpp` (`pipeline_forward_4d`, `pipeline_reverse_4d`, the
step flags read by `bomit_fwd` / `bomit_inv`).

These are not classic projections. `affine` is `PJ_IO_UNITS_WHATEVER`: the
coordinates are not converted, and the result is not multiplied by `a`.
`geogoffset` and `vertoffset` are radians in and radians out. None of the
three reads `P->k0`, so `+k` / `+k_0` are accepted and not applied.

## Quirks copied on purpose

- `s11`, `s22`, `s33` and `tscale` default to 1 only when the key is absent.
  A present value, including 0, replaces the default. Every other matrix term
  and the four offsets default to 0. The first occurrence of a key wins.
- The inverse cofactors are the expressions in `computeReverseParameters`,
  including the sign pattern (`B = -(d*i - f*g)`, and the same for `D`, `F`
  and `H`). The inverse is dropped when `det == 0` or `tscale == 0`, compared
  exactly, with no epsilon. PROJ then nulls `inv` / `inv3d` / `inv4d`.
- `geogoffset` does not call `computeReverseParameters`. The matrix stays the
  identity and the reverse subtracts `dlon`, `dlat` and `dh`.
- Arcseconds use `DEG_TO_RAD / 3600` with the `proj_internal.h` literal
  `0.017453292519943296`, not a freshly evaluated `π/180`. That constant is
  applied to `dlon`, `dlat`, `slope_lat` and `slope_lon`. `dh` stays in the
  vertical linear unit.
- Angular steps go through this crate's `prepare_forward`, the same meridian
  shift as `fwd_prepare`: subtract `from_greenwich` and `lam0`, optional
  `adjlon`, optional geocentric latitude. A non-finite angle is
  `NonFinite` here; PROJ's `fwd_prepare` lets a NaN through. The gie oracles
  are finite.
  On the way out, `z` becomes `vfr_meter * (z + z0)`. The inverse undoes that
  scale before the kernel, then `inv_finalize` adds `from_greenwich + lam0`
  and, unless `+over`, `adjlon`.
- `vertoffset` adds `lam0` back on the forward and subtracts it on the inverse.
  `fwd_prepare` had removed it, which is right for a map projection and wrong
  for an operation that only changes Z. `from_greenwich` is not restored on
  the forward. The slope uses that already-relative longitude:
  `dh + slope_lat * rho0 * (phi - phi0) + slope_lon * nu0 * lam * cos(phi)`,
  with `rho0` and `nu0` evaluated at `phi0` exactly as in the C++.
- `+omit_fwd` skips the step on the forward walk. `+omit_inv` skips it on the
  reverse walk. `+inv` on the step still chooses which of the step's two
  functions runs when the step is not skipped. `+inv` on the pipeline itself
  reverses the chain in this crate, so those two flags swap with it. PROJ's
  `pipeline.cpp` does not reverse; it only toggles each step. The swap matches
  the reversal this crate already did, and `PROJStringFormatter::stopInversion`.
- A singular affine inside `+omit_inv +inv` makes the pipeline forward call
  the missing inverse. PROJ rejects that pipeline at creation with
  `no_inverse_op`. Creation here still succeeds and the transform returns
  `NoInverse`, which is the errno the gie runner records for that case.
