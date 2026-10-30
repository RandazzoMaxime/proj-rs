# Port — `cs2cs` non-EPSG names and authority codes

Source: PROJ 9.9.0 `src/iso19111/io.cpp` (`createFromUserInput`, `searchCRS`,
`searchObject`, 8150–8260), `src/iso19111/factory.cpp`
(`createObjectsFromNameEx`, 9362–9735), `src/iso19111/metadata.cpp`
(`Identifier::canonicalizeName`, ~1280–1365), and `src/apps/cs2cs.cpp`.

The Rust side is `canonical_search_name` and `approximate_name` in
`src/iso19111/factory.rs`, `crs_from_name` and `crs_named_one` in
`src/iso19111/json.rs`, and `canonical_side` in `src/bin/cs2cs.rs`.

## Behaviour

### Names (`createFromUserInput`)

A single name is searched in two passes, as PROJ's `searchCRS` does:

1. Exact: the name or an alias equals the text, ignoring case.
2. Approximate, only when the text is longer than four bytes. A CRS matches
   when its name contains the text (ignoring case), or when its canonical name
   contains the canonical text. The match is kept only when its name is shorter
   than twice the text.

Several matches keep the 2D geographic CRS, then the 3D one, then the single
candidate whose name is equivalent to the text. Anything else is an error.
`A + B` is split, and each side is searched with the approximate pass.

### Canonical name (`canonicalizeName`)

`canonical_search_name` lowercases, drops `_ - / ( ) . & ,`, drops a ` + `
separator, and reads `19dd` as `dd`. So `ITRF1993` canonicalises to `itrf93`,
which matches the CRS named `ITRF93`.

### Authority code (`cs2cs`)

A side with no EPSG identity, such as `IAU_2015:49935`, becomes the PROJ string
of its CRS (`export_to_proj_string`) and goes through the PROJ-string path of
`cs2cs`. Before, it was passed to `Proj::new_known_crs`, which reads only
`EPSG:` codes and failed with `unknown CRS`.

## API mapping

| PROJ 9.9.0 | Rust |
|---|---|
| `searchCRS` (exact, then approximate) | `crs_named_one` with `NameMatch::Exact`, then `NameMatch::Approximate` |
| `searchObject` pick: 2D, then 3D, then `isEquivalentName` | `crs_named_one` (unchanged) |
| `ret->nameStr().size() < 2 * text.size()` | `Option::filter` in `crs_from_name` |
| `text.size() <= 4` → two passes | `text.len() <= 4` in `crs_from_name` |
| `Identifier::canonicalizeName` | `canonical_search_name` |
| `createObjectsFromNameEx` approximate `ci_find` | `approximate_name` |
| `exportToPROJString` of the CRS side | `canonical_side` → `Crs::export_to_proj_string` |

## Test mapping

| Case | Where it is covered |
|---|---|
| `test_cs2cs_various.yaml#88` (`ITRF2000 ITRF1993`) | `cli_yaml`, and `itrf1993_matches_itrf93_through_the_19dd_rule` |
| `test_cs2cs_various.yaml#86` (`+proj=longlat +R=3376200 +to IAU_2015:49935`) | `cli_yaml`, and `iau_authority_code_exports_its_proj_string` |
| `test_projinfo.yaml#109` (`projinfo EGM2008` gives the datum) | `cli_yaml` (guards the length rule) and `approximate_name_longer_than_twice_the_text_is_rejected` |

## Deliberate differences

- The approximate pass does not cap its candidates at 10 as PROJ does
  (`NameMatch::Approximate` ignores the limit in `name_rows`). Its order is
  this port's sort, not PROJ's SQL `ORDER BY`.
- The `(deprecated)` suffix is not stripped before the approximate pass, so a
  deprecated name with that suffix is searched as typed.
- Accented letters and PROJ's `_IntlFeet` rule are not applied in
  `canonical_search_name`. The cases in this lot do not use them.

## Open cases in this lot

- `test_cs2cs_various.yaml#95` and `#96` (`--authority PROJ ITRF96 ITRF2014`,
  GH-2442) were already failing on `origin/main` and still fail. They are an
  operation-selection question about `--authority`, not a name lookup.
- The other open `cs2cs` cases are listed in `docs/UNPORTED-CLI.md`. The
  `--only-best`, `--accuracy`, and `--no-ballpark` cases belong to another lot.

## Verification

Run with the reference binaries on `PATH`:

```
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --test cli_yaml
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
cargo clippy --all-targets -- -D warnings
```

The cs2cs count in `docs/UNPORTED-CLI.md` went from 31 on `origin/main` to 29.
