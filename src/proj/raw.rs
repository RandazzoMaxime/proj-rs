//! Access to the engine behind a [`Proj`] with the codes PROJ reports, for the C API.
//!
//! The façade methods of [`Proj`] turn a failed step into a message. The C API needs the
//! step error itself, to set `proj_errno`, and the coordinate as four components.

use super::{Body, Direction, Proj};
use crate::engine::{Operation, TransformError as StepError};
use crate::Xyzt;

impl Proj {
    /// Whether this object transforms coordinates: an operation, not a single CRS.
    #[doc(hidden)]
    #[must_use]
    pub fn is_transform(&self) -> bool {
        !matches!(self.body, Body::Single)
    }

    /// The engine operation of a PROJ string, `None` for a CRS or a CRS pair.
    #[doc(hidden)]
    #[must_use]
    pub fn native_operation(&self) -> Option<&Operation> {
        match &self.body {
            Body::Native(operation) => Some(operation),
            _ => None,
        }
    }

    /// Whether the two horizontal axes of the source and of the target of a CRS pair are
    /// written latitude or northing first by their CRS. A pair transforms in the order
    /// longitude or easting first (the order of `proj_normalize_for_visualization`); a caller
    /// of `proj_create_crs_to_crs` gives the order of the CRS. `None` for a non-pair.
    #[doc(hidden)]
    #[must_use]
    pub fn pair_axis_swaps(&self) -> Option<(bool, bool)> {
        match &self.body {
            Body::Iso(pipeline) => Some(pipeline.axis_swaps()),
            _ => None,
        }
    }

    /// The ISO operation of a CRS pair that has a single candidate, which PROJ returns as the
    /// pair itself. `None` for a set of candidates and for any other object.
    #[doc(hidden)]
    #[must_use]
    pub fn single_operation(&self) -> Option<&crate::iso19111::CoordinateOperation> {
        match &self.body {
            Body::Iso(pipeline) => pipeline.single_operation(),
            _ => None,
        }
    }

    /// One coordinate through the operation, with its step error. A CRS is not a transform:
    /// the caller checks [`Self::is_transform`] first.
    ///
    /// # Errors
    ///
    /// The [`StepError`] of the step that failed.
    #[doc(hidden)]
    pub fn trans_coord(&self, coord: Xyzt, direction: Direction) -> Result<Xyzt, StepError> {
        let mapped = match &self.body {
            Body::Single => return Err(StepError::NoInverse),
            Body::Native(operation) => match direction {
                Direction::Forward => operation.forward(coord),
                Direction::Inverse => operation.inverse(coord),
            },
            Body::Iso(pipeline) => pipeline.map(coord, direction),
        };
        mapped
    }

    /// Several coordinates through the operation. A failed point keeps its error in `failed`.
    #[doc(hidden)]
    pub fn trans_batch(
        &self,
        coords: &mut [Xyzt],
        direction: Direction,
        failed: &mut [Option<StepError>],
    ) {
        match &self.body {
            Body::Single => {
                for slot in failed.iter_mut() {
                    *slot = Some(StepError::NoInverse);
                }
            }
            Body::Native(operation) => match direction {
                Direction::Forward => operation.forward_batch(coords, failed),
                Direction::Inverse => operation.inverse_batch(coords, failed),
            },
            Body::Iso(pipeline) => pipeline.map_batch(coords, direction, failed),
        }
    }
}
