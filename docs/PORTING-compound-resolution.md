# Port — compound CRS resolution (`EPSG:A+B`, vertical branch)

Source: PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp`:
`createOperations` dispatch (around 3735–3790, `compoundSrc && geogDst`,
`geodSrc && compoundDst`), `createOperationsCompoundToGeog` (6454–7190, the
vertical part from "Only do a vertical transformation" on),
`createOperationsVertToGeogSynthetized` (6113), `createOperationsGeogToGeog`
(2923), `createHorizVerticalPROJBased` (2717). Input side:
`src/io.cpp` (`createFromUserInput`, `EPSG:A+B` at 7871–7891) and
`src/apps/projinfo_lib.cpp` (`outputOperations`, the 3D name promotion at
1048–1070, `is3DCRS` at 973).

## Scope

Lot: cs2cs and projinfo entries that resolve `EPSG:A+B`, and the vertical
branch of `createOperationsCompoundToGeog`. The epoch work in
`src/bin/cs2cs.rs` is not touched.

Input resolution already reads `EPSG:A+B` (`crs_from_authority_code` in
`src/iso19111/json.rs`, a port of `io.cpp`). What was missing was the factory
candidate list, so `cs2cs` failed on the old path with `unknown CRS
EPSG:4326+3855`. Two input changes were needed in `projinfo` only.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `createOperationsCompoundToGeog`, vertical part | `compound::compound_to_geog3d` (`src/iso19111/compound.rs`), called from `CoordinateOperationFactory::create_operations` |
| `applyInverse` of the dispatch for `geodSrc && compoundDst` | `compound::geog_to_compound` |
| `createOperationsVertToGeogSynthetized` (ballpark branch only) | `vertical_ballpark` |
| `createOperationsGeogToGeog`, same datum and prime meridian, the 2D to 3D pair | `null_offset` (name by `buildOpName`, `op_name`) |
| `createHorizVerticalPROJBased` (null horizontal part) | `combine`, with `retarget` copying the vertical operation |
| `createHorizVerticalPROJBased` (other horizontal part) | `combine`, a `ConcatenatedOperation` named `A + B` |
| `projinfo_lib.cpp` name promotion (`outputOperations`) | `promote_by_name`, `is_3d`, `promote` in `src/bin/projinfo.rs` |
| `projinfo --3d` (`promoteTo3D`) | `Options::promote_3d` (the flag was parsed and ignored before) |

## Behaviour

For a source compound of two components, a geographic CRS then a vertical CRS,
to a 3D geographic target (`compound_to_geog3d`):

1. Horizontal candidates: `create_operations(horizontal, target)`. If none, the
   null 2D to 3D offset of the same datum. If still none, the result is empty.
2. Vertical candidates: `create_operations(vertical, target)` (registry), plus
   the ballpark change of vertical unit. If none, PROJ returns the horizontal
   candidates unchanged, and so does this port.
3. Each pair (vertical, horizontal) is one candidate. A null horizontal part
   gives the vertical operation alone, with the compound source and the target
   and no identifier (`retarget`). The accuracy is written as a double, so
   `1.0` becomes `1` (PROJ's `toString(accuracy)`).

A geographic source and a compound target (`geog_to_compound`) take the
candidates of the compound-to-geographic direction and invert each
transformation, as PROJ's `applyInverse` does.

`projinfo` promotes a CRS given by its name to 3D when the other CRS is 3D
(`-s "WGS 84" -t "WGS 84 + EGM96 height"` gives `EPSG:4979`), and keeps the
promoted CRS only when the database gives it an identifier. `--3d` disables the
promotion, as in PROJ.

`cs2cs` needed no change: `best_pipeline` now finds the candidate, and
`select_pipeline` exports it.

## Deliberate differences and not ported

- **Interpolation CRS of a vertical operation.** PROJ builds
  `createHorizVerticalHorizPROJBased` for a vertical operation with an
  interpolation geographic CRS and skips it in the final loop. Here every
  vertical operation takes the `combine` path. The candidates are the same
  for the cases in the yaml suites, but the operations are not the PROJ ones.
- **`geogCRSTmp` block.** The retry of the vertical part on the source
  geographic CRS promoted to the target's height (for a horizontal datum that
  differs from the target) is not ported.
- **Same datum, different vertical unit.** PROJ builds a conversion or a
  PROJ-based operation. Here `null_offset` gives nothing and the ballpark is
  not built (`vertical_ballpark` returns `None` when the datum names are equal).
- **Concatenated candidates.** No extent check (`emptyIntersection`), and no
  accuracy on the concatenation (`accuracy: None`).
- **Inverse of a conversion or of a concatenation.** `geog_to_compound` keeps
  transformations only. PROJ's `applyInverse` inverts every operation.
- **2D compound target.** PROJ takes the horizontal part alone when the target
  has two axes. Here the branch returns nothing for a 2D target.
- **Projected or compound source.** The derived-CRS path
  (`createOperationsDerivedTo`, the inverse of the deriving conversion) is not
  ported, and `Conversion` has no inverse. Compound horizontal components must
  be geographic.
- **Pipeline export of a retargeted operation.** The candidate keeps the
  vertical operation's method and parameters. PROJ builds a new PROJ-based
  operation. The pipeline text of the two agrees for the cases checked.

## Preserved quirks

- A null horizontal part keeps the vertical operation's name (`Inverse of WGS 84 to EGM2008 height (1)`), as PROJ does.
- The ballpark vertical name is `Transformation from <vertical> to <target> (ballpark vertical transformation, without ellipsoid height to vertical height correction)`.
- The 2D to 3D null offset is named `Null geographic offset from WGS 84 (geog2D) to WGS 84 (geog3D)`, with the qualifiers only when the two names are equal.

## Test mapping

| Case | Result |
|---|---|
| `test_cs2cs_various.yaml#116`, `#117` (`EPSG:4326+3855` to `EPSG:4979`, grid missing) | now pass |
| `test_cs2cs_various.yaml#111` (`--only-best`, `EPSG:4326+5773` to `EPSG:4979`) | now passes (it used to fail on the unknown CRS) |
| `test_projinfo.yaml#29`, `#30` (`WGS 84` and `WGS 84 + EGM96 height`) | now pass |
| `tests/compound_operation.rs` | three characterization tests on the candidate names and the pipeline |

## Remaining cases in this lot, and their cause

| Cases | Cause |
|---|---|
| `cs2cs` `#118`–`#121` (`--only-best`, `PROJ_ONLY_BEST_DEFAULT`, `only_best_default=on`) | `cs2cs` ignores `--only-best`: `opts.only_best` is read into `_selection` (`src/bin/cs2cs.rs`). PROJ returns the first candidate's error instead of falling back, and the ballpark is printed here. This is selection, outside this lot. |
| `cs2cs` `#126`–`#128` (`EPSG:4326+5773` to `EPSG:4326+5782`) | Compound target: `createOperationsCompoundToCompound` is partly ported (`docs/PORTING-compound-to-compound.md`). `#127` passes. `#126` and `#128` need the geoid branch and an interpolation CRS, not ported. |
| `projinfo` `#32` (`EPSG:32631` to `EPSG:4326+3855 --3d`) | Projected source: the derived-CRS path (`createOperationsDerivedTo`) and the inverse of the UTM conversion are not ported. |
| `projinfo` `#108` (`EPSG:4979` to `EPSG:10645`) | Projected horizontal component (`Saba Transverse Mercator 2020`): same cause as `#32`. |

## Verification

Harness (`PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`):

- `origin/main` at `2371f34`: 34 open `cs2cs` rows and 46 open `projinfo` rows in `docs/UNPORTED-CLI.md`.
- This branch: 31 and 44. Closed: `cs2cs` `#111`, `#116`, `#117`; `projinfo` `#29`, `#30`. No row newly open. `cs2cs` `#95`, `#96` (ITRF96 to ITRF2014) stay passing.
- The run exits 0, and the previously passing check reports no regression.

`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features --no-fail-fast` (with the same `PATH` and variables): all clean, every test binary reports 0 failed.
