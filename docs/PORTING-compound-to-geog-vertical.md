# Port — compound to geographic, vertical branches (`createOperationsCompoundToGeog`)

Status: **not closed**. This lot ports nothing into `src/`. It records why the four open unit tests
of `tests/unit_op_compound_to_compound_2.rs` and the CLI case `test_cs2cs_various.yaml#126` stay
open, which PROJ branch each one needs, and what was tried. No `#[ignore]` is removed, and
`docs/UNPORTED-UNIT.md` and `docs/UNPORTED-CLI.md` are unchanged.

Source: PROJ 9.9.0, tag `9.9.0`, `src/iso19111/operation/coordinateoperationfactory.cpp`:
`createOperationsCompoundToGeog` 6454–7000, its derived branch 6474–6501, the only-vertical-ballpark
branch 6843–6919, `createOperationsGeogToVertWithAlternativeGeog` 5073, and its call from
`createOperationsFromDatabaseWithVertCRS` (the alternative step, 5224–5230). `createPROJBased` (2633) returns a
`SingleOperation`, and `ConcatenatedOperation::createComputeMetadata` is in
`src/iso19111/operation/concatenatedoperation.cpp` 674.

## PROJ branches and their Rust counterparts

| PROJ 9.9.0 | Lines | Rust (`src/iso19111/compound.rs`, at 017bd8a) |
|---|---|---|
| derived branch: projected horizontal, intermediate compound `base + vertical` | 6474–6501 | `projected_compound_to_geog` (935), an approximation: only for the same geodetic datum as the target, one operation, the vertical ballpark or a registered vertical operation |
| NAD83(CSRS)v7 hack | 6512–6576 | not ported (needs epochs) |
| bound horizontal, derived base | 6578–6611 | `compound_to_compound` (`through_crs`), not in this branch |
| `+towgs84` / geoid pair to another datum | 6616 onward | not ported |
| `horizTransforms` = `createOperations(componentsSrc[0], targetCRS)` | 6699 | `compound_vertical_pass` uses the 2D target (`target_2d`) |
| vertical operations with `SetSkipHorizontalTransform` and `SetGeogCRSOfVertCRS` | 6746–6790 | `compound_vertical_pass` (150), `create_candidates` |
| only-vertical-ballpark branch | 6843–6919 | not ported (see below) |
| `if (horizTransforms.empty() \|\| verticalTransforms.empty()) res = horizTransforms` | 6921–6925 | `compound_to_geog3d`, generic fallback |
| interpolation loop, then the non-interpolation loop | 6932 onward | `compound_vertical_pass`, the interpolation loop and the loop with `combine` |

`vertical_geog_fallback` (1154, the Rust side of `createOperationsFromDatabaseWithVertCRS`) returns
early when the geographic CRS has an identifier (`compound.rs` 1172). PROJ has no such test: its
`createOperationsGeogToVertWithAlternativeGeog` (5073) runs whenever the registry gives nothing.

## Trace evidence

The reference is PROJ 9.9.0, built out of tree from the tag, with `ENABLE_TRACING` (the
`TRACE_CREATE_OPERATIONS` macro) and a temporary `logTrace` at the entry of the only-vertical-ballpark
branch. None of that is in the repository. Each case was run with `projinfo -o PROJ`,
`--spatial-test intersects`, and the grid check of the test (`none` or `known_available`).

| Case | Branch 6847 entered | Path that gives the PROJ candidate |
|---|---|---|
| `compoundCRS_to_compoundCRS_WGS84_EGM96_to_WGS84_Belfast` (4326+5773 to 4326+5732) | no | the interpolation loop (`ETRS89-IRE` interpolation CRS); the Rust candidates are the ballpark ones |
| `compoundCRS_to_compoundCRS_issue_3328` (4326+5773 to 4326+5713) | no | PROJ's name is a `createHorizVerticalHorizPROJBased` name (`... using NAD83(CSRS) to WGS 84 (2)`), so it comes from the interpolation loop; the NAD83(CSRS) route itself was not traced |
| `..._and_ballpark_geog` (4617+5713 to 4617+6647) | no | not traced further; the candidate is not from the branch |
| `..._using_intermediate_of_horizontal_transform` (5550+7651 to 9755+3855, known available) | **yes**, once, with `src = "PNG94 + Kumul 34 height"` (the derived intermediate), `tgt = "WGS 84 (G2139)"`, 1 candidate | only after the derived branch 6474, in the call `PNG94 / PNGMG94 zone 54 + Kumul 34 height --> WGS 84 (G2139)` |
| CLI #126 (4326+5773 to 4326+5782) | no | the vertical loop: `Alicante height --> WGS 84 (geographic3D)` has the interpolation op `Inverse of ETRS89-ESP [REGENTE] to Alicante height (1)` from `createOperationsGeogToVertWithAlternativeGeog` (the registry operations to Alicante from any geographic CRS) |

So the branch is needed by one of the five cases only, and that one needs the derived branch first.
The branch cannot be the cause of the other four.

## Experiments (each measured, each reverted)

Every experiment used the ledger rule: `cargo test --test cli_yaml` after `git checkout
docs/UNPORTED-CLI.md`, and the unit file with `--include-ignored`.

1. **Only-vertical-ballpark branch** (PROJ 6847–6905, faithful: candidate intermediate geographic CRS
   from the steps of the horizontal operations to the target, promoted to 3D, identified, not the
   target, in name order; `createOperations` of the vertical part to it, skipped when all ballpark;
   non-ballpark operations from the source to it and from it to the target, chained with
   `createComputeMetadata`; placed before the early return and in front of the interpolation
   candidates). It compiles. CLI ledger unchanged; unit file 5 passed, 8 ignored, unchanged. No test
   reaches it with a result (the Kumul case needs experiment 2, and `NAD27 + NAVD88 --> WGS 84`
   enters it with zero candidates). Not committed: no test witnesses it, and CLAUDE.md asks for the
   characterising test first. Patch: `scratchpad/lot/attempt0-vertical-branch.patch` (it still
   contains two debug `eprintln!` lines).
2. **Derived branch** (PROJ 6474–6501, replacing `projected_compound_to_geog`): the intermediate
   compound is built as PROJ does, `create_candidates` then `create_operations` for the two halves.
   - attempt A (`create_candidates`): one closed test went red. EGM2008 to RD NAP
     (`compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height`) went from 4 to 8 candidates.
     The four extra candidates start with `Inverse of 'WGS 84 to WGS 84 + EGM2008 height (1)'`, which
     PROJ does not return for that pair.
   - attempt B (`create_operations`, the filtered form): the same count (8) and the same red test.
   Reverted after two attempts. The Kumul first candidate was still the ballpark one, because the
   compound-to-compound loops of `compound_to_compound` (its `through_intermediate` pass) take
   another route. Patch: `scratchpad/lot/attempt2-derived-regressed.patch`.
3. **Alternative geographic fallback and PROJ-based operations kept whole** (two changes):
   - `vertical_geog_fallback` no longer returns for an identified geographic CRS (`compound.rs`
     1172), and an error of `registered_operation` is skipped in `geog_to_vertical_alternative`;
   - `compute_metadata` does not flatten a `ConcatenatedOperation` that has `proj_based` (PROJ's
     `createPROJBased` returns a `SingleOperation`, `coordinateoperationfactory.cpp` 2633).
   Result for CLI #126 with `projinfo`: one candidate, name `Inverse of WGS 84 to EGM96 height (1) +
   ETRS89-ESP [REGENTE] to Alicante height (1) using ETRS89-ESP [REGENTE] to WGS 84 (1)`, 3.05 m, PROJ
   string equal to PROJ's (both grid steps). So the candidate matches once the two changes are in.
   But `cs2cs` then finds no operation (`select_pipeline` in `src/bin/cs2cs.rs` returns
   `no operation from EPSG:4326+5773 to EPSG:4326+5782`; the legacy path then reports `unknown CRS`),
   because the first candidate needs a grid that is missing. The ballpark candidate is then not
   used, and CLI #126 and #127 go to exit 3 (#127 was passing before). Reverted: the `cs2cs` handling
   of a missing grid in the first candidate is the per-point alternatives work of another agent.
4. **`compute_metadata` keeps PROJ-based operations whole, alone**: unit 5 passed, 8 ignored; CLI
   ledger unchanged. Faithful, but nothing observable yet. Not committed. Patch:
   `scratchpad/lot/attempt3-flatten-only.patch`.

## Open items and their causes

- **Derived branch** (6474–6501): the faithful version gives extra candidates on
  `EGM2008 to RD NAP` (`compoundCRS_to_compoundCRS_WGS84_EGM2008_to_RD_new_NAP_height`), and the
  cause is not found. Something in `compound_to_compound`'s use of `compound_to_geog3d` for a
  compound target with a projected horizontal (`geog_to_compound` of `RD + NAP`) produces the
  `WGS 84 to WGS 84 + EGM2008 height (1)` chain that PROJ does not return. This is the first thing
  to fix, and it blocks `..._using_intermediate_of_horizontal_transform`.
- **Alternative geographic fallback** (`vertical_geog_fallback`, guard at 1172): needed for CLI #126.
  The guard is a deliberate Rust choice ("an identified pair keeps the registered operations"), and
  lifting it needs the `cs2cs` selection to accept a first candidate with a missing grid (the
  per-point alternatives work), and the flatten rule of experiment 4 for the name.
- **`compute_metadata` flattening PROJ-based operations** (`compound.rs` 694): PROJ keeps them
  whole (experiment 4), which is needed for the `... using ...` names of Belfast and CLI #126.
- **Vertical branch** (6847–6919): ported in experiment 1, unwitnessed. It becomes testable with the
  derived branch: the Kumul case must then give `Inverse of Papua New Guinea Map Grid 1994 zone 54 +
  PNG94 to WGS 84 (1) + Inverse of EGM96 height to Kumul 34 height (1) + ...`.
- **Kumul vertical chain**: the `Inverse of EGM96 height to Kumul 34 height (1)` step of the Kumul
  expected name is a registered vertical operation between Kumul 34 height and EGM96 height, chained
  with the EGM96 geoid operation. Whether the Rust reaches it through the vertical-to-vertical
  intermediate or through the geographic fallback is not traced.
- **`NAD27 + NAVD88 --> WGS 84` (not a unit test)**: PROJ gives `Ballpark geographic offset from NAD27
  to WGS 84 + Transformation from NAVD88 height to WGS 84 (ballpark ...)` (one candidate, strict
  containment); `target/debug/projinfo -s EPSG:4267+5703 -t EPSG:4979` gives zero candidates. The
  ballpark loop of `compound_to_geog3d` (`vertical_ballpark` or the extent check) is the suspect; not
  traced.
- `bound_compound_to_geog3d` drops the vertical part (already listed in `PORTING-compound-to-compound-2.md`).

## Commands

```bash
# Reference build used for the traces (outside the repository):
#   cmake -S <proj-git> -B <build> -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF \
#         -DENABLE_CURL=OFF -DENABLE_TIFF=OFF -DBUILD_PROJSYNC=OFF \
#         -DEXE_SQLITE3=<shim>  (a python sqlite3 script, the proj.db build needs the binary)
#   the traced variant adds -DCMAKE_CXX_FLAGS=-DENABLE_TRACING and PROJ_TRACE_FILE=<log>
PROJ_DATA=<refdata> PROJ_TRACE_FILE=<log> projinfo -s EPSG:5550+7651 -t EPSG:9755+3855 \
    -o PROJ --spatial-test intersects --grid-check known_available
cargo test --test cli_yaml
cargo test --test unit_op_compound_to_compound_2 -- --include-ignored
```
