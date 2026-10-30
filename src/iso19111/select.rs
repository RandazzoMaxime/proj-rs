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

use super::factory::{crs_extent, operation_extent};
use super::model::{BoundingBox, CoordinateOperation, Crs, ParameterValue};
use super::{
    create_from_user_input, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, FactoryError, ProjLines,
};
use crate::gridpath;

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
    let db = DatabaseContext::create()?;
    let source_crs = create_from_user_input(source, &db)?;
    let target_crs = create_from_user_input(target, &db)?;
    let context = CoordinateOperationContext::create(&db, "");
    let operations = CoordinateOperationFactory::create().create_operations(
        &source_crs,
        &target_crs,
        &context,
    )?;
    let mut ranked = Vec::with_capacity(operations.len());
    for operation in operations {
        let characteristics = characteristics(&operation, &db, &source_crs, &target_crs);
        ranked.push((operation, characteristics));
    }
    let ranked = sort(ranked);
    let (best, characteristics) = ranked
        .into_iter()
        .next()
        .ok_or_else(|| FactoryError::Format(format!("no operation from {source} to {target}")))?;
    if !characteristics.exportable {
        return Err(FactoryError::Format(format!(
            "best operation {} has no PROJ pipeline",
            characteristics.name
        )));
    }
    let definition = export(&best, &source_crs, &target_crs)?;
    Ok((definition, lat_or_northing_first(&target_crs)))
}

fn export(
    operation: &CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> Result<String, FactoryError> {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
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

fn characteristics(
    operation: &CoordinateOperation,
    db: &DatabaseContext,
    source: &Crs,
    target: &Crs,
) -> Characteristics {
    let name = operation.name().to_owned();
    let (exportable, proj_step_count, xyz_gridshift) = match export(operation, source, target) {
        Ok(text) => {
            let xyz = text.contains("+proj=xyzgridshift");
            let steps = if xyz { 0 } else { proj_steps(&text) };
            (true, steps, xyz)
        }
        Err(_) => (false, 0, false),
    };
    let grids = grid_names(operation);
    let grids_available = grids.iter().all(|grid| gridpath::available(grid));
    let grids_known = grids
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

pub(crate) fn grid_names(operation: &CoordinateOperation) -> Vec<String> {
    let mut names = Vec::new();
    collect_grids(operation, &mut names);
    names
}

fn collect_grids(operation: &CoordinateOperation, names: &mut Vec<String>) {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            for parameter in transformation.parameters() {
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

fn lat_or_northing_first(target: &Crs) -> bool {
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
        return !a.xyz_gridshift;
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
) -> Vec<CoordinateOperation> {
    let ranked = operations
        .into_iter()
        .map(|operation| {
            let characteristics = characteristics(&operation, db, source, target);
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
