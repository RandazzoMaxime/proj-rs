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
        if !crs.is_dynamic_with(Wgs84Dynamic::TreatAsDynamic) && !has_point_motion(crs, db)? {
            return Err(FactoryError::Invalid(
                "Coordinate epoch should not be provided for a static CRS".to_owned(),
            ));
        }
        Ok(Self {
            crs: crs.clone(),
            epoch: Some(epoch),
        })
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
