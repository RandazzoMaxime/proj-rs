# Port: the grid policy reaches the operation factory (`cs2cs`)

Source: PROJ 9.9.0, `src/crs_to_crs.cpp` (the tag `9.9.0`). The reference is `cs2cs` and `projinfo`
9.9.0 built from that tag (`target/proj-reference`), run on `PROJ_DATA=target/proj-reference/build/data`
(the database and its `proj.ini`, no grid file) with `PROJ_NETWORK=OFF`.

## What was wrong

`proj_create_crs_to_crs` builds its operation factory context with a grid availability that follows
`ONLY_BEST` (`crs_to_crs.cpp:566-573`):

```
grid availability = (errorIfBestTransformationNotAvailable ||
                     warnIfBestTransformationNotAvailable ||
                     proj_context_is_network_enabled(ctx))
    ? KNOWN_AVAILABLE
    : DISCARD_OPERATION_IF_MISSING_GRID
```

`select_alternatives` (the `cs2cs` selection) built the list with the default availability and
applied the policy afterwards, by a filter on the ranked list. That gave a different list in two
cases:

| policy | PROJ | the port before |
|---|---|---|
| DISCARD (`--only-best=no`, and the default of the reference `proj.ini`, `only_best_default = off`) | the factory itself drops the operations whose grids are missing and searches the pivots behind them: `EPSG:4267` to `EPSG:4269` lists 25 candidates, `NAD27 to WGS 84 (6) + Inverse of NAD83 to WGS 84 (1)` among them | a filter on the list with grids kept only the ballpark offset: `40 -100` did not move (`cs2cs` printed `40 -100`, PROJ `39.99999688 -100.00041559`) |
| KNOWN_AVAILABLE (`--only-best`, the warning default) | the factory drops the operations whose grid is unknown to the database: `Inverse of WGS 84 to EGM2008 height (2)` is gone, `(1)` is first | `(2)` stayed and was tried first, which is why a CRS compound on one side was kept out of this path |

## Function map

| PROJ 9.9.0 | This port |
|---|---|
| `proj_create_crs_to_crs_from_pj`, the grid availability of the context (`crs_to_crs.cpp:566-573`) | `src/iso19111/select.rs`, `select_alternatives_in_area`: `context.set_grid_availability_use(DiscardIfMissing)` for `BestPolicy::Discard`, `KnownAvailable` for `Warn` and `Error` |
| the same, for a CRS compound on one side (no exception in PROJ) | the guard `a compound CRS on one side only is not prepared here` is removed |
| a missing optional grid (`+geoidgrids=@name`) does not make an operation non-instantiable | `grid_serves`, used by the single-operation path; the preparer reads optional grids as served, as the facade does |

## Result

The differential run of 63 common pairs against the reference `cs2cs` (geographic, projected,
compound, geocentric; default policy, `--only-best=no`, and with the test fixtures on `PROJ_DATA`) has
no difference. Before, `EPSG:4267` to `EPSG:4269` and to `EPSG:26915` differed under the default policy.

`--only-best` on a compound CRS on one side (`EPSG:4258+5778` to `EPSG:4979`, `EPSG:4326+3855` to
`EPSG:4979`) chooses the candidate per point, as PROJ does: a point outside the Austrian area takes
the ballpark candidate, a point inside fails with the grid message.

## Tests

- `tests/cs2cs_grid_policy_candidates.rs`: the three cases above, with the values `cs2cs` 9.9.0 prints.
  They fail without the change.
- The yaml cases `test_cs2cs_various.yaml` (grid missing with `@gridname`, EGM2008 stderr text, the
  BETA2007 pair), through `tests/cli_yaml.rs`.

## Deviations and open points

- A missing optional grid (`@`) serves in the single-operation path of `cs2cs`; PROJ skips the step
  when it builds the pipeline. The port reads the name only.
- `proj_context_is_network_enabled` is read as `GridContext::load().enabled` (the `PROJ_NETWORK` setting): with
  the network on, DISCARD gets KNOWN_AVAILABLE too, as PROJ does. The ranking after the factory still
  filters a DISCARD list on the grids on disk (`discard_missing`), which PROJ does not do with the
  network on; the sandbox has no network, so that combination is not measured.
