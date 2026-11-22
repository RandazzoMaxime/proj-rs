//! Access to the engine behind a [`Proj`] with the codes PROJ reports, for the C API.
//!
//! The façade methods of [`Proj`] turn a failed step into a message. The C API needs the
//! step error itself, to set `proj_errno`, and the coordinate as four components.

use super::{Body, Direction, Proj};
use crate::engine::{Operation, TransformError as StepError};
use crate::iso19111::LogLine;
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

    /// One coordinate through the operation, with its step error. A CRS is not a transform:
    /// the caller checks [`Self::is_transform`] first.
    ///
    /// # Errors
    ///
    /// The [`StepError`] of the step that failed.
    #[doc(hidden)]
    pub fn trans_coord(&self, coord: Xyzt, direction: Direction) -> Result<Xyzt, StepError> {
        self.trans_coord_logged(coord, direction, &mut Vec::new())
    }

    /// [`Self::trans_coord`] with the lines PROJ logs while the point runs appended to `log`:
    /// the candidate lines of a CRS pair at debug level, and the error line of a step that
    /// refuses the point (`<step>: Invalid latitude`).
    ///
    /// # Errors
    ///
    /// The [`StepError`] of the step that failed.
    #[doc(hidden)]
    pub fn trans_coord_logged(
        &self,
        coord: Xyzt,
        direction: Direction,
        log: &mut Vec<LogLine>,
    ) -> Result<Xyzt, StepError> {
        match &self.body {
            Body::Single => Err(StepError::NoInverse),
            Body::Native(operation) => {
                let done = match direction {
                    Direction::Forward => operation.forward(coord),
                    Direction::Inverse => operation.inverse(coord),
                };
                if done.is_err() {
                    if let Some(text) = operation.failure_message(coord, direction) {
                        log.push(LogLine::error(text));
                    }
                }
                done
            }
            Body::Iso(pipeline) => pipeline.map_logged(coord, direction, log),
        }
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
