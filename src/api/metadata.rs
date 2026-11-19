//! The versions the database of a context says it has, and the version of PROJ this crate ports.

use std::fmt;

use crate::api::{catalog::database_error, Context, Error};
use crate::version;

/// What the `metadata` table of the database says about its own versions. Made by
/// [`Context::database_metadata`].
///
/// A field is `None` when its key is absent from the table. The embedded database of this crate
/// has all of them.
///
/// ```
/// use proj_rs::Context;
///
/// let meta = Context::default().database_metadata().unwrap();
/// assert_eq!(meta.epsg_version.as_deref(), Some("v13.102"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseMetadata {
    /// The layout of the tables, `DATABASE.LAYOUT.VERSION.MAJOR` then `.MINOR`: `(1, 7)`.
    pub layout_version: Option<(u32, u32)>,
    /// The version of the EPSG dataset, `EPSG.VERSION`: `v13.102`.
    pub epsg_version: Option<String>,
    /// The PROJ version that wrote the database, `PROJ.VERSION`: `9.9.0`.
    pub proj_version: Option<String>,
    /// The version of the PROJ data files, `PROJ_DATA.VERSION`: `1.25`.
    pub proj_data_version: Option<String>,
}

/// A version of PROJ: `major.minor.patch`. Versions compare field by field.
///
/// ```
/// use proj_rs::proj_version;
///
/// let version = proj_version();
/// assert_eq!(version.to_string(), "9.9.0");
/// assert!(version.major == 9 && version.minor >= 9);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ProjVersion {
    /// The major number.
    pub major: u32,
    /// The minor number.
    pub minor: u32,
    /// The patch number.
    pub patch: u32,
}

impl fmt::Display for ProjVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The version of PROJ this crate ports: `9.9.0`.
///
/// ```
/// use proj_rs::proj_version;
///
/// assert_eq!(proj_version().to_string(), "9.9.0");
/// ```
#[must_use]
pub const fn proj_version() -> ProjVersion {
    ProjVersion {
        major: version::MAJOR,
        minor: version::MINOR,
        patch: version::PATCH,
    }
}

impl Context {
    /// The versions the database of this context says it has.
    ///
    /// A layout number that is not a number is read as absent.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the database cannot be opened or queried.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let meta = Context::default().database_metadata().unwrap();
    /// assert_eq!(meta.proj_version.as_deref(), Some("9.9.0"));
    /// ```
    pub fn database_metadata(&self) -> Result<DatabaseMetadata, Error> {
        let db = self.legacy.database().map_err(database_error)?;
        let value = |key: &str| db.metadata(key).map_err(database_error);
        let number = |key: &str| -> Result<Option<u32>, Error> {
            Ok(value(key)?.and_then(|text| text.parse().ok()))
        };
        Ok(DatabaseMetadata {
            layout_version: number("DATABASE.LAYOUT.VERSION.MAJOR")?
                .zip(number("DATABASE.LAYOUT.VERSION.MINOR")?),
            epsg_version: value("EPSG.VERSION")?,
            proj_version: value("PROJ.VERSION")?,
            proj_data_version: value("PROJ_DATA.VERSION")?,
        })
    }
}
