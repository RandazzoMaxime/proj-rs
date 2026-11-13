//! Alternative operations of a `cs2cs` transformation, and the choice made for each point.
//!
//! `proj_create_crs_to_crs` keeps every candidate it prepares, not only the first, when there
//! are several (`crs_to_crs.cpp:776-795`, `alternativeCoordinateOperations`). `proj_trans` then
//! takes, for each point, the first candidate whose area of use holds the point
//! (`pj_get_suggested_operation`, `trans.cpp:47-176`). When that candidate fails, it is excluded
//! and the next one is tried, at most twice more (`trans.cpp:248-303`). A grid-free candidate is
//! the last resort (`trans.cpp:305-345`).
//!
//! The candidates are built in `select.rs` (`select_alternatives`). This module holds the
//! per-point choice. The three policies are the states of `ONLY_BEST` in
//! `proj_create_crs_to_crs_from_pj` (`crs_to_crs.cpp:521-531`).

use std::cell::Cell;

use crate::engine::{Coord, Operation, TransformError};
use crate::proj::Direction;

/// Severity of one PROJ log line: `PJ_LOG_ERROR` or `PJ_LOG_DEBUG` (`proj.h:373-381`). A line
/// is written when the debug level of the context is at least its level (`log.cpp:104-115`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// `PJ_LOG_ERROR`: written at the default level.
    Error,
    /// `PJ_LOG_DEBUG`: written when `PROJ_DEBUG` is 2 or more.
    Debug,
}

/// One PROJ log line of a transformation, handed to the caller instead of written. The binary
/// decides whether it is written, as the debug level of the context decides in PROJ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// The level the line is logged at.
    pub level: LogLevel,
    /// The text, without the line break.
    pub text: String,
}

impl LogLine {
    /// A line logged at `PJ_LOG_ERROR`.
    #[must_use]
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Error,
            text: text.into(),
        }
    }

    /// A line logged at `PJ_LOG_DEBUG`.
    #[must_use]
    pub fn debug(text: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Debug,
            text: text.into(),
        }
    }
}

/// The sentence `pj_warn_about_missing_grid` adds under the warn policy, after the grids of the
/// failed operation (`trans.cpp:199-205`). It is added once per transformation instance.
pub const WARN_SUFFIX: &str = " This might become an error in a future PROJ major release. Set the ONLY_BEST option to YES or NO. This warning will no longer be emitted (for the current transformation instance).";

/// `proj_context_errno_string` of `PROJ_ERR_OTHER_NO_INVERSE_OP` (`strerrno.cpp:42`). It is the
/// errno of a candidate whose pipeline cannot run: the candidates whose grids are missing (the
/// observed case, see `docs/PORTING-cs2cs-proj-debug.md`).
const NO_INVERSE_OPERATION: &str = "No inverse operation";

/// What `proj_create_crs_to_crs` does when the best operation may lack a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BestPolicy {
    /// PROJ's default (`ONLY_BEST` not given). A point whose operation fails is retried with
    /// the next candidate, and the first failure turns the retry into the instantiable kind.
    Warn,
    /// `ONLY_BEST=YES`. The operation chosen for a point is used or the point fails with
    /// PROJ's missing-grid message. No retry.
    Error,
    /// `ONLY_BEST=NO`. Candidates whose grids are missing are not in the list at all.
    Discard,
}

/// Why a point has no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlternativeError {
    /// `ONLY_BEST=YES` and the chosen operation's grid is missing: PROJ's `pj_warn_about_missing_grid`
    /// text, one line.
    MissingGrid(String),
    /// Every candidate failed, or none covers the point and none is grid-free.
    NoOperation,
}

/// How the point is compared with the area of use of the candidates.
#[derive(Clone)]
pub(crate) enum SourceAxes {
    /// The source is geographic or projected: the candidate boxes are in its own coordinates.
    Plain,
    /// The source is geocentric: the point is converted to longitude and latitude first, by this
    /// operation (`create_operation_geocentric_crs_to_geog_crs`).
    Geocentric(Operation),
}

/// The box of a candidate, in the coordinates of the source (`minxSrc` ... `maxySrc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

/// The degree order of a box's CRS: longitude first, or latitude first, in degrees
/// (`srcIsLonLatDegree`, `srcIsLatLonDegree`, `dstIsLonLatDegree`, `dstIsLatLonDegree`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DegreeOrder {
    /// Longitude first, in degrees.
    pub(crate) lon_lat: bool,
    /// Latitude first, in degrees.
    pub(crate) lat_lon: bool,
}

/// One prepared candidate, as `PJCoordOperation` (`proj_internal.h:319`).
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one PROJ characteristic of the candidate"
)]
#[derive(Clone)]
pub(crate) struct Item {
    /// The operation name, as PROJ's `PJCoordOperation::name` (`Using coordinate operation`).
    pub(crate) name: String,
    /// The PROJ string of the operation, or `None` when it has none.
    pub(crate) definition: Option<String>,
    /// The pipeline built from `definition`. `None` when the operation is not instantiable as a
    /// pipeline (`proj_list_get` fails).
    pub(crate) operation: Option<Operation>,
    /// `proj_coordoperation_is_instantiable`: a PROJ string, and every grid on disk.
    pub(crate) instantiable: bool,
    /// `gridsNeeded(...).empty()`: the operation names no grid at all.
    pub(crate) grid_free: bool,
    /// The text of PROJ's missing-grid message for this candidate.
    pub(crate) grid_message: String,
    /// The source box of this candidate.
    pub(crate) bounds: Bounds,
    pub(crate) pseudo_area: f64,
    pub(crate) accuracy: f64,
    pub(crate) offshore: bool,
    pub(crate) unknown_area: bool,
    pub(crate) priority: bool,
    /// `srcIsLonLatDegree`: the source is longitude first, in degrees.
    pub(crate) lon_lat_degree: bool,
    /// `srcIsLatLonDegree`: the source is latitude first, in degrees.
    pub(crate) lat_lon_degree: bool,
    /// The target box of this candidate, in the target's coordinates (`minxDst`...`maxyDst`).
    pub(crate) target_bounds: Bounds,
    /// `dstIsLonLatDegree`: the target is longitude first, in degrees.
    pub(crate) target_lon_lat_degree: bool,
    /// `dstIsLatLonDegree`: the target is latitude first, in degrees.
    pub(crate) target_lat_lon_degree: bool,
    /// `ballpark` of `proj_coordoperation_has_ballpark_transformation`, kept for the rerun.
    pub(crate) approximate: bool,
}

impl Clone for Alternatives {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            policy: self.policy,
            source_axes: self.source_axes.clone(),
            target_axes: self.target_axes.clone(),
            first_axis: self.first_axis,
            warn: Cell::new(self.warn.get()),
            current: Cell::new(self.current.get()),
            last_failure: Cell::new(self.last_failure.get()),
            creation: self.creation.clone(),
        }
    }
}

impl std::fmt::Debug for Alternatives {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Alternatives")
            .field("candidates", &self.items.len())
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

/// The candidates of a pair, in PROJ's order, with the state `proj_trans` keeps between points.
pub struct Alternatives {
    items: Vec<Item>,
    policy: BestPolicy,
    source_axes: SourceAxes,
    /// How the target point is read for the inverse box test (`pjDstGeocentricToLonLat`).
    target_axes: SourceAxes,
    first_axis: FirstAxis,
    /// `P->warnIfBestTransformationNotAvailable`: cleared by the first failure under `Warn`.
    warn: Cell<bool>,
    /// `P->iCurCoordOp`: the candidate the last point used, `None` before the first point.
    current: Cell<Option<usize>>,
    /// The errno text of the last failed candidate, logged by the next retry (`trans.cpp:257-261`).
    last_failure: Cell<Option<&'static str>>,
    /// The lines `proj_create_crs_to_crs` logs when it creates the pair (`crs_to_crs.cpp:650`).
    creation: Vec<LogLine>,
}

/// The state a `proj_trans` leaves between two points: `P->iCurCoordOp`, the pending warning and
/// the last failure. See [`Alternatives::save_state`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointState {
    current: Option<usize>,
    warn: bool,
    last_failure: Option<&'static str>,
}

/// Whether the candidates that are not instantiable may be chosen (`skipNonInstantiable`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Instantiability {
    /// Every candidate may be chosen.
    Any,
    /// A candidate whose grids are missing is passed over.
    InstantiableOnly,
}

/// Whether the first output axis of the pipelines is latitude or northing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FirstAxis {
    /// Latitude or northing.
    LatitudeOrNorthing,
    /// Any other axis.
    Other,
}

/// Inclusive test of `coord` in `bounds`, as `pj_get_suggested_operation` makes it.
fn inside(bounds: &Bounds, x: f64, y: f64) -> bool {
    x >= bounds.min_x && x <= bounds.max_x && y >= bounds.min_y && y <= bounds.max_y
}

/// `normalizeLongitude` of `trans.cpp:53-64`.
fn normalize_longitude(mut x: f64) -> f64 {
    if x > 180.0 {
        x -= 360.0;
        if x > 180.0 {
            x = (x + 180.0) % 360.0 - 180.0;
        }
    } else if x < -180.0 {
        x += 360.0;
        if x < -180.0 {
            x = (x + 180.0) % 360.0 - 180.0;
        }
    }
    x
}

impl Alternatives {
    /// The candidates of one pair. `items` are in PROJ's order, and `policy` is the policy they
    /// were built under.
    pub(crate) fn new(
        items: Vec<Item>,
        policy: BestPolicy,
        source_axes: SourceAxes,
        target_axes: SourceAxes,
        first_axis: FirstAxis,
    ) -> Self {
        Self {
            items,
            policy,
            source_axes,
            target_axes,
            first_axis,
            warn: Cell::new(policy == BestPolicy::Warn),
            current: Cell::new(None),
            last_failure: Cell::new(None),
            creation: Vec::new(),
        }
    }

    /// The candidate the last point used, `P->iCurCoordOp` (`info.cpp` 170-171). `None` before
    /// the first point, and after a point that found no candidate.
    pub(crate) fn current_item(&self) -> Option<&Item> {
        self.current.get().and_then(|best| self.items.get(best))
    }

    /// The state the points leave behind: the candidate, the warning flag and the last failure.
    pub(crate) fn save_state(&self) -> PointState {
        PointState {
            current: self.current.get(),
            warn: self.warn.get(),
            last_failure: self.last_failure.get(),
        }
    }

    /// Puts back a state of [`Self::save_state`], after a probe that is not a `proj_trans`.
    pub(crate) fn restore_state(&self, state: PointState) {
        self.current.set(state.current);
        self.warn.set(state.warn);
        self.last_failure.set(state.last_failure);
    }

    /// `PJCoordOperation::operator==` over the lists (`proj_internal.h` 424-433): the same
    /// candidates in the same order, with the same names, boxes and accuracy, and the same PROJ
    /// string for each. PROJ compares the candidate objects with `PJ_COMP_STRICT`; the strings
    /// stand in for that comparison, which is exact for the candidates this port builds from them.
    pub(crate) fn is_equivalent_to(&self, other: &Self) -> bool {
        self.items.len() == other.items.len()
            && self.items.iter().zip(&other.items).all(|(a, b)| {
                a.name == b.name
                    && a.bounds == b.bounds
                    && a.target_bounds == b.target_bounds
                    && a.accuracy == b.accuracy
                    && a.definition == b.definition
            })
    }

    /// The pair was created after `pj_warn_about_missing_grid` on its single first candidate
    /// (`crs_to_crs.cpp:648-651`): the warning is a creation line, and it clears
    /// `warnIfBestTransformationNotAvailable` (`trans.cpp:199-206`), so `proj_trans` skips the
    /// candidates that are not instantiable from the first point on (`trans.cpp:237-239`).
    pub(crate) fn warned_at_creation(mut self, line: LogLine) -> Self {
        self.creation.push(line);
        self.warn.set(false);
        self
    }

    /// The lines logged when the pair was created, for the caller to write before the first point.
    #[must_use]
    pub fn creation_log(&self) -> &[LogLine] {
        &self.creation
    }

    /// Number of candidates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there is no candidate.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether the first output axis of the pipelines is latitude or northing.
    #[must_use]
    pub fn lat_or_northing_first(&self) -> bool {
        self.first_axis == FirstAxis::LatitudeOrNorthing
    }

    /// The policy the candidates were built under.
    #[must_use]
    pub fn policy(&self) -> BestPolicy {
        self.policy
    }

    /// The point in the box test's coordinates: the source's own for a forward transform, the
    /// target's for an inverse one (`pj_get_suggested_operation`, `trans.cpp:77-148`).
    fn box_point(&self, coord: Coord, way: Direction) -> Option<(f64, f64)> {
        match self.axes(way) {
            SourceAxes::Plain => Some((coord.x, coord.y)),
            SourceAxes::Geocentric(to_geographic) => to_geographic
                .forward(coord)
                .ok()
                .map(|lonlat| (lonlat.x, lonlat.y)),
        }
    }

    /// How the point of `way` is read: through the source's or the target's geographic position.
    fn axes(&self, way: Direction) -> &SourceAxes {
        match way {
            Direction::Forward => &self.source_axes,
            Direction::Inverse => &self.target_axes,
        }
    }

    /// `spatialCriterionOK` of `pj_get_suggested_operation`, for a box and its degree order.
    /// Under a geocentric reading (`trans.cpp:78-91`, `113-126`) there is no normalisation, and a
    /// world box covers every point whatever its conversion.
    fn covers(
        bounds: &Bounds,
        order: DegreeOrder,
        point: Option<(f64, f64)>,
        axes: &SourceAxes,
    ) -> bool {
        if matches!(axes, SourceAxes::Geocentric(_)) {
            let world = bounds.min_x == -180.0
                && bounds.min_y == -90.0
                && bounds.max_x == 180.0
                && bounds.max_y == 90.0;
            return world || point.is_some_and(|(x, y)| inside(bounds, x, y));
        }
        let Some((x, y)) = point else {
            return false;
        };
        if inside(bounds, x, y) {
            return true;
        }
        if order.lon_lat
            && y >= bounds.min_y
            && y <= bounds.max_y
            && normalize_longitude(x) >= bounds.min_x
            && normalize_longitude(x) <= bounds.max_x
        {
            return true;
        }
        order.lat_lon
            && x >= bounds.min_x
            && x <= bounds.max_x
            && normalize_longitude(y) >= bounds.min_y
            && normalize_longitude(y) <= bounds.max_y
    }

    /// `pj_get_suggested_operation` (`trans.cpp:47-176`) for one direction: the index of the
    /// candidate that serves `point`, skipping `excluded`, or `None`.
    fn suggest(
        &self,
        point: Option<(f64, f64)>,
        excluded: &[Option<usize>; 2],
        instantiability: Instantiability,
        way: Direction,
    ) -> Option<usize> {
        let axes = self.axes(way);
        let mut best: Option<usize> = None;
        let mut best_accuracy = f64::MAX;
        for (index, item) in self.items.iter().enumerate() {
            if excluded.contains(&Some(index)) {
                continue;
            }
            let covered = match way {
                Direction::Forward => Self::covers(
                    &item.bounds,
                    DegreeOrder {
                        lon_lat: item.lon_lat_degree,
                        lat_lon: item.lat_lon_degree,
                    },
                    point,
                    axes,
                ),
                Direction::Inverse => Self::covers(
                    &item.target_bounds,
                    DegreeOrder {
                        lon_lat: item.target_lon_lat_degree,
                        lat_lon: item.target_lat_lon_degree,
                    },
                    point,
                    axes,
                ),
            };
            if !covered {
                continue;
            }
            let accepted = match best {
                None => true,
                Some(chosen) => {
                    let chosen = &self.items[chosen];
                    let more_accurate = item.accuracy >= 0.0 && item.accuracy < best_accuracy;
                    let same_accuracy_smaller_area = item.accuracy == best_accuracy
                        && item.pseudo_area < chosen.pseudo_area
                        && !(item.unknown_area && !chosen.unknown_area)
                        && !chosen.priority;
                    (more_accurate || same_accuracy_smaller_area) && !item.offshore
                }
            };
            if !accepted
                || (instantiability == Instantiability::InstantiableOnly && !item.instantiable)
            {
                continue;
            }
            best = Some(index);
            best_accuracy = item.accuracy;
        }
        best
    }

    /// `proj_trans` (forward) for one point, `trans.cpp:233-346`, without the log lines.
    ///
    /// # Errors
    ///
    /// As [`Self::forward_logged`].
    pub fn forward(&self, coord: Coord) -> Result<Coord, AlternativeError> {
        self.forward_logged(coord, &mut Vec::new())
    }

    /// `proj_trans` (forward) for one point, `trans.cpp:233-346`. The lines PROJ logs while it
    /// runs are appended to `log`, in PROJ's order: `Using coordinate operation` when the
    /// candidate changes (`trans.cpp:269-276`), the retry lines (`trans.cpp:257-266`), the
    /// warning of the first failure (`trans.cpp:289-298`), and the fallback (`trans.cpp:323-332`).
    ///
    /// # Errors
    ///
    /// [`AlternativeError::MissingGrid`] under [`BestPolicy::Error`] when the chosen operation
    /// fails, and [`AlternativeError::NoOperation`] when no candidate gives a result.
    pub fn forward_logged(
        &self,
        coord: Coord,
        log: &mut Vec<LogLine>,
    ) -> Result<Coord, AlternativeError> {
        self.transform(coord, Direction::Forward, log)
    }

    /// `proj_trans` (inverse) for one point, `trans.cpp:233-346` with `PJ_INV`: the boxes are the
    /// target's, and each candidate is inverted.
    ///
    /// # Errors
    ///
    /// As [`Self::forward`].
    pub(crate) fn inverse(&self, coord: Coord) -> Result<Coord, AlternativeError> {
        self.transform(coord, Direction::Inverse, &mut Vec::new())
    }

    /// `proj_trans` for one point in one direction, `trans.cpp:233-346`. The log lines are
    /// appended to `log` as [`Self::forward_logged`] describes.
    fn transform(
        &self,
        coord: Coord,
        way: Direction,
        log: &mut Vec<LogLine>,
    ) -> Result<Coord, AlternativeError> {
        let point = self.box_point(coord, way);
        // `skipNonInstantiable` of `proj_trans`: the creation flag, cleared while the warning is
        // still pending and under the other policies.
        let mut instantiability = if self.policy == BestPolicy::Warn && !self.warn.get() {
            Instantiability::InstantiableOnly
        } else {
            Instantiability::Any
        };
        let mut excluded: [Option<usize>; 2] = [None, None];
        for retry in 0..=2 {
            let Some(best) = self.suggest(point, &excluded, instantiability, way) else {
                break;
            };
            if retry > 0 {
                // `proj_context_errno_string(oldErrno)` is logged at debug level, then the retry.
                if let Some(text) = self.last_failure.take() {
                    log.push(LogLine::debug(text));
                }
                log.push(LogLine::debug(
                    "Did not result in valid result. Attempting a retry with another operation.",
                ));
            }
            let item = &self.items[best];
            if self.current.get() != Some(best) {
                log.push(LogLine::debug(format!(
                    "Using coordinate operation {}",
                    item.name
                )));
                self.current.set(Some(best));
            }
            if let Some(operation) = &item.operation {
                if let Ok(result) = apply(operation, coord, way) {
                    return Ok(result);
                }
            }
            // A candidate whose grids are missing has no pipeline to run.
            self.last_failure
                .set((!item.instantiable).then_some(NO_INVERSE_OPERATION));
            match self.policy {
                BestPolicy::Error => {
                    // The error text is the caller's to write: it is the failure of the point.
                    return Err(AlternativeError::MissingGrid(item.grid_message.clone()));
                }
                BestPolicy::Warn => {
                    // `pj_warn_about_missing_grid` logs at debug level, once per instance. The
                    // first failure makes later retries skip the candidates that are not
                    // instantiable.
                    if self.warn.get() {
                        log.push(LogLine::debug(format!(
                            "{}{WARN_SUFFIX}",
                            item.grid_message
                        )));
                        self.warn.set(false);
                    }
                    instantiability = Instantiability::InstantiableOnly;
                }
                BestPolicy::Discard => {}
            }
            if retry == 2 {
                break;
            }
            excluded[retry] = Some(best);
        }
        // `trans.cpp:308-342`: the first candidate that names no grid.
        for (index, item) in self.items.iter().enumerate() {
            if item.grid_free {
                if let Some(operation) = &item.operation {
                    if self.current.get() != Some(index) {
                        log.push(LogLine::debug(format!(
                            "Using coordinate operation {} as a fallback due to lack of more appropriate operations",
                            item.name
                        )));
                        self.current.set(Some(index));
                    }
                    return apply(operation, coord, way).map_err(|_| AlternativeError::NoOperation);
                }
            }
        }
        Err(AlternativeError::NoOperation)
    }
}

/// One point through a candidate, in the direction `way`.
fn apply(operation: &Operation, coord: Coord, way: Direction) -> Result<Coord, TransformError> {
    match way {
        Direction::Forward => operation.forward(coord),
        Direction::Inverse => operation.inverse(coord),
    }
}
