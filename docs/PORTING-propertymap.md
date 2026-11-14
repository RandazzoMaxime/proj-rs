# Porting sheet: propertymap

PROJ 9.9.0 `util::PropertyMap` (`include/proj/util.hpp`, `src/iso19111/util.cpp`), the keys of
`IdentifiedObject::setProperties` (`src/iso19111/common.cpp`), and
`OperationParameter::create(properties)` (`src/iso19111/operation/singleoperation.cpp`).
Rust code: `src/iso19111/property.rs`, `OperationParameter::create_from_properties` in
`src/iso19111/model.rs`. Rust tests: `tests/unit_common_alias.rs`.

This lot closes the two `test_common.cpp` cases that were open in `docs/UNPORTED-UNIT.md`:
`common.identifiedobject_alias_invalid_type` and `common.identifiedobject_alias_array_of_invalid_type`.
The ledger itself is not edited by this lot (see "Ledger" below).

## Why an untyped map

The typed API (`OperationParameter::create(name)`, aliases as `Vec<String>`) cannot hold a
`Citation` under the alias key, so the wrong-type cases cannot be written against it. The
maintainer chose a dynamic `PropertyMap` for this lot. It is a closed enum of value types, not
a bag of `Any`.

## Function map

| PROJ 9.9.0 | Rust | Notes |
|---|---|---|
| `PropertyMap::set(key, string / int / bool / BaseObject / vector<string>)` | `PropertyMap::set(&str, PropertyValue)` | One entry point; the `PropertyValue` variant gives the type. Setting a key again replaces the value in place. |
| `PropertyMap::get(key)` | `PropertyMap::get(&str) -> Option<&PropertyValue>` | |
| `PropertyMap::getStringValue(key, out)` | not exposed | Its throw is reproduced inside `set_identified_properties`. |
| `PropertyMap::unset(key)` | not ported | No caller in this lot needs it. |
| `BoxedValue` (string, boolean, integer) | `PropertyValue::Text`, `Bool`, `Integer` | |
| `ArrayOfBaseObject` | `PropertyValue::Array(Vec<PropertyValue>)` | |
| `metadata::Citation` as a `BaseObject` | `PropertyValue::Citation(Citation)` | The existing `Citation` type. |
| `util::Identifier` as a `BaseObject` | `PropertyValue::Identifier(Identifier)` | The existing `Identifier` type. |
| `IdentifiedObject::setProperties` | `set_identified_properties` (`property.rs`) | Same key order: name, identifiers, aliases, remarks, deprecated. |
| `IdentifiedObject::Private::setName` | the `NAME_KEY` branch | |
| `IdentifiedObject::Private::setIdentifiers` | the `IDENTIFIERS_KEY` branch | |
| `IdentifiedObject::Private::setAliases` | the `ALIAS_KEY` branch | |
| `OperationParameter::create(properties)` | `OperationParameter::create_from_properties(&PropertyMap)` | Named differently because `OperationParameter::create(name)` exists. |
| `util::InvalidValueTypeException` | `InvalidValueTypeError` (`#[non_exhaustive]`, `key()`) | Same message: `Invalid value type for <key>`. |

`PropertyValue` and `InvalidValueTypeError` are both `#[non_exhaustive]`, so a later variant (a
`GenericName` value, for instance) or field does not break callers. No public enum that already
existed gained a variant.

## Keys

Constants in `iso19111` (`ALIAS_KEY`, `NAME_KEY`, `IDENTIFIERS_KEY`, `REMARKS_KEY`,
`DEPRECATED_KEY`). Their strings are PROJ's: `"alias"`, `"name"`, `"identifiers"`, `"remarks"`,
`"deprecated"` (`src/iso19111/static.cpp`).

Read by the port, with the value types PROJ accepts:

| Key | Accepted | Refused |
|---|---|---|
| `alias` | a text (one alias), or an array of texts (the array may be empty) | any other value, and an array holding any other value |
| `name` | a text | any other value, including an `Identifier` (see deviation 1) |
| `identifiers` | an `Identifier`, or an array of `Identifier` | a text, a `Citation`, an array holding anything but an `Identifier` |
| `remarks` | a text | any other value |
| `deprecated` | a boolean | any other value |

Not read by the port:

- `Identifier::CODE_KEY`, `CODESPACE_KEY`, `VERSION_KEY`: PROJ's `setIdentifiers` falls back to
  building one identifier from these keys when `identifiers` is absent. Not ported in this lot.
- `ObjectUsage` keys (`DOMAIN_OF_VALIDITY_KEY`, and the scope and usage keys): PROJ reads them in
  `ObjectUsage::setProperties`. `OperationParameter` is not an `ObjectUsage`, so they are not read
  for it.
- `GenericName` values under `alias` or `name`: the crate has no `GenericName` type and no
  `NameFactory`.

Any other key is ignored, as PROJ ignores it for an `OperationParameter`.

## Measured against PROJ 9.9.0

Measured with a C++ program linked to `target/proj-reference/build-net/lib/libproj.so` that calls
`OperationParameter::create(properties)` and prints the result. Every row is a PROJ answer, not
an answer taken from this crate.

| PropertyMap | PROJ 9.9.0 |
|---|---|
| `alias` = `"alias1"` | 1 alias, `alias1` |
| `alias` = `["alias1", "alias2"]` | 2 aliases, `alias1`, `alias2` |
| `alias` = `[]` | 0 aliases |
| `alias` = `Citation("unexpected type")` | `InvalidValueTypeException: Invalid value type for alias` |
| `alias` = `[Citation("unexpected type")]` | `InvalidValueTypeException: Invalid value type for alias` |
| `alias` = `["ok", true]` | `InvalidValueTypeException` |
| `alias` = `true`, `alias` = `3` | `InvalidValueTypeException` |
| `name` = `"the name"`, `remarks` = `"rem"`, `deprecated` = `true` | name `the name`, remarks `rem`, deprecated |
| `deprecated` = `false` | not deprecated |
| `name` = `Citation("x")` | `InvalidValueTypeException: Invalid value type for name` |
| `remarks` = `true` | `InvalidValueTypeException: Invalid value type for remarks` |
| `deprecated` = `"yes"` | `InvalidValueTypeException: Invalid value type for deprecated` |
| `identifiers` = `"string not allowed"`, `identifiers` = `Citation` | `InvalidValueTypeException: Invalid value type for identifiers` |
| `identifiers` = `Identifier::create("single_code")` | 1 identifier |
| `name` = `Identifier::create("name_code")` | accepted, the identifier is kept as the name object; `nameStr()` is empty |
| `alias` set twice, `first` then `second` | 1 alias, `second` |

The two `test_common.cpp` cases assert exactly the `Citation` refusals in the first two rows of
the refusal list above. The positive alias cases come from `TEST(common,
identifiedobject_alias_array_of_string)`, and the identifier array from `TEST(common,
identifiedobject_identifier_array_of_identifier)`.

## Deviations

1. **`name` given an `Identifier`.** PROJ accepts it and keeps the identifier as the name object
   (measured: `nameStr()` is empty). This crate's names are text, so the port
   refuses an `Identifier` under `name` with `InvalidValueTypeError`. This is a difference: a
   map that PROJ reads is refused here. It is kept as a refusal rather than guessed.
2. **`Integer` is `i64`.** PROJ's `set(key, int)` stores a C++ `int`. No key the port reads takes
   an integer, so the width is not observable.
3. **Error type.** `InvalidValueTypeError` is returned, not thrown. The partial object that PROJ
   would discard on a throw is never built here.

## Tests

`tests/unit_common_alias.rs`:

- `identifiedobject_alias_invalid_type`: `TEST(common, identifiedobject_alias_invalid_type)`.
- `identifiedobject_alias_array_of_invalid_type`: `TEST(common, identifiedobject_alias_array_of_invalid_type)`.
- `identifiedobject_alias_array_of_string_property_map`: the array form of
  `TEST(common, identifiedobject_alias_array_of_string)`, with the aliases as a `PropertyMap` value.
- `identifiedobject_alias_string_property_map`, `identifiedobject_alias_empty_array_property_map`,
  `identifiedobject_alias_set_twice_keeps_last_property_map`,
  `identifiedobject_alias_refuses_other_scalars_property_map`: the measured rows above.
- `identifiedobject_name_remarks_deprecated_property_map`,
  `identifiedobject_other_keys_refuse_wrong_type_property_map`,
  `identifiedobject_identifier_property_map`: the other keys, measured the same way.

Red step: before `property.rs` existed, `cargo test --test unit_common_alias` failed to compile
with 9 errors (`unresolved imports ... PropertyMap, PropertyValue, InvalidValueTypeError, ...` and
`no function or associated item named create_from_properties found for OperationParameter`).

## Ledger

`docs/UNPORTED-UNIT.md` still lists the two cases under `## test_common.cpp`. Removing those
lines and running `UPDATE_DOCS=1 cargo test --test docs_in_sync` is the maintainer's step
(CONTRIBUTING.md, step 7); it was not run in this lot, so `docs/UNIT-TRIAGE.md` and the counts in
`README.md` and `ROADMAP.md` are unchanged.

## Commands run

- `cargo test --test unit_common_alias`: red first (compile errors, see "Tests"), then 10 passed.
- `cargo fmt --check`: clean.
- `cargo +1.100.0 clippy --all-targets --all-features -- -D warnings`: clean.
- `cargo test --lib` and the `unit_core_*`, `test_io*` and `proj_unit` binaries: all passed.
- `cargo test --all-features --no-fail-fast`, with the reference `bin` first in `PATH`,
  `PROJ_DATA` set to `target/proj-reference/build/data`, `LD_LIBRARY_PATH` set to
  `target/proj-reference/build/lib`, `PROJ_CS2CS_REQUIRED=1` and
  `PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`: 256 test binaries ok, 2087 passed, 0 failed, 0 ignored.
- The PROJ probe: `g++ -std=c++17` with the include paths `src/include`, `src/src`,
  `build-net/src` and `build-net/include` of the PROJ 9.9.0 tree, linked to
  `build-net/lib/libproj.so`.
