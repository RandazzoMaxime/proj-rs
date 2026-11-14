//! `util::PropertyMap` of PROJ 9.9.0 (`include/proj/util.hpp`, `src/iso19111/util.cpp`): the untyped
//! map that PROJ's `create(properties)` factories read, and the keys of `IdentifiedObject`
//! (`src/iso19111/common.cpp`, `IdentifiedObject::setProperties`) that this port reads from it.

use std::fmt;

use super::metadata::Citation;
use super::model::{Identified, Identifier};

/// `IdentifiedObject::NAME_KEY`.
pub const NAME_KEY: &str = "name";
/// `IdentifiedObject::IDENTIFIERS_KEY`.
pub const IDENTIFIERS_KEY: &str = "identifiers";
/// `IdentifiedObject::ALIAS_KEY`.
pub const ALIAS_KEY: &str = "alias";
/// `IdentifiedObject::REMARKS_KEY`.
pub const REMARKS_KEY: &str = "remarks";
/// `IdentifiedObject::DEPRECATED_KEY`.
pub const DEPRECATED_KEY: &str = "deprecated";

/// A value of a [`PropertyMap`]: the `BaseObject` that PROJ stores under a key.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PropertyValue {
    /// A string (`BoxedValue` of type string).
    Text(String),
    /// A boolean (`BoxedValue` of type boolean).
    Bool(bool),
    /// An integer (`BoxedValue` of type integer, `int` in PROJ).
    Integer(i64),
    /// A `metadata::Citation`.
    Citation(Citation),
    /// A `util::Identifier`, an authority code.
    Identifier(Identifier),
    /// An `ArrayOfBaseObject`: its items, in order.
    Array(Vec<PropertyValue>),
}

/// `util::PropertyMap`: values stored under string keys, read by PROJ's factories.
///
/// Setting a key that is already set replaces its value in place, as `PropertyMap::set` does.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropertyMap {
    entries: Vec<(String, PropertyValue)>,
}

impl PropertyMap {
    /// Empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `key` to `value`, replacing the value it had.
    pub fn set(&mut self, key: &str, value: PropertyValue) -> &mut Self {
        match self.entries.iter_mut().find(|(stored, _)| stored == key) {
            Some((_, slot)) => *slot = value,
            None => self.entries.push((key.to_owned(), value)),
        }
        self
    }

    /// The value stored under `key`, when there is one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&PropertyValue> {
        self.entries
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, value)| value)
    }
}

/// `util::InvalidValueTypeException`: a property holds a value of a type that PROJ does not read
/// for its key.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InvalidValueTypeError {
    key: String,
}

impl InvalidValueTypeError {
    fn for_key(key: &str) -> Self {
        Self {
            key: key.to_owned(),
        }
    }

    /// The key whose value has the wrong type, such as [`ALIAS_KEY`].
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for InvalidValueTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid value type for {}", self.key)
    }
}

impl std::error::Error for InvalidValueTypeError {}

/// `IdentifiedObject::setProperties`: read the keys of `properties` that PROJ reads for an
/// identified object, in PROJ's order (name, identifiers, aliases, remarks, deprecated). A key
/// that is absent leaves the field as it is. A key whose value has a type PROJ refuses for it is
/// an error: the reading stops there, as PROJ's throw does.
pub(crate) fn set_identified_properties(
    identified: &mut Identified,
    properties: &PropertyMap,
) -> Result<(), InvalidValueTypeError> {
    if let Some(value) = properties.get(NAME_KEY) {
        // PROJ also takes an `Identifier` as the name object. This crate's names are text, so only
        // the text form is read (see docs/PORTING-propertymap.md).
        identified.name = match value {
            PropertyValue::Text(name) => name.clone(),
            _ => return Err(InvalidValueTypeError::for_key(NAME_KEY)),
        };
    }
    if let Some(value) = properties.get(IDENTIFIERS_KEY) {
        identified.identifiers = match value {
            PropertyValue::Identifier(identifier) => vec![identifier.clone()],
            PropertyValue::Array(items) => items
                .iter()
                .map(|item| match item {
                    PropertyValue::Identifier(identifier) => Ok(identifier.clone()),
                    _ => Err(InvalidValueTypeError::for_key(IDENTIFIERS_KEY)),
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(InvalidValueTypeError::for_key(IDENTIFIERS_KEY)),
        };
    }
    if let Some(value) = properties.get(ALIAS_KEY) {
        // PROJ's `setAliases`: a string is one alias, an array holds strings, and anything else
        // (a `Citation` included) is refused.
        identified.aliases = match value {
            PropertyValue::Text(alias) => vec![alias.clone()],
            PropertyValue::Array(items) => items
                .iter()
                .map(|item| match item {
                    PropertyValue::Text(alias) => Ok(alias.clone()),
                    _ => Err(InvalidValueTypeError::for_key(ALIAS_KEY)),
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(InvalidValueTypeError::for_key(ALIAS_KEY)),
        };
    }
    if let Some(value) = properties.get(REMARKS_KEY) {
        identified.remarks = match value {
            PropertyValue::Text(remarks) => remarks.clone(),
            _ => return Err(InvalidValueTypeError::for_key(REMARKS_KEY)),
        };
    }
    if let Some(value) = properties.get(DEPRECATED_KEY) {
        identified.deprecated = match value {
            PropertyValue::Bool(deprecated) => *deprecated,
            _ => return Err(InvalidValueTypeError::for_key(DEPRECATED_KEY)),
        };
    }
    Ok(())
}
