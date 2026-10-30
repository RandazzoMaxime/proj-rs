# projinfo: list, kind, completion, and database options

Port of the `projinfo` options that list or look up objects, from PROJ 9.9.0:
`projinfo_lib.cpp` (`main_projinfo`, `suggestCompletion`, `buildObject`,
`makeBboxFilter`, `outputObject`) and the helpers they call in
`iso19111/factory.cpp` (`getDatabaseStructure`, `createObjectsFromNameEx`,
`listAreaOfUseFromName`), `metadata.cpp` (`GeographicBoundingBox::contains`,
`intersects`) and `datum.cpp` (`Ellipsoid::_exportToPROJString`).

Covered options: `--list-crs`, `-k` (`ellipsoid`, `datum`, `ensemble`, and
`operation` as an error), `completion`, `--dump-db-structure`,
`--searchpaths`, `--remote-data`.

## API mapping

| PROJ (C++) | Rust |
|---|---|
| `main_projinfo` option loop | `Options::parse` (`src/bin/projinfo.rs`), then `run` in PROJ's order |
| `suggestCompletion(args)` | `complete_shell`, `completion_phrase`, `vertical_suggestions` |
| `listCRS` loop and `makeBboxFilter` | `list_crs`, `scope`, `Scope::admits` |
| `GeographicBoundingBox::contains` / `intersects` | `GeoBox::contains` / `GeoBox::intersects` (CLI only) |
| `buildObject` with `-k` | `build_object`, `named_object` |
| `outputObject` for a non-CRS object | `render`, `proj_text`, `wkt` over `AuthorityObject` |
| `DatabaseContext::getDatabaseStructure` | `DatabaseContext::database_structure` (`src/iso19111/factory.rs`) |
| `AuthorityFactory::createObjectsFromNameEx` | `AuthorityFactory::create_objects_from_name`, sharing `objects_from_name` with the geodetic CRS and datum lookups |
| `AuthorityFactory::listAreaOfUseFromName` | `AuthorityFactory::list_area_of_use_from_name` |
| `AuthorityFactory::createEllipsoid`, `createDatum`, `createDatumEnsemble` | the same names, already ported |
| `Ellipsoid::_exportToPROJString` | `Ellipsoid::export_to_proj_string` (`projstring.rs`) |
| `Identifier::isEquivalentName` | `equivalent_name` (`equiv.rs`), now public |
| `proj_info().searchpath`, `proj_context_is_network_enabled` | `Proj::lib_info`, `Proj::network_enabled`, `Proj::get_url_endpoint` |

New public items: `AuthorityObject::name` and `is_deprecated`, `Crs::domains`,
`Datum`, `DatumEnsemble`, and `Ellipsoid` WKT export, `NameMatch`,
`create_objects_from_name`, `list_area_of_use_from_name`, `equivalent_name`.
`NameMatch` replaces the `bool` PROJ passes for approximate matching, because
the workspace lints forbid a `bool` parameter in a public signature.

## Test mapping

Yaml cases, `tests/cli_yaml/test_projinfo.yaml`, numbered as the runner numbers them:

| Cases | Options | Status |
|---|---|---|
| 40, 41, 42, 43, 44, 47 | `-k ellipsoid`, `-k datum`, `-k ensemble` | pass |
| 45 | `-k operation EPSG:8457 -o PROJ` | open, see below |
| 48, 49, 50 | `--searchpaths`, `--remote-data` (both states) | pass |
| 66 | `--dump-db-structure` | pass |
| 67 | `--dump-db-structure --output-id HOBU:XXXX EPSG:4326` | open, see below |
| 69 to 71, 77, 78 | `--list-crs` without filter or with a type | pass (already passing) |
| 72 to 76, 79 to 82 | `--list-crs` with `--bbox`, `--area`, `geocentric,geographic_3d`, `geographic_2d,allow_deprecated` | pass |
| 91 to 104 | `completion projinfo ...` | pass |

Rust tests, one per behavior:

| Rust test | PROJ behavior |
|---|---|
| `database_structure_lists_tables_then_layout_version` (`tests/test_factory.rs`) | `getDatabaseStructure` order and the layout version rows |
| `factory_create_objects_from_name_finds_the_ellipsoid_by_name` | `createObjectsFromName` with `WGS 84` |
| `ellipsoid_proj_string_is_radius_for_a_sphere` | `Ellipsoid::_exportToPROJString` for `+R=` and `+ellps=` |
| `contains_follows_the_antimeridian_rules`, `intersects_follows_the_antimeridian_rules` | `GeographicBoundingBox::contains`, `intersects` |
| `bbox_needs_four_values` | `makeBboxFilter` error messages |
| `completion_phrase_joins_a_quoted_start` | the `lastArg` rule in `suggestCompletion` |
| `unknown_kind_is_rejected` | `-k` value check |

## Deliberate differences

- **`-k operation` is an error.** PROJ builds the operation from a URN or a
  name and writes its PROJ string as a pipeline. The PROJ string of a
  transformation is not ported (another lot), and `create_objects_from_name`
  does not load coordinate operation tables. Case 45 stays open for that reason.
- **`--dump-db-structure` with an object needs `-o SQL`.** PROJ writes the
  `INSERT` statements of the object, which needs SQL export. The port reports
  `cannot write dialect SQL` and exits 1, as `-o SQL` does (cases 6, 7). Case 67
  stays open for that reason.
- **`--searchpaths` lists only the directories the caller added** (`set_search_paths`),
  which is empty by default. PROJ also lists `PROJ_DATA`, the user-writable
  directory, and the install directory. The port does not model those, so
  it prints nothing by default. Case 48 checks the exit code only.
- **PROJJSON for an ellipsoid, datum, or ensemble** is an error. PROJ writes it.
  WKT (WKT2 and WKT1) and PROJ strings are written for them.
- **Authorities with versions.** `getVersionedAuthoritiesFromName` is not ported:
  each `--authority` name is used as given.
- **`--list-crs` without an authority** lists the authorities one after another
  in name order. The output matched PROJ on the full list in this run; the
  order is not guaranteed to match in general.
- **Latitude warnings** (`suspicious south latitude`) for `--bbox` are not written.
- **Completion with no candidate** prints nothing. PROJ prints one empty line.
- **Trailing newline.** `main` adds one newline to the output; the completion
  text is returned without its final newline, so the bytes match PROJ's
  stdout except for that empty line.

## Preserved quirks

- A name prefix that matches several authorities lists each name once per
  usage. `--list-crs` prints a code twice when PROJ does (for example
  `EPSG:25829`).
- `D_` names hide the vertical datums and, for `-k datum`, the engineering datums,
  as PROJ does for ESRI names.
- `--area` with several matches filters on the area name, not on the extent.
- A sphere prints `+R=`, not `+a=... +b=...`, for any CRS and for `-k ellipsoid`.
- Unknown `-k` values, `--list-crs` types, `--spatial-test` values, and bbox
  counts are usage errors (exit 1).

## Verification

- `cargo test --test cli_yaml` with `PROJ_CS2CS_REQUIRED=1`: no regression; the
  projinfo cases still open went from 95 to 65. Every open case of this lot
  is 45 or 67.
- `cargo test --all-features --no-fail-fast`: see the PR for the result.
- `cargo clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt`: applied.
- Byte-for-byte comparison against the PROJ 9.9.0 binary on about 40 extra
  invocations (completion prefixes, `--list-crs` types and filters, `-k` names,
  `--dump-db-structure`, `--remote-data`): identical, except the two known
  differences above (`--searchpaths` default output, and the blank line on an
  empty completion).
