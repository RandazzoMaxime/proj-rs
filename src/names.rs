//! Names and name spaces, the part of PROJ's `util` model the operations use
//! (`NameFactory`, `LocalName`, `NameSpace` in `include/proj/util.hpp`).
//!
//! A [`LocalName`] is a name inside a [`NameSpace`]. The global name space is
//! the root; its own name is `"global"`, whose scope is the global name space.
//! A name space with a parent name is joined to it with its separator.

use std::fmt;

/// A name space: the global root, or a name space named by a [`LocalName`]
/// and joined to its parts with a separator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameSpace {
    name: Option<Box<LocalName>>,
    separator: String,
}

/// A name inside a name space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalName {
    name: String,
    scope: NameSpace,
}

impl NameSpace {
    /// The global name space, the root of every scope.
    #[must_use]
    pub fn global() -> Self {
        Self {
            name: None,
            separator: "/".to_owned(),
        }
    }

    /// A name space named `name`, whose parts are joined with `separator`.
    #[must_use]
    pub fn new(name: LocalName, separator: &str) -> Self {
        Self {
            name: Some(Box::new(name)),
            separator: separator.to_owned(),
        }
    }

    /// `true` for the global name space.
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.name.is_none()
    }

    /// The name of this name space. The global one is named `"global"`, in the
    /// global name space.
    #[must_use]
    pub fn name(&self) -> LocalName {
        match &self.name {
            Some(name) => (**name).clone(),
            None => LocalName {
                name: "global".to_owned(),
                scope: Self::global(),
            },
        }
    }

    /// The separator between the parts of a fully qualified name.
    #[must_use]
    pub fn separator(&self) -> &str {
        &self.separator
    }
}

impl LocalName {
    /// A name in `scope`, or in the global name space when `scope` is `None`.
    #[must_use]
    pub fn new(name: &str, scope: Option<&NameSpace>) -> Self {
        Self {
            name: name.to_owned(),
            scope: scope.cloned().unwrap_or_else(NameSpace::global),
        }
    }

    /// The name without its scope.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name space this name is in.
    #[must_use]
    pub fn scope(&self) -> &NameSpace {
        &self.scope
    }

    /// The name with every enclosing name space, joined by their separators.
    /// A name in the global name space is its own fully qualified name.
    #[must_use]
    pub fn to_fully_qualified_name(&self) -> LocalName {
        if self.scope.is_global() {
            return self.clone();
        }
        let parent = self.scope.name().to_fully_qualified_name();
        let joined = format!("{}{}{}", parent, self.scope.separator(), self.name);
        LocalName {
            name: joined,
            scope: NameSpace::global(),
        }
    }
}

impl fmt::Display for LocalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
