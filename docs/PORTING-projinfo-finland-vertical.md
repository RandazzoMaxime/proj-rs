# Port — projinfo Finland vertical pivot (lot E)

Target: the three Finland compound cases of `tests/cli_yaml/test_projinfo.yaml`, numbered as the
runner numbers them: `#62` (KKJ + N43 height to ETRS89 + N2000 height), `#63` (its projected form,
KKJ / Finland Uniform Coordinate System + N43 height to ETRS89 / TM35FIN(E,N) + N2000 height) and
`#64` (the reverse of `#63`). All three use `--grid-check none -o PROJ -q`. The grids are absent
(`fi_nls_*.json`), so the PROJ strings are the expected text.

Source: PROJ 9.9.0. The factory is `src/iso19111/coordinateoperationfactory.cpp` of that tag, read from
the scratch checkout at `scratchpad/proj-9.9.0-src/`. The trace is from the traced PROJ build
(`PROJ_DEBUG=3`, `scratchpad/lot/projtrace-build/bin/projinfo`). The reference binaries are the
PROJ 9.9.0 `projinfo` and `cs2cs` of `scratchpad/reftest/build/bin`, run on the grid-less `proj.db`.

## Result

| case | pair | `-o PROJ` | summary | closed by |
|---|---|---|---|---|
| `#62` | KKJ + N43 height to ETRS89 + N2000 height | equal to PROJ | differs (see Open) | commit 2 |
| `#63` | KKJ / Finland Uniform CS + N43 height to ETRS89 / TM35FIN(E,N) + N2000 height | equal to PROJ | differs (see Open) | commit 2 |
| `#64` | the reverse of `#63` | equal to PROJ | differs (see Open) | commit 2 |

The ledger `docs/UNPORTED-CLI.md` lost the three rows (projinfo rows 12 to 9). No other row changed,
and no case reopened: `cargo test --test cli_yaml` passes after each commit.

Neighbours, all compared with the reference (`-o PROJ -q`, identical text):

| pair | result |
|---|---|
| KKJ + N43 height to KKJ + N60 height | equal |
| KKJ + N60 height to KKJ + N2000 height | equal |
| KKJ + N43 height to ETRS89 + N2000 height (the case `#62`) | equal |
| ETRS89 + N2000 height to KKJ + N43 height (reverse) | equal |
| KKJ / Finland Uniform CS + N43 height to KKJ / Finland Uniform CS + N60 height | equal |
| EPSG:8675 to EPSG:3900 (N43 to N2000 alone) | equal, both summary and `-o PROJ` |
| EPSG:8675 to EPSG:5717 (N43 to N60 alone) | equal |
| KKJ / Finland Uniform CS to ETRS89 / TM35FIN(E,N) | equal |
| KKJ to ETRS89 | equal (`-o PROJ`) |

`cs2cs` on sample points (grid-less, `EPSG:2393+8675`, `EPSG:3067+3900`, `EPSG:3067+5717`,
`EPSG:4123+8675`, `EPSG:4258+3900`, `EPSG:4258+5717`, both directions, three or two points each):
output identical to PROJ 9.9.0 on all eight pairs.

## Function map

| PROJ 9.9.0 | Rust (`src/iso19111/`) |
|---|---|
| `createOperationsFromDatabase`, the call of `findsOpsInRegistryWithIntermediate(..., false)` (coordinateoperationfactory.cpp:4449-4463) | `factory.rs`, `create_candidates`, the block after `vertical_pivot` (vertical pairs) and the existing geodetic block |
| `findsOpsInRegistryWithIntermediate` (:2073-2292) | `factory.rs`, `crs_codes_intermediate_operations` (already ported, generic over the CRS kind) |
| `createOperationsVertToVertWithIntermediateVert` (:4953-5068) | `compound.rs`, `vertical_pivot` (already ported) |
| `ConcatenatedOperation::create`, interpolation CRS (concatenatedoperation.cpp:175-197, 233-249, 256) | `buildop.rs`, `concatenation_interpolation`, called by `ConcatenatedOperation::create` |
| the same, for constructors of PROJ that go through `create` (imports, pivots) | `buildop.rs`, `concatenation_interpolation_of_steps`; `json_read.rs`, `wkt_import.rs`, `factory.rs` (registry concatenation) |
| `ConcatenatedOperation::createComputeMetadata` (concatenatedoperation.cpp:674-765) | `compound.rs`, `compute_metadata`; `factory.rs`, `pivot_concatenation` |
| `CoordinateOperation::interpolationCRS` (singleoperation.cpp:192) | `buildop.rs`, `interpolation_crs_of`; `model.rs`, `ConcatenatedOperation::interpolation_crs` |
| `createOperationsCompoundToCompound`, vertical transforms and interpolation loop (:7326-7396, :7660-7800) | `compound.rs`, `compound_to_compound` (the loop uses `interpolation_crs_of`) |
| `createHorizVerticalHorizPROJBased` (:2795-2880), `createPROJBased` with a null interpolation CRS | `compound.rs`, `proj_based_operation` (`interpolation: None`) |

## Commits

### 1. The registry intermediate search also runs for a vertical pair (`82a8e57`)

PROJ calls `findsOpsInRegistryWithIntermediate` in `createOperationsFromDatabase` for any pair
without a direct registry result (:4449-4463). The gate is not on geodetic CRS. The port ran the
search only for geodetic pairs, so `N43 height` to `N2000 height` lost the registered pivot
`N43 height to N60 height (1) + N60 height to N2000 height (1)` (EPSG:10704 with the N60 to N2000
operation). The trace of PROJ shows the call after `createOperationsVertToVertWithIntermediateVert`
(which finds nothing here), at the same level.

The block is placed after `vertical_pivot`, which PROJ runs first in `createOperationsFromDatabaseWithVertCRS`
(:5210-5216). It runs with no direct result before the filter, no explicit intermediate, and not inside
the datum-intermediate guard. The effect on the pair: the summary shows the pivot first, then the
ballpark change, as PROJ does. Nothing closes on its own: the pivot is one step of the compound cases,
which need the interpolation CRS of commit 2 to produce PROJ's pipeline.

### 2. A concatenated operation carries the interpolation CRS of its steps (`22c69a8`)

`ConcatenatedOperation::create` gives the concatenation the interpolation CRS that its steps share
(concatenatedoperation.cpp:175-197): the CRS is kept while each step has an equivalent one, and a
step without one resets it, so a later step's CRS is taken. Two vertical steps through a geographic
CRS take that geographic CRS (:233-249). The port left concatenated operations without one. Its
`compound_to_compound` then took the source geographic CRS as the interpolation CRS of the N43 to
N2000 pivot, where PROJ takes `KKJ / Finland Uniform Coordinate System` (EPSG:2393, the
`INTERPOLATIONCRS` of EPSG:10704 and of the N60 to N2000 operation).

With the interpolation CRS, `compound_to_compound` builds the horizontal parts in the projected
CRS (`KKJ + N43` to `KKJ / Finland Uniform CS`, then `KKJ / Finland Uniform CS` to
`ETRS89 + ...`), and the pipeline has the height TINs applied on the projected easting and northing,
between `tmerc` and the inverse `utm`, as PROJ's do. That is the whole change in the pipeline of the
three cases.

The field is set in `buildop.rs` (`ConcatenatedOperation::create`), in the constructors PROJ builds
with `createComputeMetadata` or `create` (factory pivots, `compute_metadata`, `combine`, `chain`,
bound-to-compound, the point-motion step, registry concatenations, WKT and PROJJSON imports, and
the normalised copy), and it stays `None` for the PROJ-based operations. The inverse keeps it (the
interpolation CRS is the same both ways).

Measured: `cargo test --test cli_yaml` closes `#62`, `#63` and `#64`, and the ledger loses those
three rows. Commit 1 alone changes no row.

## Deviations and open items

- **The summary of the three compound pairs is not PROJ's.** The `-o PROJ` text is equal, which is
  what the yaml checks. The summary differs in three places, all visible on
  `projinfo -s "KKJ + N43 height" -t "ETRS89 + N2000 height" --summary`:
  1. The candidate count after the filter is 8, PROJ's is 6. Two candidates through `KKJ to ETRS89 (1)`
     (EPSG:1638, 1.5 m) are kept here and not by PROJ: one after the N43 to N60 and N60 to N2000 pivot
     (`Inverse of Finland Uniform Coordinate System + KKJ to ETRS89 (1)`), one after the ballpark
     vertical change. The ballpark candidate with the EPSG:10703 horizontal part is named
     `KKJ to ETRS89 (using EPSG:10703)` here and `KKJ to ETRS89-FIN [EUREF-FIN] (3)` in PROJ. PROJ
     drops candidates in `filterAndSort`, with `filterOut` (coordinateoperationfactory.cpp:1295-1420)
     on the extents of the steps. Not traced further here. Next step: trace which candidates PROJ
     removes and the extent each one has.
  2. The inverse of the `TM35FIN` conversion is named `TM35FIN` here, `Inverse of TM35FIN` in PROJ,
     in the first candidate. On the projected pair alone the names agree, so the inversion of that
     conversion is named differently in the compound chain. Next step: compare the name of the
     inverse of a conversion in `createOperationsDerivedTo` and in the chain of
     `createHorizVerticalHorizPROJBased`.
  3. The accuracy of `KKJ to ETRS89 (using EPSG:10703)` prints `unknown accuracy` here and `0.03 m` in
     PROJ. This is also true of that operation alone (`projinfo -s KKJ -t ETRS89 --summary`), so it
     predates this lot.
- **Remarks of PROJ-based operations** (`getRemarks`, :2866) are not ported, as in lot C.
- **Accuracy of the PROJ-based horizontal-vertical-horizontal operation** uses the sum of its steps,
  as in lot C.
- The `ConcatenatedOperation` constructors listed under commit 2 that PROJ builds with `create` now
  compute the interpolation CRS, including the WKT and PROJJSON imports. The yaml and the full suite
  show no change from that, but it is a wider change than the pipeline needs; it follows PROJ.

## Commands

```bash
cargo test --test cli_yaml                      # per commit: ledger and regressions
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<PROJ 9.9.0 bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```
