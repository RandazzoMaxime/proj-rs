# Port — compound to compound (`EPSG:A+B` to `EPSG:C+D`), partial

Source: PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp`:
`createOperations` dispatch (around 3787, `compoundSrc && compoundDst`),
`createOperationsCompoundToCompound` (7214–7847), `createOperationsVertToVert`
(5995), `createOperationsFromDatabaseWithVertCRS` (5142), `createHorizVerticalPROJBased`
(2717), `createHorizVerticalHorizPROJBased` (2795).

## Scope

Lot: the vertical part of `createOperationsCompoundToCompound`, for two compound CRS
whose geographic parts are equivalent, and the ballpark of `createOperationsVertToVert`
that it needs. The geoid trigger, the three-pass selection, and the interpolation CRS
of a vertical operation are not ported: the cases that need them stay open (see below).

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `createOperationsCompoundToCompound`, vertical loop, null horizontal part | `compound::compound_to_compound` (`src/iso19111/compound.rs`), called from `CoordinateOperationFactory::create_candidates` |
| `createOperationsVertToVert`, different datums (`createChangeVerticalUnit`, `setHasBallparkTransformation`) | `compound::vertical_to_vertical`, called from `create_candidates` when the registry gives nothing |
| `createOperationsVertToVert`, same datum (a conversion) | not ported: `vertical_to_vertical` returns `None` |
| scale of the change of vertical unit, with the height/depth reversal | `compound::ballpark_scale`, shared with `vertical_ballpark` (vertical to geographic 3D) |
| `createHorizVerticalPROJBased`, null horizontal part | `retarget` (already in `compound.rs`) |

## Behaviour

`compound_to_compound` for a compound source and a compound target:

1. Both have two components. The geographic parts are equivalent (`Equivalence::Equivalent`),
   and the vertical parts differ by name. Otherwise the result is empty.
2. Vertical candidates: `create_candidates(vertical, vertical)`, the internal call that
   PROJ makes without the public filters. Registry first, then the ballpark.
3. A candidate is kept only when its horizontal part is null: a conversion, or a
   transformation with no interpolation CRS, or one whose interpolation CRS is the
   geographic part. Each kept candidate is copied with the compound source and target
   and no identifier (`retarget`).

`vertical_to_vertical` for two vertical CRS of different datums: a transformation of
method 1069 named `Transformation from <source> to <target> (ballpark vertical transformation)`,
with the scale of the unit change (negated on a height/depth reversal), and the common
extent of the two CRS, or the world extent.

A standalone vertical pair now lists its ballpark, as PROJ does
(`projinfo -s EPSG:5773 -t EPSG:5782` gives one candidate with that name, the same as PROJ).

## Deliberate differences and not ported

- **Geoid trigger and three-pass selection.** PROJ sets `bTryThroughIntermediateGeogCRS`
  when a vertical CRS has a geoid model, and first tries the concatenations
  `source compound -> WGS 84 3D -> target compound`, with the ballpark exclusions of
  three passes. None of that is ported. The vertical loop runs directly.
- **Interpolation CRS of a vertical operation.** PROJ builds
  `createHorizVerticalHorizPROJBased`, with the horizontal operations to and from the
  interpolation CRS. Here a vertical operation with an interpolation CRS other than the
  geographic part is skipped, not built. This avoids a candidate with the wrong datum shift.
- **Concatenated vertical operations** are skipped (`horizontal_is_null` is false). PROJ
  builds them with the horizontal part from the interpolation CRS.
- **Empty vertical list.** PROJ falls back to the horizontal operations, re-labelled with
  the compound CRS. Not ported: the result is empty.
- **Geographic parts that differ**, bound CRS, and the datum pivot of the horizontal part
  are not ported. The result is empty.
- **Vertical equality** is decided by name, not by `_isEquivalentTo`. Datums are compared
  by name in `vertical_to_vertical` (`vertical_datum_name`), as `vertical_ballpark` already did.
- **Same datum, different vertical unit** gives nothing. PROJ builds a conversion.

## Preserved quirks

- The ballpark vertical name is `Transformation from <source> to <target> (ballpark vertical transformation)`
  for two vertical CRS, and keeps the longer suffix for vertical to geographic 3D.
- A retargeted candidate has no identifier, and its accuracy is written as a double.

## Test mapping

| Case | Result |
|---|---|
| `test_cs2cs_various.yaml#127` (`--only-best=no`, `EPSG:4326+5773` to `EPSG:4326+5782`) | now passes. Its output is compared numerically, see below. |
| `tests/compound_operation.rs::vertical_pair_of_different_datums_lists_the_ballpark_change_of_unit` | the one candidate PROJ gives for `EPSG:5773` to `EPSG:5782` |
| `tests/compound_operation.rs::compound_pair_with_the_same_geographic_part_keeps_the_ballpark` | the ballpark is listed. PROJ also lists a grid-based candidate first, not ported |

## Remaining cases in this lot, and their cause

| Case | Cause |
|---|---|
| `cs2cs` `#126` (`EPSG:4326+5773` to `EPSG:4326+5782`, default) | Expected stderr names the candidate `Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) using ETRS89-ESP [REGENTE] to WGS 84 (1)`, 3.05 m. Its name and accuracy are consistent with the geoid branch's concatenation of `Inverse of WGS 84 to EGM96 height (1)` (1 m) and `ETRS89-ESP [REGENTE] to Alicante height (1) using ETRS89-ESP [REGENTE] to WGS 84 (1)` (2.05 m, the `EPSG:4979 -> EPSG:4326+5782` candidate). This chain was inferred from those values, not traced in PROJ. Here the candidate list is only the ballpark (measured). Two parts are missing: the vertical candidate with an interpolation CRS (`createHorizVerticalHorizPROJBased`; measured: `EPSG:4979 -> EPSG:4326+5782` gives the ballpark only, PROJ gives two candidates), and the registry operation `EPSG:11594` reached through an alternative geographic CRS (measured: `EPSG:5782 -> EPSG:4979` gives zero candidates, PROJ gives `INVERSE(DERIVED_FROM(EPSG):11594)`). |
| `cs2cs` `#128` (`--only-best`, same pair) | Still open, with a different failure. The yaml expects exit 3: the grid-based candidate is the best, and `--only-best` refuses it. Now exit 0, because the ballpark is the only candidate and `--only-best` is not applied (`src/bin/cs2cs.rs`, see `PORTING-cs2cs-operation-selection.md`). The baseline ledger showed no exit mismatch for this case, so exit 3 was matched before this lot. |
| `projinfo` `#108` (`EPSG:4979` to `EPSG:10645`) | Not in the compound branch. The target is a derived projected CRS, and PROJ's one candidate chains `Saba to Saba height (1)`, a geographic-to-vertical operation. Measured: zero candidates here. |

## Output format of a compound target (observed, not in this lot)

`cs2cs EPSG:4326+5773 EPSG:4326+5782` prints `39dE	3dS 0.000` here, where PROJ prints
`39.00	-3.00 0.00`. The compound target takes the layout of its horizontal geographic part,
so the output is DMS (`src/bin/cs2cs.rs`, `layout_of_type`, `kind == "compound"`).
The yaml comparison (`compare_numbers`, `token_number` in `tests/cli_yaml.rs`) reads
`39dE` as 39, so `#127` passes on the numbers. The output format itself is not PROJ's.

## Verification

Harness (`PATH=…/proj-ref/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`), from the clean ledger of `origin/main` at `6e128e3`:

- Before: 65 open rows (cs2cs 23, projinfo 42).
- After: 64 open rows (cs2cs 22, projinfo 42). Closed: `cs2cs` `#127`. Newly open: none.
- `cs2cs` `#95`, `#96` (ITRF96 to ITRF2014) still pass.

`cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --all-features --no-fail-fast` (same `PATH` and variables): clean. The suite
reports 558 passed and 0 failed across 26 test binaries, and the run exits 0.
