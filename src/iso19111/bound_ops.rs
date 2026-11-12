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
//! `createOperationsBoundToCompound`: a bound CRS to a compound CRS, in its three cases, then the
//! base CRS of the source to the compound CRS.
//!
//! Not ported: the Clarke 1866 to NAD83 special case and the vertical blocks of
//! `createOperationsBoundToBound` (same vertical datum, same hub for vertical bases).

use super::bound::extract_geographic;
use super::bound_vert::bound_to_vert;
use super::buildop::OpProps;
use super::compound::compute_metadata;
use super::equiv::{
    datum_equivalent_to_frame, ellipsoid_equivalent_loose, geodetic_frame, same_geodetic_datum,
    Equivalence,
};
use super::factory::{
    geographic_of, CoordinateOperationContext, CoordinateOperationFactory, GUARD_SKIP_HORIZONTAL,
};
use super::model::{
    Axis, BoundCRS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, Crs, Ellipsoid, EllipsoidalCS, GeodeticCRS, GeodeticReferenceFrame,
    GeographicCRS, Identified, UnitOfMeasure,
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
        (Crs::Bound(bound), Crs::Compound(compound)) => {
            bound_to_compound(factory, source, bound, target, compound, context).map(Some)
        }
        (Crs::Bound(bound), Crs::Vertical(_)) => {
            bound_to_vert(factory, bound, target, context).map(Some)
        }
        (Crs::Bound(bound), _) if geographic_of(target).is_some() => {
            bound_to_geog(factory, source, bound, target, context).map(Some)
        }
        (Crs::Bound(_), Crs::Geodetic(geodetic)) => {
            bound_to_geod(factory, source, geodetic, target, context).map(Some)
        }
        (_, Crs::Bound(_))
            if geographic_of(source).is_some()
                || matches!(
                    source,
                    Crs::Geodetic(GeodeticCRS::Geocentric(_)) | Crs::Vertical(_)
                ) =>
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
            // `createComputeMetadata({opFirst, opLast})`: the steps are flattened and consecutive
            // inverse steps cancel, so a bound CRS to a bound CRS with the same hub and the same
            // transformation is the operation between their bases (coordinateoperationfactory.cpp:
            // 6228-6240).
            for first in &firsts {
                for last in &lasts {
                    found.extend(compute_metadata(vec![first.clone(), last.clone()], context));
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

/// `createOperationsBoundToCompound`: a bound CRS to a compound CRS. Three cases, in PROJ's
/// order, then the base CRS of the source to the compound CRS.
///
/// - Two bound components, the first with a geographic hub and the second a vertical base with
///   a geographic hub, and the source base and the first component on the datum of that hub:
///   the source to the compound of the first component and the second, with its endpoints set
///   to the source and the target.
/// - A bound first component whose hub is on the datum of the source's hub: the source to a
///   geographic 3D CRS on that datum, then that CRS to the target. Where the two steps do not
///   meet, an adapter between them is used when one exists.
/// - The source base is 3D, its hub is 3D, and the first component is on another datum: the
///   source to its hub, then the hub to the target.
///
/// The PROJ example of the first case is `BOUND[NAD83_3D,TOWGS84]` to
/// `COMPOUND[BOUND[NAD83_2D,TOWGS84],BOUND[VERT[NAVD88],HUB=NAD83_3D,GRID]]`.
fn bound_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    bound_src: &BoundCRS,
    target: &Crs,
    compound_dst: &CompoundCRS,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let components = compound_dst.components();
    let mut res = Vec::new();

    // The TOWGS84 bound of the source is ignored: the source goes to the compound directly.
    if let [Crs::Bound(comp0), Crs::Bound(comp1), ..] = components {
        let hub1 = comp1.target_crs();
        if let (Some(src_geodetic), Some(comp0_geog), true, true) = (
            extract_geodetic(bound_src.source_crs()),
            extract_geographic(&components[0]),
            matches!(comp1.source_crs(), Crs::Vertical(_)),
            geographic_of(hub1).is_some(),
        ) {
            let comp0_geog = Crs::Geographic(comp0_geog.clone());
            if datums_equivalent(&src_geodetic, hub1) && datums_equivalent(&comp0_geog, hub1) {
                let src_new = bound_src.source_crs().clone();
                let name = format!("{} + {}", comp0.name(), components[1].name());
                let dst_new = Crs::Compound(CompoundCRS::create(
                    name,
                    vec![components[0].clone(), components[1].clone()],
                )?);
                for mut operation in factory.create_candidates(&src_new, &dst_new, context)? {
                    set_endpoints(&mut operation, source, target);
                    res.push(operation);
                }
                return Ok(res);
            }
        }
    }

    if let Some(first) = components.first() {
        // A bound first component whose hub is on the datum of the source's hub.
        if let Crs::Bound(comp0) = first {
            if let (Some(src_hub), Some(_)) = (
                geographic_of(bound_src.target_crs()),
                geographic_of(comp0.target_crs()),
            ) {
                if datums_equivalent(bound_src.target_crs(), comp0.target_crs()) {
                    let interm = Crs::Geographic(GeographicCRS::create(
                        src_hub.name(),
                        src_hub.datum().cloned(),
                        src_hub.datum_ensemble().cloned(),
                        EllipsoidalCS::create_latitude_longitude_height(
                            UnitOfMeasure::degree(),
                            UnitOfMeasure::metre(),
                        ),
                    )?);
                    let source_to_interm = factory.create_candidates(source, &interm, context)?;
                    let interm_to_target = factory.create_candidates(&interm, target, context)?;
                    for op_src in &source_to_interm {
                        for op_dst in &interm_to_target {
                            // The two steps do not meet when one is 2D and the other 3D: an
                            // adapter operation between them, when one exists.
                            let steps = match (op_src.target_crs(), op_dst.source_crs()) {
                                (Some(mid), Some(next)) if !crs_equivalent(mid, next) => factory
                                    .create_candidates(mid, next, context)?
                                    .first()
                                    .map(|adapter| {
                                        vec![op_src.clone(), adapter.clone(), op_dst.clone()]
                                    }),
                                _ => Some(vec![op_src.clone(), op_dst.clone()]),
                            };
                            if let Some(operation) =
                                steps.and_then(|steps| compute_metadata(steps, context))
                            {
                                res.push(operation);
                            }
                        }
                    }
                    return Ok(res);
                }
            }
        }

        // e.g transforming from a Bound[something_not_WGS84_but_with_TOWGS84] to a
        // CompoundCRS[WGS84, ...]
        let src_geog = extract_geographic(bound_src.source_crs());
        let hub_geog = geographic_of(bound_src.target_crs());
        let comp0_geog = extract_geographic(first);
        if let (Some(src_geog), Some(hub_geog), Some(comp0_geog)) = (src_geog, hub_geog, comp0_geog)
        {
            let src_geog = Crs::Geographic(src_geog.clone());
            let comp0_geog = Crs::Geographic(comp0_geog.clone());
            if single_axis_count(bound_src.source_crs()) == Some(3)
                && hub_geog.coordinate_system().axes().len() == 3
                && !datums_equivalent(&src_geog, &comp0_geog)
                && datums_equivalent(&Crs::Geographic(hub_geog.clone()), &comp0_geog)
            {
                let ops1 = factory.create_candidates(source, bound_src.target_crs(), context)?;
                let ops2 = factory.create_candidates(bound_src.target_crs(), target, context)?;
                for op1 in &ops1 {
                    for op2 in &ops2 {
                        if let Some(operation) =
                            compute_metadata(vec![op1.clone(), op2.clone()], context)
                        {
                            res.push(operation);
                        }
                    }
                }
                if !res.is_empty() {
                    return Ok(res);
                }
            }
        }
    }

    // There might be better things to do, but for now just ignore the transformation of the
    // bound CRS.
    factory.create_candidates(bound_src.source_crs(), target, context)
}

/// `CRS::extractGeodeticCRS` for the kinds this port binds: a geodetic CRS, or the base of a
/// projected CRS.
fn extract_geodetic(crs: &Crs) -> Option<Crs> {
    match crs {
        Crs::Geographic(_) | Crs::Geodetic(_) => Some(crs.clone()),
        Crs::Projected(projected) => Some(Crs::Geodetic(projected.base_crs().clone())),
        _ => None,
    }
}

/// `isEquivalentTo(EQUIVALENT)` of the datum of `left` with the datum of `right`.
fn datums_equivalent(left: &Crs, right: &Crs) -> bool {
    geodetic_frame(left).is_some_and(|frame| datum_equivalent_to_frame(right, &frame))
}

/// `coordinateSystem()->axisList().size()` of a single CRS of the kinds this port binds, and
/// `None` for the other kinds.
fn single_axis_count(crs: &Crs) -> Option<usize> {
    match crs {
        Crs::Geographic(geographic) | Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Some(geographic.coordinate_system().axes().len())
        }
        Crs::Geodetic(GeodeticCRS::Geocentric(_)) => Some(3),
        Crs::Projected(projected) => Some(projected.coordinate_system().axes().len()),
        Crs::Vertical(_) => Some(1),
        _ => None,
    }
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

    if let (Crs::Vertical(_), Some(hub_geog)) = (base, hub_geog) {
        let firsts = factory.create_candidates(source, hub, context)?;
        // `skipHorizontalTransformation` (coordinateoperationfactory.cpp:5861-5914): the vertical
        // transformations of `createOperationsCompoundToGeog` stop at the hub, with a change of
        // vertical unit when the height axis of the hub is not the one of the target.
        if context.has_guard(GUARD_SKIP_HORIZONTAL) {
            return skip_horizontal_transformation(firsts, hub, hub_geog, geog_dst);
        }
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

/// The `skipHorizontalTransformation` branch of `createOperationsBoundToGeog` (coordinate
/// operationfactory.cpp:5861-5914) for a bound vertical CRS: the operations to the hub, followed
/// by a change of vertical unit when both are 3D and their height axes differ in direction or
/// unit.
fn skip_horizontal_transformation(
    firsts: Vec<CoordinateOperation>,
    hub: &Crs,
    hub_geog: &GeographicCRS,
    geog_dst: &GeographicCRS,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if firsts.is_empty() {
        return Ok(firsts);
    }
    let (hub_axes, target_axes) = (
        hub_geog.coordinate_system().axes(),
        geog_dst.coordinate_system().axes(),
    );
    let (Some(src_axis), Some(dst_axis)) = (hub_axes.get(2), target_axes.get(2)) else {
        return Ok(firsts);
    };
    if hub_axes.len() != 3
        || target_axes.len() != 3
        || (src_axis.direction() == dst_axis.direction()
            && src_axis.unit().conversion_to_si() == dst_axis.unit().conversion_to_si())
    {
        return Ok(firsts);
    }
    let conv_src = src_axis.unit().conversion_to_si();
    let conv_dst = dst_axis.unit().conversion_to_si();
    if conv_dst == 0.0 {
        return Err(FactoryError::Invalid(
            "Conversion factor of target unit is 0".to_owned(),
        ));
    }
    let up = |axis: &Axis| axis.direction().eq_ignore_ascii_case("up");
    let down = |axis: &Axis| axis.direction().eq_ignore_ascii_case("down");
    let reversal = (up(src_axis) && down(dst_axis)) || (down(src_axis) && up(dst_axis));
    let factor = conv_src / conv_dst;
    let hub_3d = Crs::Geographic(super::compound::promote_with_axis(
        hub_geog,
        Some(dst_axis.clone()),
    ));
    let conversion = Conversion::create_change_vertical_unit(
        &OpProps::new().name("Change of vertical unit"),
        if reversal { -factor } else { factor },
    )?
    .with_endpoints(hub.clone(), hub_3d);
    Ok(firsts
        .into_iter()
        .map(|first| {
            concatenate(vec![
                first,
                CoordinateOperation::Conversion(conversion.clone()),
            ])
        })
        .collect())
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
pub(super) fn set_endpoints(operation: &mut CoordinateOperation, source: &Crs, target: &Crs) {
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

/// `CoordinateOperation::setCRSsUpdateInverse` (singleoperation.cpp:240-272): the endpoints of
/// `operation`, and for a concatenated operation, recursively, the source of its first step and
/// the target of its last step. The inverse objects PROJ also updates are not stored here: an
/// inverse is computed from the operation when needed.
pub(super) fn set_endpoints_update_inverse(
    operation: &mut CoordinateOperation,
    source: &Crs,
    target: &Crs,
) {
    set_endpoints(operation, source, target);
    if let CoordinateOperation::Concatenated(concatenated) = operation {
        if let Some(first) = concatenated.steps.first_mut() {
            if let Some(first_target) = step_target(first).cloned() {
                set_step_endpoints(first, source, &first_target);
            }
        }
        if let Some(last) = concatenated.steps.last_mut() {
            if let Some(last_source) = step_source(last).cloned() {
                set_step_endpoints(last, &last_source, target);
            }
        }
    }
}

/// The source of a step in the direction the concatenation runs it.
fn step_source(step: &ConcatenatedStep) -> Option<&Crs> {
    if step.reverse {
        operation_target(&step.operation)
    } else {
        operation_source(&step.operation)
    }
}

/// The target of a step in the direction the concatenation runs it.
fn step_target(step: &ConcatenatedStep) -> Option<&Crs> {
    if step.reverse {
        operation_source(&step.operation)
    } else {
        operation_target(&step.operation)
    }
}

/// [`set_endpoints_update_inverse`] on a step: a reversed step stores the forward operation, so
/// its stored ends are the swapped ones.
fn set_step_endpoints(step: &mut ConcatenatedStep, source: &Crs, target: &Crs) {
    if step.reverse {
        set_endpoints_update_inverse(&mut step.operation, target, source);
    } else {
        set_endpoints_update_inverse(&mut step.operation, source, target);
    }
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
    let interpolation = super::buildop::concatenation_interpolation(&operations).map(Box::new);
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
        interpolation,
    })
}

/// The binding transformation, with the bound CRS endpoints when it carries none. A bound
/// projected CRS binds the geographic CRS of its base, as `createFromTOWGS84` does.
pub(super) fn with_endpoints(bound: &BoundCRS) -> CoordinateOperation {
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
