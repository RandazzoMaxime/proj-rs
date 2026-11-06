# Area of use of a concatenated operation (PROJ 9.9.0)

The domain (area of use) of the concatenated operations that the factory builds from registry
transformations and geographic/geocentric conversions. Before this lot the port gave them no domain
when they came from a pivot, so their box was the world and the per-point choice of `cs2cs` kept
them for points outside the area of the transformation. Example: `cs2cs EPSG:4746 EPSG:4978`,
`54 9 0` (Germany, outside Thuringia). PROJ gives `3710957.52 587757.93 5136743.83` (ballpark
candidate); the port gave `3711110.40 587711.72 5136639.38` (PD/83 candidate, world box).

## Code

- `src/iso19111/factory.rs` `pivot_concatenation`: the domain is the intersection of the steps'
  extents, `derived::joint_extent` (the same function `derived.rs` `concatenate` uses).
- `src/iso19111/factory.rs` `geographic_geocentric` (EPSG:9602 as a transformation that keeps both
  CRS): its domain is the world, not the extent of the geographic CRS.
- Unchanged, already ported: `derived.rs` `joint_extent`, `factory.rs` `operation_extent`
  (a conversion is the world), `derived.rs` `intersect_extents` (`Extent::intersection`),
  `compound.rs` `compute_metadata`.

## PROJ file:line map

| PROJ 9.9.0 | This port |
|---|---|
| `src/iso19111/operation/oputils.cpp:464-482` `getExtent(op)`: a `Conversion` is `Extent::WORLD` (468-471), else `domains()[0]` (473-477), a concatenated operation the intersection of its steps (481-482) | `operation_extent` (factory.rs) |
| `oputils.cpp:529-553` `getExtent(ops)`: intersection in order; a step without extent is skipped; an empty intersection gives no extent | `joint_extent` (derived.rs) |
| `src/iso19111/metadata.cpp:889-909` `Extent::intersection`: the contained extent (with its description), else the common box without description | `intersect_extents` (derived.rs) |
| `src/iso19111/operation/concatenatedoperation.cpp:674-756` `createComputeMetadata`: flatten (685-700), extent (741-751), `checkExtent` throw (743-745) | `pivot_concatenation` (no check, see deviation 1); `compound.rs` `compute_metadata` and `derived.rs` `concatenate` (with the check) |
| `coordinateoperationfactory.cpp:5351-5362` geographic to geocentric on different datums: `Conversion::createGeographicGeocentric(sourceCRS, interm_crs)` (5353), then `createComputeMetadata({opFirst, opSecond}, disallowEmptyIntersection())` (5358-5362) | `geodetic_ops.rs` `geographic_geocentric_different_datum` (`pivot_concatenation`) |
| `src/iso19111/operation/conversion.cpp:2599-2620` `Conversion::createGeographicGeocentric`: no `DOMAIN_OF_VALIDITY` (its extent is the world by `getExtent`) | `factory.rs` `geographic_geocentric` (world); `geodetic_ops.rs` `geographic_geocentric_conversion` (no domain) |
| `coordinateoperationfactory.cpp:2315-2326` ballpark geographic offset: the extent of the source when both CRS have the same extent, else `WORLD` | `factory.rs` `forced_ballpark_operations`, `geodetic_ops.rs` `geographic_offset` (already matched) |
| `coordinateoperationfactory.cpp:171-172`, `587-593`: default `SMALLEST`, so `disallowEmptyIntersection()` is true | not ported (deviation 1) |

## Behaviour

- A concatenated candidate built by the factory has as domain the intersection of its steps'
  extents. A conversion step is the world. A step with no extent does not narrow the intersection.
- An intersection with no bounding box, or one that is empty, gives no domain
  (`unknown domain of validity`), as PROJ prints.
- The per-point choice (`alternatives.rs`) reads the boxes of the candidates (`select.rs`). A
  candidate whose box does not contain the point is skipped, so the next candidate serves it.

## Deviations from PROJ

1. Empty intersection. PROJ's `createComputeMetadata` throws `InvalidOperationEmptyIntersection`
   when `checkExtent` is set, and the caller drops the candidate. Under the default context the
   check is on (`SMALLEST`, `coordinateoperationfactory.cpp:172, 589-592`). `pivot_concatenation`
   keeps such a candidate, without domain. `compound.rs` and `derived.rs` do drop it. No pair of
   the three sweeps below produced an empty pivot (probe over 142 pairs, see Commands). Not ported.
2. The conversions of the factory are `Transformation` values with EPSG:9602, because the PROJ
   string export reads the ellipsoid of each side from them. Their domain is the world, which is
   what `getExtent` gives a PROJ `Conversion`.
3. Accuracy of a concatenated operation: not part of this lot. `pivot_concatenation` keeps the
   largest step accuracy. PROJ's `getAccuracy(ops)` (`oputils.cpp:587-636`) adds them, with the ETRS89
   ensemble special case (0.1 m legs). Effect: `PD/83 to WGS 84 (1) + Inverse of ED50 to WGS 84 (2)`
   is 6 m here and 7 m in PROJ. It changes the sort, and so the per-point choice of some pairs
   (see the `EPSG:4312 EPSG:4230` line below).

## Oracle comparison (`projinfo --summary`, default spatial test)

Candidates are matched by name. `DOMAIN` counts a name whose domain text differs; `ACC` a name whose
accuracy differs; `ONLY` a name present on one side only. The grid-missing tag is not compared: it
depends on the data directory (the oracle runs on `refdata`, without grids).

| sweep | pairs | DOMAIN before | DOMAIN after | ACC | ONLY |
|---|---|---|---|---|---|
| required pairs and reverse (`pairs1`) | 34 | 168 | 0 | 3 | 177 |
| common CRS pairs (`pairs2`) | 77 | 42 | 0 | 17 | 17 |
| far-apart pairs (`pairs3`) | 31 | 35 | 0 | 26 | 3 |

The required pairs:

| pair | candidates | domain before (port) | domain after (port) | PROJ 9.9.0 |
|---|---|---|---|---|
| `EPSG:4746 EPSG:4978` | 2 | unknown, unknown | `Germany - Thuringen.`, `World` | `Germany - Thuringen.`, `World` |
| `EPSG:4978 EPSG:4746` | 2 | unknown, unknown | `Germany - Thuringen.`, `World` | same |
| `EPSG:4326 EPSG:2154` | 1 | France | France | same |
| `EPSG:4267 EPSG:26915` | 5 | matched | matched | same |
| `EPSG:23031 EPSG:4326` | 16 | matched (1 accuracy) | matched (1 accuracy) | same |
| `EPSG:31467 EPSG:4258` | 7 | matched | matched | same |

Remaining differences, none of them a domain: candidate sets (`ONLY`: pivots through NAD83 for
`EPSG:4267 EPSG:4978`, the intermediate-CRS pivots of `EPSG:4230 EPSG:4978`, the axis-order variant
of `EPSG:4258 EPSG:4978`), and accuracies (deviation 3).

## Per-point comparison (`cs2cs`, 20 points, same data directory)

`pointcmp` runs the base and the fixed build on the same points. "vs oracle" counts the points
where the output differs from PROJ 9.9.0 (the oracle runs on `refdata`, without grids; the fixed
build also run on `refdata` gives the same mismatches, see the last line).

| pair | base vs oracle | fixed vs oracle |
|---|---|---|
| `EPSG:4746 EPSG:4978` | 4 | 0 |
| `EPSG:4978 EPSG:4746` | 0 | 0 |
| `EPSG:4230 EPSG:4978` | 20 | 0 |
| `EPSG:4978 EPSG:4230` | 20 (`90dN` on every point) | 0 |
| `EPSG:4267 EPSG:4978` | 19 | 0 |
| `EPSG:4978 EPSG:4267` | 20 (`90dS` on every point) | 0 |
| `EPSG:4269 EPSG:4978`, `EPSG:4230 EPSG:4326`, `EPSG:4267 EPSG:4326`, `EPSG:4272 EPSG:4326`, `EPSG:4326 EPSG:4978` | 0 | 0 |
| `EPSG:4312 EPSG:4230` | 20 | 16 (about 0.001 arc-second) |
| `EPSG:4230 EPSG:4936`, `EPSG:4936 EPSG:4230`, `EPSG:4312 EPSG:4936`, `EPSG:4277 EPSG:4978` | 20 | 0 |

On the same `refdata` directory, the fixed build differs from the oracle on 0 of 20 points for
`EPSG:4746 EPSG:4978`, `EPSG:4230 EPSG:4978`, `EPSG:4267 EPSG:4978`, `EPSG:4230 EPSG:4936`
and `EPSG:4312 EPSG:4936`. For `EPSG:4312 EPSG:4230` it differs on 16 of 20 points, by about
0.001 arc-second. PROJ's fallback there is `MGI to WGS 84 (3) + Inverse of ED50 to WGS 84 (2)`;
which candidate the port picks for each point was not traced. The cause is outside this lot
(accuracy, deviation 3, or the numeric path of that candidate).

## Tests

- `tests/concat_area_of_use.rs` (new, 3 tests), values from PROJ 9.9.0 (`projinfo` and `cs2cs`):
  - `geographic_to_geocentric_takes_the_area_of_its_transformation`
  - `geocentric_to_geographic_takes_the_area_of_its_transformation`
  - `point_outside_thuringia_takes_the_ballpark_candidate` (`54 9 0`)
  All three fail on the base commit (`d6fc3d2`, area `None`, and the point gives
  `3711110.40 587711.72 5136639.38`) and pass with the change.

## CLI ratchet (`tests/cli_yaml`)

- Previously passing cases failing: none in the final runs. One run of the final code failed
  `test_cct.yaml#11` with "Broken pipe" (a stdin write racing the child process in the harness, not
  a result difference); the immediate rerun passed. `docs/UNPORTED-CLI.md` is byte-identical to the
  base, so no row is closed or reopened (17 open cs2cs rows, as before).
- Cases whose output changed, still open: `test_projinfo.yaml#36`, `#88`, `#89`. The domain text of
  the candidates is now the one PROJ prints (`Europe - onshore and offshore - ETRF extent -
  approxim...`, `Poland ...` for #88 and #89). The rows stay open for their accuracy and candidate
  sets.

## Commands

```bash
# Reference PROJ 9.9.0 (oracle) on the data without grids, and this port on the fixtures:
PROJ_DATA=<scratch>/lot/refdata:<wt>/data/tests <scratch>/lot/projbuild/bin/projinfo -s EPSG:4746 -t EPSG:4978 --spatial-test intersects --summary
PROJ_DATA=<wt>/data:<wt>/data/tests target/debug/projinfo -s EPSG:4746 -t EPSG:4978 --spatial-test intersects --summary
echo "54 9 0" | PROJ_DATA=<wt>/data:<wt>/data/tests target/debug/cs2cs EPSG:4746 EPSG:4978   # 3710957.52 587757.93 5136743.83
cargo test --test concat_area_of_use
PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml
UPDATE_DOCS=1 cargo test --test docs_in_sync
cargo test --test docs_in_sync
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

`<wt>` is the worktree, `<scratch>/lot` the scratch directory of the lot. The sweeps were run with a
small script that parses `projinfo --summary` on both sides and matches the candidates by name; the
probe for empty pivots was a temporary `eprintln!` in `pivot_concatenation`, removed before the
commit.
