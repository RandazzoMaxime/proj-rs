# projinfo: candidate operations between two epochs (`--s_epoch`, `--t_epoch`)

Lot: the epoch part of `projinfo -s X --s_epoch E1 -t X --t_epoch E2`, for one dynamic CRS at two
epochs. Case closed: `tests/cli_yaml/test_projinfo.yaml` #84 (`NAD83(CSRS)v7`, 1997 to 2010,
`--summary --hide-ballpark --spatial-test intersects`). No case reopened.

Reference: PROJ 9.9.0 sources (`src/apps/projinfo_lib.cpp`, `src/iso19111/operation/`,
`src/iso19111/factory.cpp`, `src/iso19111/io.cpp`), the traced build with `PROJ_DEBUG=3`, and the
reference `projinfo` binary (`reftest/build/bin`) on the grid-less `oracle-db`.

## What PROJ does

| Step | PROJ | Rust |
|---|---|---|
| `buildObject` with an epoch: `CoordinateMetadata::create(crs, epoch, db)` | `projinfo_lib.cpp:562-573` | `CoordinateMetadata::create` (`candidate_operations`) |
| A static CRS with an epoch is an error, caught by the caller | `:2216-2226`, message `outputOperations() failed with: ...` | `candidate_operations`, `metadata_error` |
| No promotion by name when an input is a `CoordinateMetadata` | `:1047-1048` | `candidates` (`with_epoch`) |
| The four overloads (metadata/metadata, metadata/CRS, CRS/metadata, CRS/CRS) | `:1085-1104` | `candidate_operations`, `create_operations_between_metadata`, `_from_metadata`, `_to_metadata` |
| The overloads set the context epochs, `setSourceCoordinateEpoch` | `coordinateoperationfactory.cpp:8129-8199` | `create_operations_at_epochs` |
| Public `createOperations`: `filterAndSort`, then the epochs on each clone | `:8104-8123` | `create_operations_between_epochs` tail, `with_epochs` |
| `createOperationsFromDatabase`, point motion branch | `:4278-4345` | `point_motion_from_database` |
| `getPointMotionOperationsFor(crs, true)`: the velocity operations of the datum, PROJ grid names | `factory.cpp:10564-10607` | `get_point_motion_operations_for(.., AlternativeGridNames::Proj)` |
| `PointMotionOperation::cloneWithEpochs`: the name `... from epoch s to epoch t` | `singleoperation.cpp:5676-5703` | `point_motion_with_epochs` |
| `ConcatenatedOperation::createComputeMetadata` | `concatenatedoperation.cpp:674-` | `compound::compute_metadata` (made `pub(super)`) |
| `PointMotionOperation::substitutePROJAlternativeGridNames` | `singleoperation.cpp:5591-5629` | `proj_grid_substitution::point_motion_grid` |
| `PROJStringFormatter::toString`: `set v_4=X` with `omit_fwd` and `omit_inv` | `io.cpp:9461-9487` | `pipeline.rs` `epoch_then_omit_fwd`, `omit_inv_then_epoch` |

## Commits

### 1. `projinfo`: the epochs reach the factory

`src/bin/projinfo.rs`. `--s_epoch` and `--t_epoch` were skipped by the parser. They are now parsed
into `Options::source_epoch` and `Options::target_epoch`, and `candidate_operations` builds the
`CoordinateMetadata` of each input that has one and calls the factory overload of PROJ's
`outputOperations`. A metadata input is not promoted by name, as in PROJ. A bad epoch on a static
CRS prints `outputOperations() failed with: ...` and exits 1, as PROJ does.

Measured alone, the candidate of case #84 changed from the no-epoch `Null geographic offset ... to
...` to a bare velocity operation, with the ballpark not hidden: the simplified point motion path
of `create_operations_at_epochs` (one level, no filter) was then the only epoch code. Commit 3
replaces it.

### 2. PROJ string: `set v_4` with `omit_fwd` and `omit_inv`

`src/iso19111/pipeline.rs`. PROJ's string formatter merges two adjacent `set v_4` steps:
`set v_4=X` then `set v_4=X +omit_fwd` gives `set v_4=X`; `set v_4=X +omit_inv` then `set v_4=X`
gives `set v_4=X` (`io.cpp:9461-9487`). A concatenation with a point motion inside it (the
geocentric and the 2D pair, once the epochs are attached) needs them: the reference prints
`set v_4=2002`, `deformation`, `set v_4=2010` for `EPSG:8253`, and the port printed the two
`omit` steps as well.

### 3. The point motion branch of `createOperationsFromDatabase`

`src/iso19111/factory.rs`, `src/iso19111/compound.rs` (`compute_metadata` made `pub(super)`).

`create_operations` is now `create_operations_between_epochs(source, target, None, context)`. The
epochs of the metadata overloads are passed as `Some((source_epoch, target_epoch))` when both are
set. Then `point_motion_from_database` runs before `create_candidates`, as the point motion block of
`createOperationsFromDatabase` runs before the registry search:

1. the two epochs are equal: nothing (PROJ's `!_isEquivalentTo`);
2. the velocity operations of the source datum (`pmoSrc`), and of the target datum (`pmoDst`),
   both with PROJ grid names; nothing when `pmoSrc` is empty or the lengths differ;
3. for each pair, `pmoSrc[i]` equivalent to `pmoDst[i]` (`Equivalence::Equivalent`), else nothing;
4. `cloneWithEpochs`; the source operation `createOperations(source, pmo.sourceCRS)` when the CRS
   differs, its first candidate; the same from `pmo.sourceCRS` to the target; then
   `createComputeMetadata` of the list;
5. the forced ballpark of `createOperationsGeodToGeod` (`forced_ballpark_operations`).

The result goes through `select_candidates` (PROJ's `filterAndSort`), then the epochs are attached
to each candidate. The one-level `point_motion_between_epochs` of the earlier code, which kept
only the same-CRS case and skipped the filter, is removed.

`tests/unit_op_point_motion_geog2d.rs`: the expected inverse of the 2D case had
`+xy_in=rad +z_in=m +xy_out=deg +z_out=m` in the last unit conversion. PROJ 9.9.0 gives
`+xy_in=rad +xy_out=deg`, both in `test/unit/test_operationfactory.cpp:11406` (the traced tree) and
in the reference `projinfo` output for the case. The expected string is changed to PROJ's; the
test comment says so. The earlier value came from the simplified path, which had no null offsets.

### 4. The velocity grid of a point motion: `DERIVED_FROM` identifiers

`src/iso19111/proj_grid_substitution.rs`. PROJ reads the point motion operations with
`usePROJAlternativeGridNames = true`. When `proj.db` has a grid alternative for the velocity file,
`PointMotionOperation::substitutePROJAlternativeGridNames` returns `createSimilarPropertiesOperation`
with the same name, and the identifiers `DERIVED_FROM(EPSG):9483`. Methods 1070, 1141 (file
parameter 1050) and 1139 (north file, 1072) are ported. The reference prints
`DERIVED_FROM(EPSG):9483` for `EPSG:8254` (the 3D pair), and our output printed `EPSG:9483`.
Inside a concatenation the identifier does not show, which is why case #84 did not need it.

### 5. Tests and this sheet

`tests/unit_op_point_motion_epochs.rs` (new): the 2D concatenation (name), the 3D velocity
operation (name, `DERIVED_FROM(EPSG):9483`, `is_point_motion`), the reverse epochs (name), and equal
epochs (the operation of the CRS alone, as without epochs). Expected values: the reference
`projinfo --summary` output.

## Result table

Reference: PROJ 9.9.0 `projinfo` on `oracle-db` with `PROJINFO_NO_GRID_CHECK=YES` (the yaml
environment), ours on `data/tests`. "SAME" is a byte comparison of stdout and stderr with the return
code, on the 26 invocations of `/tmp/.../scratchpad/epoch-compare.sh`.

| Invocation | Result |
|---|---|
| `-s NAD83(CSRS)v7 --s_epoch 1997 -t NAD83(CSRS)v7 --t_epoch 2010 --summary --hide-ballpark --spatial-test intersects` (#84) | SAME (was: different candidate) |
| same, without `--hide-ballpark` (2 candidates, the ballpark second) | SAME |
| same, `-o PROJ --hide-ballpark` (the PROJ string) | SAME |
| same, `-o PROJ -q --hide-ballpark` | SAME |
| 2010 to 1997, `--summary`, with and without `--hide-ballpark`, `-o PROJ` | SAME |
| 2010 to 2010, `--summary`, `-o PROJ` (identity) | SAME |
| `EPSG:8255` 1997 to 2010, `--summary` | SAME |
| `EPSG:8254` 1997 to 2010, `--summary` (the `DERIVED_FROM` id) | SAME |
| `EPSG:8254` 2010 to 1997, `-o PROJ` | SAME |
| `EPSG:8253` 2002 to 2010, `-o PROJ` and `--summary` | SAME |
| `EPSG:4326` 1997 to `EPSG:4326` 2010 | SAME |
| `ITRF2014` 2010 to `ITRF2014` 2020, `--summary`, `-o PROJ` | SAME |
| `NAD83(CSRS)v7 --s_epoch 1997 -t NAD83(CSRS)v7` (no target epoch) | SAME |
| `-q` with a single CRS pair and no `-o` (PROJ errors, ours does not) | DIFF, not an epoch case, see open items |
| `EPSG:4326` 1997 to `EPSG:4269` 2010 (static CRS) | SAME (the error text) |
| `NAD83(CSRS)v7` 1997 to `EPSG:4326` (no target epoch) | DIFF, pre-existing, see open items |
| `EPSG:4326` 1997 to `NAD83(CSRS)v7` 2010 | DIFF, see open items |

`cs2cs` with `--s_epoch` / `--t_epoch` is a separate path (`PORTING-cs2cs-epochs.md`: the input time
is replaced by `set v_4`, the CoordinateMetadata is not built). The reference `cs2cs` gives the
input back for `EPSG:8255` and `EPSG:8254` with 1997 and 2010 (same CRS), and so does ours. It was
not changed.

## Deviations (stated)

- **Datum pivot is not epoch-aware.** `createOperationsWithDatumPivot` passes the epochs to its
  sub-calls in PROJ. The pivot here is built without them, so a pair of two datums with epochs
  gives no point motion (case 19 above) and the pivot candidates of PROJ are missing as before.
- **Empty intersection of the point motion concatenation.** `createComputeMetadata` throws in PROJ
  (`InvalidOperationEmptyIntersection`, the error escapes `createOperations`). Here the candidate is
  left out (`compute_metadata` returns `None`).
- **An empty `createOperations` for a leg.** PROJ asserts and takes `front()`. Here the branch
  returns nothing and the registry search runs.
- **Forced ballpark.** `forced_ballpark_operations` (the same-datum, same-meridian case of
  `createOperationsGeodToGeod(forceBallpark)`) is used, not all of `createOperationsGeogToGeog`.
- **Grid names of the point motion lookup** are always `AlternativeGridNames::Proj`, as PROJ passes
  `true`: `--no-proj-grid-alternatives` does not reach that lookup.
- **Epoch change in `createOperationsGeogToGeog`** (`coordinateoperationfactory.cpp:3226-3257`: the
  velocity operations of the source or of the target geographic CRS, chained with the operations
  between the CRS at the two epochs) is not ported. It runs for two geographic CRS of different
  datums with epochs, which is the pivot case of the open items.

## Open items (not in this lot)

- `EPSG:4326` to `NAD83(CSRS)v7` with `--t_epoch` (and `NAD83(CSRS)v7` to `EPSG:4326` with
  `--s_epoch`): the datum pivot, see the first deviation. Case #19 of the invocation table.
- `NAD83(CSRS)v7 --s_epoch 1997 -t EPSG:4326` without a target epoch: PROJ prints 7 candidates,
  including `Conversion from NAD83(CSRS)v7 (geog2D) to NAD83(CSRS)v7 (geocentric) + NAD83(CSRS)v7
  to NAD83(2011) (1) + ...`. Ours prints 6. Same without epochs: the datum pivot gap, not this lot.
- `projinfo -q` with no `-o` and several output formats: PROJ refuses (`-q can only be used with a
  single output format`), ours prints the first. Not an epoch case.
- `cs2cs` epochs: its own lot (`PORTING-cs2cs-epochs.md`).

## Verification

- `cargo test --test cli_yaml` (the ledger restored before the run): 12 open `projinfo` rows
  before, 11 after. `test_projinfo.yaml#84` closed; no row reopened. The runner passed.
- `cargo test --all-features --no-fail-fast`, with the reference binaries on `PATH`,
  `PROJ_DATA` on the grid-less `oracle-db`, `PROJ_CS2CS_REQUIRED=1`: 158 test binaries,
  1629 passed, 0 failed, 34 ignored (from every `test result:` line). The two doc checks
  (`counts_in_docs_match_the_ledgers`, `progress_chart_is_current`) failed before
  `UPDATE_DOCS=1` and pass after it.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- The neighbour comparison: 26 invocations, 23 SAME, 3 DIFF (the open items above).

## Commands

```bash
cargo test --test cli_yaml                      # #84 closed; 11 open rows (12 before)
cargo test --test unit_op_point_motion_epochs
cargo test --test unit_op_point_motion_geog2d
bash <scratchpad>/epoch-compare.sh <ours projinfo> <out dir>   # the reference comparison
PROJ_DEBUG=3 PROJ_DATA=<traced refdata>:data/tests <traced projinfo> -s NAD83\(CSRS\)v7 --s_epoch 1997 -t NAD83\(CSRS\)v7 --t_epoch 2010 --summary --hide-ballpark --spatial-test intersects
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
```
