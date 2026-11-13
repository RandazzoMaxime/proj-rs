# Porting sheet: grids used and compound normalization (lot `unit-grid-used-normalize`)

Two unit tests from PROJ 9.9.0 close in this lot:

- `CApi.proj_coordoperation_get_grid_used_fullname_caching`: `test/unit/test_c_api.cpp` 1775-1801,
  ported to `tests/unit_capi_grid_used_caching.rs`.
- `operation.normalizeForVisualization`: `test/unit/test_operation.cpp` 5607-5768, the two blocks
  still open (compound source, bound source) added to `tests/unit_op_normalize_for_visualization.rs`.
  Its other blocks were already ported.

## Map: PROJ function -> Rust

| PROJ 9.9.0 | Lines | Rust | Status |
|---|---|---|---|
| `proj_coordoperation_get_grid_used_count`, `proj_coordoperation_get_grid_used` (`c_api.cpp`) | 7921-8006 | `Proj::grids_needed` (`src/proj.rs`) | ported, returns the list rather than an index |
| `PROJBasedOperation::gridsNeeded` (`projbasedoperation.cpp`) | 298-322 | `CoordinateOperation::grids_needed` (`src/iso19111/select.rs`), `Conversion` arm of `collect_file_parameters` | ported |
| `PROJStringFormatter::getUsedGridNames` (`io.cpp`) | 10512-10525 | `proj_based_grid_names` (`src/iso19111/select.rs`) | ported: `grids` and `file` keys of every step |
| `CRS::applyAxisOrderReversal`, compound branch (`crs.cpp`) | 1015-1085 | `reversed`, `Crs::Compound` arm (`src/iso19111/normalize.rs`) | ported |
| `createProperties(newName)` (`crs.cpp`) | 1017-1062 | `reverse_identified(identified, new_name)` (`src/iso19111/normalize.rs`) | ported: an explicit name, else the suffix toggle |

## Behaviour checked

- `proj_create("proj=vgridshift grids=tests/test_vgrid_int16.tif")`: one grid, short name as written,
  full name on the search path (`data/tests/`), on each of two passes.
- Known grids: `grids_needed` counts a grid the database knows as available only when network access
  is enabled (`GridAvailabilityUse::KnownAvailable`), as `gridsNeeded(dbContext,
  proj_context_is_network_enabled(ctx))` does.
- Compound normalization: a compound CRS whose first component needs the reversal gets that component
  reversed, the other components kept, the name is the component names joined by ` + `, and the
  identifier is dropped (`crs.cpp` 1015-1085). The `+proj=vgridshift +grids=us_nga_egm08_25.tif`
  step of the test comes from `createOperations` with the official grid names.

## Deliberate deviations

- No cache. The C API stores the grid list in the `PJ` on the first call (`gridsNeeded`,
  `gridsNeededAsked`) and returns pointers into it. `Proj::grids_needed` returns owned values and
  recomputes on each call, opening `proj.db` each time. The regression the C++ test guards (a
  pointer into a freed or reused entry) has no counterpart; the test checks that two passes agree.
- Index access is not ported: `Vec` indexing replaces `proj_coordoperation_get_grid_used(index)`.
- `ProjError` gains a variant `GridsNeeded(String)`. The enum is `#[non_exhaustive]`.
- The bound-source block (case 6) exports with `export_to_proj_string_with_database`, where PROJ uses
  `PROJStringFormatter::create()` without a database. The operation has no grid, so the string is the
  same; the test does not check that difference.
- `reversed` is also the function `fix_steps` uses in `src/iso19111/wkt_import.rs` (WKT2_2019 import of a
  concatenated operation). A compound CRS first component is now reversed there too, as PROJ's
  `applyAxisOrderReversal` does. No existing test changed.

## Not closed

- `CApi.proj_normalize_for_visualization_with_alternatives` is ported in
  `tests/unit_capi_normalize_alternatives.rs`, but its `proj_pj_info(...).definition` check after
  `proj_trans` needs a recorded candidate on `Proj` (see `docs/PORTING-visualization-alternatives.md`).
  The line stays in `docs/UNPORTED-UNIT.md`.
- `io.createFromUserInput`: CRS blocks only (`docs/PORTING-io-create-from-user-input.md`).
- `CApi.proj_coordoperation_create_inverse`: needs `Proj` to hold an ISO pipeline, and
  `proj_as_proj_string` options `MULTILINE`, `INDENTATION_WIDTH`, `MAX_LINE_LENGTH` (the pipeline
  renderer in `src/iso19111/pipeline.rs` has fixed 2-space indentation and an 80-column limit).

## Commands

```bash
cargo test --all-features --test unit_capi_grid_used_caching
cargo test --all-features --test unit_op_normalize_for_visualization
cargo test --all-features --test unit_capi_normalize_alternatives --test unit_capi_grid_used
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --all-features --test agree_cs2cs    # reference cs2cs, both pass
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --no-fail-fast
```

`docs_in_sync` (README, `UNIT-TRIAGE.md`) and `progress_svg` fail until the generated documents are
rebuilt with `UPDATE_DOCS=1`; that step is left to the maintainer for this lot.
