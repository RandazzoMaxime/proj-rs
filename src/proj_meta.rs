//! Coordinate metadata objects: a CRS and an epoch, as libproj's
//! `proj_coordinate_metadata_create` builds them.
//!
//! The object is the CRS [`crate::Proj`] itself, with the epoch stored beside it. An
//! epoch matters only to a transformation that moves a coordinate in time. This port
//! does not apply the epoch, so a CRS-to-CRS transform that takes such an object as its
//! source or its target is refused when it is created, rather than built with the epoch
//! ignored.

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

/// The refusal of a CRS-to-CRS transform whose source or target carries an epoch.
pub(crate) fn time_dependent_refusal(source: &Proj, target: &Proj) -> Option<ProjCreateError> {
    (source.epoch.is_some() || target.epoch.is_some()).then(|| {
        ProjCreateError::ProjError(
            "coordinate metadata epoch is not applied: a time-dependent transformation is not built"
                .to_owned(),
        )
    })
}
