# Porting notes: cs2cs epochs and the datum pivot

Reference: PROJ 9.9.0. Sources: `src/apps/cs2cs.cpp` (`--s_epoch`, `--t_epoch`, `proj_trans`
with `coord.xyzt`), `src/iso19111/operation/coordinateoperationfactory.cpp`
(`createOperationsWithDatumPivot`, `findOpsInRegistryDirect`, `getCandidateAuthorities`),
`src/iso19111/factory.cpp` (`getAllowedAuthorities`).

Cases: `test/cli/test_cs2cs_various.yaml` ids 140 (`--s_epoch 2022 ITRF2014 GDA2020`) and 141
(`--t_epoch 2022 GDA2020 ITRF2014 -E`). Both now match.

## What changed

| Where | PROJ | Rust |
|---|---|---|
| `src/bin/cs2cs.rs` `emit_line` | `coord.xyzt.t` is the input time, `HUGE_VAL` when absent | `Parsed.t` is the input time, `+inf` when absent |
| `src/bin/cs2cs.rs` `Job.epoch` | `--s_epoch` sets the source CoordinateMetadata; the pipeline starts with `set v_4=<epoch>` and overrides the input time (checked: input `2000` with `--s_epoch 2022` gives the `2022` result) | `Job.epoch = s_epoch.or(t_epoch)` replaces the input time |
| `src/bin/cs2cs.rs` dynamic warning | `srcIsDynamic` only when no `--s_epoch`, `destIsDynamic` only when no `--t_epoch` | same condition |
| `src/bin/cs2cs.rs` `Engine::apply` | `proj_trans(PJ_FWD, xyzt)` | takes `t`; `Pipe` and `Pair` pass it to the operation, `Facade` does not (see gaps) |
| `src/iso19111/factory.rs` `direct_registry_operations` | `findOpsInRegistryDirect` loops over `getCandidateAuthorities`, stops at the first non-`PROJ` authority with operations | new, used by the direct search and by the pivot; the intermediate-CRS pivot block is unchanged |
| `src/iso19111/factory.rs` `candidate_authorities` | `getAllowedAuthorities` on `authority_to_authority_preference` (`EPSG,EPSG` gives `PROJ,EPSG,NKG`, so `ESRI` is not read) | new; a context authority still wins |
| `src/iso19111/factory.rs` `datum_pivot_operations` | `createOperationsWithDatumPivot`, first pass only | new; runs when the direct search is empty, both CRS are geodetic and their datums differ, before the ballpark |
| `src/iso19111/factory.rs` `pivot_leg`, `geographic_geocentric` | synthesized `Conversion` (EPSG:9602) | a `Transformation` with method 9602 that keeps both CRS, so the PROJ-string export reads the ellipsoid. A `Conversion` step has no CRS and cannot be exported here |

## Epoch semantics observed in the reference

- `--s_epoch E` with a dynamic source: the pipeline begins with `set v_4=E`, so the input time is ignored.
- `--t_epoch E` alone with a dynamic target (`GDA2020 -> ITRF2014`): the pipeline also begins with `set v_4=E`
  (the inverse helmert reads `E`).
- Input time and no epoch: the input time is used (`-30 150 0 2022` gives the `2022` result).
- No time and no epoch: `HUGE_VAL`, so the helmert uses its epoch and the shift is zero, with the warning on stderr.
- An epoch on a static CRS is an error in PROJ (`Coordinate epoch should not be provided for a static CRS`). Not ported.

## Function mapping

| PROJ | Rust |
|---|---|
| `createOperationsWithDatumPivot` (registered pass) | `datum_pivot_operations` |
| `findCandidateGeodCRSForDatum` | `candidate_geodetic_crs` (database order, not the preferred authority first) |
| `isSameGeodeticDatum` | `same_geodetic_datum` (existing) |
| `findOpsInRegistryDirect` | `direct_registry_operations` |
| `getCandidateAuthorities`, `getAllowedAuthorities` | `candidate_authorities` |
| `Conversion::createGeographicGeocentric` | `geographic_geocentric` (method EPSG:9602 as a transformation) |
| `ConcatenatedOperation::createComputeMetadata` (accuracy = sum) | `pivot_concatenation` (accuracy = sum, unknown if one step is unknown) |

## Deliberate differences and gaps

- **Passes 2 and 3 of the datum pivot are not ported.** PROJ retries through a third CRS
  (`createOperations(candSrc, candDst)`) when the first pass finds nothing. Pairs that need that
  still return a ballpark or nothing.
- **Epoch-change operations are not ported.** The `PointMotionOperation` branch
  (`getPointMotionOperationsFor`) of the pivot and of `createOperationsGeodToGeod` is absent. The
  two cases here do not need it: the reference pipelines have no velocity step.
- **`Facade` ignores `t`.** `Proj::convert_xyz` has no time argument. It is used for names that
  `build_engine` cannot resolve to an EPSG pipeline.
- **Candidate order:** PROJ puts candidates with the CRS's own authority first (`preferredAuthName`).
  The Rust query orders by `auth_name, code`.
- **Concatenated operations have no domain of validity** (`unknown domain of validity` in projinfo),
  where PROJ intersects the step extents.
- **Names:** PROJ writes `Conversion from ITRF2014 (geog2D) to ITRF2014 (geocentric)`, Rust writes
  `Conversion from ITRF2014 to ITRF2014`. Only the text differs; numbers match.
- **`--no-ballpark` is not honoured by cs2cs** (pre-existing, unchanged by this lot). Cases 98 and 97
  still produce output where PROJ fails at initialisation.

## Quirks kept

- The authority order keeps `PROJ` operations while reading the next authority, as PROJ does.
- `t` replaces the input time whenever an epoch is given, even when the line has a time.

## Verification

Reference run on origin/main 9a506a1 (clean worktree) and on this branch, with
`PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml`:

- `docs/UNPORTED-CLI.md` cs2cs rows: 36 on origin/main, 34 here. The removed ids are 140 and 141.
- Same projinfo rows (46 on both), same ids, same reasons. No cs2cs row changed reason.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --all-features --no-fail-fast`: every binary and test target passes.
- New test `tests/datum_pivot.rs`: `EPSG:9000 -> EPSG:7844` has no `ESRI` null operation, has
  `ITRF2014 to GDA2020 (1)`, and its pipeline contains `+t_epoch=2020`.
