//! The georust `proj` 0.31 [`Transform`] trait.
//!
//! Geometry crates implement this to feed coordinates through [`crate::Proj`]
//! without an intermediate point type.

use std::error::Error;
use std::fmt;

use crate::{Proj, ProjCreateError, ProjError};

/// Transform a geometry with a [`Proj`] pipeline.
pub trait Transform<T> {
    /// Owned geometry produced by [`Self::transformed`].
    type Output;

    /// Transform `self` in place.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when a coordinate has no image.
    fn transform(&mut self, proj: &Proj) -> Result<(), ProjError>;

    /// Return a transformed copy.
    ///
    /// # Errors
    ///
    /// [`ProjError`] when a coordinate has no image.
    fn transformed(&self, proj: &Proj) -> Result<Self::Output, ProjError>;

    /// Transform from `source_crs` to `target_crs` in place.
    ///
    /// Coordinate order is longitude, latitude or easting, northing.
    ///
    /// # Errors
    ///
    /// [`TransformError`] when the CRS pair is unknown or a coordinate has no image.
    fn transform_crs_to_crs(
        &mut self,
        source_crs: &str,
        target_crs: &str,
    ) -> Result<(), TransformError> {
        let proj = Proj::new_known_crs(source_crs, target_crs, None)?;
        Ok(self.transform(&proj)?)
    }

    /// Return a copy transformed from `source_crs` to `target_crs`.
    ///
    /// # Errors
    ///
    /// [`TransformError`] when the CRS pair is unknown or a coordinate has no image.
    fn transformed_crs_to_crs(
        &self,
        source_crs: &str,
        target_crs: &str,
    ) -> Result<Self::Output, TransformError> {
        let proj = Proj::new_known_crs(source_crs, target_crs, None)?;
        Ok(self.transformed(&proj)?)
    }
}

/// A CRS lookup failed, or the transform that followed it failed.
#[derive(Debug)]
pub enum TransformError {
    /// [`Proj::new_known_crs`] rejected the pair.
    ProjCreateError(ProjCreateError),
    /// A coordinate transform failed.
    ProjError(ProjError),
}

impl From<ProjError> for TransformError {
    fn from(error: ProjError) -> Self {
        Self::ProjError(error)
    }
}

impl From<ProjCreateError> for TransformError {
    fn from(error: ProjCreateError) -> Self {
        Self::ProjCreateError(error)
    }
}

impl fmt::Display for TransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjCreateError(error) => error.fmt(formatter),
            Self::ProjError(error) => error.fmt(formatter),
        }
    }
}

impl Error for TransformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProjCreateError(error) => Some(error),
            Self::ProjError(error) => Some(error),
        }
    }
}
