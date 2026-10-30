# Port: datum pivot and compound search (`createOperationsWithDatumPivot`, `createOperationsCompoundToGeog`)

Status: **not ported**. This PR is a draft. It records what was measured, the
one attempt that was reverted, and the cause of each target case that is still
open. It changes no code.

Source: PROJ 9.9.0 `src/iso19111/operation/coordinateoperationfactory.cpp`
(`createOperationsWithDatumPivot` 3167–3538, `createOperationsCompoundToGeog`
6454–7176, the ensemble branch `createOperationsEnsembleCRSToOtherGeodCRS`
4048–4246, `createGeodeticCRSFromDatum` in `factory.cpp` 9960–10032).

## Baseline

Measured on `origin/main` at `9a506a1` with the PROJ 9.9.0 binaries on `PATH`,
`cargo test --test cli_yaml`:

| | open before |
|---|---|
| `cs2cs` rows in `docs/UNPORTED-CLI.md` | 36 |
| `projinfo` rows | 46 |

The committed `docs/UNPORTED-CLI.md` was stale: it still listed
`test_cs2cs_various.yaml` 95, 96, 99 and 112–115, which pass on `main`. The
harness rewrites the file, so it is not part of this PR.

`cargo test --all-features --no-fail-fast` on the same tree: 22 test binaries,
all `ok`, exit 0.

## Attempt, reverted

A one-level datum pivot was written in `create_operations`, run only when the
registry search and the user pivot give nothing and the two datums differ:

- candidates are the geodetic CRS of each datum (`create_geodetic_crs_from_datum`
  on the datum identifiers), the pairs whose names match both CRS first, and
  that pass returns early when it gives something, as PROJ does;
- the source leg and the target leg are left out when their CRS share a datum;
  otherwise the first registered operation of the leg is used;
- the middle operations are the registry operations between the two candidates.

Result: no target case closed. `cs2cs` 95 and 96 (`ITRF96` to `ITRF2014`, which
pass on `main` with EPSG:8076) became open: the geocentric candidates give a
pivot that replaces the EPSG:8076 result. The change was removed, as the lot
rule requires. Nothing in this PR changes selection.

## Why the target cases stay open

Each row gives the measured current state (harness on this tree) and the
PROJ branch that produces the expected output.

| Cases | Current result | Cause |
|---|---|---|
| `projinfo` 29, 30 (`WGS 84` ↔ `WGS 84 + EGM96 height`) | 0 candidates | Expected: the registry op `WGS 84 to EGM96 height (1)` (EPSG:10084, 4979 → 5773), with the compound target. This is the vertical branch of `createOperationsCompoundToGeog`, not ported. |
| `projinfo` 32 (`EPSG:32631` → `EPSG:4326+3855`, 3D) | 0 candidates | Compound target: the horizontal leg (UTM inverse) and the vertical leg (EGM2008) from `createOperationsCompoundToGeog`. Not ported. |
| `projinfo` 108 (`EPSG:4979` → `EPSG:10645`) | 0 candidates | Compound target (Saba DPnet + Saba height). Same branch as 32. |
| `projinfo` 36 (`AGD66` → `WGS 84 (G1762)`) | 1 candidate (ballpark) | Expected `AGD66 to WGS 84 (18) + WGS 84 to WGS 84 (G1762)`. The second leg goes 4326 → 9057, and 4326 is not a candidate of the G1762 datum: `createGeodeticCRSFromDatum` matches the datum code only (1156). PROJ reaches 4326 through the ensemble of which 1156 is a member (6326), which is the target-ensemble branch, not ported. The GDA94 → GDA2020 entries are multi-hop, which needs a recursive `createOperations`. |
| `projinfo` 88, 89, 90 (ETRF, WGS 84 G1150 → G2296) | 1 candidate each (ballpark) | Expected multi-hop chains through ITRF/ETRF/WGS 84 realisations, with time-dependent steps. Needs a recursive `createOperations` and the point-motion branch. |
| `projinfo` 37 (`EPSG:31467` → `ETRS89`, bbox) | 0 candidates | Expected `Inverse of 3-degree Gauss-Kruger zone 3 + DHDN to ETRS89 (n)`: a projected source, so the pivot starts from the base CRS of the projection. Not ported (the pivot only runs for geodetic pairs). |
| `projinfo` 68 (`EPSG:23030` → `EPSG:25830`, bbox) | 0 candidates (`--hide-ballpark`) | Same projected-to-projected pivot through ED50. |
| `cs2cs` 116–121 (`EPSG:4326+3855` → `EPSG:4979`) | exit 3, `unknown CRS EPSG:4326+3855` | The compound code is not resolved by `create_from_user_input` (already noted for 139 and 143). Compound search comes after that. |
| `cs2cs` 126, 127 (`EPSG:4326+5773` → `EPSG:4326+5782`) | exit 3, `unknown CRS EPSG:4326+5773` | Same compound resolution. |
| `cs2cs` 124 (`NAD27` → `WGS 84`, DMA ConusW, grid missing) | `1402285.98` instead of `1402288.54` | Selection among direct operations, not a pivot: the expected `NAD27 to WGS 84 (6)` is not chosen. |
| `cs2cs` 125 (`EPSG:26915` → `EPSG:26715`, NAD83 → NAD27 through WGS 84) | exit 3, `EPSG datum 6269 to 6267` | Needs the pivot with the grid-missing fallback. The one-level pivot above did not close it. |
| `cs2cs` 128 (`--only-best`, grid missing, compound) | stderr mismatch | Compound resolution, then the fallback messages. |

## Next steps

1. Resolve compound codes (`EPSG:4326+3855`, `EPSG:4326+5773`) in the `cs2cs`
   and `projinfo` input path. This unblocks `cs2cs` 116–121, 126–128 and
   `projinfo` 32, 108.
2. Port `createOperationsCompoundToGeog` for a vertical component (29, 30, 32,
   108). It is the part that gives the EGM registry operations.
3. Port the target-ensemble branch next to `createOperationsEnsembleCRSToOtherGeodCRS`
   (36).
4. Only then add the datum pivot, with the recursion it needs (88, 89, 90, 125).
   The one-level pivot shows that a geocentric pivot must not replace a direct
   registry result: `cs2cs` 95 and 96 are the guard.

## Verification

- `cargo test --test cli_yaml` on `origin/main`: 36 `cs2cs` and 46 `projinfo`
  open rows, as listed above. Run on this tree, the code is identical to `main`.
- `cargo test --all-features --no-fail-fast`: 22 binaries, all `ok`.
- `cargo clippy` and `cargo fmt` were not run for this PR: it changes no Rust source.
