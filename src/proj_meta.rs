//! Coordinate metadata objects: a CRS and an epoch, as libproj's
//! `proj_coordinate_metadata_create` builds them.
//!
//! The object is the CRS [`crate::Proj`] itself, with the epoch stored beside it.
//! [`crate::Proj::create_crs_to_crs_from_pj`] applies one epoch by inserting
//! `proj=set` steps, the way PROJ does: two epochs set the source's first and the target's last.
//! A change of epoch through a point motion operation is refused.

use crate::iso19111::{
    create_from_user_input, has_point_motion as has_point_motion_in, DatabaseContext,
};
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

/// The refusal of a CRS-to-CRS transform whose source and target carry two different epochs and
/// whose CRS has a point motion operation: that epoch change is not built.
///
/// Without one, PROJ's `createOperations` adds no epoch change (coordinateoperationfactory.cpp:
/// 3226-3257): the operation runs at the source epoch and its export ends at the target epoch.
pub(crate) fn time_dependent_refusal(source: &Proj, target: &Proj) -> Option<ProjCreateError> {
    let (Some(source_epoch), Some(target_epoch)) = (source.epoch, target.epoch) else {
        return None;
    };
    if source_epoch == target_epoch || !(has_point_motion(source) || has_point_motion(target)) {
        return None;
    }
    Some(ProjCreateError::ProjError(
        "both CRS carry a coordinate epoch: a change of epoch through a point motion \
         operation is not built"
            .to_owned(),
    ))
}

/// Whether the geodetic CRS of `object` has a point motion operation. A CRS that cannot be read
/// counts as having one, so that the pair stays refused.
fn has_point_motion(object: &Proj) -> bool {
    let Ok(db) = DatabaseContext::create() else {
        return true;
    };
    create_from_user_input(object.definition(), &db)
        .and_then(|crs| has_point_motion_in(&crs, &db))
        .unwrap_or(true)
}

/// The epochs of the `proj=set` steps, first and last: the one epoch of a single dated side at
/// both ends, else the source's then the target's, as PROJ exports an operation whose source
/// and target carry an epoch.
pub(crate) fn epoch_to_apply(source: &Proj, target: &Proj) -> Option<(f64, f64)> {
    match (source.epoch, target.epoch) {
        (Some(year), None) | (None, Some(year)) => Some((year, year)),
        (Some(first), Some(last)) => Some((first, last)),
        (None, None) => None,
    }
}
