//! Coordinate metadata objects: a CRS and an epoch, as libproj's
//! `proj_coordinate_metadata_create` builds them.
//!
//! The object is the CRS [`crate::Proj`] itself, with the epoch stored beside it.
//! [`crate::Proj::create_crs_to_crs_from_pj`] applies one epoch by inserting
//! `proj=set` steps, the way PROJ does. Two epochs are refused: PROJ's C API does
//! not set `coordinateEpoch` when both sides carry one.

use crate::proj::{Proj, ProjCreateError};

/// The object of `crs` at `epoch`: the same CRS, carrying the epoch.
pub(crate) fn with_epoch(crs: &Proj, epoch: f64) -> Proj {
    let mut object = crs.clone();
    object.epoch = Some(epoch);
    object
}

/// The epoch of a coordinate metadata object, or NaN for an object without one.
pub(crate) fn epoch_of(object: &Proj) -> f64 {
    object.epoch.unwrap_or(f64::NAN)
}

/// The refusal of a CRS-to-CRS transform whose source and target both carry an epoch.
pub(crate) fn time_dependent_refusal(source: &Proj, target: &Proj) -> Option<ProjCreateError> {
    (source.epoch.is_some() && target.epoch.is_some()).then(|| {
        ProjCreateError::ProjError(
            "both CRS carry a coordinate epoch: a two-epoch transformation is not built".to_owned(),
        )
    })
}

/// The epoch to apply, when exactly one side carries one.
pub(crate) fn epoch_to_apply(source: &Proj, target: &Proj) -> Option<f64> {
    match (source.epoch, target.epoch) {
        (Some(year), None) | (None, Some(year)) => Some(year),
        _ => None,
    }
}
