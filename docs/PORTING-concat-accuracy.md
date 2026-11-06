# Accuracy of a concatenated operation (PROJ 9.9.0)

The accuracy of a concatenated operation built by the factory. Before this lot the port kept the
largest step accuracy (`factory.rs` `pivot_concatenation`), and `compound.rs` summed them without
PROJ's ETRS89 case. PROJ's `getAccuracy` sums the steps, with an ETRS89 ensemble case, and stores
the sum as text. Accuracy sorts the candidates, so the sum changes which candidate `cs2cs` picks.

Example: `PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (2)` (`EPSG:4746` to `EPSG:4230`) is 6 m
here and 7 m in PROJ.

## Code

- `src/iso19111/compound.rs`:
  - `flatten_steps`: the steps `createComputeMetadata` keeps (a concatenated step is flattened, a
    PROJ-based one is not).
  - `get_accuracy_of_steps`: `getAccuracy(vector)`, the sum, -1 when a step is unknown, with the
    ETRS89 case.
  - `is_etrs89_ensemble_pair`, `is_noop`: the two tests of that case.
  - `stored_accuracy`: the sum as PROJ stores it (`toString`, 15 significant digits).
  - `get_accuracy`: `getAccuracy(op)`; now calls `get_accuracy_of_steps` for a concatenated
    operation with no stored accuracy.
  - `compute_metadata`: uses `flatten_steps`, `get_accuracy_of_steps` and `stored_accuracy`.
- `src/iso19111/factory.rs` `pivot_concatenation`: the accuracy is `getAccuracy` of the flattened
  steps, stored with `stored_accuracy` (was the maximum of the steps).
- `tests/concat_accuracy.rs` (new, 3 tests).

## PROJ file:line map

| PROJ 9.9.0 | This port |
|---|---|
| `src/iso19111/operation/oputils.cpp:560-585` `getAccuracy(op)`: conversion 0, else stored accuracy, else the steps of a concatenated operation | `compound.rs` `get_accuracy` |
| `oputils.cpp:587-636` `getAccuracy(ops)`: sum; -1 when one is unknown | `compound.rs` `get_accuracy_of_steps` |
| `oputils.cpp:600-615` the ETRS89 case: the larger of two neighbouring steps, once, when both are at most 0.1 m, the middle CRS is `ETRS89` or `ETRF*`, the ends are `ETRS89-*`, and one step is a no-op | `compound.rs` `is_etrs89_ensemble_pair`, `is_noop` |
| `oputils.cpp:617-630` `IsNoOp`: the PROJ string is `+proj=noop`; an exception gives false | `compound.rs` `is_noop` |
| `src/iso19111/operation/concatenatedoperation.cpp:674-698` `createComputeMetadata`: flatten a `ConcatenatedOperation` (686-697); a PROJ-based operation is one step | `compound.rs` `flatten_steps`, `compute_metadata` |
| `concatenatedoperation.cpp:700-731` removal of consecutive inverse steps | not ported (deviation 2) |
| `concatenatedoperation.cpp:754-758` the accuracy: `getAccuracy(flattenOps)`, stored as `toString(accuracy)` when it is not -1 | `compound.rs` `compute_metadata`, `factory.rs` `pivot_concatenation`, `stored_accuracy` |
| `src/iso19111/internal.cpp:343-357` `toString(double, 15)`: `%.15g`, 14 digits when the text holds `9999999999` | `compound.rs` `stored_accuracy` |
| `src/iso19111/operation/c_api.cpp:8754-8778` `proj_coordoperation_get_accuracy`: stored accuracy only, -1 when absent, used by `crs_to_crs.cpp:153` | `suggest.rs` `prepare` (stored, unchanged) |
| `src/iso19111/operation/coordinateoperationfactory.cpp:891` the sort key `accuracy_(getAccuracy(op))` | `select.rs` `Characteristics` accuracy (criterion B) |
| `coordinateoperationfactory.cpp:1608-1655` `removeUninterestingOps` (`getAccuracy(op)` at 1621) | `factory.rs` `remove_uninteresting_ops` (criterion B) |
| `coordinateoperationfactory.cpp:3562-3574` `hasPerfectAccuracyResult` (`getAccuracy(op)` at 3568) | `factory.rs` `has_perfect_accuracy_result` (criterion B) |
| `coordinateoperationfactory.cpp:1325-1330` the desired-accuracy filter: `getAccuracy(op)`, dropped when below 0 or above `desiredAccuracy` | `factory.rs` `accuracy_allowed` (criterion B) |

## Criterion B

`getAccuracy(op)` (`compound.rs` `get_accuracy`) replaces the stored-only reads at the sort key
(`select.rs`), `remove_uninteresting_ops`, `has_perfect_accuracy_result` and `accuracy_allowed`
(`factory.rs`). The stored-only `operation_accuracy` is removed. Behaviour differs only for a
concatenated operation with no stored accuracy, which now sums its steps.

Oracle: the 32-pair table is byte-identical to criterion A (19 SAME, 13 DIFF). The CLI ratchet reopens
no case. Full suite: 1567 passed, 0 failed.

Left as stored-only on purpose: `suggest.rs` `prepare` (PROJ `proj_coordoperation_get_accuracy`,
`c_api.cpp:8754`, stored only, used by `crs_to_crs.cpp:153`) and `compound.rs` `proj_geoid_operation`
(its best-of-grid loop reads transformations, which are never concatenations; PROJ uses
`getAccuracy(transf)` at `coordinateoperationfactory.cpp:4790`, so the two are equal there).

## Deviations from PROJ

1. Flattening only feeds the accuracy. `pivot_concatenation` keeps its nested steps (the
   structure and the name of the operation are unchanged), and the accuracy is computed on the
   flattened list, as PROJ's `createComputeMetadata` does. `compute_metadata` keeps its flattened
   steps as before.
2. Consecutive inverse steps are not removed before the sum (`concatenatedoperation.cpp:700-731`),
   as the `compound.rs` module doc already says. Its effect on the accuracy is not measured: no
   pair of the table above was checked for it.
3. `get_accuracy` for a concatenated operation that has no stored accuracy sums its steps
   (`oputils.cpp:577-579`). Its callers in `compound.rs` (the candidate ranking at lines 429 and
   618) already used this function; the summing now follows the ETRS89 case as well.

## Oracle comparison (`projinfo --summary --spatial-test intersects`, 32 pairs)

Each row is a pair. `o` is the number of candidates PROJ 9.9.0 prints, `p` the number this port
prints. `SAME` means that the list of (id, name, accuracy) lines is identical, in the same order.
Both sides run on the same grid-less `proj.db` copy (the oracle on its own database, the port on
its copy of ours), with `PROJ_DATA` set to that directory only (`scratchpad/concat-accuracy/cmp.sh`,
`run_pairs.sh`, `pairs.txt`).

| pair | base | A (this commit) |
|---|---|---|
| `EPSG:26915 -> EPSG:26715` | o5 p5 SAME | o5 p5 SAME |
| `EPSG:26715 -> EPSG:26915` | o5 p5 SAME | o5 p5 SAME |
| `EPSG:4312 -> EPSG:4230` | o7 p8 DIFF | o7 p8 DIFF (7 shared accuracies now equal PROJ) |
| `EPSG:4230 -> EPSG:4312` | o7 p8 DIFF | o7 p8 DIFF |
| `EPSG:4230 -> EPSG:4978` | o36 p36 DIFF | o36 p36 DIFF |
| `EPSG:4978 -> EPSG:4230` | o36 p36 DIFF | o36 p36 DIFF |
| `EPSG:4267 -> EPSG:4978` | o79 p79 DIFF | o79 p79 DIFF |
| `EPSG:4269 -> EPSG:4978` | o54 p53 DIFF | o54 p53 DIFF |
| `EPSG:4746 -> EPSG:4978` | o2 p2 SAME | o2 p2 SAME |
| `EPSG:4978 -> EPSG:4746` | o2 p2 SAME | o2 p2 SAME |
| `EPSG:4326 -> EPSG:2154` | o1 p1 SAME | o1 p1 SAME |
| `EPSG:4267 -> EPSG:26915` | o5 p5 SAME | o5 p5 SAME |
| `EPSG:4269 -> EPSG:26710` | o5 p5 SAME | o5 p5 SAME |
| `EPSG:23031 -> EPSG:4326` | o16 p16 DIFF | o16 p16 SAME |
| `EPSG:31467 -> EPSG:4258` | o7 p7 SAME | o7 p7 SAME |
| `EPSG:4230 -> EPSG:4936` | o16 p15 DIFF | o16 p15 DIFF |
| `EPSG:4312 -> EPSG:4936` | o3 p3 SAME | o3 p3 SAME |
| `EPSG:4277 -> EPSG:4978` | o9 p9 SAME | o9 p9 SAME |
| `EPSG:4272 -> EPSG:4326` | o4 p4 DIFF | o4 p4 DIFF |
| `EPSG:4326 -> EPSG:4978` | o1 p1 SAME | o1 p1 SAME |
| `EPSG:4258 -> EPSG:4978` | o2 p1 DIFF | o2 p1 DIFF |
| `EPSG:3912 -> EPSG:3794` | o36 p24 DIFF | o36 p24 DIFF |
| `EPSG:4326 -> EPSG:32631` | o1 p1 SAME | o1 p1 SAME |
| `EPSG:4258 -> EPSG:25832` | o1 p1 SAME | o1 p1 SAME |
| `EPSG:4149 -> EPSG:4978` | o2 p2 SAME | o2 p2 SAME |
| `EPSG:4230 -> EPSG:25830` | o8 p8 SAME | o8 p8 SAME |
| `EPSG:4267 -> EPSG:4326` | o79 p79 DIFF | o79 p79 DIFF |
| `EPSG:4326 -> EPSG:4267` | o79 p79 DIFF | o79 p79 DIFF |
| `EPSG:2154 -> EPSG:4326` | o1 p1 SAME | o1 p1 SAME |
| `EPSG:4746 -> EPSG:25832` | o2 p2 SAME | o2 p2 SAME |
| `EPSG:4936 -> EPSG:4230` | o16 p15 DIFF | o16 p15 DIFF |
| `EPSG:4326 -> EPSG:3912` | o3 p3 SAME | o3 p3 SAME |

Totals: base 18 SAME, 14 DIFF; A 19 SAME, 13 DIFF. A changes one pair, `EPSG:23031 -> EPSG:4326`.
No pair got worse. Outside the harness, `EPSG:4746 -> EPSG:4230` (the example above) now lists the
same 7 candidates with the same accuracies as PROJ (7, 11, 3, 2, 2.3, 3, and the ballpark), and the
ETRS89 candidate of `EPSG:11108 -> EPSG:11128` is 0.1 m, as PROJ.

The remaining DIFF pairs differ in candidate sets and names, not in the accuracy of a candidate they
share. Examples: `EPSG:4312 -> EPSG:4230` prints one extra candidate, `MGI to ETRS89 (5) + Inverse of
ED50 to ETRS89 (10)` at 2.15 m, that PROJ does not list; `EPSG:4272 -> EPSG:4326` names a
registry step `EPSG:1670` where PROJ prints `DERIVED_FROM(EPSG):1670`; `EPSG:4258 -> EPSG:4978` lacks
the axis-order variant of PROJ.

## Tests

- `tests/concat_accuracy.rs` (new):
  - `concatenation_accuracy_is_the_sum_of_its_steps` (`EPSG:4746 -> EPSG:4230`: 7 m and 11 m).
    FAILS on the base commit (6 m).
  - `mgi_to_ed50_candidates_carry_the_summed_accuracies` (`EPSG:4312 -> EPSG:4230`: 7.5, 11.5,
    6.5, 10.5, 3.5, 2.5, and 2.14 exactly). FAILS on the base commit (6 m for 7.5).
  - `two_etrs89_legs_through_the_ensemble_count_once` (`EPSG:11108 -> EPSG:11128`: 0.1 m).
    Passes on both: it guards the ETRS89 case, which the sum would break (0.2 m).
- The three tests take the expected values from `projinfo -s ... --summary` of PROJ 9.9.0, not
  from this port.
- Full suite with the oracle `cs2cs` on `PATH` (`agree_cs2cs` runs against it): criterion A 1567
  passed, 0 failed; with criterion B also 1567 passed, 0 failed. The base commit had 1564 passed;
  the three new tests make 1567.

## CLI ratchet (`tests/cli_yaml`)

No row of `docs/UNPORTED-CLI.md` closed or reopened: the file is byte-identical to the committed
file after the run.

## Test expectations that disagree with the oracle

None found in this lot.

## Open items

- `EPSG:26915 -> EPSG:26715` (`test_cs2cs_various.yaml#125`) is not closed by accuracy. PROJ's
  `cs2cs`, with the grid missing, falls back from the 0.15 m candidate to the WGS 84 pivot
  `Inverse of UTM zone 15N + NAD83 to WGS 84 (1) + Inverse of NAD27 to WGS 84 (6) + UTM zone 15N`
  (11 m). That pivot exists only because of `hasTooSmallAreas` (`coordinateoperationfactory.cpp:
  3897`, used at `4474`), which this port does not implement; the other agent working on
  intermediate-CRS pivots owns it. With `--pivot-crs always` the oracle lists the pivot with 11 m, so
  the accuracy computed here is the one PROJ gives it.
- `EPSG:4312 -> EPSG:4230`: the extra candidate `MGI to ETRS89 (5) + Inverse of ED50 to ETRS89 (10)`
  (2.15 m) is not in PROJ's list. Cause not identified; it is not an accuracy difference, since its
  accuracy matches the oracle's form.
- `test_cs2cs_various.yaml#139` (`EPSG:3912+EPSG:5779 -> EPSG:3794+EPSG:8690`) is not examined in this
  lot; it stays open.
- The consecutive-inverse removal of `createComputeMetadata` (deviation 2) is not ported.
- Criterion B (second commit, see "Criterion B" below): the sort key and the three factory helpers
  read `getAccuracy(op)` as PROJ does. It changes no row of the oracle table, so it is kept on the
  faithful-port argument only; its effect on a DB concatenation with no stored accuracy is not
  measured.

## Commands

```bash
# oracle (PROJ 9.9.0 build, data in build/data)
env PATH=<oracle build>/bin:$PATH PROJ_DATA=<oracle build>/data:$PWD/data:$PWD/data/tests \
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
projinfo -s EPSG:4312 -t EPSG:4230 --summary --spatial-test intersects
bash scratchpad/concat-accuracy/run_pairs.sh <out>   # the 32-pair table
cargo test --test cli_yaml
cargo test --test concat_accuracy
```
