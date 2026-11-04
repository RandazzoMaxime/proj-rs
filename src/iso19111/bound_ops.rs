//! `createOperationsBoundToGeog` of PROJ's `coordinateoperationfactory.cpp`: a bound CRS to a
//! geographic CRS, chaining the base CRS, the binding transformation, and the hub to the target.
//! A geodetic source and a bound target is its inverse (`applyInverse`).
//!
//! `createOperationsBoundToBound` of the same file: two bound CRS with the same geographic hub
//! chain the source to its hub and the hub to the target.
//!
//! `createOperationsToGeod`: a bound CRS to a geodetic CRS that is not geographic, through a
//! geographic 3D CRS on the target's datum.
//!
//! Not ported: the Clarke 1866 to NAD83 special case, the vertical unit change that
//! `skipHorizontalTransformation` adds, and the vertical blocks of `createOperationsBoundToBound`
//! (same vertical datum, same hub for vertical bases).

use super::bound::extract_geographic;
use super::equiv::{
    datum_equivalent_to_frame, ellipsoid_equivalent_loose, geodetic_frame, same_geodetic_datum,
    Equivalence,
};
use super::factory::{geographic_of, CoordinateOperationContext, CoordinateOperationFactory};
use super::model::{
    BoundCRS, ConcatenatedOperation, ConcatenatedStep, CoordinateOperation, Crs, Ellipsoid,
    EllipsoidalCS, GeodeticCRS, GeodeticReferenceFrame, GeographicCRS, Identified, UnitOfMeasure,
};
use super::niche::DerivedKind;
use super::FactoryError;

/// Candidates for a pair where one side is a bound CRS. `None` for any other pair.
pub(super) fn bound_candidates(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Option<Vec<CoordinateOperation>>, FactoryError> {
    match (source, target) {
        (Crs::Bound(bound), Crs::Bound(target_bound)) => {
            bound_to_bound(factory, source, bound, target, target_bound, context).map(Some)
        }
        (Crs::Bound(bound), _) if geographic_of(target).is_some() => {
            bound_to_geog(factory, source, bound, target, context).map(Some)
        }
        (Crs::Bound(_), Crs::Geodetic(geodetic)) => {
            bound_to_geod(factory, source, geodetic, target, context).map(Some)
        }
        (_, Crs::Bound(_))
            if geographic_of(source).is_some()
                || matches!(source, Crs::Geodetic(GeodeticCRS::Geocentric(_))) =>
        {
            let forward = factory.create_candidates(target, source, context)?;
            Ok(Some(
                forward.iter().map(CoordinateOperation::inverse).collect(),
            ))
        }
        _ => Ok(None),
    }
}

/// `createOperationsBoundToBound`, without its vertical blocks: when both hubs are the same
/// geographic CRS, the source to its hub, then the hub to the target. Otherwise the base
/// CRS of the source to the base CRS of the target.
fn bound_to_bound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    source_bound: &BoundCRS,
    target: &Crs,
    target_bound: &BoundCRS,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let hub_src = source_bound.target_crs();
    let hub_dst = target_bound.target_crs();
    if let (Some(hub_src_geog), Some(hub_dst_geog)) =
        (geographic_of(hub_src), geographic_of(hub_dst))
    {
        if hub_src_geog.is_equivalent_to_with(hub_dst_geog, Equivalence::Equivalent) {
            let firsts = factory.create_candidates(source, hub_src, context)?;
            let lasts = factory.create_candidates(hub_src, target, context)?;
            let mut found = Vec::new();
            for first in &firsts {
                for last in &lasts {
                    found.push(concatenate(vec![first.clone(), last.clone()]));
                }
            }
            if !found.is_empty() {
                return Ok(found);
            }
        }
    }
    factory.create_candidates(
        source_bound.source_crs(),
        target_bound.source_crs(),
        context,
    )
}

/// `createOperationsToGeod`: the source to a geographic 3D CRS named and datum-ed as the
/// target, then that CRS to the target. The first step takes that pair (`setCRSs`), so a
/// Helmert step whose target is 3D writes no `push v_3`/`pop v_3`.
///
/// The C++ intermediate also has the WORLD domain of validity. It is not set here: with it,
/// the datum pivot of this port drops the Helmert to ETRS89 (an area of Europe), and the
/// chain changes.
pub(super) fn bound_to_geod(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    geodetic: &GeodeticCRS,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let intermediate = Crs::Geographic(GeographicCRS::create(
        geodetic.name(),
        geodetic.datum().cloned(),
        geodetic.datum_ensemble().cloned(),
        EllipsoidalCS::create_latitude_longitude_height(
            UnitOfMeasure::degree(),
            UnitOfMeasure::metre(),
        ),
    )?);
    let firsts = factory.create_candidates(source, &intermediate, context)?;
    let lasts = factory.create_candidates(&intermediate, target, context)?;
    let Some(last) = lasts.first() else {
        return Ok(Vec::new());
    };
    Ok(firsts
        .into_iter()
        .map(|mut first| {
            set_endpoints(&mut first, source, &intermediate);
            concatenate(vec![first, last.clone()])
        })
        .collect())
}

fn bound_to_geog(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    bound: &BoundCRS,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(geog_dst) = geographic_of(target) else {
        return Ok(Vec::new());
    };
    let base = bound.source_crs();
    let hub = bound.target_crs();
    let hub_geog = geographic_of(hub);
    let base_geog = match base {
        // A derived geographic CRS is replaced by its base, as `createOperationsBoundToGeog` does.
        Crs::Derived(derived) if derived.kind() == DerivedKind::Geographic => {
            Some(derived.base_crs().clone()).filter(|crs| geographic_of(crs).is_some())
        }
        _ => extract_geographic(base).map(|geog| Crs::Geographic(geog.clone())),
    };
    let bound_transformation = with_endpoints(bound);
    let transformation = &bound_transformation;

    if let Some(base_geog) = &base_geog {
        if same_geodetic_datum(base_geog, target) {
            let mut found = factory.create_candidates(base, target, context)?;
            if found.is_empty() {
                found.extend(super::compound::null_offset(base, target));
            }
            return Ok(found);
        }
    }

    let mut tried_hub_is_target = false;
    if let (Some(hub_geog), Some(base_geog)) = (hub_geog, &base_geog) {
        let intermediate = intermediate_operation(factory, base_geog, transformation, context)?;
        if hub_geog.is_equivalent_to_with(geog_dst, Equivalence::Equivalent)
            || hub_geog.is_2d_part_of_3d(geog_dst)
        {
            tried_hub_is_target = true;
            if matches!(base, Crs::Geographic(_) | Crs::Geodetic(_)) {
                return Ok(vec![match intermediate {
                    Some(first) => concatenate(vec![first, transformation.clone()]),
                    None => transformation.clone(),
                }]);
            }
            let mut found = Vec::new();
            for first in factory.create_candidates(base, base_geog, context)? {
                let mut steps = vec![first];
                steps.extend(intermediate.clone());
                steps.push(transformation.clone());
                found.push(concatenate(steps));
            }
            if !found.is_empty() {
                return Ok(found);
            }
        } else if same_geodetic_datum(hub, target) {
            let firsts = factory.create_candidates(base, base_geog, context)?;
            let lasts = factory.create_candidates(hub, target, context)?;
            let mut found = Vec::new();
            if !firsts.is_empty() && !lasts.is_empty() {
                for first in &firsts {
                    for last in &lasts {
                        let mut steps = vec![first.clone()];
                        steps.extend(intermediate.clone());
                        steps.push(transformation.clone());
                        steps.push(last.clone());
                        found.push(concatenate(steps));
                    }
                }
            }
            if !found.is_empty() {
                return Ok(found);
            }
        } else if is_clarke_66_nad27_to_nad83(base_geog, hub, target) {
            if base.is_equivalent_to(base_geog) {
                return Ok(vec![clarke_66_transformation(transformation, base, target)]);
            }
            let found: Vec<CoordinateOperation> = factory
                .create_candidates(base, base_geog, context)?
                .into_iter()
                .map(|first| {
                    let transf = clarke_66_transformation(transformation, base_geog, target);
                    concatenate(vec![first, transf])
                })
                .collect();
            if !found.is_empty() {
                return Ok(found);
            }
        }
    }

    if let (Some(hub_geog), Crs::Vertical(_)) = (hub_geog, base) {
        if hub_geog.is_equivalent_to_with(geog_dst, Equivalence::Equivalent) {
            let Some(vertical_source) = operation_source(transformation) else {
                return Ok(vec![transformation.clone()]);
            };
            if matches!(vertical_source, Crs::Vertical(_))
                && !base.is_equivalent_to(vertical_source)
            {
                return Ok(factory
                    .create_candidates(base, vertical_source, context)?
                    .into_iter()
                    .map(|first| concatenate(vec![first, transformation.clone()]))
                    .collect());
            }
            return Ok(vec![transformation.clone()]);
        }
    }

    if let (false, Some(_), Some(_)) = (tried_hub_is_target, hub_geog, &base_geog) {
        let firsts = factory.create_candidates(source, hub, context)?;
        let lasts = factory.create_candidates(hub, target, context)?;
        let mut found = Vec::new();
        for first in &firsts {
            for last in &lasts {
                if lasts.len() > 1 || !last.has_ballpark_transformation() {
                    found.push(concatenate(vec![first.clone(), last.clone()]));
                }
            }
        }
        if !found.is_empty() {
            return Ok(found);
        }
    }

    if let (Crs::Vertical(_), Some(_)) = (base, hub_geog) {
        let firsts = factory.create_candidates(source, hub, context)?;
        let lasts = factory.create_candidates(hub, target, context)?;
        let mut found = Vec::new();
        for first in &firsts {
            for last in &lasts {
                if !last.has_ballpark_transformation() {
                    found.push(concatenate(vec![first.clone(), last.clone()]));
                }
            }
        }
        if !found.is_empty() {
            return Ok(found);
        }
    }

    factory.create_candidates(base, target, context)
}

/// `createOperationsBoundToGeog`, the Clarke 1866 case: a base on the Clarke 1866 ellipsoid
/// bound to WGS 84 (`+nadgrids=ntv1_can.dat,conus`), and a NAD83 target. NAD27 and NAD83 are
/// taken as the same datum for that binding.
fn is_clarke_66_nad27_to_nad83(base_geog: &Crs, hub: &Crs, target: &Crs) -> bool {
    geodetic_frame(base_geog).is_some_and(|frame| {
        ellipsoid_equivalent_loose(frame.ellipsoid(), &Ellipsoid::clarke_1866())
    }) && datum_equivalent_to_frame(hub, &GeodeticReferenceFrame::epsg_6326())
        && datum_equivalent_to_frame(target, &GeodeticReferenceFrame::epsg_6269())
}

/// The binding transformation of the Clarke 1866 case, named and set from `source` to
/// `target` (`setCRSs`).
fn clarke_66_transformation(
    transformation: &CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> CoordinateOperation {
    let mut transf = transformation.clone();
    set_endpoints(&mut transf, source, target);
    let name = format!("Transformation from {} to {}", source.name(), target.name());
    match &mut transf {
        CoordinateOperation::Conversion(conversion) => conversion.identified.name = name,
        CoordinateOperation::Transformation(transformation) => {
            transformation.identified.name = name
        }
        CoordinateOperation::Concatenated(concatenated) => concatenated.identified.name = name,
    }
    transf
}

/// The operation from the geographic CRS of the base to the source of the binding
/// transformation, when they are not equivalent.
fn intermediate_operation(
    factory: &CoordinateOperationFactory,
    base_geog: &Crs,
    transformation: &CoordinateOperation,
    context: &CoordinateOperationContext,
) -> Result<Option<CoordinateOperation>, FactoryError> {
    let Some(transformation_source) = operation_source(transformation) else {
        return Ok(None);
    };
    if crs_equivalent(base_geog, transformation_source) {
        return Ok(None);
    }
    Ok(factory
        .create_candidates(base_geog, transformation_source, context)?
        .into_iter()
        .next())
}

pub(super) fn operation_source(operation: &CoordinateOperation) -> Option<&Crs> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.source_crs(),
        CoordinateOperation::Transformation(transformation) => transformation.source_crs(),
        CoordinateOperation::Concatenated(concatenated) => concatenated.source_crs(),
    }
}

pub(super) fn operation_target(operation: &CoordinateOperation) -> Option<&Crs> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.target_crs(),
        CoordinateOperation::Transformation(transformation) => transformation.target_crs(),
        CoordinateOperation::Concatenated(concatenated) => concatenated.target_crs(),
    }
}

fn crs_equivalent(left: &Crs, right: &Crs) -> bool {
    match (geographic_of(left), geographic_of(right)) {
        (Some(left), Some(right)) => left.is_equivalent_to_with(right, Equivalence::Equivalent),
        _ => left.is_equivalent_to(right),
    }
}

/// `CoordinateOperation::setCRSs`: the source and target the operation stores.
fn set_endpoints(operation: &mut CoordinateOperation, source: &Crs, target: &Crs) {
    let (stored_source, stored_target) = match operation {
        CoordinateOperation::Conversion(conversion) => {
            (&mut conversion.source, &mut conversion.target)
        }
        CoordinateOperation::Transformation(transformation) => {
            (&mut transformation.source, &mut transformation.target)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            (&mut concatenated.source, &mut concatenated.target)
        }
    };
    *stored_source = Some(Box::new(source.clone()));
    *stored_target = Some(Box::new(target.clone()));
}

/// `ConcatenatedOperation::createComputeMetadata`: the steps, named by joining theirs.
fn concatenate(operations: Vec<CoordinateOperation>) -> CoordinateOperation {
    let name = operations
        .iter()
        .map(CoordinateOperation::name)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = operations
        .first()
        .and_then(operation_source)
        .cloned()
        .map(Box::new);
    let target = operations
        .last()
        .and_then(operation_target)
        .cloned()
        .map(Box::new);
    CoordinateOperation::Concatenated(ConcatenatedOperation {
        identified: Identified::named(name),
        steps: operations
            .into_iter()
            .map(|operation| ConcatenatedStep {
                operation,
                reverse: false,
            })
            .collect(),
        source,
        target,
        accuracy: None,
        version: None,
        computed_name: true,
        source_epoch: None,
        target_epoch: None,
        proj_based: None,
    })
}

/// The binding transformation, with the bound CRS endpoints when it carries none. A bound
/// projected CRS binds the geographic CRS of its base, as `createFromTOWGS84` does.
fn with_endpoints(bound: &BoundCRS) -> CoordinateOperation {
    let mut operation = bound.transformation().clone();
    if let CoordinateOperation::Transformation(transformation) = &mut operation {
        let base = bound.source_crs();
        let source = match (base, extract_geographic(base)) {
            (Crs::Projected(_), Some(geographic)) => Crs::Geographic(geographic.clone()),
            _ => base.clone(),
        };
        if transformation.source.is_none()
            || matches!(transformation.source.as_deref(), Some(Crs::Projected(_)))
        {
            transformation.source = Some(Box::new(source));
        }
        if transformation.target.is_none() {
            transformation.target = Some(Box::new(bound.target_crs().clone()));
        }
    }
    operation
}
