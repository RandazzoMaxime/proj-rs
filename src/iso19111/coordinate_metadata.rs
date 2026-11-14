//! `coordinates::CoordinateMetadata`: a CRS and the coordinate epoch of the data.
//!
//! A dynamic CRS needs an epoch, a static one does not. PROJ's `CoordinateMetadata::create`
//! (`src/iso19111/coordinates.cpp`, 9.9.0) also accepts an epoch on a static CRS whose
//! geodetic CRS has point motion operations.

use super::crs_ops::Wgs84Dynamic;
use super::factory::{AlternativeGridNames, AuthorityFactory, DatabaseContext};
use super::model::{Crs, FactoryError, GeodeticCRS};

/// A CRS with the decimal year of its coordinates, when it has one.
#[derive(Clone, Debug)]
pub struct CoordinateMetadata {
    crs: Crs,
    epoch: Option<f64>,
}

impl CoordinateMetadata {
    /// `CoordinateMetadata::create(crs, epoch, dbContext)`: `crs` with `epoch`, a decimal year.
    ///
    /// # Errors
    ///
    /// `crs` is static and has no point motion operation in `db`.
    pub fn create(crs: &Crs, epoch: f64, db: &DatabaseContext) -> Result<Self, FactoryError> {
        Self::checked(crs, epoch, Some(db))
    }

    /// `CoordinateMetadata::create(crs, epoch)`, the overload with no database context. A
    /// static CRS is then refused, since no point motion operation can be looked up. A dynamic
    /// CRS, and WGS 84 under its dynamic reading, are accepted.
    ///
    /// # Errors
    ///
    /// `crs` is static.
    pub fn create_without_database(crs: &Crs, epoch: f64) -> Result<Self, FactoryError> {
        Self::checked(crs, epoch, None)
    }

    fn checked(crs: &Crs, epoch: f64, db: Option<&DatabaseContext>) -> Result<Self, FactoryError> {
        if !crs.is_dynamic_with(Wgs84Dynamic::TreatAsDynamic) {
            let point_motion = match db {
                Some(db) => has_point_motion(crs, db)?,
                None => false,
            };
            if !point_motion {
                return Err(FactoryError::Invalid(
                    "Coordinate epoch should not be provided for a static CRS".to_owned(),
                ));
            }
        }
        Ok(Self {
            crs: crs.clone(),
            epoch: Some(epoch),
        })
    }

    /// `CoordinateMetadata::promoteTo3D(newName, dbContext)`: this object with its CRS promoted
    /// to 3D (`Crs::promote_to_3d`), and the same epoch. A CRS already 3D is kept. The name is the
    /// CRS's own, which is what PROJ does for an empty `newName`.
    #[must_use]
    pub fn promote_to_3d(&self, db: Option<&DatabaseContext>) -> Self {
        Self {
            crs: self.crs.promote_to_3d(db),
            epoch: self.epoch,
        }
    }

    /// `CoordinateMetadata::create(crs)`: `crs` with no epoch. Only a static CRS takes none.
    ///
    /// # Errors
    ///
    /// `crs` is dynamic.
    pub fn create_without_epoch(crs: &Crs) -> Result<Self, FactoryError> {
        if crs.is_dynamic_with(Wgs84Dynamic::Static) {
            return Err(FactoryError::Invalid(
                "Coordinate epoch should be provided for a dynamic CRS".to_owned(),
            ));
        }
        Ok(Self {
            crs: crs.clone(),
            epoch: None,
        })
    }

    /// The CRS.
    #[must_use]
    pub fn crs(&self) -> &Crs {
        &self.crs
    }

    /// The coordinate epoch as a decimal year, when the object has one.
    #[must_use]
    pub fn coordinate_epoch(&self) -> Option<f64> {
        self.epoch
    }
}

/// The coordinate metadata a `urn:ogc:def:coordinateMetadata:AUTH:VERSION:CODE` text names, as
/// `createFromURNPart` reads it (PROJ's extension of the OGC URN syntax). `None` when the text
/// is not such a URN.
///
/// # Errors
///
/// The text is such a URN, but its authority, version or code is not in `db`.
pub fn coordinate_metadata_from_urn(
    text: &str,
    db: &DatabaseContext,
) -> Result<Option<CoordinateMetadata>, FactoryError> {
    let Some(rest) = text.strip_prefix("urn:ogc:def:coordinateMetadata:") else {
        return Ok(None);
    };
    let parts: Vec<&str> = rest.split(':').collect();
    let [authority, version, code] = parts[..] else {
        return Err(FactoryError::Invalid(format!(
            "{text} is not a coordinateMetadata URN"
        )));
    };
    let authority = if version.is_empty() {
        authority.to_owned()
    } else {
        db.versioned_authority(authority, version).ok_or_else(|| {
            FactoryError::Invalid(format!("unknown version {version} of {authority}"))
        })?
    };
    AuthorityFactory::create(db, authority)
        .create_coordinate_metadata(code)
        .map(Some)
}

/// Whether the geodetic CRS of `crs` has a point motion operation in `db`.
fn has_point_motion(crs: &Crs, db: &DatabaseContext) -> Result<bool, FactoryError> {
    let Some(geodetic) = extract_geodetic(crs) else {
        return Ok(false);
    };
    let operations = AuthorityFactory::create(db, "")
        .get_point_motion_operations_for(&geodetic, AlternativeGridNames::Official)?;
    Ok(!operations.is_empty())
}

/// `CRS::extractGeodeticCRS`: the geodetic CRS itself, the base of a projected CRS, or the first
/// component of a compound CRS that has one. Bound and other kinds yield `None`.
fn extract_geodetic(crs: &Crs) -> Option<GeodeticCRS> {
    match crs {
        Crs::Geographic(geographic) => Some(GeodeticCRS::Geographic(geographic.clone())),
        Crs::Geodetic(geodetic) => Some(geodetic.clone()),
        Crs::Projected(projected) => Some(projected.base_crs().clone()),
        Crs::Compound(compound) => compound.components().iter().find_map(extract_geodetic),
        _ => None,
    }
}
