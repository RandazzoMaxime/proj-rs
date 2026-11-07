# Port — compound to compound, unit fix lot (`createOperationsCompoundToCompound` family)

Lot `compound-to-compound`, branch `unit-fix-compound-to-compound`, base `origin/main` (7b1bce9).
Source: PROJ 9.9.0, `src/iso19111/operation/coordinateoperationfactory.cpp`, read from the tag
(`createOperationsCompoundToCompound` 7214-7847, `createOperationsCompoundToGeog` 6454-6905,
`createOperationsToGeod` 7178-7211, `createOperationsVertToGeog` 6070-6109).

## Result

No test was closed in this lot. The six open tests of `tests/unit_op_compound_to_compound_2.rs`
stay `#[ignore]`, each with its cause in the reason. One test was added and is open: the
`compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` draft (`operation_compound_with_bound_geog_and_geoid_to_geod_nad2011_ctxt`,
`#[ignore]`). No `src/` change is committed.

The reason is that every open case depends on code outside the compound-to-compound functions
of this lot, or on a context feature this port does not have. Each cause below was checked against
the reference C API (`proj_create_operations`, 9.9.0, grid availability ignored, partial
intersection, authority `""` or `EPSG` as the test uses) and the candidate names, not only read.

## Tests

| PROJ `TEST()` | Ours / PROJ | Cause (checked) | Where it lives |
|---|---|---|---|
| `compoundCRS_to_compoundCRS_context` | 137 / 158 candidates (test asserts >= 152) | The 21 missing candidates are `NGVD29 height (ftUS) to NAVD88 height (N) + NAD27 to WGS 84 (M) + Inverse of NAD83(NSRS2007) to WGS 84 (1)`. The vertical parts are the same. The horizontal part `createOperations(NAD27, NAD83(NSRS2007))` gives 67 here and 76 in PROJ: the missing ones are `NAD27 to WGS 84 (1|2|3|12|13|18|33|34) + Inverse of NAD83(NSRS2007) to WGS 84 (1)` and `NAD27 to NAD83 (4) + NAD83 to NAD83(CSRS) (4) + NAD83(CSRS) to WGS 84 (2) + ...`. `NAD27 -> WGS 84` alone matches (79 and 79). | datum pivot candidate set (`createOperationsWithDatumPivot`, `findsOpsInRegistryWithIntermediate`), not this lot |
| `compoundCRS_to_compoundCRS_OSGB36_BNG_ODN_height_to_WGS84_EGM96` | first candidate wrong | Compound to `OSGB36 (geographic3D)` (unidentified, promoted) gives one candidate here, `Inverse of British National Grid + Inverse of ETRS89-GBR [OSNet v2009] to ODN height (2)`, with no horizontal shift. PROJ gives `OSGB36 to ETRS89-GBR (2) + Inverse of ETRS89-GBR to ODN height (2) + Ballpark geographic offset from ETRS89-GBR to OSGB36` and three more. Here the cause is `projected_compound_to_geog` (`compound.rs`), which returns early and never reaches the interpolation loop of `compound_vertical_pass`. See "Experiment" below. | compound-to-geog, projected horizontal part |
| `compoundCRS_to_compoundCRS_issue_3152_ch1903lv03_ln02_bound` | empty list | Same as the previous lot's notes: `Crs::is_equivalent_to_with` has no geographic arm (`equiv.rs`), and `bound_compound_to_geog3d` drops the LN02 height. Not re-traced here. | `equiv.rs`, `bound_compound_to_geog3d` |
| `compoundCRS_to_compoundCRS_concatenated_operation_with_two_vert_transformation_and_different_source_dest_interp` | one extra leg | The first candidate has `Inverse of BD72 to ETRS89-BEL (3) + BD72 to ETRS89 (3)` before `Inverse of Amersfoort to ETRS89 (9)`. `BD72 -> ETRS89 (4258)` (2 and 2) and `BD72 -> ETRS89-BEL [BEREF2011]` geographic 2D (EPSG:11215, 2 and 2) match. `ETRS89-BEL [BEREF2011]` geographic 3D (EPSG:11214) to ETRS89 3D (EPSG:4937) differs: PROJ `Null geographic offset from ETRS89-BEL [BEREF2011] to ETRS89`, here `Inverse of ETRS89 to ETRS89-BEL [BEREF2011]` (1 and 1, different names). The BD72 pivot leg is then taken for the ETRS89-BEL -> ETRS89 step. | geodetic candidate set (null offset of a datum ensemble), then the pivot |
| `compoundCRS_to_compoundCRS_using_intermediate_of_horizontal_transform` | 2 / 1 candidates | PROJ has one candidate, the six-step chain through `WGS 84 (G2139)` and the EGM96 geoid. It comes from the only-vertical-ballpark branch (C++ 6843-6905) of `createOperationsCompoundToGeog`, which takes the intermediate geographic CRS from the horizontal transformations (`candidateIntermGeogCRS`). Here the two ballpark `Kumul 34 height to EGM2008 height` candidates remain. That branch uses `geogCRSOfVertCRSStack` and `SetSkipHorizontalTransform`; `compound_vertical_pass` is an approximation without the stack. | compound-to-geog, needs the `geogCRSOfVertCRSStack` context |
| `compoundCRS_with_boundGeogCRS_and_geoid_to_geodCRS_NAD2011_ctxt` (added, `#[ignore]`) | `Inverse of unknown + ... + Conversion from NAD83(2011) (geog3D) to NAD83(2011) (geocentric)` not produced | The compound source to the geocentric target goes through `bound_to_geod` (`createOperationsToGeod`), which takes the first candidate of `createOperations(source, NAD83(2011) geog3D)`. Here that call gives 0 candidates (PROJ: 1, `Inverse of unknown + Transformation from NAD83 to WGS84 + Inverse of NAD83(2011) to WGS 84 (1)`), because the projected horizontal part is taken by `projected_compound_to_geog`. The projected source `EPSG:2229 -> EPSG:6318` is also short (ours 1, PROJ 4): the `NAD83 to WGS 84 (1)` pivot legs are missing there too. | compound-to-geog, projected horizontal part, and the datum pivot set |

## Experiment: the derived branch of `createOperationsCompoundToGeog` (not kept)

PROJ 6473-6502 handles a derived (projected) horizontal part before anything else: the source is
chained to the compound of the base CRS and the vertical part (`opsFirst.front()`), then that
compound to the target (`opsLast`), with `createComputeMetadata` on each pair. `compound.rs` has
`projected_compound_to_geog` instead, a different construction (one horizontal operation, one
vertical operation chosen by `createOperations(vertical, target)`, no interpolation loop).

Replacing `projected_compound_to_geog` by the faithful branch (`CompoundCRS` of the base and the
vertical part, `create_candidates` on both sides, first of the first step kept) was tried on this
branch and reverted:

- compound to `OSGB36 (geographic3D)` (unidentified): 11 candidates here, 5 in PROJ. The extra ones
  end with `Inverse of OSGB36 to ETRS89 (2)` or `... to ETRS89-GBR (2)` as a last step, which PROJ
  does not keep. The interpolation pairs of `compound_vertical_pass` (`same_geog`, datum set,
  `compatible_transformations`) are not the PROJ filters, so this was not fixed.
- `operation_compound_to_compound_issue_4550`: the proj string export fails with
  `this operation is not a projection of the source CRS`. The derived first step is not accepted
  by the export of the chained operation.
- `operation_compound_to_compound_wgs84_egm2008_to_rd_new_nap_height`: 8 candidates, the test
  expects 4. The extra four come from the derived branch on the RD New + NAP compound (reached
  through `geog_to_compound`); which of its steps PROJ does not keep was not traced.

The change regressed two tests that passed before, so it was reverted. The next attempt has to port
the derived branch together with the interpolation loop of `createOperationsCompoundToGeog`
(6906-7000 and the pair filters at 7100-7130) and check `issue_4550` and the RD New + NAP test in
the same step.

## Shared code outside this lot, and what it needs

| Need | PROJ 9.9.0 | Owner |
|---|---|---|
| Derived projected horizontal part of `createOperationsCompoundToGeog` | 6473-6502 | compound-to-geog lot |
| Interpolation pair loop with its filters | 7040-7130 (`createHorizVerticalHorizPROJBased` 7142-7162) | compound-to-geog lot |
| `geogCRSOfVertCRSStack` and `SetSkipHorizontalTransform` contexts | 6741-6785 | context (`factory.rs`) |
| Only-vertical-ballpark intermediate branch | 6843-6905 | compound-to-geog lot, after the stack |
| Datum pivot legs `NAD27 -> WGS 84 (N) + Inverse of NAD83(NSRS2007) -> WGS 84 (1)` | `createOperationsWithDatumPivot` | pivot candidate set |
| Null geographic offset for `ETRS89-BEL [BEREF2011] -> ETRS89` | `createOperationsGeodToGeod` (same datum ensemble) | geodetic lot |

## Commands run

- `git fetch origin`; branch `unit-fix-compound-to-compound` from `origin/main` (7b1bce9).
- `cargo test --test unit_op_compound_to_compound_2 -- --include-ignored`: 8 passed and 5 failed on
  `origin/main` (the five ignored tests); 8 passed and 6 failed with the added NAD2011 draft; with
  the experiment, 6 passed and 8 failed (two regressions); 8 passed and 6 ignored after the revert.
- Reference candidate lists: a C program against the 9.9.0 `libproj` in the scratch build,
  `proj_create_operations` with `proj_create_operation_factory_context`, grid availability
  `IGNORED` or `KNOWN_AVAILABLE`, spatial criterion `PARTIAL_INTERSECTION` or `STRICT_CONTAINMENT`.
  The names were compared with the names of `create_operations` (a temporary probe test, removed).
- `projinfo --grid-check none --spatial-test intersects` was not used for the counts: its
  candidate filtering differs from the C++ API the tests use.
