# Porting sheet: `ob_tran` with `+o_proj=longlat`

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, case "Check ob_tran
with o_proj=longlat (#1525)", and the case right after it (inverse handling).

## Behavior

`+proj=ob_tran +o_proj=longlat` returns geographic **degrees**. Before this lot
the target was treated like `latlon`, so the rotated angles came out in radians
and `cs2cs` printed `-2.3038346 0.8028515` where PROJ prints `-132 46`.

## What changed

- `src/projections/ob_tran.rs`: `longlat` and `lonlat` build a child
  `proj=unitconvert xy_in=rad xy_out=deg`, reusing the existing child path, so
  the inverse converts degrees back to radians before the rotation.
- `latlon` and `latlong` keep the radians result. The gie suite pins it
  (`builtins.gie`, `more_builtins.gie`), so it is untouched.

## Deliberate deviation and its ceiling

- Only `longlat` is pinned by PROJ 9.9.0 test data. `lonlat` takes the same path
  as an assumption, not from a PROJ test.
- `latlong` stays on the radians path. Whether PROJ treats it as `latlon` (radians)
  or as `longlat` (degrees) is not pinned by any test we have. Upgrade path: check
  PROJ `src/operation/ob_tran.cpp` for the `o_proj` string comparison and align it.

## Ledger

`test_cs2cs_various.yaml#89` (the case above) and `#90` (its inverse) now pass and
leave `docs/UNPORTED-CLI.md`.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml` : passes.
- `devhub run --kind test --env UPDATE_DOCS=1 -- cargo test --test docs_in_sync`
  and the same for `progress_svg`: regenerated README, ROADMAP, UNIT-TRIAGE and
  `docs/progress.svg`.
- `cargo fmt --check` : clean.
- Full `cargo test --all-features --no-fail-fast` : the only failure is
  `agree_cs2cs::named_crs_pairs_match_cs2cs` (EPSG:4326 to EPSG:5070, about 0.16 m
  off). It also fails on the base commit without this change, so it is not caused
  by this lot. Open separately.
