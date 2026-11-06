# Porting sheet: heights through PROJ-string pairs (`+proj=sch`, `+proj=cart`)

PROJ 9.9.0 reference: `test/cli/test_cs2cs_various.yaml`, "Test SCH forward projection" (#71) and
"Test SCH inverse projection" (#72). Oracle: the PROJ 9.9.0 `cs2cs` built from the `9.9.0` tag,
run with `PROJ_DATA` pointing at its `data/` and `-f %.7f`.

## Behavior (PROJ)

- A projection with `fwd3d` / `inv3d` computes the third axis: `sch.cpp:184-185` sets
  `P->fwd3d = sch_forward3d` and `P->inv3d = sch_inverse3d`, and the height is the sch height
  (`sch_forward3d` at `sch.cpp:85`; `sch_inverse3d` at `sch.cpp:59`). `+proj=cart` / `geocent` do the same (geodetic height in,
  geocentric Z out and back).
- A 2D projection only sets `P->fwd` / `P->inv`. `src/fwd.cpp:217-220` and `src/inv.cpp:170-172`
  dispatch to the lowest-dimensional transformer; `coo.v[2]` is left as it came in, so a 2D step
  passes the height through.
- A `+towgs84` (or `+datum=` shift) on a PROJ-string side moves the horizontal position only.
  The height of the output is the height of the input. Observed on the oracle for latlong to
  latlong and for a projected target (table below); this is the rule of
  `docs/PORTING-cs2cs-init-towgs84.md`, now stated per side.
- `cs2cs` does not treat 2D specially: it calls `proj_trans` with `xyzt.z = z` and prints
  `coord.xyz.z` (`src/apps/cs2cs.cpp:197-205`). The height of the output is the third axis the
  pair produces.

## What changed

- `src/engine.rs`, `Step::computes_height` (default `false`) and `Operation::computes_height`:
  the public query for "this step sets the third axis", the analogue of `P->fwd3d` being set.
- `src/projections/sch.rs`, `Sch::computes_height` = `true` (`sch.cpp:184-185`).
- `src/engine.rs`, `CartStep::computes_height` = `true`: `cart` and `geocent` are 3D
  (`CartStep::forward` and `inverse` carry `z`).
- `src/projections/legacy.rs`, `Legacy::computes_height`: the inner step, or a geoid grid.
- `src/bin/cs2cs.rs`, `SidePair`: the old rule was "a target without a vertical key returns the
  input height". That was wrong for `sch` and `cart` (the height came from the sch/cart step and
  was overwritten) and wrong on the source side (a `cart` source kept the raw Z). The new rule:
  - `src_has_height` / `dst_has_height` = a vertical key, or `Operation::computes_height()`.
  - Source inverse: a side without a height keeps the input height. Its datum shift does not
    move the height.
  - Target forward: a side without a height keeps the height it received. A side with one
    (sch, cart) keeps the value it computed.

## Ledger

- `test_cs2cs_various.yaml#71` (SCH forward) and `#72` (SCH inverse) pass and leave
  `docs/UNPORTED-CLI.md`.
- `tests/cs2cs_sch_heights.rs` (new): six tests against the oracle values, with the 2D yaml
  cases covered by the runner. Five failed before the change; the `+towgs84` test passed before
  and after.

## Oracle table

Inputs are `lon lat [h]` for `+proj=latlong`, as cs2cs reads them. Output is `-f %.7f`.
"same" means the current build prints the same line as the oracle.

| Case | Pair | Input | PROJ 9.9.0 | Status |
|---|---|---|---|---|
| #71 | latlong WGS84 to sch | `0 0`, `45 45`, `30 45` (2D) | `-1977112.0305592 5551475.1418378 6595.7256583`, `1630035.5650122 -342353.6396475 128.3445654` (2D row), `1974596.2356203 787409.8217445 773.0028577` | same |
| sch-fwd-3d | latlong WGS84 to sch | `0 0 0`, `45 45 100`, `30 45 -50` | `6595.7256583`; `1630035.5391401 -342353.6343437 228.3445620`; `1974596.2639720 787409.8687649 723.0028878` | same |
| sch-fwd-h0 | latlong WGS84 to sch, `+h_0=100` | `45 45 10`, `30 45 0` | `1630061.2211312 -342359.0281723 38.3445651`; `1974627.3181230 787422.2165154 673.0028577` | same |
| #72 | sch to latlong WGS84 | `0 0 2`, `0 1000`, `1000 0`, `1000 1000` | `45 30 2`; `44.9898625 29.9981240 -0.0003617`; `44.9978450 30.0088238 5.0000000`; `44.9877066 30.0069477 -3.0005228` | same |
| sch-inv-h0 | sch to latlong WGS84, `+h_0=100` | `0 0 10`, `0 1000 0` | `45 30 110`; `44.9898627 29.9981241 99.9996383` | same |
| cart-latlong-3d | cart WGS84 to latlong WGS84 | `5000000 1000000 3000000` | `11.3099325 30.6518815 -456535.5639578` | same |
| utm-to-sch-3d | utm 32 WGS84 to sch | `500000 3300000 10` | `1352821.6057157 3210562.4722966 4437.8692212` | same |
| latlong-geocent-3d | latlong WGS84 to geocent | `10 34 100`, `0 0 0` | geocentric X Y Z (3546502.4830710 for the first) | same |
| utm-fwd-3d | latlong WGS84 to utm 32 | `10 34 100`, `10 34 -20` | height kept | same |
| utm-inv-3d | utm 32 to latlong WGS84 | `592302.98 3762148.73 100` | height kept | same |
| utm-towgs84-fwd-3d | latlong WGS84 to utm 32, `+towgs84` | `10 34 100` | `592302.9827263 3762148.7368674 100.0000000` | same |
| explicit-towgs84-3d | latlong clrk80ign `+towgs84` to latlong WGS84 | `10 34 100` | `10.0005583 34.0016039 100.0000000` | same |
| carthage-latlong-3d | latlong carthage to latlong WGS84 | `10 34 100` | `10.0005583 34.0016039 100.0000000` | same |
| nzgd49-utm-3d | latlong WGS84 to utm 59 nzgd49 | `170 -45 50` | height kept | same |
| hermannskogel-tmerc-3d | latlong WGS84 to tmerc hermannskogel | `16 48 25` | height kept | same |
| merc-3d | latlong WGS84 to merc | `10 34 100` | height kept | same |
| sch-to-carthage | sch WGS84 to latlong carthage | `0 1000 3` | `44.9898625 29.9981240 2.9996383` | z same; x, y differ (open) |
| carthage-to-sch | latlong carthage to sch WGS84 | `45 30 100` | `0.0000000 0.0000000 100.0000000` | z and x, y differ (open) |
| geocent-latlong-3d | geocent WGS84 to latlong WGS84 | `6378137 0 0` | `0.0000000 0.0000000 0.0000000` | sign of zero differs (open) |
| topocentric-3d | latlong WGS84 to topocentric | `0 40 100` | PROJ: "Mismatched units between step 1 and 2" | differs (open) |
| explicit-helmert-3d | latlong WGS84 to `+proj=helmert` | `10 34 100` | no output | differs (open) |
| latlong-same-3d | latlong WGS84 `+lon_0=10` | `10 34 100` | `0.0000000 34.0000000 100.0000000` | differs, not a height (open) |

## Deviations and open items

- The horizontal datum shift of a pair with a `+proj=sch` side is not PROJ's. In PROJ,
  `sch` (WGS84) to latlong carthage and latlong carthage to `sch` apply no shift; this build
  applies the carthage shift (about 220 m on the first row). The iso19111 path does not serve
  `+proj=sch` (`unsupported +proj=sch for a CRS`), so the pair goes through `SidePair`. Not fixed
  here: the cause is the CRS built from a PROJ string with an unknown projection, not the height.
- `+proj=pipeline` as a side: `Operation::computes_height` is `false`, so a 3D step inside it does
  not set the height in `SidePair`. No case in this lot.
- An explicit `+proj=helmert` (3D in PROJ, `fwd3d`) is not flagged; the oracle gives no output for
  the case above, so no rule is pinned.
- The sign of a zero height (`-0.0000000` against `0.0000000`) comes from the iso19111 path
  (`geocent` to `latlong`), not from `SidePair`.
- `topocentric` and `+lon_0` on a `latlong` target are other lots.

## Commands run

- `cargo test --test cli_yaml` before and after: pass; `docs/UNPORTED-CLI.md` lost #71 and #72.
- `cargo test --test gie_run` before and after: pass.
- `cargo test --test cs2cs_sch_heights`: 5 failed before the change, 6 pass after.
- Oracle comparison: `cs2cs` of the PROJ 9.9.0 build against `target/debug/cs2cs`, same inputs,
  `-f %.7f`, `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`.
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg`.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`.
- `cargo test --all-features --no-fail-fast`.
