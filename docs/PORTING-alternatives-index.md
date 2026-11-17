# Port — per-point choice among candidates, with a grid

Source: PROJ 9.9.0 `src/transform.cpp`/`src/trans.cpp` `pj_get_suggested_operation`
(`trans.cpp:47-176`), ported in `src/iso19111/alternatives.rs`.

## Deviation (implementation only, no change of result)

PROJ tests the point against every candidate's box. With 80 candidates
(`EPSG:4267` to `EPSG:4326`) that is 80 box tests per point, about 90 % of the
cost of a point.

`BoxIndex` (`src/iso19111/alternatives_index.rs`) is a 48 by 24 grid over the
candidate boxes, built once per direction at the first point. A cell lists the
candidates whose box overlaps it, in increasing candidate index. `suggest` runs
the same greedy choice (`accepted`, `offshore`, `priority`, `pseudo_area`,
`excluded`, instantiability) over the cell's list instead of the whole list, so
the order of the candidates seen is the original one and the choice is the
same.

The grid only discards candidates whose box cannot contain the point. The exact
`covers` test still runs on the others. The grid is not used, and the full scan
runs, when:

- the point is `None` or has a NaN coordinate;
- the source is read as geocentric (a world box covers every point);
- a box read as longitude and latitude (or latitude and longitude) would see
  `normalizeLongitude` change the point: the antimeridian and longitudes outside
  `[-180, 180]`;
- there are fewer than 6 candidates.

## Proof of equivalence

`grid_choice_equals_full_scan` (and `grid_choice_equals_full_scan_heavy`,
ignored) compare the grid with `suggest_full`, the old scan, on random points,
exact box edges and their neighbours, `±180`, `±360`, NaN and infinities, with
random `excluded` sets, both instantiability modes and both directions, on pairs
that cover forward, inverse, latitude-first, longitude-first, geocentric and
antimeridian pieces.
