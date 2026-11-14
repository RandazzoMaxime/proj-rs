# Port: IGNF datum pivot in one pass, and the area of interest of `cs2cs`

Source: PROJ 9.9.0. The line numbers below are of the tag `9.9.0`. The reference is `cs2cs` and
`projinfo` 9.9.0 built from that tag (`target/proj-reference/build-net`, with curl), run with
`PROJ_DATA=<repo>/data:<repo>/data/tests` (the environment of the yaml runner), `PROJ_NETWORK=ON`
and `OFF`.

## Result

Four cases left `docs/UNPORTED-CLI.md`, and the 14 `test_cs2cs_ignf.yaml` cases that failed whenever
the CDN was unreachable now pass without it.

| case | before | now |
|---|---|---|
| `test_cs2cs_ignf.yaml#9` (`IGNF:LAMBE` to `IGNF:LAMB93`) | `* * inf` | `1238837.253 5057451.037`, as PROJ |
| `test_cs2cs_ignf.yaml#17` (`IGNF:LAMBE` to `IGNF:GEOPORTALFXX`) | `* * inf` | `659421.855 3603178.891` |
| `test_cs2cs_datumfile.yaml#3` (`--bbox ... NAD27 NAD83`) | `* * inf` | `40d30'0.368"N 59d59'56.617"W` |
| `test_cs2cs_ignf.yaml#2-8, 10-16` with the CDN refused | `* * inf` | the numbers of the yaml |

The causes had been measured in a status sheet before this lot (the two mechanisms A and B). Both
come from the candidate list, not from the deferred grids: PROJ never lists the operation whose grid
is remote and absent, and the port listed it first.

## Function map

| PROJ 9.9.0 | This port |
|---|---|
| `createOperationsWithDatumPivot`, `nIters = (isIGNF(sourceCRS) && isIGNF(targetCRS)) ? 1 : 3` (`coordinateoperationfactory.cpp:3425-3440`): between two IGNF CRS there is one pass, `useOnlyDirectRegistryOp` is false (`iter == 0 && nIters == 3`), and the kind filter of iterations 1 and 2 does not apply | `src/iso19111/factory.rs`, `datum_pivot_operations`: `is_ignf`, `single_pass` skips the direct-only loop and runs loops A and B with iteration 0, without the kind filter |
| `isIGNF` (`coordinateoperationfactory.cpp:3436-3439`): the first identifier is in the `IGNF` code space | `is_ignf` |
| `proj_create_crs_to_crs_from_pj`, the area of interest (`crs_to_crs.cpp:557-566`): `proj_operation_factory_context_set_area_of_interest`, then `..._set_area_of_interest_name` when the area has a name | `src/iso19111/select.rs`, `select_alternatives_in_area` (the context carries the area of interest) |
| `proj_operation_factory_context_set_area_of_interest_name` (`c_api.cpp:8153-8185`): the extent of the one area of use of that name, with its description | `area_of_interest_from_name` |
| `FilterResults` with `foundExtentWithExpectedDescription` (`coordinateoperationfactory.cpp:1313-1346`): only the operations whose extent has the description of the area | already in the factory; it needed the description in the area of interest |
| `cs2cs.cpp:780-808`: `--area` gives the box and the name, `--bbox` the box only | `src/bin/cs2cs.rs`, `build_engine` |

## What changed in behavior

- `cs2cs --area` and `--bbox` no longer take the facade path (`Proj::new_known_crs`): they run the
  same selection as the no-area case, with the area of interest, as
  `proj_create_crs_to_crs_from_pj` does. Before this, the facade candidates were built with a
  context that had no area of interest (`facade_candidates` in `select.rs`), so the area filtered
  nothing.
- A pair of two IGNF CRS lists the IGNF operations only (`projinfo --spatial-test intersects` gives
  the same two candidates as `contains`); the EPSG operations `NTF (1)` and `NTF (2)` no longer
  appear for it.

## Tests

- `tests/ignf_single_pass.rs`: the candidate names of `IGNF:LAMBE` to `IGNF:LAMB93` and to
  `IGNF:GEOPORTALFXX`, from `projinfo --summary --spatial-test intersects` 9.9.0. They fail without
  the change (the EPSG `NTF (1)`/`NTF (2)` list) and pass with it.
- `tests/cs2cs_area_of_interest.rs`: a named area leaves one operation; a box keeps the ten
  operations that intersect it, and the point takes the Canadian grid (`40d30'0.368"N`).
- The yaml cases above, through `tests/cli_yaml.rs`.

## Deviations

None for the selection. Open, outside this lot: `PROJ_DEBUG` messages, and the pivot candidate set of
non-IGNF pairs (about 100 candidates against 11 for `EPSG:26915` to `EPSG:26715`, see
`docs/PORTING-pivot-candidate-set.md`).
