# Port — SCH (Spherical Cross-Track Height)

Source: PROJ 9.9.0 `src/projections/sch.cpp`. The C code is two sub-PJs
(`cart` on the ellipsoid, `cart` on a sphere of radius `rcurv`) and a
`fwd3d` / `inv3d` pair. Here the same maths is one `Step` in
`src/projections/sch.rs`, built by `build` and registered in
`src/projections/mod.rs`.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `PJ_PROJECTION(sch)` parameter reading (`plat_0`, `plon_0`, `phdg_0`, `h_0`) | `build`, `Params::radians` / `Params::number`, `CreateError::MissingArgument` |
| `pj_sch_setup`: `temp`, `reast`, `rnorth`, `rcurv` | `build`: same formulas, `rcurv` as `Sch::rcurv` |
| `Q->cart` (`+proj=cart` on the P ellipsoid) | `geodetic_to_cartesian` / `cartesian_to_geodetic` with `frame.earth` |
| `Q->cart_sph` (`+proj=cart +a=rcurv +es=0`) | `Ellipsoid::try_from_a_es(rcurv, 0.0)` as `Sch::sphere` |
| `transMat[9]` | `Sch::rot`, row-major, transposed in `forward` as in C |
| `xyzoff[3]` | `Sch::offset` |
| `sch_forward3d` | `Step::forward` |
| `sch_inverse3d` | `Step::inverse` |
| `pj_sch_destructor` | none; the sub-PJs are values, not owned pointers |
| `plat_0`, `plon_0`, `phdg_0` (degrees) | radians via `Params::radians` |

The PROJ 3D wrapper (`pj_fwd3d` / `pj_inv3d` and `fwd_finalize` /
`inv_prepare`) is the engine's `Classic` and `CartStep` convention:
`fr_meter * (… + x_0)`, `vfr_meter * (z + z_0)` forward, and
`to_meter * x - x_0`, `vto_meter * z - z_0` inverse. `prepare_forward` and
`finish_angular` handle `lon_0`, the prime meridian, `+over`, and `+geoc`.

## Test mapping

| PROJ test | Rust test |
|---|---|
| `test/cli/test_cs2cs_various.yaml`, "Test SCH forward projection" | `tests/cli_yaml`, `test_cs2cs_various.yaml#71` |
| `test/cli/test_cs2cs_various.yaml`, "Test SCH inverse projection" | `tests/cli_yaml`, `test_cs2cs_various.yaml#72` |
| `test/unit/test_operation.cpp` `sch_export` | not ported: it checks the ISO 19111 `Conversion` object and its PROJ-string export, which this crate does not expose |
| `test/unit/test_io.cpp` (SCH conversion export, line 2341) | not ported, same reason |
| values from `cs2cs 9.9.0` at a point | `tests/proj_unit.rs` `sch_matches_cs2cs_and_round_trips` |

## Deliberate differences

- **Output in metres on the sphere.** The C code returns `λ · rcurv / a` and
  the wrapper multiplies by `a`. The Rust step returns `rcurv · λ` directly.
  The value is the same.
- **`ellps` and `datum` are not special.** `sch` takes the frame ellipsoid
  from `+ellps`, `+datum`, or `+a` / `+es`, as every other projection does.
- **Angular input, Cartesian output.** `input_kind` is `Angular` and
  `output_kind` is `Cartesian`, like `CartStep`. The step takes the height in
  `z`; no separate 3D trait is needed.
- **Numerical agreement.** With `%.9f` output, `cs2cs 9.9.0` and this port
  differ by at most one unit in the last printed digit (about 1e-9 m) on the
  points checked. The cause was not isolated.

## Preserved quirks

- `rcurv = h_0 + reast·rnorth / (reast·cos²(phdg) + rnorth·sin²(phdg))`:
  the height `h_0` is added to the radius of curvature, not only to the
  origin. It is kept as in C.
- `plat_0`, `plon_0`, and `phdg_0` are required; `h_0` defaults to 0.
- `+nodefs` is accepted. The yaml case `test_cs2cs_various.yaml#71` uses it.
  It was added to the engine's global keys next to `no_defs`; the engine still
  rejects other unknown keys, so this is a narrow exception.

## Verification

- `cargo test --test cli_yaml` (with `PROJ_CS2CS_REQUIRED=1`, PROJ 9.9.0
  binaries on `PATH`): `test_cs2cs_various.yaml#71` and `#72` pass.
- `cargo test --test proj_unit sch`: `sch_matches_cs2cs_and_round_trips` passes.
- `cargo test --test ported_names`: `sch` builds.
- Manual comparison with `cs2cs 9.9.0` over 6 points and 3 peg/ellipsoid
  settings, forward and inverse, plus one case with `+h_0`, `+x_0`, `+y_0`,
  `+z_0`: all agree within 1e-9 at `%.9f`.
