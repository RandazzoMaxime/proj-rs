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

use super::factory::{
    crs_extent, operation_extent, GridAvailabilityUse, GridDescription, GridInfo, SpatialCriterion,
};
use super::model::{BoundingBox, CoordinateOperation, Crs, Parameter, ParameterValue};
use super::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, FactoryError, ProjLines,
};
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
    /// The first operation whose grids are on disk.
    Off,
    /// The first operation, whether or not its grids are on disk.
    On,
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
        BestOperation::MissingGrid { message } => Err(FactoryError::Format(message)),
    }
}

/// The best operation from `source` to `target` as `cs2cs` takes it under
/// `only_best`.
///
/// The order is `SortFunction`'s, grid availability included. Without
/// `ONLY_BEST`, the first operation whose grids are on disk is used, as PROJ
/// falls back to it after a failed grid. With `ONLY_BEST`, the first operation
/// is used even when its grids are missing, and fails at each transformation.
///
/// # Errors
///
/// As [`best_pipeline`].
pub fn best_pipeline_with(
    source: &str,
    target: &str,
    only_best: OnlyBest,
) -> Result<BestOperation, FactoryError> {
    best_operation(source, target, only_best).map(|(best, _)| best)
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
        (BestOperation::MissingGrid { message }, _) => Err(FactoryError::Format(message)),
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

/// [`best_pipeline_with`], and whether the chosen operation is approximate.
fn best_operation(
    source: &str,
    target: &str,
    only_best: OnlyBest,
) -> Result<(BestOperation, bool), FactoryError> {
    best_operation_full(source, target, only_best).map(|chosen| (chosen.best, chosen.approx))
}

/// [`best_operation`], with the chosen operation and the two CRS.
fn best_operation_full(
    source: &str,
    target: &str,
    only_best: OnlyBest,
) -> Result<Chosen, FactoryError> {
    let db = DatabaseContext::create()?;
    let source_crs = create_from_user_input(source, &db)?;
    let target_crs = create_from_user_input(target, &db)?;
    let mut context = CoordinateOperationContext::create(&db, "");
    context.set_spatial_criterion(SpatialCriterion::PartialIntersection);
    let operations = CoordinateOperationFactory::create().create_operations(
        &source_crs,
        &target_crs,
        &context,
    )?;
    let mut ranked = Vec::with_capacity(operations.len());
    for operation in operations {
        let characteristics = characteristics(
            &operation,
            &db,
            &source_crs,
            &target_crs,
            GridSteps::Unwritten,
            GridAvailabilityUse::UseForSorting,
        );
        ranked.push((operation, characteristics));
    }
    let ranked = sort(ranked);
    let instantiable = ranked
        .iter()
        .filter(|(_, characteristics)| {
            characteristics.exportable && characteristics.grids_available
        })
        .count();
    let chosen = match only_best {
        OnlyBest::On => ranked.into_iter().next(),
        OnlyBest::Off => ranked
            .into_iter()
            .find(|(_, characteristics)| characteristics.grids_available),
    };
    let (best, characteristics) = chosen
        .ok_or_else(|| FactoryError::Format(format!("no operation from {source} to {target}")))?;
    if !characteristics.exportable {
        return Err(FactoryError::Format(format!(
            "best operation {} has no PROJ pipeline",
            characteristics.name
        )));
    }
    if !characteristics.grids_available {
        let grids = proj_grids(&best, &db)?;
        return Ok(Chosen {
            best: BestOperation::MissingGrid {
                message: missing_grid_message(&characteristics.name, &grids),
            },
            approx: characteristics.approx,
            operation: best,
            source: source_crs,
            target: target_crs,
            instantiable,
        });
    }
    let definition = export(&best, &source_crs, &target_crs, &db)?;
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
    })
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
        CoordinateOperation::Transformation(transformation) => transformation
            .export_to_proj_string_with(ProjLines::Single, proj_grid_names(operation, db)?),
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .export_to_proj_string_with(ProjLines::Single, proj_grid_names(operation, db)?),
        // A conversion the factory chained carries its CRS pair, which is the operation's
        // own; a bare one keeps the CRS the caller gave.
        CoordinateOperation::Conversion(conversion)
            if conversion.source_crs().is_some() && conversion.target_crs().is_some() =>
        {
            conversion.export_pipeline(ProjLines::Single)
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

/// Whether a candidate's grid and deformation steps are written when its PROJ string is
/// taken for ordering. PROJ's `isPROJExportable` writes them, so the candidate list of
/// `createOperations` follows it. The `cs2cs` choice writes them only where it has
/// always written them, so its pipeline is unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridSteps {
    /// Grid and deformation steps are written, as `isPROJExportable` takes them.
    Written,
    /// Grid and deformation steps are not written: the `cs2cs` choice.
    Unwritten,
}

fn characteristics(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
    grids: GridSteps,
    availability: GridAvailabilityUse,
) -> Characteristics {
    let name = operation.name().to_owned();
    let text = match (grids, operation) {
        (
            GridSteps::Written,
            CoordinateOperation::Transformation(_) | CoordinateOperation::Concatenated(_),
        ) => operation.export_to_proj_string_with_database(db, ProjLines::Single),
        _ => export(operation, source, target, db),
    };
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
    let (area_source, area_target) = (crs_extent(source), crs_extent(target));
    let op_box = operation_extent(operation).and_then(|extent| extent.bounding_box());
    let area = pseudo_area_of(
        op_box,
        area_source.and_then(|extent| extent.bounding_box()),
        area_target.and_then(|extent| extent.bounding_box()),
    );
    let accuracy = match operation {
        CoordinateOperation::Conversion(_) => 0.0,
        _ => operation.accuracy().unwrap_or(-1.0),
    };
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

/// `getPseudoArea` of the operation extent, intersected with the CRS extents
/// as PROJ does when no area of interest is given.
fn pseudo_area_of(
    operation: Option<BoundingBox>,
    source: Option<BoundingBox>,
    target: Option<BoundingBox>,
) -> f64 {
    let Some(operation) = operation else {
        return 0.0;
    };
    let area_of = |bbox: Option<BoundingBox>| bbox.map_or(0.0, |bbox| bbox.pseudo_area());
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
/// name the database gives a renamed grid.
fn proj_grids(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
) -> Result<Vec<String>, FactoryError> {
    let renamed = proj_grid_names(operation, db)?;
    Ok(grid_names(operation)
        .into_iter()
        .map(|grid| {
            renamed
                .iter()
                .find(|(original, _)| *original == grid)
                .map_or(grid, |(_, proj)| proj.clone())
        })
        .collect())
}

/// `(official name, PROJ name)` of the grids of `operation` that the database
/// renames, as `substitutePROJAlternativeGridNames` writes them.
pub(crate) fn proj_grid_names(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
) -> Result<Vec<(String, String)>, FactoryError> {
    let mut renamed = Vec::new();
    for grid in grid_names(operation) {
        if let Some(proj) = db.proj_grid_name(&grid)? {
            renamed.push((grid, proj));
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
        CoordinateOperation::Concatenated(concatenated) => {
            (concatenated.source_crs(), concatenated.target_crs())
        }
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
) -> Vec<CoordinateOperation> {
    let ranked = operations
        .into_iter()
        .map(|operation| {
            let characteristics = characteristics(
                &operation,
                db,
                source,
                target,
                GridSteps::Written,
                availability,
            );
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
