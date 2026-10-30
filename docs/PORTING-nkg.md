# Port — NKG coordinate operations

Source: PROJ 9.9.0 `data/sql/nkg.sql`, loaded the way
`AuthorityFactory::createCoordinateOperation` reads `helmert_transformation`,
`concatenated_operation`, and `other_transformation` in
`src/iso19111/factory.cpp`. A Helmert row becomes `+proj=helmert` in
`SingleOperation::_exportToPROJString` (`src/iso19111/operation/singleoperation.cpp`).
The numbers follow `update_parameters` and the approximate branch of
`build_rot_matrix` in `src/transformations/helmert.cpp`. Unit factors are the
SI conversions in `src/iso19111/static.cpp` (`YEAR` = 31556925.445 s).

`tests/gie/nkg.gie` names `urn:ogc:def:coordinateOperation:NKG::…` operations.
Every one of those chains includes `+proj=deformation` or `+proj=xyzgridshift`.
Creation returns an error. The crate does not download the PROJ-data CDN and
does not apply the Helmert steps alone: the grid is several millimetres on the
gie points, inside the 0.1 mm and 1 mm tolerances.

## Quirks copied on purpose

- Concatenated steps in `nkg.sql` are numbered from 2, not 1. The loader
  orders by `step_number` and does not invent a missing first step.
- `step_direction = reverse` swaps forward and inverse for that step, then
  composes. A reverse of an already reversed inner step points forward again.
- Translations, rotations, and scale are converted by the ratio of
  `unit_of_measure.conv_factor` values, which are SI per second for rates.
  Millimetres per year (EPSG:1027) become metres per year because both factors
  contain the same Julian year. Epoch values in years divide by the year
  factor and stay years. `P(t) = P(epoch) + rate * (t - epoch)`.
- The rotation is the linearized matrix, not the exact trigonometric one.
  Position vector (EPSG:1033, EPSG:1053) keeps the sign; coordinate frame
  (EPSG:1032, EPSG:1056) flips it. That is the same matrix as
  `Helmert::apply` / `Helmert::apply_inverse`.
- Unset time in this crate is `0` (the gie parser) or non-finite (PROJ's
  `HUGE_VAL`). Both use `t_epoch`, so the rates contribute nothing. An
  observation epoch of year 0 cannot be told apart from a missing time.
- Methods 1031, 1032, 1033, 1053, 1056, 1065, and 1066 are geocentric. The
  gie input is already ECEF metres. There is no `+proj=cart` wrapper.
  A Molodensky pivot, or a geographic-domain method, is an error rather than
  a Helmert that ignores the pivot.
- A PROJ-string step that names a grid with `+grids=`, `+xy_grids=`, or
  `+z_grids=` is built when every required grid (not `@`-optional) is on disk
  or fetchable with the network (`gridpath::available`). Otherwise the whole
  URN is `Unsupported("grid")`. The 19 `PROJString` rows of `nkg.sql` all name
  a grid, so they follow this rule. A nested NKG URN inside that string is
  rejected, so the loader cannot call itself.

## Verification

Grid-free `NKG:P1_2008_DK` and the time-dependent step EPSG:7941 match
`cct -d 9` to 1e-6 m, including `t = 2000` and a missing time.

Offline (no `PROJ_NETWORK`, no grids on disk), the 26 chained URNs return
`CreateError::Unsupported("grid")`, and the gie runner skips them. With
`PROJ_NETWORK=ON` and the grids fetched, `tests/gie/nkg.gie` builds all 33
cases and the network ratchet holds at 6,856.
