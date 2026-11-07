# Port — PROJ pipeline text of the deformation-model and TIN-shift operations

Status: **3 of 12 target cases closed** (`test_projinfo.yaml#59`, `#60`, `#61`). Nine remain open
(`#52`-`#57`, `#62`-`#64`). Their blockers are in the operation factory (candidate construction), not in
the pipeline text, and are described below with the evidence. No case was reopened.

The lot is five commits, one cause each, on `lot-projinfo-pipeline-text`:

| commit | cause | PROJ 9.9.0 | closes |
|---|---|---|---|
| `49453f8` | a concatenated operation's conversion steps get their CRS pair, also when the steps store their directions | `factory.cpp:7028` (`fixSteps`, called unconditionally), `concatenatedoperation.cpp:285`, `382-419` (first step) | `#59` |
| `d3fe78e` | the vertical TIN offset method (1137) writes its `tinshift` step | `singleoperation.cpp:5057-5083` | none alone |
| `9f0be21` | a projected interpolation CRS of a proj-based operation swaps its axes | `coordinateoperationfactory.cpp:2533-2536`, `2547-2550` | `#60`, `#61` |
| `9011235` | a reversed step keeps its direction when a concatenated operation is flattened | `concatenatedoperation.cpp:686-697` (`operations()` holds inverses), `:285` | none (fixes a wrong pipeline exposed by `49453f8`) |
| `9dab463` | a step followed by its own inverse cancels when operations are concatenated | `concatenatedoperation.cpp:700-732` | none (candidate name and pipeline of `EPSG:2393` to `EPSG:4258`) |

## Causes and ports

### 1. Conversion steps of a stored concatenated operation (`49453f8`)

`src/iso19111/factory.rs`, `concatenated()` and `assign_missing_conversion_crs`.

PROJ's loader calls `ConcatenatedOperation::fixSteps` on every concatenated operation read from
`proj.db` (`factory.cpp:7028`). The direction fixes inside it are gated on `fixDirectionAllowed`, the CRS
assignment is not. The port ran the assignment only when no step stored a direction, so `KKJ to ETRS89`
(`PROJ:KKJ_TO_ETRS89`, steps `EPSG:18193`, `EPSG:10703`, `EPSG:16065` reverse) had bare conversions with
no CRS, and their `tmerc` was refused.

Changes:
- The assignment runs also with explicit directions (`else` branch in `concatenated()`).
- The first step gets the concatenated source and the next step's source (`concatenatedoperation.cpp:412-418`).
- The assignment follows the direction the step runs: a reversed step stores its pair the other way round
  (`effective_endpoints`, `set_effective_endpoints`). Assigning the pair in stored order made
  `check_steps_chaining` fail on the first reversed step, which is how the orientation was found.

### 2. Vertical TIN offset (`d3fe78e`)

`src/iso19111/transform_export.rs`, `tin_shift`, method `METHOD_TIN_VERTICAL` (1137).

Port of `singleoperation.cpp:5057-5083`: both CRS must be `VerticalCRS` (checked before the file, as PROJ
does), the `file` parameter is written, no unit conversion. A method named `Inverse of ...` writes the step
inverted (`isMethodInverseOf`, `singleoperation.cpp:3586`).

### 3. Projected interpolation CRS of a proj-based operation (`9f0be21`)

`src/iso19111/transform_export.rs`, `interpolation_axes`.

PROJ's `MyPROJStringExportableHorizVerticalHorizPROJBased` writes `addAngularUnitConvertAndAxisSwap` for a
geographic interpolation CRS and `addUnitConvertAndAxisSwap` for a projected one
(`coordinateoperationfactory.cpp:2533-2536` and `2547-2550`). The port wrote nothing for a projected one.
The vertical TIN steps of the Finnish heights use the projected `KKJ / Finland Uniform Coordinate System`
as interpolation CRS. Each side's swap then cancels the projection's own swap in the pipeline, which is why
PROJ's text has none.

### 4. Reversed steps when flattening (`9011235`)

`src/iso19111/model.rs` (`ConcatenatedStep::oriented_operation`), `derived.rs` (`concatenate`),
`compound.rs` (`flatten_steps`, now calls `oriented_operation`).

A concatenated operation from `proj.db` stores a reverse step as its forward operation with
`reverse: true`. The two flatteners took the operation and dropped the flag, so the step ran forward. PROJ
keeps the inverse in the list (`fixSteps` sets `op = op->inverse()`, `concatenatedoperation.cpp:343, 358,
408-410`; `operations()` returns that list). The step's export then wrote `utm` where PROJ writes
`+inv utm`. This candidate (projected KKJ / Finland UCS to ETRS89) was not exportable before cause 1; once
it was, it became the first printed candidate. Cause 4 is needed for the output not to be wrong.

### 5. Consecutive inverse steps (`9dab463`)

`src/iso19111/derived.rs` (`remove_consecutive_inverse`), called from `concatenate` and from
`compound.rs::compute_metadata`.

Port of `ConcatenatedOperation::createComputeMetadata`, "Remove consecutive inverse operations"
(`concatenatedoperation.cpp:700-732`): a step followed by the inverse of itself, with the CRS meeting, is
removed; the loop repeats; it never goes below two steps, as PROJ bails out at two. The candidate of
`EPSG:2393` to `EPSG:4258` now starts with `YKJ to ETRS89-FIN / TM35FIN(E,N) (1) + Inverse of TM35FIN`, the
name PROJ gives it, and its pipeline is PROJ's.

## Deviations from PROJ

- `remove_consecutive_inverse` compares operations with the strict `is_equivalent_to` and CRS with
  `Equivalence::Equivalent`. PROJ uses `_isEquivalentTo(EQUIVALENT)` for both. No case distinguishes them.
- `assign_missing_conversion_crs` covers the bare-conversion branches of `fixSteps` (first, intermediate,
  last), not its derived-CRS branches (`concatenatedoperation.cpp:383-411`, `424-446`) nor the axis-order
  reversal steps (`:304-330`). The WKT import keeps its own `fix_steps` (`wkt_import.rs:3495`), unchanged.

## Oracle table

Each pair: `projinfo -q -o PROJ` (yaml flags: `--spatial-test intersects --hide-ballpark` for NZ,
`--grid-check none` otherwise), `PROJINFO_NO_GRID_CHECK=YES`. "base" is `bca19c5`, "ours" is this branch
(`9dab463`), "oracle" is PROJ 9.9.0. The table covers the 27 pairs of the comparison script, which is
described in the commands section.

| pair | base vs oracle | ours vs oracle | output changed | args |
|---|---|---|---|---|
| NZGD2000 to ITRF96 (`#52`) | differs | differs | no | `-s NZGD2000 -t ITRF96` |
| NZGD2000 to ITRF2014 (`#57`) | differs | differs | no | `-s NZGD2000 -t ITRF2014` |
| ITRF2014 to NZGD2000 (inverse of `#57`) | differs | differs | no | `-s ITRF2014 -t NZGD2000` |
| ITRF2000 to NZGD2000 (inverse of `#54`) | differs | differs | no | `-s ITRF2000 -t NZGD2000` |
| EPSG:4959 to EPSG:7907 (3D) | match | match | no | `-s EPSG:4959 -t EPSG:7907` |
| EPSG:4167 to EPSG:4959 (2D to 3D) | match | match | no | `-s EPSG:4167 -t EPSG:4959` |
| KKJ to ETRS89 (`#59`) | differs | match | yes | `-s KKJ -t ETRS89` |
| KKJ / FUCS to ETRS89 / TM35FIN (`#58`) | match | match | no | `-s "KKJ / Finland Uniform Coordinate System" -t "ETRS89 / TM35FIN(E,N)"` |
| inverse of `#58` | match | match | no | `-s "ETRS89 / TM35FIN(E,N)" -t "KKJ / Finland Uniform Coordinate System"` |
| KKJ + N43 height to KKJ + N60 height (`#60`) | differs | match | yes | `-s "KKJ + N43 height" -t "KKJ + N60 height"` |
| KKJ + N60 height to KKJ + N2000 height (`#61`) | differs | match | yes | `-s "KKJ + N60 height" -t "KKJ + N2000 height"` |
| inverse of `#60` | differs | match | yes | `-s "KKJ + N60 height" -t "KKJ + N43 height"` |
| KKJ + N43 height to ETRS89 + N2000 height (`#62`) | differs | differs | yes | `-s "KKJ + N43 height" -t "ETRS89 + N2000 height"` |
| KKJ/FUCS + N43 to ETRS89/TM35FIN + N2000 (`#63`) | differs | differs | no | `-s "KKJ / Finland Uniform Coordinate System + N43 height" -t "ETRS89 / TM35FIN(E,N) + N2000 height"` |
| inverse of `#63` (`#64`) | differs | differs | no | `-s "ETRS89 / TM35FIN(E,N) + N2000 height" -t "KKJ / Finland Uniform Coordinate System + N43 height"` |
| EPSG:2393 to EPSG:3067 | match | match | no | `-s EPSG:2393 -t EPSG:3067` |
| EPSG:3067 to EPSG:2393 | match | match | no | `-s EPSG:3067 -t EPSG:2393` |
| EPSG:4123 to EPSG:4258 | differs | match | yes | `-s EPSG:4123 -t EPSG:4258` |
| EPSG:4258 to EPSG:4123 | differs | match | yes | `-s EPSG:4258 -t EPSG:4123` |
| EPSG:2393 to EPSG:4258 | differs | match | yes | `-s EPSG:2393 -t EPSG:4258` |
| EPSG:4123 to EPSG:3067 | differs | match | yes | `-s EPSG:4123 -t EPSG:3067` |
| EPSG:8675 to EPSG:5717 (vertical) | differs | match | yes | `-s EPSG:8675 -t EPSG:5717` |
| "N43 height" to "N60 height" (vertical) | differs | match | yes | `-s "N43 height" -t "N60 height"` |
| EPSG:4326 to EPSG:3857 (control) | differs | differs | no | `-s EPSG:4326 -t EPSG:3857` |
| EPSG:4258 to EPSG:4326 (control) | match | match | no | `-s EPSG:4258 -t EPSG:4326` |
| EPSG:4267 to EPSG:4326 (control) | match | match | no | `-s EPSG:4267 -t EPSG:4326` |
| EPSG:27700 to EPSG:4326 (control) | match | match | no | `-s EPSG:27700 -t EPSG:4326` |

Controls: no pair that matched before has changed output or stopped matching. The control
`EPSG:4326` to `EPSG:3857` differs from the oracle both before and after this lot; it is not in this lot's
scope. `cs2cs` results for the Finnish pairs (`cs2cs -f %.6f`, sample points) are closer to the oracle
after the lot, for example `EPSG:4258` to `EPSG:2393` at (58.9 N, 25.9 E): base is 172 m off on the
easting (19 m on the northing), ours is 0.1 m off on both axes.

## Open items (blocked in the factory, not the pipeline text)

### New Zealand, `#52`-`#57`: 2D candidate built from 3D operations

PROJ's candidate for `NZGD2000` (EPSG:4167, 2D) to `ITRF96` is the 3D operation `EPSG:9090` with 2D CRS
(EPSG:4167 and a 2D `ITRF96`), named `INVERSE(DERIVED_FROM(EPSG)):9090`. Its `unitconvert` steps carry no
`z_in`/`z_out`. Ours returns the same operation with the 3D CRS (`EPSG:4959`, `EPSG:7907`), so the
conversion steps carry `z_in=m`/`z_out=m`. The export is faithful to the pair it gets
(`GeodeticCRS::addAngularUnitConvertAndAxisSwap`, `crs.cpp:2440-2470`, writes z only for 3D). The `DERIVED_FROM`
identifier comes from `addModifiedIdentifier` (`oputils.cpp:177-205`), so the 2D operation is a derived
copy of `EPSG:9090`. Which PROJ path makes that copy is not identified in the factory port.

### Finnish heights, `#62`-`#64`: the vertical chain N43 to N60 to N2000 is missing

Vertical only: `N43 height` to `N2000 height`. PROJ gives one candidate, `N43 height to N60 height (1) + N60
height to N2000 height (1)` (`EPSG:10704` then `EPSG:10705`, the latter in the registry with accuracy 0.005).
Ours gives no candidate (`Candidate operations found: 0`). The intermediate `N60 height` shares the datum of
neither end (datums `EPSG:1213` N43, `EPSG:5116` N60, `EPSG:1030` N2000, checked in `proj.db`), so
`compound.rs::vertical_pivot` (the port of `createOperationsVertToVertWithIntermediateVert`,
`coordinateoperationfactory.cpp:4953`, datum strategies) finds nothing. PROJ reaches the chain by a route
not yet identified. Once it exists, cases `#62`-`#64` need the same export as `#60`/`#61`, which is in place.

### Smaller candidate differences (not blocking any closed case)

- `KKJ to ETRS89` and `KKJ / FUCS` pairs: PROJ also lists `KKJ to ETRS89-FIN [EUREF-FIN] (2)` (0.5 m);
  ours does not list it. Candidate selection, not text.
- The `--summary` line of some candidates reads `unknown domain of validity` where PROJ names the area
  (for example the `N43 height to N60 height` candidate). Not in the PROJ text of the cases.

## Commands

```bash
# baseline and after, per commit
cargo test --test cli_yaml        # rewrites docs/UNPORTED-CLI.md, fails on a regression
cargo test --test gie_run         # gie ratchet: unchanged
UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg
cargo test --test docs_in_sync --test progress_svg
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<oracle bin>:$PATH PROJ_DATA=<oracle-db> PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast

# one case, both binaries (the yaml env: PROJINFO_NO_GRID_CHECK=YES, PROJ_NETWORK=OFF)
projinfo -s NZGD2000 -t ITRF96 -o PROJ -q --spatial-test intersects --hide-ballpark
<oracle>/bin/projinfo  (same args, PROJ_DATA=<oracle>/data:data/tests)
```

The baseline binary was built from `git archive bca19c5` into the scratch directory, so that the table
compares the same process. The oracle is the PROJ 9.9.0 build in the scratch `reftest` directory.

## Test totals

`cargo test --all-features --no-fail-fast` with the oracle on `PATH`, at the head of the branch after the
regenerated docs: **1604 passed, 0 failed, 34 ignored** (summed over 153 `test result` lines). `cli_yaml` and `gie_run` pass;
`docs_in_sync` and `progress_svg` pass after `UPDATE_DOCS=1`. `cargo fmt --check` and
`cargo clippy --all-targets --all-features -- -D warnings` are clean.
