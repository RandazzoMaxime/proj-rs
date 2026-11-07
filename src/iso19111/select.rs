//! Operation selection for `cs2cs`, as `proj_create_crs_to_crs`.
//!
//! The registered operations between two CRS come from
//! [`CoordinateOperationFactory::create_operations`]. Each one gets the
//! characteristics `PrecomputedOpCharacteristics` holds in PROJ, and the list
//! is ordered by `SortFunction` (`coordinateoperationfactory.cpp`). The first
//! operation is exported as a PROJ pipeline.
//!
//! Not ported: the area of interest (`cs2cs` passes none), the `removeSynthetic*`
//! and `removeUninterestingOps` filters, the pivot search through
//! `createOperationsWithDatumPivot`, and the grid availability of a
//! concatenated operation's steps beyond their file names. A pair that needs
//! one of those is either sorted differently or reported as an error, and the
//! caller keeps its previous path.

use super::alternatives::{
    Alternatives, BestPolicy, Bounds, FirstAxis, Item, LogLine, SourceAxes, WARN_SUFFIX,
};
use super::factory::{
    operation_extent, sort_extents, GridAvailabilityUse, GridDescription, GridInfo, SortExtents,
    SpatialCriterion,
};
use super::model::{
    Axis, BoundingBox, CoordinateOperation, Crs, EllipsoidalCS, GeodeticCRS, GeographicCRS,
    ObjectDomain, Parameter, ParameterValue, UnitOfMeasure,
};
use super::suggest::{area_of_use, priority_operation};
use super::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, FactoryError, ProjLines,
};
use crate::engine::{Coord, Operation};
use crate::gridpath;

const NADCON_LONGITUDE_FILE_PARAMETER: &str = "8658";
const BALLPARK_GEOGRAPHIC_OFFSET: &str = "Ballpark geographic offset";
const BALLPARK_VERTICAL_TRANSFORMATION: &str = "Ballpark vertical transformation";
const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";
const NULL_GEOCENTRIC_TRANSLATION: &str = "Null geocentric translation";
const BALLPARK_GEOCENTRIC_TRANSLATION: &str = "Ballpark geocentric translation";

/// Characteristics of one candidate, as `PrecomputedOpCharacteristics`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one PROJ sort criterion"
)]
struct Characteristics {
    name: String,
    area: f64,
    accuracy: f64,
    exportable: bool,
    has_grids: bool,
    grids_available: bool,
    grids_known: bool,
    step_count: usize,
    proj_step_count: usize,
    approx: bool,
    ballpark_vertical: bool,
    null: bool,
    xyz_gridshift: bool,
    etrs_pair: bool,
    uses_etrf2000: bool,
    uses_etrs89_generic: bool,
}

/// `ONLY_BEST` of `proj_create_crs_to_crs`: whether the best operation may
/// have grids that are not on disk. `cs2cs --only-best` sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnlyBest {
    /// The first operation whose grids are on disk. The georust facade uses it.
    Off,
    /// No `ONLY_BEST` option, `cs2cs` without `--only-best`: [`OnlyBest::Off`], and when no
    /// candidate is both instantiable and free of a ballpark step, the search runs again with
    /// `DISCARD_OPERATION_IF_MISSING_GRID` (`crs_to_crs.cpp:668-735`).
    Unset,
    /// The first operation, whether or not its grids are on disk.
    On,
    /// `ONLY_BEST=NO` (`--only-best=no`): the registry drops an operation whose grid is not on
    /// disk (`DISCARD_OPERATION_IF_MISSING_GRID`), and the search falls through to the operations
    /// that need none, as PROJ's fallbacks do. The first operation whose grids are on disk.
    Discard,
}

/// The `ONLY_BEST` state when no option is given: `PROJ_ONLY_BEST_DEFAULT`, else the
/// `only_best_default` key of `proj.ini`. A non-empty `PROJ_ONLY_BEST_DEFAULT` set to YES, ON or
/// TRUE is [`OnlyBest::On`], and any other non-empty value is [`OnlyBest::Discard`]. An empty
/// one is no setting and hides `proj.ini`. With no setting the state is [`OnlyBest::Unset`],
/// PROJ's warning default (`filemanager.cpp:2023-2031`, `2101-2107`, `crs_to_crs.cpp:494-504`).
#[must_use]
pub fn only_best_default() -> OnlyBest {
    match crate::network::GridContext::load().only_best_default {
        Some(true) => OnlyBest::On,
        Some(false) => OnlyBest::Discard,
        None => OnlyBest::Unset,
    }
}

/// What `cs2cs` does with the best operation of a pair.
#[derive(Debug, PartialEq, Eq)]
pub enum BestOperation {
    /// A PROJ pipeline, and whether its first output axis is latitude or northing.
    Pipeline {
        /// The pipeline text.
        definition: String,
        /// Whether the first output axis is latitude or northing.
        lat_or_northing_first: bool,
    },
    /// `ONLY_BEST=YES` kept an operation whose grids are missing. PROJ fails each
    /// transformation with this message, as `pj_warn_about_missing_grid`.
    MissingGrid {
        /// The failure text, one line.
        message: String,
    },
    /// `ONLY_BEST=YES`, a single candidate whose grids are missing: PROJ's
    /// `proj_create_crs_to_crs` returns no transformation (`crs_to_crs.cpp:622-641`).
    Refused {
        /// The `pj_warn_about_missing_grid` text, one line.
        message: String,
    },
}

/// Whether `ALLOW_BALLPARK` of `proj_create_crs_to_crs` admits ballpark transformations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ballpark {
    /// Ballpark transformations are candidates, as by default.
    Allowed,
    /// `ALLOW_BALLPARK=NO`: an operation with a ballpark step is not a candidate.
    Forbidden,
}

/// The candidate filters of `proj_create_crs_to_crs`: `ACCURACY` and `ALLOW_BALLPARK`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateFilter {
    /// `ACCURACY` in metres: keep an operation whose accuracy is known and at most this.
    pub accuracy: Option<f64>,
    /// Whether ballpark transformations are candidates.
    pub ballpark: Ballpark,
}

impl CandidateFilter {
    /// No filter: every candidate PROJ lists is kept.
    pub const NONE: Self = Self {
        accuracy: None,
        ballpark: Ballpark::Allowed,
    };

    /// Whether a candidate passes the filter. PROJ drops an operation of unknown accuracy
    /// when an accuracy is requested.
    fn keeps(&self, operation: &CoordinateOperation) -> bool {
        // PROJ applies the accuracy test only when the requested value is not 0
        // (`coordinateoperationfactory.cpp`, `desiredAccuracy != 0`).
        if let Some(limit) = self.accuracy.filter(|limit| *limit != 0.0) {
            let accuracy = super::compound::get_accuracy(operation);
            if accuracy < 0.0 || accuracy > limit {
                return false;
            }
        }
        match self.ballpark {
            Ballpark::Allowed => true,
            Ballpark::Forbidden => !operation.has_ballpark_transformation(),
        }
    }
}

/// [`best_pipeline_with`], over the candidates `filter` keeps. An empty set is an error,
/// never a fallback: a filtered pair fails closed.
///
/// # Errors
///
/// As [`best_pipeline`], and when `filter` keeps no candidate.
pub fn best_pipeline_filtered(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    filter: CandidateFilter,
) -> Result<BestOperation, FactoryError> {
    best_pipeline_filtered_with_authority(source, target, only_best, filter, "")
}

/// [`best_pipeline_filtered`], over the candidates of `authority` only: the `AUTHORITY=` option
/// of `proj_create_crs_to_crs` (`crs_to_crs.cpp:507-508`), which `cs2cs --authority` sets. `""`
/// is every authority, `"any"` too.
///
/// # Errors
///
/// As [`best_pipeline_filtered`].
pub fn best_pipeline_filtered_with_authority(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    filter: CandidateFilter,
    authority: &str,
) -> Result<BestOperation, FactoryError> {
    best_operation_filtered(source, target, only_best, filter, authority).map(|chosen| chosen.best)
}

/// PROJ pipeline of the best operation from `source` to `target`, and whether
/// its first output axis is latitude or northing.
///
/// Both CRS are read with [`create_from_user_input`]. Coordinates in the
/// returned pipeline use the source CRS axis order and units, as PROJ's do.
///
/// # Errors
///
/// A CRS cannot be read, no operation is registered, or the best operation
/// cannot be written as a PROJ pipeline.
pub fn best_pipeline(source: &str, target: &str) -> Result<(String, bool), FactoryError> {
    match best_pipeline_with(source, target, OnlyBest::Off)? {
        BestOperation::Pipeline {
            definition,
            lat_or_northing_first,
        } => Ok((definition, lat_or_northing_first)),
        BestOperation::MissingGrid { message } | BestOperation::Refused { message } => {
            Err(FactoryError::Format(message))
        }
    }
}

/// The best operation from `source` to `target` as `cs2cs` takes it under
/// `only_best`.
///
/// The order is `SortFunction`'s. Without `ONLY_BEST`, grid availability is part of
/// it, and the first operation whose grids are on disk is used, as PROJ falls back
/// to it after a failed grid. With `ONLY_BEST` (`KNOWN_AVAILABLE`), availability does
/// not order the candidates: the first one is used, and it fails with PROJ's message
/// when its grids are missing.
///
/// # Errors
///
/// As [`best_pipeline`].
pub fn best_pipeline_with(
    source: &str,
    target: &str,
    only_best: OnlyBest,
) -> Result<BestOperation, FactoryError> {
    best_pipeline_with_authority(source, target, only_best, "")
}

/// [`best_pipeline_with`], over the candidates of `authority` only (see
/// [`best_pipeline_filtered_with_authority`]).
///
/// # Errors
///
/// As [`best_pipeline`].
pub fn best_pipeline_with_authority(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    authority: &str,
) -> Result<BestOperation, FactoryError> {
    best_operation_filtered(source, target, only_best, CandidateFilter::NONE, authority)
        .map(|chosen| chosen.best)
}

/// The best operation of a pair under `OnlyBest::Off`, as the facade needs it: the pipeline,
/// the operation object it was written from, and the two CRS. An approximate operation (a
/// ballpark transformation, a zero shift) is an error here: the facade never returns a result
/// it cannot stand behind, and keeps its previous path instead.
#[derive(Debug, Clone)]
pub(crate) struct ExactChoice {
    pub(crate) definition: String,
    pub(crate) operation: CoordinateOperation,
    pub(crate) source: Crs,
    pub(crate) target: Crs,
    /// How many candidate operations libproj could build: PROJ lists them all, and reports no
    /// single definition for the pair when more than one is instantiable.
    pub(crate) instantiable: usize,
}

/// The operation `cs2cs` takes for a pair, with what the facade needs of the selection.
struct Chosen {
    best: BestOperation,
    approx: bool,
    operation: CoordinateOperation,
    source: Crs,
    target: Crs,
    instantiable: usize,
    log: PipelineLog,
}

pub(crate) fn best_operation_exact(
    source: &str,
    target: &str,
) -> Result<ExactChoice, FactoryError> {
    let Chosen {
        best,
        approx,
        operation,
        source: source_crs,
        target: target_crs,
        instantiable,
        log: _,
    } = best_operation_full(source, target, OnlyBest::Off)?;
    match (best, approx) {
        (BestOperation::Pipeline { definition, .. }, false) => Ok(ExactChoice {
            definition,
            operation,
            source: source_crs,
            target: target_crs,
            instantiable,
        }),
        (BestOperation::Pipeline { .. }, true) => Err(FactoryError::Format(format!(
            "best operation from {source} to {target} is approximate"
        ))),
        (BestOperation::MissingGrid { message } | BestOperation::Refused { message }, _) => {
            Err(FactoryError::Format(message))
        }
    }
}

/// A same-datum geographic pair, which the factory does not synthesize.
///
/// PROJ's `createOperationsGeogToGeog` builds a null geographic offset for it
/// (`+proj=noop`). The factory leaves that out, so the facade asks for it only
/// when no registered operation exists. `instantiable` is 1: that offset is the
/// only operation.
pub(crate) fn same_datum_noop(source: &str, target: &str) -> Result<ExactChoice, FactoryError> {
    let db = DatabaseContext::create()?;
    let source_crs = create_from_user_input(source, &db)?;
    let target_crs = create_from_user_input(target, &db)?;
    let operation = super::compound::null_offset(&source_crs, &target_crs)
        .ok_or_else(|| FactoryError::Format(format!("no null offset from {source} to {target}")))?;
    let definition = export_operation(&operation, &db)?;
    Ok(ExactChoice {
        definition,
        operation,
        source: source_crs,
        target: target_crs,
        instantiable: 1,
    })
}

/// The best operation of a pair over every authority, with the chosen operation and the two CRS.
fn best_operation_full(
    source: &str,
    target: &str,
    only_best: OnlyBest,
) -> Result<Chosen, FactoryError> {
    best_operation_filtered(source, target, only_best, CandidateFilter::NONE, "")
}

/// [`best_operation_full`], keeping only the candidates `filter` accepts, and only those of
/// `authority` (`""` for every authority), as `proj_create_operation_factory_context` does.
fn best_operation_filtered(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    filter: CandidateFilter,
    authority: &str,
) -> Result<Chosen, FactoryError> {
    let db = DatabaseContext::create()?;
    let source_crs = create_from_user_input(source, &db)?;
    let target_crs = create_from_user_input(target, &db)?;
    let mut context = CoordinateOperationContext::create(&db, authority);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    // `proj_create_crs_to_crs` asks KNOWN_AVAILABLE with ONLY_BEST=YES and
    // DISCARD_OPERATION_IF_MISSING_GRID with ONLY_BEST=NO (`crs_to_crs.cpp:570-576`). The
    // registry then drops an operation whose grid is missing, and the search falls through to
    // the operations that need none.
    match only_best {
        OnlyBest::On => {
            context.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
        }
        OnlyBest::Discard => {
            context.set_grid_availability_use(GridAvailabilityUse::DiscardIfMissing);
        }
        OnlyBest::Off | OnlyBest::Unset => {}
    }
    let extents = sort_extents(&context, &source_crs, &target_crs);
    // `proj_create_crs_to_crs` with ONLY_BEST asks for KNOWN_AVAILABLE, which does not
    // clear `gridsAvailable` in the sort (`createOperations`, 1477-1491); the chosen
    // operation's grids are checked afterwards. Otherwise the sort follows availability.
    let availability = match only_best {
        OnlyBest::On => GridAvailabilityUse::KnownAvailable,
        OnlyBest::Off | OnlyBest::Unset | OnlyBest::Discard => GridAvailabilityUse::UseForSorting,
    };
    let ranked_with = |context: &CoordinateOperationContext| -> Result<Vec<_>, FactoryError> {
        let operations = CoordinateOperationFactory::create()
            .create_operations(&source_crs, &target_crs, context)?
            .into_iter()
            .filter(|operation| filter.keeps(operation));
        let mut ranked = Vec::new();
        for operation in operations {
            let characteristics = characteristics(
                &operation,
                &db,
                &source_crs,
                &target_crs,
                availability,
                &extents,
            );
            ranked.push((operation, characteristics));
        }
        Ok(sort(ranked))
    };
    let mut ranked = ranked_with(&context)?;
    // With no ONLY_BEST option, when no candidate is both instantiable and free of a ballpark
    // step, PROJ runs the search again with DISCARD_OPERATION_IF_MISSING_GRID and appends its
    // operations to the non-ballpark ones of the first list (`crs_to_crs.cpp:668-735`).
    if only_best == OnlyBest::Unset
        && !ranked.iter().any(|(operation, characteristics)| {
            characteristics.grids_available && !operation.has_ballpark_transformation()
        })
    {
        let mut discard = context.clone();
        discard.set_grid_availability_use(GridAvailabilityUse::DiscardIfMissing);
        let rerun = ranked_with(&discard)?;
        if !rerun.is_empty() {
            ranked.retain(|(operation, _)| !operation.has_ballpark_transformation());
            ranked.extend(rerun);
        }
    }
    let instantiable = ranked
        .iter()
        .filter(|(_, characteristics)| {
            characteristics.exportable && characteristics.grids_available
        })
        .count();
    // `op_count == 1` under ONLY_BEST=YES (`crs_to_crs.cpp:617-641`): the only candidate is kept
    // whatever its grids, and a missing grid refuses the creation.
    let single_candidate = ranked.len() == 1;
    let mut log = PipelineLog::default();
    let warned_at_creation = only_best == OnlyBest::Unset
        && creation_warning(&context, &source_crs, &target_crs, filter, &db, &mut log)?;
    let ranked_len = ranked.len();
    let chosen = match only_best {
        OnlyBest::On => ranked.into_iter().next(),
        OnlyBest::Off | OnlyBest::Unset | OnlyBest::Discard => ranked
            .into_iter()
            .find(|(_, characteristics)| characteristics.grids_available),
    };
    let (best, characteristics) = chosen
        .ok_or_else(|| FactoryError::Format(format!("no operation from {source} to {target}")))?;
    // A list of several candidates is an alternatives list: `proj_trans` logs the candidate it
    // uses at the first point, since no candidate was used before (`trans.cpp:269-276`). Under the
    // warn policy the first use is the first instantiable candidate (the warning cleared
    // `skipNonInstantiable`, `crs_to_crs.cpp:650`). Under `ONLY_BEST=YES` it is the first one.
    if ranked_len > 1 && (warned_at_creation || only_best == OnlyBest::On) {
        log.first_use = Some(LogLine::debug(format!(
            "Using coordinate operation {}",
            characteristics.name
        )));
    }
    // A missing grid is a result of its own, checked before the export: PROJ builds the
    // pipeline naming the grid and fails on the grid, so the export is not what decides.
    // `proj_coordoperation_is_instantiable` is the check: each grid of the pipeline, by its
    // PROJ name, must open.
    let grids = proj_grids(&best, &db)?;
    if !grids.iter().all(|grid| gridpath::available(grid)) {
        let message = missing_grid_message(&characteristics.name, &grids);
        return Ok(Chosen {
            best: if only_best == OnlyBest::On && single_candidate {
                BestOperation::Refused { message }
            } else {
                BestOperation::MissingGrid { message }
            },
            approx: characteristics.approx,
            operation: best,
            source: source_crs,
            target: target_crs,
            instantiable,
            log,
        });
    }
    if !characteristics.exportable {
        return Err(FactoryError::Format(format!(
            "best operation {} has no PROJ pipeline",
            characteristics.name
        )));
    }
    let definition = export_operation(&best, &db)?;
    let lat_first = lat_or_northing_first(&target_crs);
    Ok(Chosen {
        best: BestOperation::Pipeline {
            definition,
            lat_or_northing_first: lat_first,
        },
        approx: characteristics.approx,
        operation: best,
        source: source_crs,
        target: target_crs,
        instantiable,
        log,
    })
}

/// The creation warning of the warn policy, `crs_to_crs.cpp:617-651`. The first list is read with
/// `KNOWN_AVAILABLE`, as PROJ reads it under the warn policy (`crs_to_crs.cpp:570-576`). When it
/// has one candidate and that candidate's grids are not on disk, PROJ logs
/// `pj_warn_about_missing_grid` and clears the warning flag of the instance. Returns whether the
/// warning was logged. The search then goes on with the rerun, as in [`best_operation_filtered`].
fn creation_warning(
    context: &CoordinateOperationContext,
    source: &Crs,
    target: &Crs,
    filter: CandidateFilter,
    db: &DatabaseContext,
    log: &mut PipelineLog,
) -> Result<bool, FactoryError> {
    let mut known = context.clone();
    known.set_grid_availability_use(GridAvailabilityUse::KnownAvailable);
    let known_ops = CoordinateOperationFactory::create()
        .create_operations(source, target, &known)?
        .into_iter()
        .filter(|operation| filter.keeps(operation))
        .collect::<Vec<_>>();
    let [operation] = known_ops.as_slice() else {
        return Ok(false);
    };
    let grids = proj_grids(operation, db).unwrap_or_else(|_| grid_names(operation));
    let on_disk = grids.iter().all(|grid| gridpath::available(grid));
    let instantiable = on_disk && export(operation, source, target, db).is_ok();
    if instantiable {
        return Ok(false);
    }
    log.creation.push(LogLine::debug(format!(
        "{}{WARN_SUFFIX}",
        missing_grid_message(operation.name(), &grids)
    )));
    Ok(true)
}

/// The PROJ log lines of a pair under the warn policy (no `ONLY_BEST` option), for the binary to
/// write. PROJ writes the creation lines when `proj_create_crs_to_crs` returns and the first use
/// when the first `proj_trans` runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PipelineLog {
    /// Written by the creation of the pair (`crs_to_crs.cpp:650`).
    pub creation: Vec<LogLine>,
    /// Written by the first transformation, when the pair keeps several candidates
    /// (`trans.cpp:269-276`).
    pub first_use: Option<LogLine>,
}

/// [`best_pipeline_with`] and [`best_pipeline_filtered`] with the log lines PROJ writes for the
/// pair, under the warn policy (`OnlyBest::Unset`). Under the other states the log is empty.
///
/// # Errors
///
/// As [`best_pipeline`].
pub fn best_pipeline_logged(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    filter: CandidateFilter,
) -> Result<(BestOperation, PipelineLog), FactoryError> {
    best_pipeline_logged_with_authority(source, target, only_best, filter, "")
}

/// [`best_pipeline_logged`], over the candidates of `authority` only (see
/// [`best_pipeline_filtered_with_authority`]).
///
/// # Errors
///
/// As [`best_pipeline`].
pub fn best_pipeline_logged_with_authority(
    source: &str,
    target: &str,
    only_best: OnlyBest,
    filter: CandidateFilter,
    authority: &str,
) -> Result<(BestOperation, PipelineLog), FactoryError> {
    best_operation_filtered(source, target, only_best, filter, authority)
        .map(|chosen| (chosen.best, chosen.log))
}

/// `pj_warn_about_missing_grid`: the failure text for `name`, naming each grid
/// that is not on disk.
fn missing_grid_message(name: &str, grids: &[String]) -> String {
    let mut message = format!("Attempt to use coordinate operation {name} failed.");
    for grid in grids.iter().filter(|grid| !gridpath::available(grid)) {
        message.push_str(&format!(
            " Grid {grid} is not available. Consult https://proj.org/resource_files.html for guidance."
        ));
    }
    message
}

/// The PROJ string of an operation that carries its own CRS pair, in one line, with its
/// grid and deformation steps written as `exportToPROJString` writes them.
pub(crate) fn export_operation(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
) -> Result<String, FactoryError> {
    match operation {
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_some() && conversion.target_crs().is_some() =>
        {
            conversion.export_pipeline(ProjLines::Single)
        }
        // A bare axis-order change has no CRS to read units from, so PROJ writes the swap alone
        // (singleoperation.cpp:3479-3500 with null CRS). Its inverse is the same swap.
        CoordinateOperation::Conversion(conversion)
            if matches!(conversion.method().code(), "9843" | "9844") =>
        {
            let mut formatter = super::pipeline::PipelineFormatter::default();
            super::transform_export::axis_order_reversal(None, None, &mut formatter)?;
            Ok(formatter.finish(ProjLines::Single))
        }
        CoordinateOperation::Conversion(conversion) => Err(FactoryError::Format(format!(
            "conversion {} has no CRS pair",
            conversion.name()
        ))),
        _ => operation.export_to_proj_string_with_database(db, ProjLines::Single),
    }
}

fn export(
    operation: &CoordinateOperation,
    source: &Crs,
    target: &Crs,
    db: &DatabaseContext,
) -> Result<String, FactoryError> {
    match operation {
        CoordinateOperation::Transformation(_) | CoordinateOperation::Concatenated(_) => {
            operation.export_to_proj_string_with_database(db, ProjLines::Single)
        }
        // A conversion the factory chained carries its CRS pair, which is the operation's
        // own; a bare one keeps the CRS the caller gave.
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_some() && conversion.target_crs().is_some() =>
        {
            conversion.export_pipeline(ProjLines::Single)
        }
        // A bare axis-order change (EPSG:9843 or 9844, read from `proj.db` without its CRS pair)
        // is the swap of its two horizontal axes, whatever the other CRS of the pair is: PROJ's
        // `_exportToPROJString` writes `axisswap order=2,1` and no projection
        // (singleoperation.cpp:3479-3500). Its inverse is the same swap, since `axisswap
        // order=2,1` is its own inverse and the pipeline cleanup drops the `+inv` (io.cpp:8938).
        // The pair is the caller's, read in the direction of the operation.
        CoordinateOperation::Conversion(conversion)
            if matches!(conversion.method().code(), "9843" | "9844") =>
        {
            let mut formatter = super::pipeline::PipelineFormatter::default();
            if conversion.inverted {
                formatter.start_inversion();
            }
            super::transform_export::axis_order_reversal(
                Some(source),
                Some(target),
                &mut formatter,
            )?;
            if conversion.inverted {
                formatter.stop_inversion();
            }
            Ok(formatter.finish(ProjLines::Single))
        }
        CoordinateOperation::Conversion(conversion) if conversion.inverted => {
            Err(FactoryError::Format(format!(
                "inverse conversion {} has no PROJ string in this crate",
                conversion.name()
            )))
        }
        CoordinateOperation::Conversion(_) => {
            super::projstring::operation_proj_string(source, target)
        }
    }
}

/// `isPROJExportable` of a candidate: its PROJ string, with the grid and deformation steps
/// written, as `exportToPROJString` writes them.
fn characteristics(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    availability: GridAvailabilityUse,
    extents: &SortExtents,
) -> Characteristics {
    let name = operation.name().to_owned();
    let text = export(operation, source, target, db);
    let (exportable, proj_step_count, xyz_gridshift) = match text {
        Ok(text) => {
            let xyz = text.contains("+proj=xyzgridshift");
            let steps = if xyz { 0 } else { proj_steps(&text) };
            (true, steps, xyz)
        }
        Err(_) => (false, 0, false),
    };
    let grids = grid_names(operation);
    // Only `UseForSorting` looks at the disk: PROJ clears `gridsAvailable` under that policy alone.
    let grids_available = availability != GridAvailabilityUse::UseForSorting
        || proj_grids(operation, db)
            .is_ok_and(|names| names.iter().all(|grid| gridpath::available(grid)));
    let known_names = proj_grids(operation, db).unwrap_or_else(|_| grids.clone());
    let grids_known = known_names
        .iter()
        .all(|grid| db.grid_is_known(grid).unwrap_or(false));
    let op_box = operation_extent(operation).and_then(|extent| extent.bounding_box());
    let area = pseudo_area_of(op_box, extents);
    let accuracy = super::compound::get_accuracy(operation);
    let etrs_pair = etrs_pair(operation);
    Characteristics {
        area,
        accuracy,
        exportable,
        has_grids: !grids.is_empty(),
        grids_available,
        grids_known,
        step_count: step_count(operation),
        proj_step_count,
        approx: operation.has_ballpark_transformation(),
        ballpark_vertical: name.contains(BALLPARK_VERTICAL_TRANSFORMATION),
        null: is_null_transformation(&name),
        xyz_gridshift,
        etrs_pair,
        uses_etrf2000: name.contains("ETRF2000"),
        uses_etrs89_generic: name.contains("ETRS89 to ETRS89-"),
        name,
    }
}

/// `SortFunction`'s area: `getPseudoArea` of the operation extent, intersected with the area of
/// interest when there is one, else with the CRS extents as PROJ does.
fn pseudo_area_of(operation: Option<BoundingBox>, extents: &SortExtents) -> f64 {
    let Some(operation) = operation else {
        return 0.0;
    };
    let bbox_of = |domain: Option<&ObjectDomain>| domain.and_then(ObjectDomain::bounding_box);
    let area_of = |bbox: Option<BoundingBox>| bbox.map_or(0.0, |bbox| bbox.pseudo_area());
    if let Some(area) = bbox_of(extents.area.as_ref()) {
        return area_of(operation.intersection(&area));
    }
    let source = bbox_of(extents.source.as_ref());
    let target = bbox_of(extents.target.as_ref());
    match (source, target) {
        (Some(source), Some(target)) => {
            let x = operation.intersection(&source);
            let y = operation.intersection(&target);
            let both = match (x, y) {
                (Some(x), Some(y)) => x.intersection(&y).map_or(0.0, |bbox| bbox.pseudo_area()),
                _ => 0.0,
            };
            area_of(x) + area_of(y) - both
        }
        (Some(source), None) => area_of(operation.intersection(&source)),
        (None, Some(target)) => area_of(operation.intersection(&target)),
        (None, None) => operation.pseudo_area(),
    }
}

/// The grids of `operation` under the names PROJ reads them by, which is the
/// name the database gives a renamed grid. PROJ's `gridsNeeded` is a `std::set`
/// ordered by the grid's name as the pipeline carries it, so each grid is listed
/// once, in that order.
fn proj_grids(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
) -> Result<Vec<String>, FactoryError> {
    let renamed = proj_grid_names(operation, db)?;
    let mut names: Vec<String> = grid_names(operation)
        .into_iter()
        .map(|grid| {
            renamed
                .iter()
                .find(|(original, _, _)| *original == grid)
                .map_or(grid, |(_, proj, _)| proj.clone())
        })
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

/// `(official name, PROJ name, inverse direction)` of the grids of `operation` that the
/// database renames, as `substitutePROJAlternativeGridNames` writes them.
pub(crate) fn proj_grid_names(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
) -> Result<Vec<(String, String, bool)>, FactoryError> {
    let mut renamed = Vec::new();
    for grid in grid_names(operation) {
        if let Some((proj, inverse)) = db.proj_grid_alternative(&grid)? {
            renamed.push((grid, proj, inverse));
        }
    }
    Ok(renamed)
}

pub(crate) fn grid_names(operation: &CoordinateOperation) -> Vec<String> {
    let mut names = Vec::new();
    collect_grids(operation, &mut names);
    names
}

fn collect_grids(operation: &CoordinateOperation, names: &mut Vec<String>) {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            for parameter in transformation.parameters() {
                // A NADCON pair is one GTiff once its alternative is substituted: only the
                // latitude file counts.
                if parameter.code() == NADCON_LONGITUDE_FILE_PARAMETER {
                    continue;
                }
                if let ParameterValue::Filename(text) = parameter.value() {
                    names.extend(
                        text.split(',')
                            .filter(|name| !name.is_empty() && *name != "null")
                            .map(str::to_owned),
                    );
                }
            }
        }
        CoordinateOperation::Concatenated(concatenated) => {
            for step in concatenated.steps() {
                collect_grids(&step.operation, names);
            }
        }
        CoordinateOperation::Conversion(_) => {}
    }
}

impl CoordinateOperation {
    /// `gridsNeeded`: each grid named by a file parameter of the operation or of its steps,
    /// sorted by name, without repeats (PROJ keeps them in a set of the names).
    ///
    /// `known` is [`GridAvailabilityUse::KnownAvailable`] for `considerKnownGridsAsAvailable`.
    /// The other policies look for the files only.
    ///
    /// # Errors
    ///
    /// The database rejects a grid lookup.
    pub fn grids_needed(
        &self,
        db: &DatabaseContext,
        known: GridAvailabilityUse,
    ) -> Result<Vec<GridDescription>, FactoryError> {
        let mut names = Vec::new();
        collect_file_parameters(self, &mut names);
        names.sort();
        names.dedup();
        names
            .into_iter()
            .map(|short_name| {
                let info = match db.look_for_grid_info(&short_name, known)? {
                    Some(info) => info,
                    None => unrecorded_grid(&short_name),
                };
                Ok(GridDescription { short_name, info })
            })
            .collect()
    }

    /// `isPROJInstantiable`: the operation writes a PROJ string, and each grid it needs is
    /// available. A grid whose name starts with `@` is optional. `known` is as in
    /// [`Self::grids_needed`].
    #[must_use]
    pub fn is_proj_instantiable(&self, db: &DatabaseContext, known: GridAvailabilityUse) -> bool {
        if self
            .export_to_proj_string_with_database(db, ProjLines::Single)
            .is_err()
        {
            return false;
        }
        self.grids_needed(db, known).is_ok_and(|grids| {
            grids
                .iter()
                .all(|grid| grid.info.available || grid.short_name.starts_with('@'))
        })
    }
}

/// The file names of the file parameters of `operation`, then of its steps, in order.
fn collect_file_parameters(operation: &CoordinateOperation, names: &mut Vec<String>) {
    let values: Vec<&ParameterValue> = match operation {
        CoordinateOperation::Transformation(transformation) => transformation
            .parameters()
            .iter()
            .map(Parameter::value)
            .collect(),
        CoordinateOperation::Conversion(conversion) => {
            conversion.parameters.iter().map(Parameter::value).collect()
        }
        CoordinateOperation::Concatenated(concatenated) => {
            for step in concatenated.steps() {
                collect_file_parameters(&step.operation, names);
            }
            return;
        }
    };
    for value in values {
        if let ParameterValue::Filename(text) = value {
            names.extend(
                text.split(',')
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned),
            );
        }
    }
}

/// `lookForGridInfo` for a grid the database has no row for: available when its file is on
/// the search path, and a URL when the name is one.
fn unrecorded_grid(name: &str) -> GridInfo {
    let url = if name.starts_with("http://") || name.starts_with("https://") {
        name.to_owned()
    } else {
        String::new()
    };
    let (full_name, available) = match gridpath::resolve(name) {
        Some(path) if url.is_empty() => (path.to_string_lossy().into_owned(), true),
        _ => (String::new(), false),
    };
    GridInfo {
        full_name,
        package_name: String::new(),
        url,
        direct_download: false,
        open_license: false,
        available,
    }
}

fn step_count(operation: &CoordinateOperation) -> usize {
    match operation {
        CoordinateOperation::Concatenated(concatenated) => concatenated.steps().len(),
        _ => 1,
    }
}

/// Steps of a PROJ pipeline text, one for a single operation.
fn proj_steps(text: &str) -> usize {
    if !text.contains("+proj=pipeline") {
        return 1;
    }
    text.split_whitespace()
        .filter(|token| *token == "+step")
        .count()
}

pub(crate) fn is_null_transformation(name: &str) -> bool {
    if name.contains(" + ") {
        return false;
    }
    [
        BALLPARK_GEOCENTRIC_TRANSLATION,
        BALLPARK_GEOGRAPHIC_OFFSET,
        NULL_GEOGRAPHIC_OFFSET,
        NULL_GEOCENTRIC_TRANSLATION,
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

fn etrs_pair(operation: &CoordinateOperation) -> bool {
    let (source, target) = match operation {
        CoordinateOperation::Transformation(transformation) => {
            (transformation.source_crs(), transformation.target_crs())
        }
        CoordinateOperation::Concatenated(concatenated) => (
            concatenated
                .source_crs()
                .or_else(|| concatenated.steps.first()?.operation.source_crs()),
            concatenated
                .target_crs()
                .or_else(|| concatenated.steps.last()?.operation.target_crs()),
        ),
        CoordinateOperation::Conversion(_) => return false,
    };
    let starts = |crs: Option<&Crs>| crs.is_some_and(|crs| crs.name().starts_with("ETRS89-"));
    starts(source) && starts(target)
}

/// Whether the first axis of a geographic or projected CRS is latitude or northing, so
/// the pipeline writes that axis first. `false` for any other kind.
pub(crate) fn lat_or_northing_first(target: &Crs) -> bool {
    let axes = match target {
        Crs::Geographic(crs) => crs.coordinate_system().axes(),
        Crs::Projected(crs) => crs.coordinate_system().axes(),
        _ => return false,
    };
    axes.first()
        .is_some_and(|axis| matches!(axis.direction.as_str(), "north" | "south"))
}

/// `SortFunction::compare` with `a` and `b` in that order: true when `a` goes first.
fn precedes(a: &Characteristics, b: &Characteristics) -> bool {
    if a.exportable != b.exportable {
        return a.exportable;
    }
    if a.approx != b.approx {
        return !a.approx;
    }
    if a.ballpark_vertical != b.ballpark_vertical {
        return !a.ballpark_vertical;
    }
    if a.null != b.null {
        return !a.null;
    }
    if a.grids_available != b.grids_available {
        return a.grids_available;
    }
    if a.grids_known != b.grids_known {
        return a.grids_known;
    }
    if (a.accuracy >= 0.0) != (b.accuracy >= 0.0) {
        return a.accuracy >= 0.0;
    }
    if a.accuracy < 0.0 && b.accuracy < 0.0 && a.has_grids != b.has_grids {
        return a.has_grids;
    }
    if a.area > 0.0 {
        if a.area > b.area {
            return true;
        }
        if a.area < b.area {
            return false;
        }
    } else if b.area > 0.0 {
        return false;
    }
    if a.accuracy >= 0.0 && a.accuracy < b.accuracy {
        return true;
    }
    if b.accuracy >= 0.0 && b.accuracy < a.accuracy {
        return false;
    }
    if a.accuracy >= 0.0 && a.accuracy == b.accuracy && a.has_grids != b.has_grids {
        return !a.has_grids;
    }
    if a.xyz_gridshift != b.xyz_gridshift {
        return a.xyz_gridshift;
    }
    if a.etrs_pair && b.etrs_pair {
        if b.uses_etrs89_generic && !b.uses_etrf2000 && a.uses_etrf2000 && !a.uses_etrs89_generic {
            return true;
        }
        if a.uses_etrs89_generic && !a.uses_etrf2000 && b.uses_etrf2000 && !b.uses_etrs89_generic {
            return false;
        }
    }
    if a.step_count != b.step_count {
        return a.step_count < b.step_count;
    }
    if a.proj_step_count != 0 && b.proj_step_count != 0 && a.proj_step_count != b.proj_step_count {
        return a.proj_step_count < b.proj_step_count;
    }
    if let Some(order) = ballpark_similarity(&a.name, &b.name) {
        return order;
    }
    if a.name.len() != b.name.len() {
        return a.name.len() < b.name.len();
    }
    if let Some(order) = ntf_preference(&a.name, &b.name) {
        return order;
    }
    a.name > b.name
}

/// Ballpark offsets whose from and to CRS are alike go first.
fn ballpark_similarity(a: &str, b: &str) -> Option<bool> {
    let from_of = |name: &str| {
        let start = name.find(BALLPARK_GEOGRAPHIC_OFFSET)? + BALLPARK_GEOGRAPHIC_OFFSET.len();
        let rest = name[start..].strip_prefix(" from ")?;
        let split = rest.find(" to ")?;
        let (from, after) = rest.split_at(split);
        let after = &after[" to ".len()..];
        let to = match after.find(" + ") {
            Some(end) => &after[..end],
            None => after,
        };
        Some((from.to_owned(), to.to_owned()))
    };
    let (from_a, to_a) = from_of(a)?;
    let (from_b, to_b) = from_of(b)?;
    let similar_a = from_a.starts_with(&to_a) || to_a.starts_with(&from_a);
    let similar_b = from_b.starts_with(&to_b) || to_b.starts_with(&from_b);
    match (similar_a, similar_b) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

fn ntf_preference(a: &str, b: &str) -> Option<bool> {
    const PAIRS: [(&str, &str); 2] = [
        ("NTF (Paris) to NTF (1)", "NTF (Paris) to NTF (2)"),
        (
            "NTF (Paris) to ETRS89-FRA [RGF93 v1] (1)",
            "NTF (Paris) to ETRS89-FRA [RGF93 v1] (2)",
        ),
    ];
    for (first, second) in PAIRS {
        if a.contains(first) && b.contains(second) {
            return Some(true);
        }
        if a.contains(second) && b.contains(first) {
            return Some(false);
        }
    }
    None
}

/// `operations` in `SortFunction` order, as `createOperations` returns them.
pub(crate) fn sort_candidates(
    operations: Vec<CoordinateOperation>,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    availability: GridAvailabilityUse,
    extents: &SortExtents,
) -> Vec<CoordinateOperation> {
    let ranked = operations
        .into_iter()
        .map(|operation| {
            let characteristics =
                characteristics(&operation, db, source, target, availability, extents);
            (operation, characteristics)
        })
        .collect();
    sort(ranked)
        .into_iter()
        .map(|(operation, _)| operation)
        .collect()
}

/// Insertion sort on `precedes`. Each candidate is placed before the first
/// element it precedes, so ties keep their registry order and the result never
/// depends on a comparison that is not a total order.
fn sort(
    candidates: Vec<(CoordinateOperation, Characteristics)>,
) -> Vec<(CoordinateOperation, Characteristics)> {
    let mut sorted: Vec<(CoordinateOperation, Characteristics)> = Vec::new();
    for candidate in candidates {
        let at = sorted
            .iter()
            .position(|other| precedes(&candidate.1, &other.1))
            .unwrap_or(sorted.len());
        sorted.insert(at, candidate);
    }
    sorted
}

/// What `proj_create_crs_to_crs` makes of a pair under a [`BestPolicy`].
#[derive(Debug)]
pub enum Selection {
    /// One operation: PROJ returns it without alternatives.
    Single(BestOperation),
    /// Two or more candidates, chosen per point by `proj_trans`.
    Several(Alternatives),
}

/// `proj_create_crs_to_crs` for `source` to `target` under `policy`: the candidates PROJ keeps
/// (`createOperations`, the grid rerun of `crs_to_crs.cpp:612-773`, and
/// `pj_create_prepared_operations`, `crs_to_crs.cpp:362-466`), or [`Selection::Single`] when
/// there is only one. A CRS that is neither geodetic nor projected is an error: the caller keeps
/// its previous path.
///
/// # Errors
///
/// A CRS cannot be read, or no candidate can be prepared.
pub fn select_alternatives(
    source: &str,
    target: &str,
    policy: BestPolicy,
    filter: CandidateFilter,
) -> Result<Selection, FactoryError> {
    select_alternatives_with_authority(source, target, policy, filter, "")
}

/// [`select_alternatives`], over the candidates of `authority` only: the `AUTHORITY=` option of
/// `proj_create_crs_to_crs`, which `cs2cs --authority` sets (`""` is every authority).
///
/// # Errors
///
/// As [`select_alternatives`].
pub fn select_alternatives_with_authority(
    source: &str,
    target: &str,
    policy: BestPolicy,
    filter: CandidateFilter,
    authority: &str,
) -> Result<Selection, FactoryError> {
    let db = DatabaseContext::create()?;
    let source_crs = create_from_user_input(source, &db)?;
    let target_crs = create_from_user_input(target, &db)?;
    let (Some(source_base), Some(target_base)) =
        (reference_base(&source_crs), reference_base(&target_crs))
    else {
        return Err(FactoryError::Format(format!(
            "{source} or {target} has no geodetic base to prepare its candidates"
        )));
    };
    let mut context = CoordinateOperationContext::create(&db, authority);
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let operations = CoordinateOperationFactory::create()
        .create_operations(&source_crs, &target_crs, &context)?
        .into_iter()
        .filter(|operation| filter.keeps(operation))
        .collect::<Vec<_>>();
    let extents = sort_extents(&context, &source_crs, &target_crs);
    // `proj_create_crs_to_crs` asks KNOWN_AVAILABLE unless ONLY_BEST=NO (DISCARD), and the sort
    // then does not look at the disk.
    let availability = match policy {
        BestPolicy::Discard => GridAvailabilityUse::UseForSorting,
        BestPolicy::Warn | BestPolicy::Error => GridAvailabilityUse::KnownAvailable,
    };
    let ranked = rank(
        operations,
        &db,
        &source_crs,
        &target_crs,
        availability,
        &extents,
    );
    let ranked = match policy {
        BestPolicy::Discard => discard_missing(ranked),
        BestPolicy::Warn | BestPolicy::Error => ranked,
    };
    // `op_count == 1` returns the operation before any preparation (`crs_to_crs.cpp:617-647`).
    if let [(operation, characteristics)] = ranked.as_slice() {
        let best = single_operation(operation, characteristics, &db, &source_crs, &target_crs)?;
        return Ok(Selection::Single(best));
    }
    if ranked.is_empty() {
        return Err(FactoryError::Format(format!(
            "no operation from {source} to {target}"
        )));
    }
    let source_to_crs = geodetic_to_crs(&source_base, &source_crs, &db)?;
    let target_to_crs = geodetic_to_crs(&target_base, &target_crs, &db)?;
    let geocentric = matches!(source_crs, Crs::Geodetic(GeodeticCRS::Geocentric(_)));
    let to_lon_lat = if geocentric {
        Some(geocentric_to_lon_lat(&source_crs, &db)?)
    } else {
        None
    };
    let (source_lon_lat, source_lat_lon) = degree_order(&source_crs);
    let preparer = Preparer {
        db: &db,
        source: &source_crs,
        target: &target_crs,
        source_to_crs: source_to_crs.as_ref(),
        target_to_crs: target_to_crs.as_ref(),
        geocentric,
        source_lon_lat,
        source_lat_lon,
    };
    let mut prepared = preparer.prepare(&ranked);
    if policy != BestPolicy::Discard
        && !prepared
            .iter()
            .any(|item| !item.approximate && item.instantiable)
    {
        // `crs_to_crs.cpp:685-750`: no instantiable candidate that is not a ballpark. PROJ runs
        // `proj_create_operations` again with DISCARD_OPERATION_IF_MISSING_GRID, not a filter of
        // the first list: the registry then drops the operations whose grids are missing, and
        // the intermediate CRS behind them is searched (PD/83 to WGS 84 through ETRS89 for
        // EPSG:4746 to EPSG:4978 with BETA2007 missing). A search that fails leaves the first
        // list, as PROJ's `if (op_list)`.
        let mut discard_context = context.clone();
        discard_context.set_grid_availability_use(GridAvailabilityUse::DiscardIfMissing);
        let discard_ops = CoordinateOperationFactory::create()
            .create_operations(&source_crs, &target_crs, &discard_context)
            .unwrap_or_default();
        let discarded = discard_missing(rank(
            discard_ops,
            &db,
            &source_crs,
            &target_crs,
            GridAvailabilityUse::UseForSorting,
            &extents,
        ));
        let mut kept: Vec<Item> = prepared
            .into_iter()
            .filter(|item| !item.approximate)
            .collect();
        kept.extend(preparer.prepare(&discarded));
        prepared = kept;
    }
    match prepared.len() {
        0 => Err(FactoryError::Format(format!(
            "no operation from {source} to {target}"
        ))),
        1 => {
            let item = &prepared[0];
            let best = match &item.definition {
                _ if !item.instantiable => BestOperation::MissingGrid {
                    message: item.grid_message.clone(),
                },
                Some(definition) => BestOperation::Pipeline {
                    definition: definition.clone(),
                    lat_or_northing_first: lat_or_northing_first(&target_crs),
                },
                None => {
                    return Err(FactoryError::Format(
                        "best operation has no PROJ pipeline".to_owned(),
                    ))
                }
            };
            Ok(Selection::Single(best))
        }
        _ => {
            let source_axes = match to_lon_lat {
                Some(operation) => SourceAxes::Geocentric(operation),
                None => SourceAxes::Plain,
            };
            let first_axis = if lat_or_northing_first(&target_crs) {
                FirstAxis::LatitudeOrNorthing
            } else {
                FirstAxis::Other
            };
            Ok(Selection::Several(Alternatives::new(
                prepared,
                policy,
                source_axes,
                first_axis,
            )))
        }
    }
}

/// `rank` of the candidates: each one with its characteristics, in `SortFunction` order.
fn rank(
    operations: Vec<CoordinateOperation>,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    availability: GridAvailabilityUse,
    extents: &SortExtents,
) -> Vec<(CoordinateOperation, Characteristics)> {
    let ranked = operations
        .into_iter()
        .map(|operation| {
            let characteristics =
                characteristics(&operation, db, source, target, availability, extents);
            (operation, characteristics)
        })
        .collect();
    sort(ranked)
}

/// The single operation of a pair, as the single-pipeline path reads it: its grids checked first,
/// then its PROJ string.
fn single_operation(
    operation: &CoordinateOperation,
    characteristics: &Characteristics,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
) -> Result<BestOperation, FactoryError> {
    let grids = proj_grids(operation, db)?;
    if !grids.iter().all(|grid| gridpath::available(grid)) {
        return Ok(BestOperation::MissingGrid {
            message: missing_grid_message(&characteristics.name, &grids),
        });
    }
    if !characteristics.exportable {
        return Err(FactoryError::Format(format!(
            "best operation {} has no PROJ pipeline",
            characteristics.name
        )));
    }
    Ok(BestOperation::Pipeline {
        definition: export(operation, source, target, db)?,
        lat_or_northing_first: lat_or_northing_first(target),
    })
}

/// `createOperations` with DISCARD: the candidates whose grids are on disk, in the same order.
fn discard_missing(
    ranked: Vec<(CoordinateOperation, Characteristics)>,
) -> Vec<(CoordinateOperation, Characteristics)> {
    ranked
        .into_iter()
        .filter(|(_, characteristics)| characteristics.grids_available)
        .collect()
}

/// The CRS an operation to the CRS is taken from: its longitude and latitude CRS, or the CRS itself
/// when it is geocentric (not reprojected, see [`geodetic_to_crs`]).
fn reference_base(crs: &Crs) -> Option<Crs> {
    match crs {
        Crs::Geodetic(GeodeticCRS::Geocentric(_)) => Some(crs.clone()),
        _ => lon_lat_base(crs),
    }
}

/// The longitude and latitude CRS on the datum of a CRS, as `create_operation_to_geog_crs` makes
/// it (`crs_to_crs.cpp:174-224`). Its axis order is the one an operation from it to the CRS must
/// bridge, so a latitude-first source gets its axis change here. The prime meridian is the CRS's
/// own, not zero as PROJ makes it (not ported). `None` for a CRS with no geodetic datum.
fn lon_lat_base(crs: &Crs) -> Option<Crs> {
    let geographic = match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            geographic
        }
        Crs::Projected(projected) => match projected.base_crs() {
            GeodeticCRS::Geographic(geographic) => geographic,
            _ => return None,
        },
        Crs::Geodetic(_) => return None,
        _ => return None,
    };
    GeographicCRS::create(
        "unnamed crs",
        geographic.datum().cloned(),
        geographic.datum_ensemble().cloned(),
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    )
    .ok()
    .map(Crs::Geographic)
}

/// The operation from the geodetic base of `crs` to `crs`, used to box the area of use of a
/// candidate in the coordinates of `crs` (`create_operation_to_geog_crs`, reversed). A grid-free
/// operation is preferred. `None` when `crs` is geocentric (not reprojected) or is its own base.
fn geodetic_to_crs(
    base: &Crs,
    crs: &Crs,
    db: &DatabaseContext,
) -> Result<Option<Operation>, FactoryError> {
    if matches!(crs, Crs::Geodetic(GeodeticCRS::Geocentric(_))) {
        return Ok(None);
    }
    let context = CoordinateOperationContext::create(db, "");
    let operations = CoordinateOperationFactory::create().create_operations(base, crs, &context)?;
    let chosen = operations
        .iter()
        .find(|operation| grid_names(operation).is_empty())
        .or_else(|| operations.first())
        .ok_or_else(|| {
            FactoryError::Format(format!("no operation from the base of {}", crs.name()))
        })?;
    let text = export_operation(chosen, db)?;
    Operation::create(&text)
        .map(Some)
        .map_err(|error| FactoryError::Format(error.to_string()))
}

/// `create_operation_geocentric_crs_to_geog_crs`: the one operation from a geocentric CRS to
/// longitude and latitude on its own datum.
fn geocentric_to_lon_lat(crs: &Crs, db: &DatabaseContext) -> Result<Operation, FactoryError> {
    let Crs::Geodetic(geocentric) = crs else {
        return Err(FactoryError::Format("not a geocentric CRS".to_owned()));
    };
    let lon_lat = Crs::Geographic(GeographicCRS::create(
        "unnamed crs",
        geocentric.datum().cloned(),
        geocentric.datum_ensemble().cloned(),
        EllipsoidalCS::create_longitude_latitude(UnitOfMeasure::degree()),
    )?);
    let context = CoordinateOperationContext::create(db, "");
    let operations =
        CoordinateOperationFactory::create().create_operations(crs, &lon_lat, &context)?;
    let [operation] = operations.as_slice() else {
        return Err(FactoryError::Format(
            "no single operation from a geocentric CRS to geographic".to_owned(),
        ));
    };
    let text = export_operation(operation, db)?;
    Operation::create(&text).map_err(|error| FactoryError::Format(error.to_string()))
}

/// `IsLonLatOrLatLon` of `PJCoordOperation`: whether the first, and whether the second, axis of
/// a geographic CRS is an east axis in degrees.
fn degree_order(crs: &Crs) -> (bool, bool) {
    let Some(geographic) = super::factory::geographic_of(crs) else {
        return (false, false);
    };
    let axes = geographic.coordinate_system().axes();
    let is_degree_east = |axis: Option<&Axis>| {
        axis.is_some_and(|axis| {
            axis.direction() == "east"
                && (axis.unit().conversion_to_si() - std::f64::consts::PI / 180.0).abs() < 1e-14
        })
    };
    (is_degree_east(axes.first()), is_degree_east(axes.get(1)))
}

/// The per-candidate part of `pj_create_prepared_operations`, with the CRS of the pair.
struct Preparer<'a> {
    db: &'a DatabaseContext,
    source: &'a Crs,
    target: &'a Crs,
    source_to_crs: Option<&'a Operation>,
    target_to_crs: Option<&'a Operation>,
    geocentric: bool,
    source_lon_lat: bool,
    source_lat_lon: bool,
}

impl Preparer<'_> {
    /// One [`Item`] per candidate, two when its box crosses the antimeridian. A piece whose box
    /// cannot be reprojected into the source or the target is dropped (`add_coord_op_to_list`,
    /// `minxSrc <= maxxSrc && minxDst <= maxxDst`).
    fn prepare(&self, ranked: &[(CoordinateOperation, Characteristics)]) -> Vec<Item> {
        let mut items = Vec::new();
        for (operation, characteristics) in ranked {
            let (area, (west, south, east, north)) = area_of_use(operation);
            let pieces = if west <= east {
                vec![(west, east)]
            } else {
                vec![(west, 180.0), (-180.0, east)]
            };
            let grids = proj_grids(operation, self.db).unwrap_or_else(|_| grid_names(operation));
            let instantiable =
                characteristics.exportable && grids.iter().all(|grid| gridpath::available(grid));
            let definition = export(operation, self.source, self.target, self.db).ok();
            for (piece_west, piece_east) in pieces {
                let source_box = if self.geocentric {
                    Some(Bounds {
                        min_x: piece_west,
                        min_y: south,
                        max_x: piece_east,
                        max_y: north,
                    })
                } else {
                    reproject_box(self.source_to_crs, piece_west, south, piece_east, north)
                };
                let target_valid = self.target_to_crs.is_none()
                    || reproject_box(self.target_to_crs, piece_west, south, piece_east, north)
                        .is_some();
                let Some(bounds) = source_box.filter(|_| target_valid) else {
                    continue;
                };
                let w = piece_west.to_radians();
                let e = piece_east.to_radians();
                items.push(Item {
                    name: characteristics.name.clone(),
                    operation: definition
                        .as_deref()
                        .and_then(|text| Operation::create(text).ok()),
                    definition: definition.clone(),
                    instantiable,
                    grid_free: grid_names(operation).is_empty(),
                    grid_message: missing_grid_message(&characteristics.name, &grids),
                    bounds,
                    pseudo_area: (e - w) * (north.to_radians().sin() - south.to_radians().sin()),
                    accuracy: characteristics.accuracy,
                    offshore: area.contains("- offshore"),
                    unknown_area: area.is_empty() || area == "unknown",
                    priority: priority_operation(&characteristics.name),
                    lon_lat_degree: self.source_lon_lat,
                    lat_lon_degree: self.source_lat_lon,
                    approximate: characteristics.approx,
                });
            }
        }
        items
    }
}

/// `reproject_bbox` of `crs_to_crs.cpp:54-100`: the box of a longitude and latitude box in the
/// coordinates of the operation's target, from 20 samples per side of its boundary. The world
/// is not reprojected. `None` when no sample gives a result. With no operation (a geographic
/// CRS in its own axis order) the box is the longitude and latitude one.
fn reproject_box(
    operation: Option<&Operation>,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> Option<Bounds> {
    if west == -180.0 && east == 180.0 && south == -90.0 && north == 90.0 {
        return Some(Bounds {
            min_x: -f64::MAX,
            min_y: -f64::MAX,
            max_x: f64::MAX,
            max_y: f64::MAX,
        });
    }
    let Some(operation) = operation else {
        return Some(Bounds {
            min_x: west,
            min_y: south,
            max_x: east,
            max_y: north,
        });
    };
    const STEPS: u32 = 20;
    let step_lon = (east - west) / f64::from(STEPS);
    let step_lat = (north - south) / f64::from(STEPS);
    let mut samples = Vec::with_capacity(4 * (usize::try_from(STEPS).unwrap_or(0) + 1));
    for j in 0..=STEPS {
        let lon = west + f64::from(j) * step_lon;
        let lat = south + f64::from(j) * step_lat;
        samples.push((lon, south));
        samples.push((lon, north));
        samples.push((west, lat));
        samples.push((east, lat));
    }
    let mut bounds: Option<Bounds> = None;
    for (lon, lat) in samples {
        let Ok(point) = operation.forward(Coord::xy(lon, lat)) else {
            continue;
        };
        let current = bounds.get_or_insert(Bounds {
            min_x: point.x,
            min_y: point.y,
            max_x: point.x,
            max_y: point.y,
        });
        current.min_x = current.min_x.min(point.x);
        current.min_y = current.min_y.min(point.y);
        current.max_x = current.max_x.max(point.x);
        current.max_y = current.max_y.max(point.y);
    }
    bounds
}
