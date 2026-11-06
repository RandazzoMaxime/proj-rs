//! Operation suggested for a coordinate, as `proj_get_suggested_operation`.
//!
//! `pj_create_prepared_operations` (`crs_to_crs.cpp`) gives each candidate the
//! area of use of its operation, in the axis order of the source. A box that
//! crosses the antimeridian is two candidates. `pj_get_suggested_operation`
//! (`trans.cpp`) then takes the first candidate whose box holds the coordinate,
//! and replaces it by one of lower accuracy, or of the same accuracy and smaller
//! pseudo area. An offshore area is never taken over a candidate already chosen.
//!
//! Only a geographic source is handled. The area boxes are read in degrees, as
//! PROJ does for the axis units of those CRS.

use super::model::{CoordinateOperation, Crs};
use super::FactoryError;

/// Axis order of a geographic source: the first axis is latitude or longitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisOrder {
    LatitudeFirst,
    LongitudeFirst,
}

/// One candidate: an operation's box, or one half of a box split at the antimeridian.
struct Prepared {
    /// Position of the operation in the list the caller passed.
    index: usize,
    /// Longitude bounds of this piece, in degrees.
    west: f64,
    east: f64,
    /// Latitude bounds in degrees.
    south: f64,
    north: f64,
    pseudo_area: f64,
    accuracy: f64,
    offshore: bool,
    unknown_area: bool,
    priority: bool,
}

/// Index in `operations` of the operation suggested for the forward coordinate `coord`,
/// in the axis order of `source`. `None` when no operation covers it.
pub(super) fn suggested_forward(
    operations: &[CoordinateOperation],
    source: &Crs,
    coord: (f64, f64),
) -> Result<Option<usize>, FactoryError> {
    let order = axis_order(source)?;
    if operations.len() == 1 {
        return Ok(Some(0));
    }
    let candidates = prepare(operations);
    let mut best: Option<usize> = None;
    let mut best_accuracy = f64::MAX;
    for (position, candidate) in candidates.iter().enumerate() {
        if !covers(candidate, coord, order) {
            continue;
        }
        let replaces = match best {
            None => true,
            Some(current) => {
                let chosen = &candidates[current];
                let better_accuracy =
                    candidate.accuracy >= 0.0 && candidate.accuracy < best_accuracy;
                let same_accuracy_smaller_area = candidate.accuracy == best_accuracy
                    && candidate.pseudo_area < chosen.pseudo_area
                    && !(candidate.unknown_area && !chosen.unknown_area)
                    && !chosen.priority;
                (better_accuracy || same_accuracy_smaller_area) && !candidate.offshore
            }
        };
        if replaces {
            best = Some(position);
            best_accuracy = candidate.accuracy;
        }
    }
    Ok(best.map(|position| candidates[position].index))
}

/// Axis order of the source. Only a geographic CRS is accepted.
fn axis_order(source: &Crs) -> Result<AxisOrder, FactoryError> {
    let Crs::Geographic(geographic) = source else {
        return Err(FactoryError::Unsupported(
            "suggested operation needs a geographic source CRS".to_owned(),
        ));
    };
    let axes = geographic.coordinate_system().axes();
    match axes.first().map(|axis| axis.direction()) {
        Some("north" | "south") => Ok(AxisOrder::LatitudeFirst),
        Some("east" | "west") => Ok(AxisOrder::LongitudeFirst),
        _ => Err(FactoryError::Unsupported(
            "source CRS has no geographic axis order".to_owned(),
        )),
    }
}

/// `pj_create_prepared_operations`: one candidate per operation, two when the box
/// crosses the antimeridian. An operation with no area of use spans the world.
fn prepare(operations: &[CoordinateOperation]) -> Vec<Prepared> {
    let mut candidates = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let (area, bounds) = area_of_use(operation);
        let (west, south, east, north) = bounds;
        let accuracy = operation.accuracy().unwrap_or(-1.0);
        let name = operation.name();
        let priority = priority_operation(name);
        let offshore = area.contains("- offshore");
        let unknown_area = area.is_empty() || area == "unknown";
        let west_rad = west.to_radians();
        let mut east_rad = east.to_radians();
        if west_rad > east_rad {
            east_rad += 2.0 * std::f64::consts::PI;
        }
        let pseudo_area =
            (east_rad - west_rad) * (north.to_radians().sin() - south.to_radians().sin());
        let pieces = if west <= east {
            vec![(west, east)]
        } else {
            vec![(west, 180.0), (-180.0, east)]
        };
        for (piece_west, piece_east) in pieces {
            candidates.push(Prepared {
                index,
                west: piece_west,
                east: piece_east,
                south,
                north,
                pseudo_area,
                accuracy,
                offshore,
                unknown_area,
                priority,
            });
        }
    }
    candidates
}

/// `isPriorityOp` of `PJCoordOperation`: the operations PROJ prefers at equal accuracy.
pub(super) fn priority_operation(name: &str) -> bool {
    name.contains("NAD83 to NAD83(HARN) (47)")
        || name.contains("NAD83 to NAD83(HARN) (48)")
        || name.contains("NAD83 to NAD83(HARN) (49)")
        || name.contains("NAD83 to NAD83(HARN) (50)")
        || name.contains("GDA94 to WGS 84 (1)")
        || name.contains("GDA2020 to WGS 84 (2)")
}

/// Area of use of the first usage that has a bounding box, with its name. The world
/// when none does, and an empty name.
pub(super) fn area_of_use(operation: &CoordinateOperation) -> (String, (f64, f64, f64, f64)) {
    for domain in operation.domains() {
        let (Some(west), Some(south), Some(east), Some(north)) = (
            domain.west_bound_longitude(),
            domain.south_bound_latitude(),
            domain.east_bound_longitude(),
            domain.north_bound_latitude(),
        ) else {
            continue;
        };
        return (
            domain.area().unwrap_or("").to_owned(),
            (west, south, east, north),
        );
    }
    (String::new(), (-180.0, -90.0, 180.0, 90.0))
}

/// Whether `coord` lies in the candidate box. The longitude is also tried normalised to
/// [-180, 180], as the `srcIsLonLatDegree` and `srcIsLatLonDegree` branches do.
fn covers(candidate: &Prepared, coord: (f64, f64), order: AxisOrder) -> bool {
    let (lat, lon) = match order {
        AxisOrder::LatitudeFirst => (coord.0, coord.1),
        AxisOrder::LongitudeFirst => (coord.1, coord.0),
    };
    if !(lat >= candidate.south && lat <= candidate.north) {
        return false;
    }
    let inside = |value: f64| value >= candidate.west && value <= candidate.east;
    inside(lon) || inside(normalize_longitude(lon))
}

/// `normalizeLongitude` of `trans.cpp`.
fn normalize_longitude(mut x: f64) -> f64 {
    if x > 180.0 {
        x -= 360.0;
        if x > 180.0 {
            x = (x + 180.0) % 360.0 - 180.0;
        }
    } else if x < -180.0 {
        x += 360.0;
        if x < -180.0 {
            x = (x + 180.0).rem_euclid(360.0) - 180.0;
        }
    }
    x
}
