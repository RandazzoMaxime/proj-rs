# Port — `cs2cs --authority` (`AUTHORITY=` of `proj_create_crs_to_crs`)

Source: PROJ 9.9.0 `src/apps/cs2cs.cpp` (option parsing and the `AUTHORITY=`
option), `src/crs_to_crs.cpp` (the option is read, and the operation factory
context is built with it), `src/iso19111/c_api.cpp`
(`proj_create_operation_factory_context`), `src/iso19111/factory.cpp`
(`AuthorityFactory::create`, `hasAuthorityRestriction`) and
`src/iso19111/operation/coordinateoperationfactory.cpp` (`getCandidateAuthorities`,
the per-authority search). The Rust side is `build_engine` in `src/bin/cs2cs.rs`,
`select_alternatives_with_authority` and `best_pipeline_with_authority` in
`src/iso19111/select.rs`, and `CoordinateOperationContext::create`,
`direct_registry_operations`, `operations_between` and `authority_restricts` in
`src/iso19111/factory.rs`.

Closes `test_cs2cs_various.yaml#96` (`--authority (GH-2442) - a no-op`), which
expected `49dN	2dE 0.000` and got `49d0'0.002"N	2dE 0.018`.

## Observed behaviour (cs2cs 9.9.0)

`--authority` is not a no-op. It limits the candidate operations to the named
authority. `PROJ` has no registered operation between ITRF96 and ITRF2014, so
the only candidate is the ballpark geographic offset (`+proj=noop`), and the
input is returned unchanged. Without the option, the EPSG:8076 operation
applies.

| Invocation (input) | Output |
|---|---|
| `ITRF96 ITRF2014` (`49 2`), no option | `49d0'0.002"N	2dE 0.018` |
| `--authority PROJ ITRF96 ITRF2014` (`49 2`) | `49dN	2dE 0.000` |

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `cs2cs.cpp:502-509`: `--authority {name}` | `Opts::authority`, parsed in `src/bin/cs2cs.rs` at the `--authority` arm (line 1163) |
| `cs2cs.cpp:965-972`: `AUTHORITY=` option of `proj_create_crs_to_crs_from_pj` | `build_engine(..., opts.authority.as_deref())`, `src/bin/cs2cs.rs:461` |
| `crs_to_crs.cpp:507-508`: `AUTHORITY=` read into `authority` | `authority.unwrap_or("")` in `build_engine` |
| `crs_to_crs.cpp:544` and `:691`: `proj_create_operation_factory_context(ctx, authority)` (first search and the grid rerun) | `select_alternatives_with_authority` (`src/iso19111/select.rs:1067`) and `best_operation_filtered` (`select.rs:340`) build `CoordinateOperationContext::create(&db, authority)`. The rerun clones that context, so it keeps the authority |
| `c_api.cpp:8049-8072`: `AuthorityFactory::create(db, authority)` then `CoordinateOperationContext::create` | `CoordinateOperationContext::create`, `src/iso19111/factory.rs:4930`, which canonicalises the name |
| `factory.cpp:4400-4417`: `EPSG`, `ESRI`, `PROJ` spelled as PROJ spells them, any case | `canonical_authority`, `src/iso19111/factory.rs:7520` |
| `factory.cpp:4138-4140`: `hasAuthorityRestriction()` = not empty and not `any` | `authority_restricts`, `src/iso19111/factory.rs:7514` (used at every SQL restriction) |
| `coordinateoperationfactory.cpp:1863-1883`: `getCandidateAuthorities`, `any` gives one unrestricted search (`""`) | `candidate_authorities`, `src/iso19111/factory.rs:7533`, unchanged |
| `coordinateoperationfactory.cpp:2209-2213`: the leg factory is `""` for `any` | `pivot_authorities` (`factory.rs:6600`): a named authority, `any` included, gives no leg list |

## Test mapping

| PROJ test | Rust test |
|---|---|
| `test/cli/test_cs2cs_various.yaml` `--authority PROJ ITRF96 ITRF2014` (#96) | `tests/cli_yaml`, `test_cs2cs_various.yaml#96` (now passes; the ledger row is removed) |
| The same pair, and the `EPSG`, `epsg`, `any`, `NONEXISTENT` variants, at `-f %.9f` | `tests/cs2cs_authority.rs`, the `cs2cs` binary (8 tests) |
| `best_pipeline_with_authority`, `epsg` and `EPSG` choose the same operation, `PROJ` another | `tests/cs2cs_authority.rs::library_selection_canonicalizes_the_authority_name` |

The expected values in `tests/cs2cs_authority.rs` are the output of the PROJ
9.9.0 `cs2cs` binary run with `-f %.9f`, not the output of this port.

## Oracle comparison

Oracle: PROJ 9.9.0 `cs2cs` (`scratchpad/reftest/build/bin/cs2cs`). Ours:
`target/debug/cs2cs`. Each invocation compares stdout, stderr and the exit
status; the banner's git hash is normalised, since the oracle prints it and
this port does not.

Three pairs, each with the eight authority variants (24 invocations):

| `--authority` | `ITRF96 ITRF2014` (`49 2`) | `EPSG:4230 EPSG:4258` (`50 4`) | `EPSG:31370 EPSG:4326` (`150000 170000`) |
|---|---|---|---|
| (none) | `49d0'0.002"N	2dE 0.018` | `49d59'56.897"N	3d59'55.436"E 0.000` | `50d50'25.481"N	4d22'7.508"E 0.000` |
| `EPSG` | as (none) | as (none) | as (none) |
| `epsg` | as (none) | as (none) | as (none) |
| `Epsg` | as (none) | as (none) | as (none) |
| `PROJ` | `49dN	2dE 0.000` | `50dN	4dE 0.000` | `50d50'27.442"N	4d22'2.951"E 0.000` |
| `proj` | as `PROJ` | as `PROJ` | as `PROJ` |
| `any` | as (none) | as (none) | as (none) |
| `NONEXISTENT` | `49dN	2dE 0.000` | `50dN	4dE 0.000` | `50d50'27.442"N	4d22'2.951"E 0.000` |

Full matrix: 17 pairs or pair forms, each with the eight variants, 136
invocations. Ours matches the oracle on 128, and all 128 are identical in
stdout, stderr and exit status. The other 8 are one pair form, `EPSG:4326` to
`+proj=merc +datum=WGS84`, which mismatches the same way with and without
`--authority` (see open items).

Before the lot, every variant gave the no-option output, because the option was
read and discarded. After the first wiring, `--authority any` on ITRF96 to
ITRF2014 gave a height of `0.018364699` m where PROJ gives `0.018364660` m; the
`any` fix in attempt 3 below removes that difference.

## Attempts

1. **Wire the option to the selection** (`select_alternatives_with_authority`,
   `best_pipeline_with_authority`, `build_engine`). Case #96 passes. The
   `PROJ` and `NONEXISTENT` variants match. Lowercase names do not: the
   registry search compares `auth_name` literally.
2. **Canonicalise the context name** (`canonical_authority`, factory.cpp:4400-4417).
   `epsg`, `Epsg` and `proj` now match. `cli_yaml` is still green.
3. **`any` is unrestricted in every SQL restriction** (`authority_restricts`,
   factory.cpp:4138-4140). Before this step `any` reached SQL as
   `auth_name = 'any'`, so the direct search found nothing and the answer came
   from the datum pivot with an unrestricted search. That gave a 4e-8 m height
   difference for `--authority any`. An attempt to apply the same predicate to
   `pivot_authorities` was wrong: PROJ's leg factory is `""` for `any`
   (coordinateoperationfactory.cpp:2209-2213), so `pivot_authorities` keeps the
   `!is_empty()` guard, and only the SQL predicates changed.

## Deliberate differences

- **The legacy fallbacks take no authority.** `build_engine` falls back to the
  side pair, the EPSG pipeline and the facade when the ISO selection fails.
  PROJ's `proj_create_crs_to_crs` has no such fallback. Under `--authority`, a
  fallback could answer where PROJ fails. None of the 136 invocations reaches
  one, so this is a known gap, not a verified divergence.
- **`AuthorityFactory::create` does not canonicalise.** Only the operation
  context does. `projinfo --authority epsg` and other callers of
  `AuthorityFactory::create` still see the name as typed. PROJ canonicalises
  both (factory.cpp:4400-4417).

## Open items (not closed by this lot)

- `projinfo --authority` is not passed to the operation context
  (`src/bin/projinfo.rs:1116` uses `""`). PROJ's `projinfo --authority PROJ` on
  ITRF96 to ITRF2014 lists one candidate (the ballpark offset); ours lists two.
- `cs2cs EPSG:4326 +proj=merc +datum=WGS84` (an EPSG source followed by `+`
  tokens): PROJ 9.9.0 reports `missing target CRS and source CRS is not a
  projected CRS` and exits 3, and this port computes the result. Same with and
  without `--authority`.
- The banner of error messages: the oracle prints `Rel. 9.9.0 (1df2be6), ...`,
  and this port prints `Rel. 9.9.0, ...`. Not related to `--authority`.

## Commands

```bash
# baseline and gate
cargo test --test cli_yaml                       # #96 passes, one ledger row removed
cargo test --test cs2cs_authority

# docs generated from the ledger
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg

# oracle, one invocation (PROJ 9.9.0 build tree)
PROJ_DATA=<oracle data>:data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  <oracle bin>/cs2cs --authority PROJ ITRF96 ITRF2014 -f %.9f <<< "49 2"
PROJ_DATA=data:data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  target/debug/cs2cs --authority PROJ ITRF96 ITRF2014 -f %.9f <<< "49 2"

# quality gates
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<oracle bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

The 136-invocation comparison is a loop over the pairs and the eight authority
variants with the oracle and this binary, comparing the two captured outputs.
Its script is not committed: it lives in the session scratchpad.
