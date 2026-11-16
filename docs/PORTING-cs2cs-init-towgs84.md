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

- Closed (lot C3b): `+init=epsg:27700` to WGS 84 was about 100 m off, because the port expanded the
  side to a PROJ string. PROJ's `cs2cs` calls `proj_create` under the PROJ 4 rules, which return the
  database CRS with its axes forced (`src/iso19111/io.cpp:12514-12640`). The port now parses a
  `+init=epsg:` side that way (`InitRules::Proj4`, `select_alternatives_in_area_with_rules`) and
  runs the ISO selection on it. `cs2cs +init=epsg:27700 +to +init=epsg:4326` gives the reference
  to 10 decimals. Pairs with a `+towgs84` or another parameter beside `over`, `no_defs`, `wktext`
  and `type=crs` stay on the PROJ-string route, as PROJ reads them.
- Closed (lot C6): a south-oriented Transverse Mercator (EPSG:2046 to 2055, method 9808) had the
  opposite signs and the wrong order in the ISO pipeline. Cause: the projection step ended with
  `+proj=axisswap +order=-1,-2`, a trailing swap that PROJ does not write. PROJ's method 9808 maps
  to `tmerc +axis=wsu`, and an `axis=` aux suppresses the trailing swap (`bAxisSpecFound`,
  `conversion.cpp:4285-4313`, `crs.cpp:4768`). The port now does the same
  (`trailing_axis_order` in `src/iso19111/projstring.rs`). The pipeline of
  `projinfo -s EPSG:4326 -t EPSG:2046 -o PROJ -q --single-line` is the reference text.
  `EPSG:2046 +to EPSG:4326` on `-1000000 -3000000 0` gives `26.7538956993 25.0226371876`, as the
  reference does. The `+init=` bypass of the lot C3b (`init_side_south_oriented`) is removed: the
  mixed pair `EPSG:2046 +to +init=epsg:4326` now matches (`test_cs2cs_init_epsg.yaml#10`).

## Ledger

`test_cs2cs_various.yaml#59` (MGI, #207) keeps passing, and so do `#67` and `#68` (bug 244, the
`+proj=aeqd` pair), which the axis override protects. `test_cs2cs_init_epsg.yaml`: 12 of 12 cases
pass (since lot C6, `#10` included). `docs/UNPORTED-CLI.md` has no row left.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml --test gie_run` : pass.
- `UPDATE_DOCS=1` for `docs_in_sync` and `progress_svg`.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` : clean.
- Full `cargo test --all-features --no-fail-fast` : only `agree_cs2cs::named_crs_pairs_match_cs2cs`
  fails, with the same numbers as on `main`.
- Lot C3b : `cargo test --test cli_yaml` : `test result: ok. 3 passed`. `cargo test --test
  agree_cs2cs` runs against the reference only with the reference `cs2cs` on `PATH` and
  `PROJ_CS2CS_REQUIRED=1`; without them the test returns at once. So run it that way: 2 passed.
- Lot C6 : `cargo test --test cli_yaml` avant : `test result: ok. 3 passed; 0 failed; 0 ignored;
  0 measured; 0 filtered out` (un cas `#10` ouvert). Après les cas ajoutés et avant la correction :
  `test result: FAILED. 2 passed; 1 failed`, quatre cas `test_cs2cs_various.yaml#144` à `#147` et
  trois `test_projinfo.yaml#113` à `#115` en écart. Après la correction : `test result: ok. 3 passed;
  0 failed; 0 ignored; 0 measured; 0 filtered out` ; `docs/UNPORTED-CLI.md` vide.
