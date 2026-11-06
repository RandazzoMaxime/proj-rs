# Porting sheet: `+init=epsg:` datum shift and 2D height

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, "Test MGI datum gives expected results
(#207)": `+proj=latlong +datum=WGS84 +to +init=epsg:31284` on `16.33 48.20`, expected
`595710.3731286 5357598.4645652 0.0000000`. The `facade_info` test pins what PROJ gives for
`+init=epsg:4326`. Oracle used for the diagnosis: the PROJ 9.9.0 package from conda-forge, and
`projinfo`/`cs2cs` run with `PROJ_DEBUG=3`.

## Behavior

- `+init=epsg:N` expands to the PROJ string of the CRS, and a projected CRS on a datum with a
  known shift carries that shift (`+towgs84`), as `createBoundCRSToWGS84IfPossible` does. Before,
  the expansion read the raw table definition, which has no `+towgs84`: the MGI point was 85 m
  off.
- For a 2D target, the height of the output is the height of the input. A `+towgs84` shift
  moves the horizontal position only, as PROJ prints `0.0000000` for the explicit form too.

## What changed

- `src/crs.rs`, `init_definition`: a projected EPSG CRS expands from its bound-to-WGS 84 export.
  A geographic CRS keeps the ellipsoid definition, so `facade_info` still holds.
- `src/bin/cs2cs.rs`, `SidePair`: `dst_has_height` from the target definition (a vertical key:
  `geoidgrids`, `geoid`, `vunits`, `vto_meter`, `vgridshift`). A 2D target returns the input height.

## Ceilings

- The projected-only rule is a heuristic over the PROJ export. A geographic CRS with a datum shift
  in the registry would keep the ellipsoid definition, not the shift. Upgrade path: follow PROJ's
  `proj_as_proj_string` for geographic CRS too, and update `tests/facade_info.rs` with it.
- The residual difference of the MGI point is 0.2 mm (x) and 2.4 mm (y) against PROJ 9.9.0. The
  runner accepts it under its tolerance. An exact match needs the 7-parameter Helmert path checked
  against PROJ's `pj_datum_transform`.

## Ledger

`test_cs2cs_various.yaml#59` now passes and leaves `docs/UNPORTED-CLI.md`.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml --test gie_run` : pass.
- `UPDATE_DOCS=1` for `docs_in_sync` and `progress_svg`.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` : clean.
- Full `cargo test --all-features --no-fail-fast` : only `agree_cs2cs::named_crs_pairs_match_cs2cs`
  fails, with the same numbers as on `main`.
