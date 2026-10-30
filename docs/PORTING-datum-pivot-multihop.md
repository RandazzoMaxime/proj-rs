# Datum pivot and compounds: multi-hop lot (status)

Scope of the lot: the open `cs2cs` cases 124–128 and the open `projinfo` cases
36, 37, 68, 88, 89, 90, 108, 32 (`docs/UNPORTED-CLI.md`), against PROJ 9.9.0
`coordinateoperationfactory.cpp`: `createOperationsWithDatumPivot`,
`createOperationsCompoundToCompound`, `derivedHSrc`.

This document records what was checked. It contains no code change: no
change that was tried reached a verified, non-regressing state, so none is
merged. Baseline: `origin/main` at 6e128e3 (#44).

## Baseline

`cargo test --test cli_yaml` with the reference PATH and `PROJ_CS2CS_REQUIRED=1`
on the lot's worktree:

| app | open before |
|---|---|
| cs2cs | 23 |
| projinfo | 42 |

The regenerated `docs/UNPORTED-CLI.md` is identical to the committed one.

## What the reference does, and what the port does

Each case was run with the reference binaries and the port's `cs2cs` and
`projinfo` built from this branch, with the same `proj.db` and no grids.

### cs2cs 124 (NAD27 to UTM zone 10 via WGS 84, grid missing)

- Reference: the first candidate, `Inverse of NAD27 to WGS 84 (33)`, needs
  a grid that is missing. Its retry, `Inverse of NAD27 to WGS 84 (6)`, gives
  `1402288.54 5076296.64`.
- Port: `1402285.98 5076292.42`, about 2.5 m off.
- Cause not isolated: the port's retry after a missing grid selects another
  operation. Not a datum pivot in the strict sense; the two CRS are geodetic
  and the direct registry operations already exist.

### cs2cs 125 (EPSG:26915 to EPSG:26715, grid missing)

- Reference: `569720.46 4268813.88`, through a WGS 84 pivot.
- Port: fails with `EPSG datum 6269 to 6267 needs a grid, a deformation model,
  or a method this crate does not build`.
- Projinfo shows the direction of the ballpark differs. Reference candidate:
  `Inverse of UTM zone 15N + Ballpark geographic offset from NAD83 to NAD27 +
  UTM zone 15N`. Port candidate: `Inverse of UTM zone 15N + Inverse of
  Ballpark geographic offset from NAD27 to NAD83 + UTM zone 15N`.
- The pivot in the reference is not reached through `datum_pivot_operations`
  (`src/iso19111/factory.rs`): that pass takes candidate geodetic CRS of the
  datums, not a WGS 84 intermediate.

### cs2cs 126, 127, 128 (EPSG:4326+5773 to EPSG:4326+5782)

- Reference: one candidate, `Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP
  [REGENTE] to Alicante height (1) using ETRS89-ESP [REGENTE] to WGS 84 (1)`.
- Port: `projinfo -s EPSG:4326+5773 -t EPSG:4326+5782` gives 0 candidates;
  cs2cs then fails with `unknown CRS EPSG:4326+5773`, from the facade in
  `src/crs.rs`.
- Cause: `createOperationsCompoundToCompound` is not ported. `compound.rs` has
  only `compound_to_geog3d` and `geog_to_compound`. The missing part is the
  vertical branch (`verticalTransforms`, `bTryThroughIntermediateGeogCRS`,
  `createWithIntermediateCRS`), the 3-step selection of lines 7452–7563 that
  uses `bestAccuracy`/`bestStepCount`, and the final `createHorizVerticalHorizPROJBased`
  step. The C++ function is about 640 lines.

### projinfo 88, 89, 90 (multi-hop between realizations)

- Reference, 88 (`ETRF2005` to `ETRF2014`): 2 candidates, both through
  `ITRF2005` or `ETRF2000` as a registry intermediate, time-dependent.
- Reference, 89 (`ETRS89-POL [PL-ETRF2000]` to `ETRF2014`): 2 candidates
  through `ITRF2020` or `ETRF2000`.
- Reference, 90 (`WGS 84 (G1150)` to `WGS 84 (G2296)`): 1 candidate, 3 registry
  steps through G1762, G2139.
- Port, all three: 1 candidate, `Ballpark geographic offset from ...`.
- Cause: `findsOpsInRegistryWithIntermediate` (C++ line 2073) and its database
  step `createBetweenGeodeticCRSWithDatumBasedIntermediates` are not ported.
  The port's only pivot is the user list in `CoordinateOperationContext`
  (`set_intermediate_crs`), which is empty by default. `datum_pivot_operations`
  (`src/iso19111/factory.rs`) is a one-level pivot on candidate geodetic CRS of
  each datum; it does not recurse and does not read the registry intermediates.

### projinfo 36, 37, 68, 108, 32

- 68 (`EPSG:23030` to `EPSG:25830`, `--bbox -6,40,-5,41`): reference 2 candidates,
  port 3.
- 108 (`EPSG:4979` to `EPSG:10645`): reference 1 candidate (`Inverse of Saba to
  WGS 84 (1) + Saba to Saba height (1) + Saba Transverse Mercator 2020`), port 0.
  This is the `derivedHSrc` family; not analysed further.
- 37 (`EPSG:31467` to `ETRS89`): same candidate count (4), content not compared.
- 36 (`AGD66` to `WGS 84 (G1762)`): the shell quoting used for this run was wrong,
  so no count was read. Not analysed.
- 32 (`EPSG:32631` to `EPSG:4326+3855`, `--3d`): same count (3), content not compared.

## Why no change was merged

The reference paths for these cases recurse through `createOperations`
(`createOperationsWithDatumPivot` calls it for the first and third legs and for
the middle leg, with an anti-recursion flag), and through the registry
intermediate search. Main already has the one-level pivot of `datum_pivot_operations`. A
previous attempt at a one-level pivot regressed ITRF96 to ITRF2014 (cs2cs 95/96,
which pass on main through `EPSG:8076`) and was removed. A change of that shape needs the full
recursion and the registry intermediate step first.

## Next steps, in order

1. `findsOpsInRegistryWithIntermediate` with `createBetweenGeodeticCRSWithDatumBasedIntermediates`
   (cases 88, 89, 90). Verify on cs2cs 95/96 before anything else.
2. `createOperationsCompoundToCompound` vertical branch (cs2cs 126–128, and
   139, 143 when they are compound-to-compound).
3. Direction of the ballpark in the datum pivot (cs2cs 125, projinfo 37).
4. `derivedHSrc` (projinfo 108, 32).
5. The retry after a missing grid (cs2cs 124).

## Verification

- `cargo test --test cli_yaml` with the reference PATH: cs2cs 23 open, projinfo 42
  open, identical to the committed `docs/UNPORTED-CLI.md`.
- Commands and results of the full suite: see the pull request description.
