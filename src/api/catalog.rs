//! The catalogue of the CRS of the database: one entry per CRS and per area of use, filtered by
//! kind, authority and deprecation.
//!
//! It reads the rows of `AuthorityFactory::get_crs_info_list`, the query `projinfo --list-crs`
//! lists, and keeps the ones the filter asks for. No type of the ISO 19111 model is exposed.

use std::collections::BTreeSet;

use crate::api::{BBox, Context, Error};
use crate::iso19111::{AuthorityFactory, CrsInfo};

/// The kind of a CRS of the database, as `projinfo --list-crs` filters it.
///
/// It is not [`CrsKind`](crate::CrsKind): that one describes a [`Crs`](crate::Crs) that was
/// read, and does not tell a two-dimensional geographic CRS from a three-dimensional one.
///
/// ```
/// use proj_rs::{CatalogKind, Context, CrsFilter};
///
/// let entries = Context::default()
///     .crs_catalog()
///     .list(&CrsFilter::new().kinds([CatalogKind::Vertical]).authority("EPSG"))
///     .unwrap();
/// assert!(entries.iter().any(|entry| entry.code == "5773"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CatalogKind {
    /// Latitude and longitude on an ellipsoid (`geographic 2D`).
    Geographic2D,
    /// Latitude, longitude and height on an ellipsoid (`geographic 3D`).
    Geographic3D,
    /// Cartesian axes from the centre of the Earth (`geocentric`).
    Geocentric,
    /// A map projection of a geographic CRS (`projected`).
    Projected,
    /// A map projection derived from another projected CRS (`derived projected`).
    DerivedProjected,
    /// A height or a depth (`vertical`).
    Vertical,
    /// A horizontal CRS and a vertical one (`compound`).
    Compound,
    /// A local system with no geodetic datum (`engineering`).
    Engineering,
    /// Any other type of the database, not named above.
    Other,
}

impl CatalogKind {
    fn from_type_name(name: &str) -> Self {
        match name {
            "geographic 2D" => Self::Geographic2D,
            "geographic 3D" => Self::Geographic3D,
            "geocentric" => Self::Geocentric,
            "projected" => Self::Projected,
            "derived projected" => Self::DerivedProjected,
            "vertical" => Self::Vertical,
            "compound" => Self::Compound,
            "engineering" => Self::Engineering,
            _ => Self::Other,
        }
    }
}

/// Whether a listing keeps the deprecated CRS of the database.
///
/// ```
/// use proj_rs::DeprecatedCrs;
///
/// assert_eq!(DeprecatedCrs::default(), DeprecatedCrs::Exclude);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DeprecatedCrs {
    /// Deprecated CRS are left out. The default.
    #[default]
    Exclude,
    /// Deprecated CRS are listed, with [`CrsEntry::deprecated`] set.
    Include,
}

/// Which CRS of the database a [`CrsCatalog::list`] call returns.
///
/// The default filter lists every kind of every authority, without the deprecated CRS. Each
/// setting narrows it. A filter is a value: build it, then pass it by reference.
///
/// ```
/// use proj_rs::{CatalogKind, Context, CrsFilter, DeprecatedCrs};
///
/// let filter = CrsFilter::new()
///     .kinds([CatalogKind::Geographic2D, CatalogKind::Projected])
///     .deprecated(DeprecatedCrs::Exclude)
///     .authority("EPSG");
/// let entries = Context::default().crs_catalog().list(&filter).unwrap();
/// assert!(entries.iter().any(|entry| entry.code == "4326"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrsFilter {
    kinds: BTreeSet<CatalogKind>,
    deprecated: DeprecatedCrs,
    authority: Option<String>,
}

impl CrsFilter {
    /// The filter that lists everything but the deprecated CRS.
    ///
    /// ```
    /// use proj_rs::CrsFilter;
    ///
    /// assert_eq!(CrsFilter::new(), CrsFilter::default());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep only these kinds. An empty set keeps every kind, as the default does.
    ///
    /// ```
    /// use proj_rs::{CatalogKind, Context, CrsFilter};
    ///
    /// let entries = Context::default()
    ///     .crs_catalog()
    ///     .list(&CrsFilter::new().kinds([CatalogKind::Vertical]).authority("EPSG"))
    ///     .unwrap();
    /// assert!(entries.iter().all(|entry| entry.kind == CatalogKind::Vertical));
    /// ```
    #[must_use]
    pub fn kinds(mut self, kinds: impl IntoIterator<Item = CatalogKind>) -> Self {
        self.kinds = kinds.into_iter().collect();
        self
    }

    /// Keep or leave out the deprecated CRS.
    ///
    /// ```
    /// use proj_rs::{Context, CrsFilter, DeprecatedCrs};
    ///
    /// let entries = Context::default()
    ///     .crs_catalog()
    ///     .list(&CrsFilter::new().deprecated(DeprecatedCrs::Include).authority("EPSG"))
    ///     .unwrap();
    /// assert!(entries.iter().any(|entry| entry.deprecated));
    /// ```
    #[must_use]
    pub fn deprecated(mut self, deprecated: DeprecatedCrs) -> Self {
        self.deprecated = deprecated;
        self
    }

    /// Keep only the CRS of one authority, `EPSG` for `EPSG:4326`. An empty name keeps every
    /// authority, as no call to this method does.
    ///
    /// ```
    /// use proj_rs::{Context, CrsFilter};
    ///
    /// let entries = Context::default()
    ///     .crs_catalog()
    ///     .list(&CrsFilter::new().authority("NO-SUCH-AUTHORITY"))
    ///     .unwrap();
    /// assert!(entries.is_empty());
    /// ```
    #[must_use]
    pub fn authority(mut self, authority: impl Into<String>) -> Self {
        let authority = authority.into();
        self.authority = (!authority.is_empty()).then_some(authority);
        self
    }

    fn keeps(&self, row: &CrsInfo) -> bool {
        (self.deprecated == DeprecatedCrs::Include || !row.deprecated)
            && (self.kinds.is_empty()
                || self
                    .kinds
                    .contains(&CatalogKind::from_type_name(&row.type_name)))
    }
}

/// One row of [`CrsCatalog::list`]: a CRS, and one of its areas of use.
///
/// A CRS with several areas of use has one entry for each, as `projinfo --list-crs` prints it.
/// The entries are in the order of the database: by authority, then by code.
///
/// ```
/// use proj_rs::{CatalogKind, Context, CrsFilter};
///
/// let entries = Context::default()
///     .crs_catalog()
///     .list(&CrsFilter::new().kinds([CatalogKind::Geographic2D]).authority("EPSG"))
///     .unwrap();
/// let wgs84 = entries.iter().find(|entry| entry.code == "4326").unwrap();
/// assert_eq!(wgs84.name, "WGS 84");
/// // The listing gives the database's own description; `Crs::area_name` gives "World.".
/// assert!(wgs84.area_name.as_deref().unwrap().starts_with("World: "));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CrsEntry {
    /// The authority of the CRS: `EPSG` for `EPSG:4326`.
    pub authority: String,
    /// The code of the CRS in its authority: `4326` for `EPSG:4326`.
    pub code: String,
    /// The name of the CRS.
    pub name: String,
    /// The kind of the CRS.
    pub kind: CatalogKind,
    /// Whether the CRS is deprecated.
    pub deprecated: bool,
    /// The name of the area of use of this entry, `None` when the database gives none.
    pub area_name: Option<String>,
    /// The box of the area of use, in degrees, `None` when the database gives none.
    pub area_of_use: Option<BBox>,
}

/// The entry of one row of the engine's listing. Private: the row type belongs to `unstable`.
fn entry_from_row(row: CrsInfo) -> CrsEntry {
    let area_of_use = match (row.west_lon, row.south_lat, row.east_lon, row.north_lat) {
        (Some(west), Some(south), Some(east), Some(north)) => {
            Some(BBox::new(west, south, east, north))
        }
        _ => None,
    };
    CrsEntry {
        kind: CatalogKind::from_type_name(&row.type_name),
        area_name: (!row.area_name.is_empty()).then_some(row.area_name),
        authority: row.authority,
        code: row.code,
        name: row.name,
        deprecated: row.deprecated,
        area_of_use,
    }
}

/// The CRS of the database a [`Context`] opens. Made by [`Context::crs_catalog`].
///
/// ```
/// use proj_rs::{CatalogKind, Context, CrsFilter};
///
/// let catalog = Context::default().crs_catalog();
/// let projected = catalog
///     .list(&CrsFilter::new().kinds([CatalogKind::Projected]).authority("EPSG"))
///     .unwrap();
/// assert!(projected.iter().any(|entry| entry.code == "3857"));
/// ```
#[derive(Clone, Debug)]
pub struct CrsCatalog {
    context: Context,
}

impl CrsCatalog {
    /// The CRS that pass `filter`, one entry per area of use.
    ///
    /// # Errors
    ///
    /// [`Error::Database`] when the database cannot be opened or queried. An authority the
    /// database does not have is not an error: the list is empty.
    ///
    /// ```
    /// use proj_rs::{CatalogKind, Context, CrsFilter};
    ///
    /// let entries = Context::default()
    ///     .crs_catalog()
    ///     .list(&CrsFilter::new().kinds([CatalogKind::Vertical]).authority("EPSG"))
    ///     .unwrap();
    /// assert_eq!(entries.len(), 308);
    /// ```
    pub fn list(&self, filter: &CrsFilter) -> Result<Vec<CrsEntry>, Error> {
        let db = self.context.legacy.database().map_err(database_error)?;
        let authority = filter.authority.clone().unwrap_or_default();
        let rows = AuthorityFactory::create(&db, authority)
            .get_crs_info_list()
            .map_err(database_error)?;
        Ok(rows
            .into_iter()
            .filter(|row| filter.keeps(row))
            .map(entry_from_row)
            .collect())
    }
}

impl Context {
    /// The catalogue of the CRS of the database this context opens.
    ///
    /// ```
    /// use proj_rs::Context;
    ///
    /// let _catalog = Context::default().crs_catalog();
    /// ```
    #[must_use]
    pub fn crs_catalog(&self) -> CrsCatalog {
        CrsCatalog {
            context: self.clone(),
        }
    }
}

pub(crate) fn database_error(error: impl std::fmt::Display) -> Error {
    Error::Database(error.to_string())
}
