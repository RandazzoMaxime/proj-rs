# Porting: geographic to compound CRS, geoid candidate

Lot `lot-geog-to-compound-geoid`, from base `017bd8a`. PROJ 9.9.0 reference:
`src/apps/cs2cs.cpp`, `src/apps/projinfo_lib.cpp`,
`src/iso19111/operation/coordinateoperationfactory.cpp`, and
`test/unit/test_operationfactory.cpp`.

## Result

| Case | Before | After |
|---|---|---|
| `test_cs2cs_various.yaml#102` `"WGS 84" "WGS 84 + EGM96 height" -d 3`, `49 2 50` → `4.936` | `50.000` | closed |
| `test_cs2cs_various.yaml#103` `"WGS 84 + EGM96 height" "WGS 84" -d 3`, `49 2 0` → `45.064` | `0.000` | closed |

No unit test is ported in this lot. The three unit tests named in the lot are
still open (see "Unit tests" below). No case was reopened.

## Diagnosis

The factory does not produce the geoid candidate for a 2D geographic source,
and PROJ does not either:

- `createOperations` (`coordinateoperationfactory.cpp:3782-3783`): a geodetic
  source to a compound target is `applyInverse(createOperations(compound,
  geodetic))`. The compound-to-geographic search then takes the 2D branch of
  `createOperationsCompoundToGeog` (`6687`: "Only do a vertical transformation
  if the target CRS is 3D"), which keeps the horizontal part only.
- Rust agrees: `projinfo -s EPSG:4326 -t EPSG:4326+5773 -o PROJ` lists no
  `vgridshift`. `projinfo -s EPSG:4979 -t EPSG:4326+5773` lists one.

The geoid candidate appears in PROJ only when the 2D CRS is promoted to 3D
first. That happens in the command-line tools, not in the factory:

- `cs2cs.cpp:893-916`: when the other CRS is 3D, a CRS given by its name
  (`Identifier::isEquivalentName(fromStr, proj_get_name(src))`, with the text
  as typed) and which has an identifier is replaced by `proj_crs_promote_to_3D`,
  if the promoted CRS keeps an identifier. `"WGS 84"` becomes EPSG:4979.
- `projinfo_lib.cpp:1048-1072`: the same rule for `projinfo`. The Rust port has
  it already (`src/bin/projinfo.rs`, `promote_by_name`).

Before this lot, `src/bin/cs2cs.rs` had no such promotion (`promote_3d` was
parsed and ignored, see `_selection`).

## Rust counterparts

| PROJ | Rust | Notes |
|---|---|---|
| `cs2cs.cpp:893-916` (name promotion) | `src/bin/cs2cs.rs` `promoted_by_name`, called from `main` before `build_engine` | Builds from the promoted text. Printed names and layouts still come from the typed text. |
| `cs2cs.cpp` `promoteTo3D` result, `proj_get_id_code` check | `promoted_text` | The identifier is written as `AUTH:CODE` (here `EPSG:4979`), and the engine parses it back. |
| `cs2cs.cpp` `is3DCRS` | `is_3d_crs` | Same variants as `projinfo.rs` `is_3d`. |
| `projinfo_lib.cpp:1048` | `src/bin/projinfo.rs` `promote_by_name`, `is_3d` | Unchanged. |
| `createOperations` `geodSrc && compoundDst` (3782) | `factory.rs` `create_candidates` (`geog_to_compound` in `compound.rs`) | Unchanged. The 2D source keeps no geoid candidate, as PROJ. |

## Deviations (kept on purpose or for now)

1. The engine is built from the promoted identifier text, not from the promoted
   CRS object. PROJ passes the object to `proj_create_crs_to_crs`. The text is
   the same database object, since the promoted CRS keeps its identifier, and it
   is what the Rust engine takes everywhere (`select_pipeline`,
   `select_pipeline_filtered`, the facade, and the legacy EPSG path).
2. `is_3d_crs` and `promote_by_name` are repeated in `src/bin/cs2cs.rs` and
   `src/bin/projinfo.rs`. They are small and use only the public API. A shared
   helper in the library would add public surface, which this lot does not
   take.
3. `cs2cs --3d` (`cs2cs.cpp:881-890`, promotion of both CRS) is still not
   implemented. This was already the case, and it is not part of this lot.
   The name promotion is skipped when `--3d` is given, as PROJ does
   (`if (promoteTo3D) ... else`).
4. The monitor output (`-v`/`-m`) prints the typed texts, as before.

## Unit tests

`test/unit/test_operationfactory.cpp` names of this lot. None is closed here.

| Test | Line in PROJ 9.9.0 | Status |
|---|---|---|
| `operation.geog3DCRS_to_geog2DCRS_plus_vertCRS_context` | `1055`, inside `#if no_longer_work_since_epsg_13_001` (`1053-1097`) | Not compiled by PROJ 9.9.0. Nothing to port. |
| `operation.geogCRS_promoted_to_3D_to_geogCRS_3D_context` | `1662` | Live. EPSG:4269 promoted to 3D toward EPSG:6319 (grids ignored, partial intersection). Rust's first candidate is `NAD83 to NAD83(2011) (NADCON5, CONUS)`. PROJ's first is the four-step chain through NAD83(HARN), NAD83(FBN), NAD83(NSRS2007). This is geodetic candidate ordering, not the compound path. Stays open (see `PORTING-opfac-core-2.md`). |
| `operation.geogCRS_to_compoundCRS_with_boundVerticalCRS_and_derivedProjected` | `11310` | Live. The derived projected CRS in the test is WKT with a `DERIVINGCONVERSION` whose method is `PROJ-based operation method: +proj=pipeline ...`. Rust's WKT import fails: `Invalid("unsupported +proj=pipeline for a CRS")`. The compound search itself was not reached. Stays open: a WKT import gap. |

A draft with PROJ's expected strings was written and run, and it failed on the
two live tests above, so it was removed rather than committed. No expected value
was taken from the Rust output. An oracle run was not possible in this session
(no `cs2cs` 9.9.0 binary; the PROJ sources were read, not built).

## Commands

```bash
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml
PROJ_DATA=$PWD/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs "WGS 84" "WGS 84 + EGM96 height" -d 3   # 49 2 50 -> 4.936
PROJ_DATA=$PWD/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs "WGS 84 + EGM96 height" "WGS 84" -d 3   # 49 2 0 -> 45.064
PROJ_DATA=$PWD/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/projinfo -s "WGS 84" -t "WGS 84 + EGM96 height" --spatial-test intersects -o PROJ
PROJ_DATA=$PWD/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/projinfo -s EPSG:4326 -t EPSG:4326+5773 -o PROJ   # no vgridshift
```

## Overlap

`compound.rs` (`compound_to_geog3d`, the compound-to-geographic side) is not
changed here. Case #103 is closed by the command-line promotion, not by a factory
change. The other lot working on compound-to-geographic should not need to
change its code for these two cases.
