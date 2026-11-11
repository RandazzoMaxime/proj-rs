# Lot `io-create-from-user-input`: `createFromUserInput`

Branch `unit-io-create-from-user-input`, base `origin/main`. Scope: the CRS-returning blocks of
`TEST(io, createFromUserInput)`, the entry point without a database, and the URN and
authority-fallback forms those blocks need.

## PROJ functions read

| PROJ 9.9.0 | Lines | Role |
|---|---|---|
| `test/unit/test_io.cpp` `TEST(io, createFromUserInput)` | 13718-14275 | the test |
| `src/iso19111/io.cpp` `createFromURNPart` | 7652-7707 | URN object lookup, versioned fallback |
| `src/iso19111/io.cpp` `createFromUserInput` (internal) | 7711-8302 | routing of the text forms |
| `src/iso19111/io.cpp` `urn:ogc:def:crs,` branches | 7915-8090 | compound and derived CRS URNs |
| `src/iso19111/io.cpp` 2-token `AUTH:CODE` fallback | 7832-7888 | versioned and case-insensitive authority |
| `src/iso19111/io.cpp` public wrappers | 8361-8423 | `usePROJ4InitRules`, null context |
| `src/iso19111/io.cpp` vertical derived names | 8003-8052 | unit suffix, height/depth rename |
| `src/proj_constants.h` | 837-840, 1007 | EPSG methods 1069, 1104 (unit), 1068 (height/depth) |

## Map: PROJ function -> Rust

| PROJ | Rust (`src/iso19111/json.rs` unless said) | Status |
|---|---|---|
| `createFromUserInput(text, db, rules)` | `create_from_user_input_with_optional_database` (new), `create_from_user_input_with_init_rules` delegates | ported, routing kept |
| `createFromUserInput(text, nullptr)` | `create_from_user_input_with_optional_database(text, None, rules)` | ported: authority, URN, name, OGC URL and `init=` refused; PROJ strings and WKT read |
| `createFromURNPart` | `crs_from_urn_part` | ported for CRS; other object types refused |
| `urn:ogc:def:crs,` compound | `crs_from_urn_combination` | ported |
| `urn:ogc:def:crs,` derived | `crs_from_urn_derived` | ported for Conversion; projected, geodetic, geographic, vertical |
| vertical renaming (io.cpp 8003-8052) | `vertical_renamed` | ported |
| 7-token and legacy 6-token URN | `crs_from_urn` | ported |
| 2-token `AUTH:CODE`, versioned and ci fallback (7851-7887) | `crs_from_authority_code` | ported; the `+` split runs inside the case-insensitive candidate, as in PROJ |
| `init=` refusal without a database | `crs_from_init_text` takes `Option<&DatabaseContext>` | ported |
| `pj_create` refuses a name outside `pj_list.h` (io.cpp 12799-12896) | `PJ_LIST_NAMES`, `is_pj_list_name` (`projstring.rs`) | ported (round 2) |

## Map: PROJ `TEST()` -> Rust test

`tests/unit_io_create_from_user_input.rs`, one test `io_create_from_user_input`, in the C++
order, one block per C++ block that returns a CRS. Expected values are copied from the C++ test.

Covered (C++ line ranges): 13720-13725, 13727-13745, 13764-13773, 13785-13797, 13800-13816,
13818-13843, 13845-13865, 13968-13982, 13986-14019, 14021-14093, 14105-14117, 14120, 14123,
14197-14200, 14211-14214.

Not covered (not in the test; missing piece in brackets):

- 13746-13762, 14096-14103: `urn:ogc:def:coordinateOperation,` and `coordinateOperation:` URNs
  [the entry point returns `Crs`; there is no operation variant in `UserObject`].
- 13774-13775, `urn:ogc:def:coordinateOperation:EPSG::1671` [same].
- 13776-13783: `urn:ogc:def:datum`, `ensemble`, `meridian`, `ellipsoid` [`Crs` return; `UserObject`
  has the geodetic datum but no ensemble, meridian or ellipsoid].
- 13767 `NN2000:2018 height`: not read here; the database content was not checked.
- 13867-13904 `EPSG:4326+4326` (expects a GeographicCRS 4979 with a COMPD_CS WKT) and 13910-13960
  `EPSG:32631+4326` (ProjectedCRS with a 3D base): the non-conformant compound path of the factory
  is not ported; the port builds a lax compound.
- 13907-13908 `EPSG:4326+4258` `InvalidCompoundCRSException`: same missing path.
- 14133-14137, 14141-14190, 14203-14208, 14217-14274: name searches (`UTM zone 31N`, `WGS84`
  alias, `ID74`, the vertical approximate matches, `EGM96 height` compounds, `WGS 84 UTM 31 north`,
  `WGS 85`, the ensemble name): the name search is not ported in full (it searches only CRS
  objects, and the compound name split uses approximate matches only).
- 14219-14268: the `@` epoch forms (`ITRF2014@2025.1`, `EPSG:9000 @ 2025.1`, `@`, `RGF93@2025`)
  [need `CoordinateMetadata` as a return object].

## Deviations

- A text that PROJ reads with `NoSuchAuthorityCodeException` gives `FactoryError::NotFound`; a
  `ParsingException` gives `FactoryError::Format` (text forms) or `FactoryError::Invalid`.
- Without a database, a JSON text is refused. PROJ reads it with a null context (not in this lot).
- The derived CRS of a URN whose conversion is not a `Conversion` is refused with PROJ's
  "unsupported combination" message. Operations are not returned by this entry point.
- The `+` compound of an authority code is built with `implicit_compound` (lax), as before. The
  name is the same as PROJ's `CompoundCRS::createLax` name.
- `urn:ogc:def:crs,` compound uses `CompoundCRS::create` (not lax), as PROJ does.
- The version of a `urn:ogc:def:crs:AUTH:VERSION:CODE` is used only when the authority without
  it fails, as `createFromURNPart` does.

## Callers at risk

The routing order of the text forms is unchanged for the forms that worked before: JSON, OGC
`AUTO:`, `init=`, `proj=`, OGC CRS URL, `AUTH:CODE`, URN (new position: after `AUTH:CODE`, before
names, as in `io.cpp`), names, WKT. A text that failed before can now succeed: a lower-case
authority (`epsg:4326`), a versioned authority (`IAU:1000`), a URN CRS (`urn:ogc:def:crs:...`).
Each of these callers passes user text to `create_from_user_input` and can see such a text now
resolved instead of rejected:

- `src/bin/cs2cs.rs`: 861 (fallback path), 1611-1612 (source and target).
- `src/bin/projinfo.rs`: 1274, 1612, 1730.
- `src/bin/proj.rs`: 338.
- `src/crs.rs`: 848 (`EPSG:{code}`, unchanged input).
- `src/proj_info.rs`: 239, 260.
- `src/proj_export.rs`: 117 (re-reads a stored definition).
- `src/iso19111/select.rs`: 284-285, 307-308, 1122-1123, 1327-1328 (source and target of a
  pipeline).
- `src/iso19111/json.rs`: `epsg_identity` (1921), `create_object_from_user_input` (2885).
- `src/iso19111/factory.rs`: 3559 (`CoordinateMetadata` row without a CRS code).
- `src/lib.rs`: 95 (re-export, unchanged signature).

No existing signature changed. `create_from_user_input` and `create_from_user_input_with_init_rules`
keep their parameters.

Round 2 (`projstring.rs`): a `+proj=` name is now a PROJ-based CRS only when it is in PROJ's
`pj_list.h`. The only existing test strings with `+type=crs` use PROJ names (checked by grep over
`tests/` and `src/`); `refused("proj=unknown +type=crs")` in `tests/test_io_units.rs` already
expected an error. Callers that pass a made-up `+proj=` name with `+type=crs` now get an error
instead of a PROJ-based CRS.

## Open

- The blocks listed under "Not covered": the object kinds (operation, ensemble, ellipsoid, prime
  meridian, coordinate metadata) need a return type wider than `Crs`; the non-conformant compound
  path needs the factory's `+` handling.
- Name search parity with `searchObject` (`io.cpp` 8161-8298): C++ searches ellipsoid, datum,
  ensemble and operation names in passes 2 and 4, and the ci authority lookup of the factory.
  Not reviewed in this lot.

## Round 2: coordinator run of the test (round 1)

Our output (the coordinator's run): `io_create_from_user_input` FAILED at
`tests/unit_io_create_from_user_input.rs:40`, `read_without_database("+proj=unhandled +type=crs").is_err()`
was false. The test stops at its first failing assert, so this is the only failure reported; the
blocks after it had not run.

PROJ's output: `createFromUserInput("+proj=unhandled +type=crs", nullptr)` throws
`ParsingException` (test_io.cpp:13722). Trace: `createFromUserInput` routes the `+proj=` text to
`PROJStringParser().createFromPROJString` (io.cpp 7804-7818). The structure check leaves
`unexpectedStructure` set (`unhandled` is no projected step, io.cpp 12740-12749), then
`pj_create_internal(... +disable_grid_presence_check)` fails because `pj_init` looks the name up in
`pj_list.h` and does not find it (init.cpp 424-434). `valid` is false and the `ParsingException`
of io.cpp 12894-12896 is thrown.

Our trace: `crs_from_proj_string` (`projstring.rs`) found no method for `unhandled`, and then
built a PROJ-based projected CRS for any identifier-like name when `+type=crs` is set. A name that
PROJ does not know is not a PROJ-based CRS.

Fix (commit "projstring: a +proj name outside PROJ's table is refused as a CRS"): the name must be
in PROJ 9.9.0's `pj_list.h` table (`PJ_LIST_NAMES`). `mbt_s`, `peirce_q` and `ob_tran` stay
PROJ-based, as before. The old `proj_identifier` check is removed.

Audit of the blocks after line 40, by reasoning against the C++ (none of them has run yet):

- Error-variant asserts (`NotFound`): an unknown authority or code is `NoSuchAuthorityCodeException`
  in PROJ; ours is `FactoryError::NotFound` through `FactoryError::missing`.
- Token splits: PROJ's `split` (`internal.cpp` 301-311) keeps empty fields, as Rust's `split` does,
  so the URN token counts of the error cases match.
- Versioned and case-insensitive authority fallback (io.cpp 7851-7887), IAU and `epsg:4326`.
- Compound and derived URNs (io.cpp 7920-8058): name composition, `(3D)`, the derived geodetic,
  geographic, projected and vertical kinds, and the vertical unit and height/depth renames
  (io.cpp 8003-8052). `US survey foot` is the name of EPSG 9003 in both (static.cpp 338).
- `WGS 84` exact name with two matches, and `WGS84` alias: the 2-D pick (io.cpp 8175-8185).

Still open, not in this test: `NN2000:2018 height` (line 13767) is refused here. The name search
of `createFromUserInput` (io.cpp 8157-8299) does not exclude texts with `:` or `+`; our gate in
`crs_from_name` (`json.rs`, `!text.contains(['[', '+', ':'])`) does. Not changed in this lot.

Tests written, not run by the author: the coordinator runs them.

Commands run: `cargo fmt`; `cargo check -p proj-rs --tests` (a run, round 1; a run, round 2, succeeded).
