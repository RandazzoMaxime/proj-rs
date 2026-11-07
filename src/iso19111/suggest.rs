//! Operation suggested for a coordinate, as `proj_get_suggested_operation`.
//!
//! `pj_create_prepared_operations` (`crs_to_crs.cpp`) gives each candidate the
//! area of use of its operation, in the coordinates of the source. A box that
//! crosses the antimeridian is two candidates. `pj_get_suggested_operation`
//! (`trans.cpp`) then takes the first candidate whose box holds the coordinate,
//! and replaces it by one of lower accuracy, or of the same accuracy and smaller
//! pseudo area. An offshore area is never taken over a candidate already chosen.
//!
//! The kind of the source decides how the boxes and the point are read:
//! - a geographic source: the boxes are in degrees, the point is in the axis order
//!   of the source, with the degree-order fallback of `srcIsLonLatDegree`;
//! - a projected source: the boxes are the longitude and latitude ones reprojected
//!   into the source's coordinates (`reproject_bbox`, through the operation from the
//!   geodetic base of the source, `create_operation_to_geog_crs`), and the point is
//!   tested in them as it is;
//! - a geocentric source: the boxes are in degrees, and the point is converted to
//!   longitude and latitude first (`pjSrcGeocentricToLonLat`); a world box holds
//!   every point.
//!
//! The target is not read here. PROJ also drops a candidate whose box cannot be
//! reprojected into the target (`minxDst <= maxxDst`); this port keeps it.

use super::alternatives::Bounds;
use super::factory::{geographic_of, DatabaseContext};
use super::model::{CoordinateOperation, Crs, GeodeticCRS, GeographicCRS};
use super::select::{geocentric_to_lon_lat, geodetic_to_crs, reference_base, reproject_box};
use super::FactoryError;
use crate::engine::{Coord, Operation};

/// Axis order of a geographic source: the first axis is latitude or longitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisOrder {
    LatitudeFirst,
    LongitudeFirst,
}

/// How the source reads the point and the candidate boxes.
enum SourceKind {
    /// Geographic: the boxes are in degrees, in the axis order of the source.
    Geographic(AxisOrder),
    /// Projected: the boxes are in the source's coordinates, reprojected through this
    /// operation from the geodetic base of the source (`pjGeogToSrc`, used forward).
    Projected(Operation),
    /// Geocentric: the point is converted to longitude and latitude by this operation
    /// (`pjSrcGeocentricToLonLat`).
    Geocentric(Operation),
}

/// One candidate: an operation's box, or one half of a box split at the antimeridian.
struct Prepared {
    /// Position of the operation in the list the caller passed.
    index: usize,
    /// The box, in the coordinates of the source.
    bounds: Bounds,
    pseudo_area: f64,
    accuracy: f64,
    offshore: bool,
    unknown_area: bool,
    priority: bool,
}

/// Index in `operations` of the operation suggested for the forward coordinate `coord`
/// (`x`, `y`, `z`), in the coordinates of `source`. `None` when no operation covers it.
pub(super) fn suggested_forward(
    operations: &[CoordinateOperation],
    source: &Crs,
    coord: (f64, f64, f64),
) -> Result<Option<usize>, FactoryError> {
    // `proj_get_suggested_operation` returns the unique operation of a list of one, before
    // any other check (`c_api.cpp`, "always use the unique operation").
    if operations.len() == 1 {
        return Ok(Some(0));
    }
    let Some(kind) = source_kind(source)? else {
        // `pj_create_prepared_operations` prepares no candidate: no suggestion.
        return Ok(None);
    };
    let candidates = prepare(operations, &kind);
    let point = match &kind {
        SourceKind::Geocentric(to_lon_lat) => to_lon_lat
            .forward(Coord {
                x: coord.0,
                y: coord.1,
                z: coord.2,
                t: 0.0,
            })
            .ok()
            .map(|lonlat| (lonlat.x, lonlat.y)),
        SourceKind::Geographic(_) | SourceKind::Projected(_) => Some((coord.0, coord.1)),
    };
    let mut best: Option<usize> = None;
    let mut best_accuracy = f64::MAX;
    for (position, candidate) in candidates.iter().enumerate() {
        if !covers(candidate, point, &kind) {
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

/// The kind of the source. `None` when the source has no geodetic base to be read from, or the
/// operation from that base cannot be built: PROJ then prepares no candidate.
fn source_kind(source: &Crs) -> Result<Option<SourceKind>, FactoryError> {
    if let Some(geographic) = geographic_of(source) {
        return Ok(Some(SourceKind::Geographic(axis_order(geographic)?)));
    }
    let db = DatabaseContext::create()?;
    if let Crs::Geodetic(GeodeticCRS::Geocentric(_)) = source {
        return Ok(geocentric_to_lon_lat(source, &db)
            .ok()
            .map(SourceKind::Geocentric));
    }
    let Some(base) = reference_base(source) else {
        return Ok(None);
    };
    Ok(geodetic_to_crs(&base, source, &db)
        .ok()
        .flatten()
        .map(SourceKind::Projected))
}

/// Axis order of a geographic source: its first axis is latitude or longitude.
fn axis_order(geographic: &GeographicCRS) -> Result<AxisOrder, FactoryError> {
    let axes = geographic.coordinate_system().axes();
    match axes.first().map(|axis| axis.direction()) {
        Some("north" | "south") => Ok(AxisOrder::LatitudeFirst),
        Some("east" | "west") => Ok(AxisOrder::LongitudeFirst),
        _ => Err(FactoryError::Unsupported(
            "source CRS has no geographic axis order".to_owned(),
        )),
    }
}

/// `pj_create_prepared_operations`: one candidate per operation, two when the box crosses the
/// antimeridian. An operation with no area of use spans the world. A piece whose box cannot be
/// reprojected into a projected source is dropped (`minxSrc <= maxxSrc`).
fn prepare(operations: &[CoordinateOperation], kind: &SourceKind) -> Vec<Prepared> {
    let mut candidates = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let (area, (west, south, east, north)) = area_of_use(operation);
        let accuracy = operation.accuracy().unwrap_or(-1.0);
        let priority = priority_operation(operation.name());
        let offshore = area.contains("- offshore");
        let unknown_area = area.is_empty() || area == "unknown";
        let pieces = if west <= east {
            vec![(west, east)]
        } else {
            vec![(west, 180.0), (-180.0, east)]
        };
        for (piece_west, piece_east) in pieces {
            let bounds = match kind {
                SourceKind::Geographic(_) | SourceKind::Geocentric(_) => Some(Bounds {
                    min_x: piece_west,
                    min_y: south,
                    max_x: piece_east,
                    max_y: north,
                }),
                SourceKind::Projected(to_source) => {
                    reproject_box(Some(to_source), piece_west, south, piece_east, north)
                }
            };
            let Some(bounds) = bounds else {
                continue;
            };
            // The pseudo area of the piece, as `add_coord_op_to_list` integrates it.
            let w = piece_west.to_radians();
            let e = piece_east.to_radians();
            candidates.push(Prepared {
                index,
                bounds,
                pseudo_area: (e - w) * (north.to_radians().sin() - south.to_radians().sin()),
                accuracy,
                offshore,
                unknown_area,
                priority,
            });
        }
    }
    candidates
}

/// `spatialCriterionOK` of `pj_get_suggested_operation` for one candidate. `point` is the
/// point in the coordinates of the source, or its longitude and latitude for a geocentric
/// source (`None` when it cannot be converted).
fn covers(candidate: &Prepared, point: Option<(f64, f64)>, kind: &SourceKind) -> bool {
    let bounds = &candidate.bounds;
    match kind {
        SourceKind::Projected(_) => point.is_some_and(|(x, y)| inside(bounds, x, y)),
        SourceKind::Geocentric(_) => {
            let world = bounds.min_x == -180.0
                && bounds.min_y == -90.0
                && bounds.max_x == 180.0
                && bounds.max_y == 90.0;
            world || point.is_some_and(|(x, y)| inside(bounds, x, y))
        }
        SourceKind::Geographic(order) => {
            let Some((x, y)) = point else {
                return false;
            };
            let (lat, lon) = match order {
                AxisOrder::LatitudeFirst => (x, y),
                AxisOrder::LongitudeFirst => (y, x),
            };
            if !(lat >= bounds.min_y && lat <= bounds.max_y) {
                return false;
            }
            let inside_lon = |value: f64| value >= bounds.min_x && value <= bounds.max_x;
            inside_lon(lon) || inside_lon(normalize_longitude(lon))
        }
    }
}

/// Inclusive test of a point in a box.
fn inside(bounds: &Bounds, x: f64, y: f64) -> bool {
    x >= bounds.min_x && x <= bounds.max_x && y >= bounds.min_y && y <= bounds.max_y
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
