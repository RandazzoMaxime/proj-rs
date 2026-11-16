use std::fmt;

use crate::engine::TransformError as StepError;
use crate::errno;
use crate::iso19111::AlternativeError;

/// Why a [`Crs`](crate::Crs) or a [`Transformer`](crate::Transformer) could not be built, or why a
/// point could not be transformed.
///
/// The enum is `#[non_exhaustive]`: a new cause is not a breaking change. [`Error::code`] gives
/// the PROJ error code closest to each cause, as `proj_errno` does.
///
/// ```
/// use proj_rs::{Context, Crs, Error};
///
/// let ctx = Context::default();
/// let error = Crs::from_user_input(&ctx, "EPSG:999999").unwrap_err();
/// assert!(matches!(error, Error::Crs(_)));
/// assert_eq!(error.code(), 1025);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The text is not usable input: it is empty or contains a nul byte.
    InvalidInput(String),
    /// The text names no CRS this crate can read.
    Crs(String),
    /// No operation connects the two CRS, or the filters left none.
    NoOperation(String),
    /// The operation needs a grid that is not available.
    MissingGrid(String),
    /// A CRS cannot be written in the requested format.
    Export(String),
    /// The point is outside the domain of the projection.
    OutsideDomain,
    /// The point is outside the area a shift grid covers.
    OutsideGrid,
    /// The operation needs a coordinate time, and the point has none.
    MissingTime,
    /// The operation has no inverse.
    NoInverse,
    /// A coordinate is not finite, or a latitude or longitude is out of range.
    InvalidCoordinate,
    /// A grid could not be downloaded.
    Network,
    /// No candidate operation covers the point.
    NoOperationForPoint,
}

impl Error {
    /// The PROJ error code closest to this error (`PROJ_ERR_*` of `proj.h`).
    ///
    /// ```
    /// use proj_rs::Error;
    ///
    /// assert_eq!(Error::OutsideDomain.code(), 2050);
    /// ```
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) | Self::Crs(_) => errno::PROJ_ERR_INVALID_OP_WRONG_SYNTAX,
            Self::NoOperation(_) | Self::NoOperationForPoint => {
                errno::PROJ_ERR_COORD_TRANSFM_NO_OPERATION
            }
            Self::MissingGrid(_) => errno::PROJ_ERR_INVALID_OP_FILE_NOT_FOUND_OR_INVALID,
            Self::Export(_) => errno::PROJ_ERR_OTHER,
            Self::OutsideDomain => errno::PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN,
            Self::OutsideGrid => errno::PROJ_ERR_COORD_TRANSFM_OUTSIDE_GRID,
            Self::MissingTime => errno::PROJ_ERR_COORD_TRANSFM_MISSING_TIME,
            Self::NoInverse => errno::PROJ_ERR_OTHER_NO_INVERSE_OP,
            Self::InvalidCoordinate => errno::PROJ_ERR_COORD_TRANSFM_INVALID_COORD,
            Self::Network => errno::PROJ_ERR_OTHER_NETWORK_ERROR,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(text) => write!(f, "invalid input: {text}"),
            Self::Crs(text) => write!(f, "cannot read the CRS: {text}"),
            Self::NoOperation(text) => write!(f, "no operation: {text}"),
            Self::MissingGrid(text) => write!(f, "missing grid: {text}"),
            Self::Export(text) => write!(f, "cannot export the CRS: {text}"),
            Self::OutsideDomain => f.write_str("point is outside the projection domain"),
            Self::OutsideGrid => f.write_str("point is outside the grid"),
            Self::MissingTime => f.write_str("coordinate is missing a time"),
            Self::NoInverse => f.write_str("the inverse is not available"),
            Self::InvalidCoordinate => f.write_str("coordinate is invalid"),
            Self::Network => f.write_str("a grid could not be downloaded"),
            Self::NoOperationForPoint => f.write_str("no operation covers the point"),
        }
    }
}

impl std::error::Error for Error {}

impl From<StepError> for Error {
    fn from(error: StepError) -> Self {
        match error {
            StepError::OutsideDomain => Self::OutsideDomain,
            StepError::OutsideGrid => Self::OutsideGrid,
            StepError::MissingTime => Self::MissingTime,
            StepError::NoInverse => Self::NoInverse,
            StepError::InvalidCoord | StepError::NonFinite => Self::InvalidCoordinate,
            StepError::NetworkUnavailable => Self::Network,
        }
    }
}

impl From<AlternativeError> for Error {
    fn from(error: AlternativeError) -> Self {
        match error {
            AlternativeError::MissingGrid(text) => Self::MissingGrid(text),
            AlternativeError::NoOperation => Self::NoOperationForPoint,
            AlternativeError::Network => Self::Network,
        }
    }
}
