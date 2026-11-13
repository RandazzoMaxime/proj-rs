//! The settings a PROJ context carries for its database and for the `init=`
//! rules of user input: PROJ's `PJ_CONTEXT` state for `proj_context_*`.
//!
//! A [`Context`] is a value. A new context takes the settings of the process
//! default (PROJ's `proj_context_create`), and a clone keeps its own copy
//! (`proj_context_clone`). The default is changed with the `set_default_*`
//! functions, which are PROJ's `proj_context_*(NULL, ...)` calls.
//!
//! The database opened from a context is a [`DatabaseContext`]. Auxiliary
//! databases are attached by [`DatabaseContext::create_with_auxiliary`]. A
//! named SQLite VFS is applied to the main database only, and it is refused
//! together with auxiliary databases.

use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

use crate::iso19111::{
    create_from_user_input_with_init_rules, Crs, DatabaseContext, FactoryError, InitRules,
};

/// Where the `proj.db` of a context comes from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DatabaseSource {
    /// The `proj.db` built into the crate.
    #[default]
    Embedded,
    /// A `proj.db` file, opened read-only.
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Settings {
    database: DatabaseSource,
    auxiliary: Vec<PathBuf>,
    sqlite_vfs: Option<String>,
    init_rules: InitRules,
}

impl Settings {
    const BUILT_IN: Self = Self {
        database: DatabaseSource::Embedded,
        auxiliary: Vec::new(),
        sqlite_vfs: None,
        init_rules: InitRules::Iso19111,
    };
}

/// The process default that [`Context::new`] copies.
static DEFAULT: RwLock<Settings> = RwLock::new(Settings::BUILT_IN);

fn default_settings() -> Settings {
    DEFAULT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

fn update_default(change: impl FnOnce(&mut Settings)) {
    change(&mut DEFAULT.write().unwrap_or_else(PoisonError::into_inner));
}

/// The database and user-input settings of one PROJ context.
///
/// ```
/// use proj_rs::{Context, DatabaseSource, InitRules};
///
/// let context = Context::builder()
///     .sqlite_vfs_name(None)
///     .init_rules(InitRules::Proj4)
///     .build();
/// assert_eq!(context.database_source(), &DatabaseSource::Embedded);
/// assert!(context.database().is_ok());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    settings: Settings,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// A context with the settings of the process default, as PROJ's
    /// `proj_context_create()`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            settings: default_settings(),
        }
    }

    /// A builder that starts from the process default.
    #[must_use]
    pub fn builder() -> ContextBuilder {
        ContextBuilder {
            settings: default_settings(),
        }
    }

    /// The database this context opens.
    #[must_use]
    pub fn database_source(&self) -> &DatabaseSource {
        &self.settings.database
    }

    /// The file of the database, or `None` for the embedded one.
    #[must_use]
    pub fn database_path(&self) -> Option<&Path> {
        match &self.settings.database {
            DatabaseSource::Embedded => None,
            DatabaseSource::File(path) => Some(path.as_path()),
        }
    }

    /// The auxiliary databases, in the order they were given.
    #[must_use]
    pub fn auxiliary_database_paths(&self) -> &[PathBuf] {
        &self.settings.auxiliary
    }

    /// The SQLite VFS the database is opened through, or `None` for SQLite's
    /// default.
    #[must_use]
    pub fn sqlite_vfs_name(&self) -> Option<&str> {
        self.settings.sqlite_vfs.as_deref()
    }

    /// The `init=` rules of user input read through this context.
    #[must_use]
    pub fn init_rules(&self) -> InitRules {
        self.settings.init_rules
    }

    /// Change the database this context opens.
    pub fn set_database(&mut self, source: DatabaseSource) {
        self.settings.database = source;
    }

    /// Change the auxiliary databases of this context.
    pub fn set_auxiliary_database_paths(&mut self, paths: Vec<PathBuf>) {
        self.settings.auxiliary = paths;
    }

    /// Change the SQLite VFS; `None` restores SQLite's default.
    pub fn set_sqlite_vfs_name(&mut self, name: Option<&str>) {
        self.settings.sqlite_vfs = name.map(str::to_owned);
    }

    /// Change the `init=` rules of user input read through this context.
    pub fn set_init_rules(&mut self, rules: InitRules) {
        self.settings.init_rules = rules;
    }

    /// The database source of the process default.
    #[must_use]
    pub fn default_database() -> DatabaseSource {
        default_settings().database
    }

    /// The `init=` rules of the process default.
    #[must_use]
    pub fn default_init_rules() -> InitRules {
        default_settings().init_rules
    }

    /// Change the database of the process default, as PROJ's
    /// `proj_context_set_database_path(NULL, ...)`. Contexts created afterwards
    /// take it; existing contexts keep theirs.
    pub fn set_default_database(source: DatabaseSource) {
        update_default(|settings| settings.database = source);
    }

    /// Change the SQLite VFS of the process default; `None` restores SQLite's
    /// default. Contexts created afterwards take it.
    pub fn set_default_sqlite_vfs_name(name: Option<&str>) {
        update_default(|settings| settings.sqlite_vfs = name.map(str::to_owned));
    }

    /// Change the `init=` rules of the process default. Contexts created
    /// afterwards take them.
    pub fn set_default_init_rules(rules: InitRules) {
        update_default(|settings| settings.init_rules = rules);
    }

    /// Open the database of this context.
    ///
    /// # Errors
    ///
    /// The file cannot be opened, including through an unknown SQLite VFS (the
    /// message is PROJ's `Open of <path> failed`). The layout version is not
    /// the one this factory reads. A SQLite VFS is named together with
    /// auxiliary databases: this is not supported yet.
    pub fn database(&self) -> Result<DatabaseContext, FactoryError> {
        match self.sqlite_vfs_name() {
            None => DatabaseContext::create_with_auxiliary(
                self.database_path(),
                &self.settings.auxiliary,
            ),
            Some(vfs) if self.settings.auxiliary.is_empty() => {
                DatabaseContext::open_with_vfs(self.database_path(), vfs)
            }
            Some(_) => Err(FactoryError::Invalid(
                "auxiliary databases with a SQLite VFS are not supported yet".to_owned(),
            )),
        }
    }

    /// `createFromUserInput` with the `init=` rules of this context.
    ///
    /// # Errors
    ///
    /// As `create_from_user_input`, or when [`Context::database`] fails.
    pub fn create_from_user_input(&self, text: &str) -> Result<Crs, FactoryError> {
        let db = self.database()?;
        create_from_user_input_with_init_rules(text, &db, self.settings.init_rules)
    }
}

/// Builds a [`Context`] from the process default and the settings given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextBuilder {
    settings: Settings,
}

impl ContextBuilder {
    /// The database to open.
    #[must_use]
    pub fn database(mut self, source: DatabaseSource) -> Self {
        self.settings.database = source;
        self
    }

    /// Add an auxiliary database, after the ones already given.
    #[must_use]
    pub fn auxiliary_database(mut self, path: impl Into<PathBuf>) -> Self {
        self.settings.auxiliary.push(path.into());
        self
    }

    /// The SQLite VFS to open the database through; `None` for SQLite's default.
    #[must_use]
    pub fn sqlite_vfs_name(mut self, name: Option<&str>) -> Self {
        self.settings.sqlite_vfs = name.map(str::to_owned);
        self
    }

    /// The `init=` rules of user input.
    #[must_use]
    pub fn init_rules(mut self, rules: InitRules) -> Self {
        self.settings.init_rules = rules;
        self
    }

    /// The context.
    #[must_use]
    pub fn build(self) -> Context {
        Context {
            settings: self.settings,
        }
    }
}
