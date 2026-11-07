# Porting sheet: miscellaneous string and name causes (unit lot `strings-misc`)

Branch `unit-fix-strings-misc`, base `origin/main`. Tests of PROJ 9.9.0 `test/unit/test_crs.cpp`,
`test_operationfactory.cpp` and `test_operation.cpp` whose failing assertion is a string or a name:
`crs.norway_ntm`, `operation.nadgrids_with_pm`, `operation.ETRS89_3D_to_proj_string_with_geoidgrids_nadgrids`,
`operation.boundCRS_to_geogCRS_hubCRS_and_targetCRS_same_but_baseCRS_not`,
`operation.vertCRS_to_geogCRS_context`, `crs.crs_createBoundCRSToWGS84IfPossible`,
`operation.normalizeForVisualization`. One cause per commit. Expected values come from PROJ 9.9.0,
checked with the reference `projinfo` (`scratchpad/reftest/build/bin/projinfo`, `PROJ_DATA` on the oracle
`proj.db`) before any code was changed.

## Commit 1 — ESRI WKT1 name of a CRS with no ESRI alias: the `EPSG_OLD` fallback

### What was wrong

`EPSG:5105` (ETRS89-NOR [EUREF89] / NTM zone 5) exports to WKT1_ESRI with the base geographic CRS named
`GCS_ETRS89-NOR_EUREF89` and the datum `D_EUREF89`. PROJ writes `GCS_ETRS_1989` and `D_ETRS_1989`.
The base CRS `ETRS89-NOR [EUREF89]` (EPSG:10873) has no ESRI alias, so the name is built from its
own name. PROJ has a fallback for that case, which this port did not have.

### What PROJ does

- `GeodeticCRS::_exportToWKT`, ESRI branch (crs.cpp:2225-2330): the ESRI alias by identifier, the
  ESRI alias by official name, the ESRI name of the same name (`createObjectsFromName`), then, when
  none of these names the CRS, the `EPSG_OLD` alias of the CRS (crs.cpp:2269-2300). The single old
  name is looked up among the EPSG geodetic CRSs (`createObjectsFromNameEx`, no aliases); the first
  object with the same nature (2D, 3D, geocentric) and a single ESRI alias gives the name. For
  EPSG:10873 the old name `ETRS89` is EPSG:4258, whose ESRI alias is `GCS_ETRS_1989`.
- `GeodeticReferenceFrame::_exportToWKT`, ESRI branch (datum.cpp:1385-1443): the same chain for the
  datum; the `EPSG_OLD` name of EUREF89 (1407) is `European Terrestrial Reference System 1989`, which
  PROJ turns into the ensemble name `... 1989 ensemble` (EPSG:6258), whose ESRI alias is `D_ETRS_1989`.
- The fallback is used only when the earlier steps found no name, and only when the object has an
  identifier.

### Port

| PROJ 9.9.0 | Rust (`src/iso19111`) |
|---|---|
| `GeodeticCRS::_exportToWKT`, ESRI, `EPSG_OLD` fallback (crs.cpp:2269-2300) | `wkt.rs`, `esri_geographic_name` -> `esri_crs_epsg_old_name` |
| `GeodeticReferenceFrame::_exportToWKT`, ESRI, `EPSG_OLD` fallback (datum.cpp:1392-1433) | `wkt.rs`, `wkt1_datum_name` -> `esri_datum_epsg_old_name` |

Gap kept: PROJ's `createObjectsFromNameEx(..., useAliases = false)` matches the official name only.
The port uses `NameMatch::Exact`, which also matches aliases. For EPSG:10873 both give EPSG:4258.

### Import side, same lot

The ESRI WKT1 of EPSG:5105 must come back as EPSG:5105 with identification 100 (the second half of the
C++ test). The import had no port of PROJ's ETRS89 projected fixup, io.cpp:4690-4716: a projected CRS
named `ETRS89 / ...` (or `ETRS_1989_...` in ESRI style) on a 2D ETRS89 geographic base takes the EPSG
CRS of that name when the bases are equivalent but for the axis order. The fixup reads the name of the
WKT node (`projCRSName`, io.cpp:4600), before the ESRI alias of the projected CRS is applied.
`wkt_import.rs`, `build_projected`, the `else if` after the `ETRF2000-PL / CS92` branch.

Not closed: the WKT2 part of the C++ test (`ETRS89 / NTM zone 5` with an ensemble base, before EPSG
12.025). The fixup finds the candidate EPSG:5105 there, but the base equivalence (`EQUIVALENT_EXCEPT_AXIS_ORDER`
with the database) does not see the ensemble ETRS89 as equivalent to the frame EUREF89 of EPSG:10873.
That is an equivalence gap in `equiv.rs` (ensemble against frame), outside this commit.

### Tests

File `tests/unit_crs_strings_misc.rs`, PROJ `crs.norway_ntm`. The test stays in `docs/UNPORTED-UNIT.md`
(the WKT2 part is open). The two parts that pass are:

| Rust test | PROJ | Status |
|---|---|---|
| `crs_norway_ntm_esri_wkt1_export` | `crs.norway_ntm`, first assertion (test_crs.cpp:8002-8006) | closed as a part, commit 1 (failed on the base: `GCS_ETRS89-NOR_EUREF89`) |
| `crs_norway_ntm_esri_wkt1_import_identifies_5105` | `crs.norway_ntm`, ESRI import and identify (test_crs.cpp:8009-8021) | closed as a part, commit 1 (the base test stopped at the export; with the export fixed, the import gave base `ETRS89`) |
