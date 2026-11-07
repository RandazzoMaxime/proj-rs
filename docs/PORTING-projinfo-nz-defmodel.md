# Status: NZGD2000 to ITRFxx, New Zealand deformation model (projinfo)

Lot: `test_projinfo.yaml#52` to `#57`, `projinfo -s NZGD2000 -t ITRFxx -o PROJ -q --spatial-test intersects --hide-ballpark`.
Reference: PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp` (the only source file of the tag at hand),
traced with `PROJ_DEBUG=3` on the traced build.

This lot closes no case. Its code is reverted (see "Attempts"). This sheet keeps what was found, so that the
next attempt starts from it.

## What PROJ does (traced)

For `NZGD2000 (geog2D, 4167) -> ITRF96 (geog2D, 8995)`:

1. `createOperationsFromDatabase`: `findOpsInRegistryDirect(4167, 8995)` is empty.
2. `createOperationsWithDatumPivot(4167, 8995)` (3167-3520). The candidate geodetic CRS of each datum are
   listed (`findCandidateGeodCRSForDatum`). Both CRS are 2D, so `sourceAndTargetAre3D` is false and nothing is
   promoted to 3D. The first pass (`iter=0`, `useOnlyDirectRegistryOp`) tries the pairs in order:
   `4167 -> 4917`, `4167 -> 7907`, `4167 -> 8995`, `4958 -> ...` (all empty), then
   `4959 (NZGD2000 geog3D) -> 7907 (ITRF96 geog3D)`. The direct lookup gives one operation, the inverse of
   the registry operation 9090 (`filtering reduced from 1 to 1`).
3. `createTransformations` builds the subOps. `opFirst` is the null geographic offset `4167 -> 4959` and
   `opThird` the null offset `7907 -> 8995`, so both are null (`isNullFirst`, `isNullThird`). Only the
   registry operation stays in `subOps`. Its identifier becomes `INVERSE(DERIVED_FROM(EPSG)):9090`
   (3297-3305). `setCRSs` gives it the source 4167 and the target 8995 (3392-3399). The result of
   `createComputeMetadata` for one operation is that operation: a `SingleOperation`, not a concatenation.
4. The reference PROJ string has no `z` unit step, because the CRS are 2D.

The second and third passes (iter 1 and 2) and `findsOpsInRegistryWithIntermediate` (2073-2298) add more
candidates. The reference list for ITRF96 has 1 candidate; for ITRF97 it has 2.

## Function map

| PROJ (coordinateoperationfactory.cpp) | Rust (`src/iso19111/factory.rs`) |
|---|---|
| `createOperationsWithDatumPivot`, `createTransformations`, 3167-3340 | `datum_pivot_operations` (first loop: pairs of candidate CRS) |
| `isNullFirst` / `isNullThird`, `addModifiedIdentifier` with `derivedFrom`, 3297-3319 | none (the null legs are dropped by `pivot_leg`, but the identifier is not changed) |
| `setCRSs` on `subOps[0]` and on the last subOp, 3392-3399 | none (the legs keep the candidate CRS, which is the cause below) |
| single-operation `createComputeMetadata` | none (the pivot is always a `Concatenated`) |
| iteration 1 and 2 of `createOperationsWithDatumPivot` (3450-3520) | `datum_pivot_operations` (second loop) |
| `findsOpsInRegistryWithIntermediate`, 2073-2298 | `datum_based_intermediate_operations`, `crs_codes_intermediate_operations` |
| `findOpsInRegistryDirect`, 1889-1970 | `direct_registry_operations` |

## Cause of the open cases

The port's datum pivot (`datum_pivot_operations`) builds `[first, operation, third]` with the null legs
dropped, and returns a `Concatenated` with no endpoint set. Its source is then the first step's CRS:
for the `4959` candidate, the 3D `NZGD2000`. PROJ sets the endpoints of the first and last subOp to the
request (`setCRSs`), and returns one operation when only one is left.

Two consequences in the yaml cases:

- `#52` (ITRF96): the port's pivot is a `Concatenated` named `Inverse of ITRF96 to NZGD2000 (7)` with a 3D
  source. Its PROJ string writes `unitconvert ... +z_in=m +z_out=m` and the result is `unknown id`.
  PROJ's is a single 2D defmodel operation, with `INVERSE(DERIVED_FROM(EPSG)):9090`.
- `#53` to `#57` (ITRF97, ITRF2000, ITRF2005, ITRF2008, ITRF2014): the first candidate of the port is the
  four-step concatenation `Inverse of ITRF96 to NZGD2000 (1) + Conversion from ITRF96 (geog3D) to ITRF96
  (geocentric) + Inverse of ITRF97 to ITRF96 (2) + Conversion from ITRF97 (geocentric) to ITRF97 (geog2D)`,
  with a 3D source (`NZGD2000 geog3D, 4959`). The steps and their text match the yaml. Only the `z` steps of
  the first `unitconvert` differ. The reference's source is the 2D `NZGD2000`.

## Attempts

Attempt 1 (closes `#52`, regresses `unit_op_no_id_crs`):

- In both loops of `datum_pivot_operations`, the legs are set with the request's endpoints, as
  `createTransformations` does (`pivot_endpoints`). A pivot of one step returns that step (no `Concatenated`).
  A null leg marks the registry operation with `DERIVED_FROM` (`derived_from_identifier`, following
  `addModifiedIdentifier`).
- Result: `#52` identical to PROJ; `cli_yaml` passes with `#52` removed and none added.
- Regression: `tests/unit_op_no_id_crs.rs`, `operation_geog_crs_without_id_to_geog_crs_3d_context`. The two
  first candidates are the same registry operation, `Amersfoort to ETRS89 (8)`, but the pivot now stores the
  requested source. The registry result has source `EPSG:4289` (name `Amersfoort`); the request is an unnamed
  2D CRS with the same datum and ellipsoid. `transformation_equivalent` compares the endpoints with `Strict`
  (`src/iso19111/equiv.rs`, `endpoint_equivalent`). Before this attempt the pivot was a `Concatenated`, whose
  equivalence ignores its own endpoints. PROJ's test expects the two to be equivalent.

Attempt 2 (same regression, not kept):

- `endpoint_equivalent` with `Equivalence::Equivalent`, and a `Geographic` arm in `Crs::is_equivalent_to_with`
  (which applied the criterion only to projected, engineering and compound CRS). The test still fails: the
  geographic comparison of the unnamed CRS does not match `EPSG:4289` under `Equivalent`. Not investigated
  further.

The two attempts regressed the same unit test, so the code was reverted, as the lot's rules say. The patch
of both attempts is kept outside the repository, at
`/tmp/claude-0/-home-user-proj-rs/3f8aee11-8dc5-5177-89d0-561606698b41/scratchpad/nz/attempts-nz-defmodel.patch`.

Investigated without a fix (for `#53` to `#57`):

- The four-step candidate is not built by `datum_pivot_operations`, `crs_codes_intermediate_operations`,
  `datum_based_intermediate_operations` (no leg reaches it), `pivot_concatenation` (no input has the ITRF96
  names), `ConcatenatedOperation::inverse`, `compute_metadata`, or `derived::concatenate`. The `pivots_used`
  loop of `create_candidates` needs `context.intermediates`, which projinfo leaves empty. Setting the endpoints
  in `crs_codes_intermediate_operations` and in `datum_based_intermediate_operations` changed nothing.
- The reference's second candidate for ITRF97, `Null geographic offset from NZGD2000 (geog2D) to NZGD2000
  (geog3D) + Inverse of ITRF96 to NZGD2000 (7) + ...`, is not produced by the port. Its export fails in PROJ
  (`Unimplemented ITRF96 to ITRF97 (1)`). The yaml compares only the first candidate.

## Next step

1. Fix the equivalence of the registry endpoints first, as its own lot: find why PROJ's
   `CoordinateOperation::_isEquivalentTo` accepts `EPSG:4289` and an unnamed 2D CRS with the same datum
   (`tests/unit_op_no_id_crs.rs`). Only then apply `pivot_endpoints` (the attempt 1 patch) on top of it.
2. Find the function that builds the four-step candidate for ITRF97 (PROJ `createFromCRSCodesWithIntermediates`,
   `io.cpp`, is not in the source tarball, so the trace is the reference). Give its first step the request's
   endpoints, as `createTransformations` does.

## Result per case (`-o PROJ -q`, port before the reverted code, reference)

| yaml | target | port | status |
|---|---|---|---|
| #52 | ITRF96 | `z` steps, `unknown id` concatenation | open (attempt 1 closes it, reverted) |
| #53 | ITRF97 | `z` steps only | open |
| #54 | ITRF2000 | `z` steps only | open |
| #55 | ITRF2005 | `z` steps only | open |
| #56 | ITRF2008 | `z` steps only | open |
| #57 | ITRF2014 | `z` steps only | open |

Neighbours (checked with attempt 1 and PROJ 9.9.0, `-o PROJ -q --spatial-test intersects --hide-ballpark`):
`ITRF96 -> NZGD2000` identical; `NZGD1949 -> ITRF96` identical.

## Commands run

- `cargo test --test cli_yaml`: passes with attempt 1 (`#52` removed, none added), and on the reverted tree.
- Full suite with attempt 1 (`cargo test --all-features --no-fail-fast`, PROJ 9.9.0 on `PATH`): 157 test
  binaries, 1622 passed, 3 failed, 34 ignored. The 3 failures: `counts_in_docs_match_the_ledgers` and
  `progress_chart_is_current` (the ledger changed, expected), and
  `operation_geog_crs_without_id_to_geog_crs_3d_context` (the regression above).
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`: clean with attempt 1.
