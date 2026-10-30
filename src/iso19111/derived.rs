//! The derived-CRS branches of PROJ's `createOperations`
//! (`coordinateoperationfactory.cpp`). A projected CRS is a derived CRS: its deriving
//! conversion, inverted, is chained with the operations from its base CRS
//! (`createOperationsDerivedTo`). For a projected target, the answer is the inverse of
//! the operations from the target to the source.
//!
//! Not ported: the recursion limit of `createOperations`, and the epochs.

use super::factory::{
    intersect_extents, operation_extent, pivot_concatenation, CoordinateOperationContext,
    CoordinateOperationFactory,
};
use super::model::{Conversion, CoordinateOperation, Crs, ObjectDomain};
use super::FactoryError;

/// Candidates when `source` or `target` is a derived CRS, or `None` when neither is.
/// A derived source has precedence, as in PROJ.
pub(super) fn derived_candidates(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Option<Vec<CoordinateOperation>>, FactoryError> {
    if let Some((conversion, base)) = derived_parts(source) {
        return derived_to(factory, conversion, &base, target, context).map(Some);
    }
    if derived_parts(target).is_none() {
        return Ok(None);
    }
    let forward = factory.create_candidates(target, source, context)?;
    Ok(Some(
        forward.iter().map(CoordinateOperation::inverse).collect(),
    ))
}

/// The deriving conversion of a derived CRS, and its base CRS.
fn derived_parts(crs: &Crs) -> Option<(&Conversion, Crs)> {
    match crs {
        Crs::Projected(projected) => Some((
            projected.deriving_conversion(),
            Crs::Geodetic(projected.base_crs().clone()),
        )),
        Crs::DerivedProjected(derived) => Some((
            derived.deriving_conversion(),
            Crs::Projected(derived.base_crs().clone()),
        )),
        _ => None,
    }
}

/// `createOperationsDerivedTo`: the inverse of the deriving conversion, chained with each
/// operation from the base CRS to `target`. A base equal to the target gives the inverse
/// conversion alone.
fn derived_to(
    factory: &CoordinateOperationFactory,
    conversion: &Conversion,
    base: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let first = CoordinateOperation::Conversion(conversion.inverse());
    if base.is_equivalent_to(target) {
        return Ok(vec![first]);
    }
    let mut found = Vec::new();
    for second in factory.create_candidates(base, target, context)? {
        if let Some(operation) = concatenate(&[first.clone(), second], context) {
            found.push(operation);
        }
    }
    Ok(found)
}

/// `ConcatenatedOperation::createComputeMetadata`: the steps of a concatenated operation
/// are flattened in, and the extent is the intersection of the steps'. `None` when the
/// extents do not meet and the context disallows that, which PROJ skips silently.
fn concatenate(
    operations: &[CoordinateOperation],
    context: &CoordinateOperationContext,
) -> Option<CoordinateOperation> {
    let mut steps = Vec::new();
    for operation in operations {
        match operation {
            CoordinateOperation::Concatenated(concatenated) => {
                steps.extend(concatenated.steps.iter().map(|step| step.operation.clone()));
            }
            other => steps.push(other.clone()),
        }
    }
    let extent = match joint_extent(&steps) {
        Some(extent) => extent,
        None if context.disallows_empty_intersection() => return None,
        None => None,
    };
    let mut concatenated = pivot_concatenation(steps);
    if let Some(domain) = extent {
        concatenated.identified.domains = vec![domain];
    }
    Some(CoordinateOperation::Concatenated(concatenated))
}

/// The intersection of the steps' extents, as `getExtent` takes it. The outer `None` is an
/// empty intersection. A step without an extent is skipped.
fn joint_extent(steps: &[CoordinateOperation]) -> Option<Option<ObjectDomain>> {
    let mut extent: Option<ObjectDomain> = None;
    for part in steps.iter().filter_map(operation_extent) {
        extent = match extent {
            None => Some(part),
            Some(sofar) => Some(intersect_extents(sofar, part)?),
        };
    }
    Some(extent)
}
