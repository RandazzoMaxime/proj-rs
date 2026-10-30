# projinfo: derived source, derived target, and the inverse of a conversion

Lot of the `projinfo` candidate port (`docs/PORTING-projinfo-candidates.md`). It ports
the derived branches of `CoordinateOperationFactory::Private::createOperations`
(`src/iso19111/operation/coordinateoperationfactory.cpp`, PROJ 9.9.0):
`createOperationsDerivedTo` for a projected source, the `applyInverse` of a projected
target, and the parts they need: `Conversion::inverse`, the inverse of a concatenated
operation, `ConcatenatedOperation::createComputeMetadata`, the extent intersection
`Extent::intersection`, the 2D target branch of `createOperationsCompoundToGeog`, and the
`--3d` promotion of `projinfo_lib.cpp`.

Not in this lot: the derived branch of a compound source (`derivedHSrc` in
`createOperationsCompoundToGeog`), `createOperationsCompoundToCompound`, the
`skipHorizontalTransformation` context, and the PROJ string of a conversion step.
See "Cases still open".

## API mapping

| PROJ (C++) | Rust |
|---|---|
| `createOperations`, `derivedSrc` branch (`createOperationsDerivedTo`) | `derived_to` (`src/iso19111/derived.rs`), reached from `derived_candidates` |
| `createOperations`, `derivedDst` branch: `applyInverse(createOperations(target, source))` | `derived_candidates`, second branch: `CoordinateOperation::inverse` on the candidates of `target` to `source` |
| `createOperationsDerivedTo`, base equivalent to target: `res.emplace_back(opFirst)` | `derived_to`: `base.is_equivalent_to(target)` returns the inverse conversion alone |
| `derivedSrc->derivingConversion()->inverse()` | `Conversion::inverse` (`src/iso19111/buildop.rs`) |
| `Conversion::inverse`, generic branch (`InverseConversion`) | `Conversion::inverse`: same method and parameters, `Inverse of` name, `INVERSE(auth)` code space, `inverted` flag |
| `createPropertiesForInverse` (name) | `inverse_name`, `uninverse_name` (`Transformation::inverse`, `Conversion::inverse`), `concatenated_inverse_name` |
| `ConcatenatedOperation::inverse` (steps reversed and inverted, name recomputed when `computedName_`) | `ConcatenatedOperation::inverse`, with `computed_name` (`src/iso19111/model.rs`) |
| `CoordinateOperation::inverse` | `CoordinateOperation::inverse` (`src/iso19111/buildop.rs`) |
| `ConcatenatedOperation::createComputeMetadata`: flatten, name, extent, accuracy | `concatenate` (`src/iso19111/derived.rs`), which reuses `pivot_concatenation` (`src/iso19111/factory.rs`) for the name and the accuracy sum |
| `getExtent(ops)`: intersection, `emptyIntersection` | `joint_extent` (`src/iso19111/derived.rs`) |
| `Extent::intersection`: the extent that contains the other is returned whole, with its description | `intersect_extents` (`src/iso19111/factory.rs`). Before this lot it always dropped the description |
| `disallowEmptyIntersection` (`crs-extent-use` is not `none`) | `CoordinateOperationContext::disallows_empty_intersection` |
| `createOperations` public call: `filterAndSort` | `select_candidates` (`src/iso19111/factory.rs`), called once by `create_operations` |
| `createOperations` private call, unfiltered | `create_candidates` (`src/iso19111/factory.rs`), used by the derived branches and by the 2D compound branch |
| `createOperationsCompoundToGeog`, `geogDst` with `axisList().size() <= 2`: `createOperations(componentsSrc[0], targetCRS)` | `horizontal_only` (`src/iso19111/compound.rs`) |
| `projinfo_lib.cpp`, `--3d`: `promoteTo3D` on source and target, and no promotion by name | `candidates` (`src/bin/projinfo.rs`) |

`geog_to_compound` (`applyInverse` of the compound branch) now inverts every candidate,
not only transformations. Before, a conversion or a concatenated candidate was dropped.

## Test mapping

Yaml cases in `tests/cli_yaml/test_projinfo.yaml` (numbered as the runner numbers them).

| Case | Command | Status |
|---|---|---|
| 16 | `-s EPSG:23031 -t EPSG:4326 --bbox … --crs-extent-use none --summary`: one candidate, `Inverse of UTM zone 31N + ED50 to WGS 84 (42)`, area `Portugal - mainland - offshore.` | pass |
| 31 | `-s EPSG:32631 -t EPSG:4326+3855 --summary`: one candidate, `Inverse of UTM zone 31N + Inverse of Null geographic offset from WGS 84 to WGS 84`, 0 m | pass |
| 32 | `-s EPSG:32631 -t EPSG:4326+3855 --3d --summary` | open, see below |
| 108 | `-s EPSG:4979 -t EPSG:10645 --hide-ballpark --summary` | open, see below |

Rust tests in `tests/derived_operation.rs`:

| Test | What it checks |
|---|---|
| `projected_to_its_base_is_the_inverse_deriving_conversion` | `32631` to `4326`: `Inverse of UTM zone 31N`, `INVERSE(EPSG):16031` (the reference prints the same line) |
| `inverse_conversion_has_no_forward_proj_string` | the inverse conversion has no forward PROJ string |
| `projected_to_compound_chains_the_inverse_with_the_base_null_offset` | `32631` to `4326+3855`: one concatenated candidate, two steps, 0 m, no identifier |
| `projected_to_geographic_inside_a_bbox_names_the_area_of_the_chained_operation` | case 16 through the library: the area of the concatenation is the description of the ED50 operation |

The 2D target branch is covered by the same candidates (`4326` to `4326+3855`, yaml
output `Inverse of Null geographic offset from WGS 84 to WGS 84`, checked by hand against
the reference). `tests/compound_operation.rs` (PR #38) still passes.

## Deliberate differences

- **Recursion is unfiltered, as in PROJ.** The derived branches and the 2D compound
  branch call `create_candidates`. A sub-list is not filtered, sorted, or stripped of
  its last ballpark before it is chained. Compound code of PR #38 still calls
  `create_operations` for its vertical branch; that is unchanged.
- **Registry first.** PROJ looks up the registry before the derived branch, and skips
  the lookup when the base is the target. Here the derived branch runs only when the
  candidates found so far are empty. Same result for the cases above.
- **No PROJ string for an inverse conversion.** `Conversion::export_to_proj_string`
  returns an error when `inverted`. PROJ writes `+step +inv +proj=utm …` followed by
  axis steps that depend on the target CRS, which the export does not have here. The
  error keeps `projinfo -o PROJ` from printing the forward string, which is wrong for an
  inverse. Such a candidate is listed without a `PROJ string:` block.
- **No inverse-pair removal.** `createComputeMetadata` removes consecutive
  `op, op.inverse()` pairs of a flattened list. Not ported; no case here produces one.
- **No recursion limit and no epochs.** PROJ throws at depth 10 and carries the
  source and target epochs. Neither is ported.
- **`Extent::intersection` is the PROJ rule.** The description of the contained
  extent is kept, which `compound` CRS extents also use (`crs_extent`). Its effect on
  the other cases was not isolated; see the verification list below.

## Preserved quirks

- A projected source whose base is the target gives the inverse deriving conversion
  alone, with its `INVERSE(...)` identifier. The concatenation is not used.
- The concatenated name is the step names joined by ` + `, with the steps reversed and
  each one inverted (`computed_name`). A stored concatenated operation takes
  `Inverse of 'name'` when its name has ` + `, as PROJ does.

## Verification

Reference: PROJ 9.9.0 under `/private/var/folders/q4/wf_7s_hs3fqftp47sll2f25c0000gn/T/opencode/proj-ref/bin`.
`PROJINFO_NO_GRID_CHECK=YES` was set for both programs, as the yaml case sets it.

- `cargo test --test cli_yaml` (harness, `PATH` to the reference): ok. The regenerated
  `docs/UNPORTED-CLI.md` is not committed.
- `projinfo` open cases on `origin/main` (f4c2ab0): **50**. On this branch: **42**.
  Cases now passing: 11, 12, 16, 29, 30, 31, 34, 107. Nothing newly failing.
- cs2cs open cases: 43 on `origin/main`, 29 on this branch. Nothing newly failing.
  Cases 95 and 96 (`ITRF96 ITRF2014`, with and without `--authority PROJ`) fail on
  `origin/main` and pass here. The cause was not isolated.
- `cargo test --all-features --no-fail-fast`: 545 passed, 0 failed, 1 ignored. The
  ignored test was already ignored before the lot (541 passed before, plus the four
  new tests).
- `cargo fmt --check`: clean.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.

## Cases still open

**Case 32** (`-s EPSG:32631 -t EPSG:4326+3855 --3d --summary`): the candidate list
matches the reference (three candidates, the same names, accuracies, areas, and
ballpark flag), but the order does not. Cause: none of the three candidates has a PROJ
string here (`projinfo -o PROJ` prints no `PROJ string:` block for them). Each is a
concatenation with the inverse UTM conversion as a step, and `export_steps`
(`src/iso19111/transform_export.rs`) returns an error for a conversion step. The sort
(`precedes`, `src/iso19111/select.rs`) ranks exportability first. All three being
non-exportable, it falls back to accuracy, which puts `(2)` (0.11) before `(1)`
(0.113), and to the ballpark flag, which puts the ballpark last. The reference has both
geoid candidates exportable (`+proj=vgridshift +grids=us_nga_egm08_25.tif`, and a grid
that is not in `grid_alternatives`), and orders them by the grid-known criterion: `(1)`
has a download URL under an open licence in `grid_alternatives`, `(2)` has none, so
`(1)` comes first.
Two things are missing for the order to match: the PROJ string of a conversion step
(it needs the CRS of the step, which `Conversion` does not carry), and the export of
`Geoid (height correction) model` (EPSG:9666, `vgridshift`), which `export_forward` does
not write.

**Case 108** (`-s EPSG:4979 -t EPSG:10645 --hide-ballpark --summary`): zero candidates
here. `EPSG:10645` is a compound CRS (`Saba DPnet + Saba height`) whose horizontal part
is the derived `Saba Transverse Mercator 2020`. The reference reaches it through the
`derivedHSrc` branch of `createOperationsCompoundToGeog` (an intermediate compound of the
base and the vertical CRS, then `createOperationsCompoundToCompound`), and composes the
vertical part first. `compound.rs` handles only a geographic horizontal component, so the
branch returns nothing. The reverse, `-s EPSG:10645 -t EPSG:4979`, is open for the same
reason.

## Not done

- `src/bin/bench.rs` cases and the `bench/history.jsonl` line that `CONTRIBUTING.md`
  asks for. The lot changes no measured path, but the requirement is not met.
