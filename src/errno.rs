//! PROJ error codes and their messages (`proj.h` and `src/strerrno.cpp`).
//!
//! PROJ keeps one integer error per context. This crate returns typed errors,
//! so [`ProjError::code`], [`ProjCreateError::code`] and [`TransformError::code`]
//! give the PROJ code closest to a value, and [`error_string`] gives PROJ's
//! message for any code. The mapping is the crate's own; PROJ does not expose
//! it, so it is documented per variant.

use crate::proj::{ProjCreateError, ProjError};
use crate::transform::TransformError;

/// Operation initialization failed with no more specific code.
pub const PROJ_ERR_INVALID_OP: i32 = 1024;
/// Invalid pipeline structure, missing `+proj`, etc.
pub const PROJ_ERR_INVALID_OP_WRONG_SYNTAX: i32 = PROJ_ERR_INVALID_OP + 1;
/// A required operation parameter is missing.
pub const PROJ_ERR_INVALID_OP_MISSING_ARG: i32 = PROJ_ERR_INVALID_OP + 2;
/// An operation parameter has an illegal value.
pub const PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE: i32 = PROJ_ERR_INVALID_OP + 3;
/// Mutually exclusive arguments were given.
pub const PROJ_ERR_INVALID_OP_MUTUALLY_EXCLUSIVE_ARGS: i32 = PROJ_ERR_INVALID_OP + 4;
/// A file was not found or is not valid.
pub const PROJ_ERR_INVALID_OP_FILE_NOT_FOUND_OR_INVALID: i32 = PROJ_ERR_INVALID_OP + 5;
/// A transformation failed with no more specific code.
pub const PROJ_ERR_COORD_TRANSFM: i32 = 2048;
/// The coordinate is invalid, for example a latitude beyond 90 degrees.
pub const PROJ_ERR_COORD_TRANSFM_INVALID_COORD: i32 = PROJ_ERR_COORD_TRANSFM + 1;
/// The coordinate is outside the projection domain.
pub const PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN: i32 = PROJ_ERR_COORD_TRANSFM + 2;
/// No operation matches the criteria.
pub const PROJ_ERR_COORD_TRANSFM_NO_OPERATION: i32 = PROJ_ERR_COORD_TRANSFM + 3;
/// The point falls outside the grid or subgrid.
pub const PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID: i32 = PROJ_ERR_COORD_TRANSFM + 4;
/// The point falls in a grid cell that evaluates to nodata.
pub const PROJ_ERR_COORD_TRANSFM_GRID_AT_NODATA: i32 = PROJ_ERR_COORD_TRANSFM + 5;
/// An iterative method did not converge.
pub const PROJ_ERR_COORD_TRANSFM_NO_CONVERGENCE: i32 = PROJ_ERR_COORD_TRANSFM + 6;
/// The operation needs a time, and none was given.
pub const PROJ_ERR_COORD_TRANSFM_MISSING_TIME: i32 = PROJ_ERR_COORD_TRANSFM + 7;
/// Other errors.
pub const PROJ_ERR_OTHER: i32 = 4096;
/// The API was used incorrectly.
pub const PROJ_ERR_OTHER_API_MISUSE: i32 = PROJ_ERR_OTHER + 1;
/// No inverse method is available.
pub const PROJ_ERR_OTHER_NO_INVERSE_OP: i32 = PROJ_ERR_OTHER + 2;
/// A network resource could not be accessed.
pub const PROJ_ERR_OTHER_NETWORK_ERROR: i32 = PROJ_ERR_OTHER + 3;

/// PROJ's message for an error code, as `proj_errno_string` returns it.
///
/// `None` for code `0`, which is "no error". A code without a message of its
/// own is `"Unknown error (code N)"`, or one of the two "Unspecified error"
/// messages when it falls in the operation or transformation range.
#[must_use]
pub fn error_string(code: i32) -> Option<String> {
    if code == 0 {
        return None;
    }
    let listed = match code {
        PROJ_ERR_INVALID_OP_WRONG_SYNTAX => Some("Invalid PROJ string syntax"),
        PROJ_ERR_INVALID_OP_MISSING_ARG => Some("Missing argument"),
        PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE => Some("Invalid value for an argument"),
        PROJ_ERR_INVALID_OP_MUTUALLY_EXCLUSIVE_ARGS => Some("Mutually exclusive arguments"),
        PROJ_ERR_INVALID_OP_FILE_NOT_FOUND_OR_INVALID => Some("File not found or invalid"),
        PROJ_ERR_COORD_TRANSFM_INVALID_COORD => Some("Invalid coordinate"),
        PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN => {
            Some("Point outside of projection domain")
        }
        PROJ_ERR_COORD_TRANSFM_NO_OPERATION => {
            Some("No operation matching criteria found for coordinate")
        }
        PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID => Some("Coordinate to transform falls outside grid"),
        PROJ_ERR_COORD_TRANSFM_GRID_AT_NODATA => {
            Some("Coordinate to transform falls into a grid cell that evaluates to nodata")
        }
        PROJ_ERR_COORD_TRANSFM_NO_CONVERGENCE => {
            Some("Iterative method fails to converge on coordinate to transform")
        }
        PROJ_ERR_COORD_TRANSFM_MISSING_TIME => Some("Coordinate to transform lacks time"),
        PROJ_ERR_OTHER_API_MISUSE => Some("API misuse"),
        PROJ_ERR_OTHER_NO_INVERSE_OP => Some("No inverse operation"),
        PROJ_ERR_OTHER_NETWORK_ERROR => Some("Network error when accessing a remote resource"),
        _ => None,
    };
    if let Some(text) = listed {
        return Some(text.to_owned());
    }
    if code > 0 && code & PROJ_ERR_INVALID_OP != 0 {
        return Some("Unspecified error related to coordinate operation initialization".to_owned());
    }
    if code > 0 && code & PROJ_ERR_COORD_TRANSFM != 0 {
        return Some("Unspecified error related to coordinate transformation".to_owned());
    }
    Some(format!("Unknown error (code {code})"))
}

impl ProjCreateError {
    /// The PROJ code closest to this error.
    ///
    /// The crate's own mapping: an unknown or unparsable definition is
    /// `PROJ_ERR_INVALID_OP_WRONG_SYNTAX`, anything else is
    /// `PROJ_ERR_INVALID_OP`.
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::ArgumentNulError(_) => PROJ_ERR_INVALID_OP_WRONG_SYNTAX,
            _ => PROJ_ERR_INVALID_OP,
        }
    }
}

impl ProjError {
    /// The PROJ code closest to this error. The crate's own mapping:
    /// coordinate failures map to the `PROJ_ERR_COORD_TRANSFM` range, a
    /// missing grid to `PROJ_ERR_INVALID_OP_FILE_NOT_FOUND_OR_INVALID`, and
    /// network failures to `PROJ_ERR_OTHER_NETWORK_ERROR`.
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::Projection(_) => PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN,
            Self::GridsNeeded(_) => PROJ_ERR_INVALID_OP_FILE_NOT_FOUND_OR_INVALID,
            Self::Network
            | Self::RemoteCallbacks
            | Self::RequestCloneError
            | Self::ContentLength
            | Self::HeaderError(_)
            | Self::DownloadError(..) => PROJ_ERR_OTHER_NETWORK_ERROR,
            Self::FloatConversion => PROJ_ERR_COORD_TRANSFM_INVALID_COORD,
            _ => PROJ_ERR_COORD_TRANSFM,
        }
    }
}

impl TransformError {
    /// The PROJ code closest to this error: the create error's code for a
    /// failed pair, the transform error's code otherwise.
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::ProjCreateError(error) => error.code(),
            Self::ProjError(error) => error.code(),
        }
    }
}
