# Porting sheet: `cs2cs --accuracy` and `--no-ballpark`

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, "Test effect of --accuracy"
(`--accuracy 0.05 EPSG:4326 EPSG:4258`) and "Test effect of --no-ballpark"
(`--no-ballpark EPSG:4267 EPSG:4258`), both expecting exit 3 and
`cannot initialize transformation`. The unit side is
`CApi.proj_create_crs_to_crs_from_pj_accuracy_filter` (`ACCURACY=0.05` gives NULL)
and `CApi.proj_create_crs_to_crs_from_pj_ballpark_filter` (`ALLOW_BALLPARK=NO` gives NULL).

## Behavior

- `ACCURACY` keeps a candidate only when its accuracy is known and at most the
  requested value. A candidate of unknown accuracy is dropped.
- `ALLOW_BALLPARK=NO` drops every candidate with a ballpark transformation step.
- When the filter drops every candidate, the pair fails at initialization with exit 3.
  No other path is tried: a filtered pair fails closed.

## What changed

- `src/iso19111/select.rs`: `CandidateFilter` (`accuracy`, `ballpark: Ballpark`), applied
  to the candidate list in `best_operation_filtered`, and the public
  `best_pipeline_filtered`. The enum `Ballpark` keeps a bool out of the public signature.
- `src/iso19111/compound.rs`: `get_accuracy` becomes `pub(crate)`, so the filter uses the
  same accuracy as PROJ's sort (0 for a conversion, -1 when unknown).
- `src/bin/cs2cs.rs`: the options reach `build_engine`. A non-default filter takes
  `select_pipeline_filtered`, which returns an error instead of falling back.

## Deliberate deviations and their ceilings

- The cause text is `(null)`. PROJ prints that for a NULL result with no error text, and
  the yaml pins it. A filter-empty result is the only error the filtered path produces, so
  the mapping is narrow. Upgrade path: carry PROJ's error kind through `FactoryError`.
- A filter on a PROJ-string pair (`+proj=...`) is not applied: `build_engine` sends
  projected inputs to the PROJ-string path. Ceiling: `--accuracy` on a PROJ string is
  silently ignored. Upgrade path: apply the filter in the PROJ-string branch too.

## Ledger

`test_cs2cs_various.yaml#97` and `#98` now pass and leave `docs/UNPORTED-CLI.md`.
`#123` (`--only-best --no-ballpark`, grid missing) now exits 3 as PROJ does, but its stderr
still differs: PROJ reports the missing grid at initialization with `File not found or
invalid`, and we report it per point. It stays open, with a new reason in the ledger.

## Commands run

- `devhub run --kind test -- cargo test --all-features --test cli_yaml` : passes.
- `UPDATE_DOCS=1` for `docs_in_sync` and `progress_svg` : regenerated the generated figures.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
- Full `cargo test --all-features --no-fail-fast`: see the PR description for the result.
