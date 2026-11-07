//! The vertical branch of PROJ's `createOperationsCompoundToGeog`
//! (`coordinateoperationfactory.cpp`): a compound CRS of a geographic CRS and a
//! vertical CRS, to a 3D geographic CRS. Also `createOperationsCompoundToCompound`
//! for two compound CRS, and the ballpark of `createOperationsVertToVert`.
//!
//! Not ported: the interpolation CRS of a vertical operation
//! (`createHorizVerticalHorizPROJBased`, which the vertical operation then
//! skips), the geographic CRS of a horizontal operation that differs from the
//! target (`geogCRSTmp`), the change of vertical unit when the two datums are
//! the same, the extent check of a concatenated candidate, the point motion
//! branch and the NAD83(CSRS)v7 hack of `createOperationsCompoundToCompound`
//! (both need epochs), and the removal of consecutive inverse steps in
//! `createComputeMetadata`.

use super::bound::extract_geographic;
use super::bound_ops::{operation_source, operation_target};
use super::buildop::OpProps;
use super::derived::joint_extent;
use super::equiv::{geodetic_frame, same_geodetic_datum, Equivalence};
use super::factory::{
    crs_extent, geographic_of, world_extent, CoordinateOperationContext,
    CoordinateOperationFactory, DatabaseContext, GridAvailabilityUse, GUARD_ALTERNATIVE_GEOG,
    GUARD_VERTICAL_INTERMEDIATE, GUARD_VERTICAL_PIVOT,
};
use super::model::{
    Axis, BoundCRS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, CoordinateOperation, Crs,
    GeographicCRS, GeoidModel, Identified, OperationMethod, Parameter, ParameterValue, ProjBased,
    ProjBasedKind, Transformation, VerticalCRS,
};
use super::pipeline::ProjLines;
use super::FactoryError;

const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";
const BALLPARK_VERTICAL_TRANSFORMATION: &str = "ballpark vertical transformation";

/// Candidates of the vertical branch for `source` to `target`. Empty for any
/// pair that is not a two-component compound of a geographic and a vertical
/// CRS, to a 3D geographic CRS. To a 2D geographic CRS, the horizontal part alone.
pub(super) fn compound_to_geog3d(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Crs::Compound(compound) = source else {
        return Ok(Vec::new());
    };
    let Some(geog_dst) = geographic_of(target) else {
        return Ok(Vec::new());
    };
    let [horizontal, vertical] = compound.components() else {
        return Ok(Vec::new());
    };
    if geog_dst.coordinate_system().axes().len() != 3 {
        return horizontal_only(factory, source, target, horizontal, context);
    }
    if let Crs::Projected(projected) = horizontal {
        if let Crs::Vertical(vertical_crs) = vertical {
            if let Some(operation) = projected_compound_to_geog(
                factory,
                source,
                target,
                projected,
                vertical,
                vertical_crs,
                geog_dst,
                context,
            )? {
                return Ok(vec![operation]);
            }
        }
    }
    let Some(geog_src) = extract_geographic(horizontal) else {
        return Ok(Vec::new());
    };
    let vertical_crs = match vertical {
        Crs::Vertical(vertical_crs) => Some(vertical_crs),
        Crs::Bound(bound) if matches!(bound.source_crs(), Crs::Vertical(_)) => None,
        _ => return Ok(Vec::new()),
    };

    if matches!(horizontal, Crs::Bound(_)) || matches!(vertical, Crs::Bound(_)) {
        return bound_compound_to_geog3d(factory, source, target, context);
    }

    if let Some(vertical_crs) = vertical_crs {
        let found = compound_vertical_pass(
            factory,
            source,
            target,
            horizontal,
            vertical,
            vertical_crs,
            geog_src,
            geog_dst,
            context,
        )?;
        if !found.is_empty() {
            return Ok(found);
        }
    }

    // The horizontal part is searched against the 2D target, as PROJ's geographic-to-geographic
    // search does for a 3D target: the 2D registry operation is then promoted.
    let target_2d = target.demote_to_2d(None);
    let inherited = context.with_inherited_extents(source, target);
    let mut horizontal_ops = factory.create_operations(horizontal, &target_2d, &inherited)?;
    if horizontal_ops.is_empty() {
        horizontal_ops.extend(null_offset(horizontal, target));
    }
    if horizontal_ops.is_empty() {
        return Ok(Vec::new());
    }

    let vertical_ops = factory.create_operations(vertical, target, context)?;
    if vertical_ops.is_empty() {
        return Ok(horizontal_ops);
    }

    let mut found = Vec::with_capacity(horizontal_ops.len() * vertical_ops.len());
    for vertical_op in &vertical_ops {
        for horizontal_op in &horizontal_ops {
            found.push(combine(source, target, horizontal_op, vertical_op));
        }
    }
    Ok(found)
}

/// `createOperationsCompoundToGeog`, vertical part: the vertical operations to the target, and
/// when the geographic CRS of the source is another one, to its own 3D form. Each vertical
/// operation that names an interpolation CRS is chained with the horizontal operations to and
/// from that CRS (`createHorizVerticalHorizPROJBased`); the others take the horizontal
/// operations to the target (`createHorizVerticalPROJBased`) when no operation of the first
/// kind exists or the vertical one is ballpark.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of one compound search, as PROJ passes them"
)]
fn compound_vertical_pass(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    horizontal: &Crs,
    vertical: &Crs,
    vertical_crs: &VerticalCRS,
    geog_src: &GeographicCRS,
    geog_dst: &GeographicCRS,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let db = context.database();
    let ignore_grids = context.grid_availability == GridAvailabilityUse::IgnoreGridAvailability;
    let target_2d = target.demote_to_2d(None);
    let inherited = context.with_inherited_extents(source, target);
    let horizontal_ops = factory.create_operations(horizontal, &target_2d, &inherited)?;
    let mut vertical_ops = factory.create_candidates(vertical, target, &inherited)?;
    let has_all_grids = |operations: &[CoordinateOperation]| {
        operations.iter().any(|operation| {
            !operation.identifiers().is_empty()
                && (ignore_grids
                    || operation
                        .grids_needed(db, GridAvailabilityUse::KnownAvailable)
                        .is_ok_and(|grids| grids.iter().all(|grid| grid.info.available)))
        })
    };
    vertical_ops.extend(vertical_ballpark(
        vertical,
        vertical_crs,
        geographic_datum_name(geog_src).as_deref(),
        geographic_datum_name(geog_dst).as_deref(),
        geog_dst,
        target,
    ));
    let first_has_grids = has_all_grids(&vertical_ops);
    let src_geographic = Crs::Geographic(geog_src.clone());
    if !src_geographic.is_equivalent_to_with(target, Equivalence::Equivalent)
        && !geog_src.is_2d_part_of_3d(geog_dst)
    {
        let interp = src_geographic.promote_to_3d(Some(db));
        let tmp = factory.create_candidates(vertical, &interp, &inherited)?;
        let tmp_has_grids = has_all_grids(&tmp);
        let registered = tmp
            .iter()
            .any(|operation| !operation.identifiers().is_empty());
        if tmp_has_grids && !first_has_grids {
            vertical_ops = tmp;
        } else if registered {
            vertical_ops.extend(tmp);
        }
    }
    if horizontal_ops.is_empty() || vertical_ops.is_empty() {
        return Ok(Vec::new());
    }
    let mut res = Vec::new();
    let mut with_interpolation = false;
    for vertical_op in &vertical_ops {
        let Some(interpolation_geog) = interpolation_geographic(vertical_op) else {
            continue;
        };
        let interpolation = &Crs::Geographic(interpolation_geog);
        // The height axis of the 3D interpolation CRS is the one of the target, so that the
        // vertical operation alone converts the vertical unit.
        let interpolation_3d = match (&interpolation, geog_dst.coordinate_system().axes().get(2)) {
            (Crs::Geographic(geographic), Some(axis)) => {
                Crs::Geographic(promote_with_axis(geographic, Some(axis.clone())))
            }
            _ => interpolation.promote_to_3d(Some(db)),
        };
        let src_to_interp = factory.create_candidates(horizontal, interpolation, &inherited)?;
        let interp_to_target = factory.create_candidates(&interpolation_3d, target, &inherited)?;
        let same_geog = horizontal.is_equivalent_to_with(&target_2d, Equivalence::Equivalent);
        let only_one = src_to_interp.len() == 1 && interp_to_target.len() == 1;
        for first in &src_to_interp {
            for last in &interp_to_target {
                if !only_one
                    && ((same_geog && datum_names_used(first) != datum_names_used(last))
                        || !compatible_transformations(first, last))
                {
                    continue;
                }
                if let Some(operation) = horiz_vertical_horiz_checked(
                    source,
                    target,
                    first,
                    vertical_op,
                    last,
                    interpolation,
                    context,
                ) {
                    res.push(operation);
                    with_interpolation = true;
                }
            }
        }
    }
    if !with_interpolation {
        // Without a vertical operation through an interpolation CRS, the ballpark and geoid
        // candidates of the caller are kept.
        return Ok(Vec::new());
    }
    for vertical_op in &vertical_ops {
        let has_interpolation = interpolation_geographic(vertical_op).is_some();
        if !has_interpolation && (!with_interpolation || vertical_op.has_ballpark_transformation())
        {
            for horizontal_op in &horizontal_ops {
                res.push(combine(source, target, horizontal_op, vertical_op));
            }
        }
    }
    Ok(res)
}

/// The reverse of [`compound_to_geog3d`], for a geographic source and a compound
/// target (`applyInverse` of PROJ's dispatch).
pub(super) fn geog_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if !matches!(target, Crs::Compound(_)) || geographic_of(source).is_none() {
        return Ok(Vec::new());
    }
    Ok(compound_to_geog3d(factory, target, source, context)?
        .iter()
        .map(CoordinateOperation::inverse)
        .collect())
}

/// `createOperationsCompoundToCompound` for two compound CRS of the same number of components.
/// The horizontal parts go through the geographic CRS of each, promoted to 3D: the registry
/// operations between them, the vertical operations between the vertical parts, and the
/// intermediate geographic CRS when the vertical operation is ballpark or a geoid trigger
/// applies. Only the first of PROJ's branches that gives a result is kept. Epochs are not
/// carried, so the point motion branch does not run.
pub(super) fn compound_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (Crs::Compound(src), Crs::Compound(dst)) = (source, target) else {
        return Ok(Vec::new());
    };
    let (src_components, dst_components) = (src.components(), dst.components());
    if src_components.is_empty() || src_components.len() != dst_components.len() {
        return Ok(Vec::new());
    }
    let (Some(src_geog), Some(dst_geog)) = (
        src_components[0].extract_geographic_crs(),
        dst_components[0].extract_geographic_crs(),
    ) else {
        return Ok(Vec::new());
    };
    let db = context.database();
    let same_geog = src_geog.is_equivalent_to_with(&dst_geog, Equivalence::Equivalent);
    let inter_src = Crs::Geographic(src_geog.clone()).promote_to_3d(Some(db));
    let inter_dst = if same_geog {
        inter_src.clone()
    } else {
        Crs::Geographic(dst_geog.clone()).promote_to_3d(Some(db))
    };
    let geog_ops = factory.create_candidates(&inter_src, &inter_dst, context)?;
    let mut res = Vec::new();

    if let (
        [Crs::Bound(src_horizontal), Crs::Bound(src_vertical)],
        [Crs::Bound(dst_horizontal), Crs::Bound(dst_vertical)],
    ) = (src_components, dst_components)
    {
        if src_horizontal
            .target_crs()
            .is_equivalent_to_with(dst_horizontal.target_crs(), Equivalence::Equivalent)
            && !Crs::Bound(src_vertical.clone())
                .is_equivalent_to_with(&Crs::Bound(dst_vertical.clone()), Equivalence::Equivalent)
        {
            let hub = src_horizontal.target_crs().promote_to_3d(Some(db));
            let ops1 = factory.create_candidates(source, &hub, context)?;
            let ops2 = factory.create_candidates(&hub, target, context)?;
            for op1 in &ops1 {
                for op2 in &ops2 {
                    res.extend(compute_metadata(vec![op1.clone(), op2.clone()], context));
                }
            }
            return Ok(res);
        }
    }

    let mut vertical_transforms = Vec::new();
    let mut tried_vertical = false;
    let mut through_intermediate = false;
    if src_components.len() >= 2 {
        if let (Some(vert_src), Some(vert_dst)) = (
            src_components[1].extract_vertical_crs(),
            dst_components[1].extract_vertical_crs(),
        ) {
            if !src_components[1].is_equivalent_to_with(&dst_components[1], Equivalence::Equivalent)
            {
                let has_geoid =
                    !vert_src.geoid_models().is_empty() || !vert_dst.geoid_models().is_empty();
                if has_geoid
                    && !cgvd_transition(&vert_src, &vert_dst)
                    && !cgvd_transition(&vert_dst, &vert_src)
                {
                    through_intermediate = true;
                } else {
                    tried_vertical = true;
                    vertical_transforms = factory.create_candidates(
                        &src_components[1],
                        &dst_components[1],
                        context,
                    )?;
                    through_intermediate = (vertical_transforms.len() == 1
                        && vertical_transforms[0].has_ballpark_transformation())
                        || (!same_geog
                            && !inter_src.identifiers().is_empty()
                            && !inter_dst.identifiers().is_empty()
                            && !geog_ops.is_empty()
                            && !geog_ops[0].has_ballpark_transformation());
                }
            }
        }
    }

    if through_intermediate {
        if let Crs::Bound(bound) = &src_components[0] {
            if !matches!(dst_components[0], Crs::Bound(_))
                && bound
                    .target_crs()
                    .is_equivalent_to_with(&dst_components[0], Equivalence::Equivalent)
            {
                let interm = dst_components[0].promote_to_3d(Some(db));
                through_crs(factory, source, target, &interm, context, &mut res)?;
                if !res.is_empty() {
                    return Ok(res);
                }
            }
        } else if let Crs::Bound(bound) = &dst_components[0] {
            if bound
                .target_crs()
                .is_equivalent_to_with(&src_components[0], Equivalence::Equivalent)
            {
                let interm = src_components[0].promote_to_3d(Some(db));
                through_crs(factory, source, target, &interm, context, &mut res)?;
                if !res.is_empty() {
                    return Ok(res);
                }
            }
        }

        let ops_src_to_geog = factory.create_candidates(source, &inter_src, context)?;
        let ops_geog_to_target = factory.create_candidates(&inter_dst, target, context)?;
        let has_nontrivial_src = has_nontrivial_transformation(db, &ops_src_to_geog);
        let has_nontrivial_target = has_nontrivial_transformation(db, &ops_geog_to_target);
        let mut best = (-1.0_f64, 0_usize);
        if has_nontrivial_src && has_nontrivial_target {
            for pass in 0..=1 {
                if !res.is_empty() {
                    break;
                }
                for op1 in &ops_src_to_geog {
                    if (pass == 0 && op1.has_ballpark_transformation()) || is_ballpark_vertical(op1)
                    {
                        continue;
                    }
                    for op2 in &geog_ops {
                        for op3 in &ops_geog_to_target {
                            if (pass == 0 && op3.has_ballpark_transformation())
                                || is_ballpark_vertical(op3)
                            {
                                continue;
                            }
                            let steps = if same_geog {
                                vec![op1.clone(), op3.clone()]
                            } else {
                                vec![op1.clone(), op2.clone(), op3.clone()]
                            };
                            if let Some(operation) = compute_metadata(steps, context) {
                                let accuracy = get_accuracy(&operation);
                                if improves(accuracy, get_step_count(&operation), best) {
                                    best = (accuracy, get_step_count(&operation));
                                }
                                res.push(operation);
                            }
                        }
                    }
                }
            }
        }

        if res.is_empty()
            || inter_src.identifiers().is_empty()
            || inter_dst.identifiers().is_empty()
        {
            if !src_geog.identifiers().is_empty()
                && !inter_dst.identifiers().is_empty()
                && has_nontrivial_target
            {
                let ops_src_to_inter_dst =
                    factory.create_candidates(source, &inter_dst, context)?;
                if has_nontrivial_transformation(db, &ops_src_to_inter_dst) {
                    two_steps(
                        &ops_src_to_inter_dst,
                        &ops_geog_to_target,
                        context,
                        &mut res,
                        &mut best,
                    );
                }
            }
            if !dst_geog.identifiers().is_empty()
                && !inter_src.identifiers().is_empty()
                && has_nontrivial_src
            {
                let ops_inter_src_to_target =
                    factory.create_candidates(&inter_src, target, context)?;
                if has_nontrivial_transformation(db, &ops_inter_src_to_target) {
                    two_steps(
                        &ops_src_to_geog,
                        &ops_inter_src_to_target,
                        context,
                        &mut res,
                        &mut best,
                    );
                }
            }
        }
        if !res.is_empty() {
            return Ok(res);
        }

        if !tried_vertical {
            vertical_transforms =
                factory.create_candidates(&src_components[1], &dst_components[1], context)?;
        }
    }

    let mut interpolation_names: Vec<String> = Vec::new();
    let mut vector_interpolation: Vec<Crs> = Vec::new();
    for vertical_transform in &vertical_transforms {
        let mut interpolation = Crs::Geographic(src_geog.clone());
        if let Some(interpolation_crs) = interpolation_of(vertical_transform) {
            if !interpolation_names
                .iter()
                .any(|name| name == interpolation_crs.name())
            {
                interpolation_names.push(interpolation_crs.name().to_owned());
                let interpolation_3d = interpolation_crs.promote_to_3d(Some(db));
                if !interpolation_3d.identifiers().is_empty() {
                    vector_interpolation.push(interpolation_3d);
                }
            }
            interpolation = interpolation_crs.clone();
        } else if let (Crs::Bound(src0), Crs::Bound(dst0)) =
            (&src_components[0], &dst_components[0])
        {
            if let Crs::Geographic(hub) = src0.target_crs() {
                if src0
                    .target_crs()
                    .is_equivalent_to_with(dst0.target_crs(), Equivalence::Equivalent)
                {
                    interpolation = Crs::Geographic(hub.clone());
                }
            }
        } else if let Crs::Bound(src0) = &src_components[0] {
            if src0
                .target_crs()
                .is_equivalent_to_with(&Crs::Geographic(dst_geog.clone()), Equivalence::Equivalent)
            {
                interpolation = Crs::Geographic(dst_geog.clone());
            }
        }

        let ops_src = factory.create_candidates(&src_components[0], &interpolation, context)?;
        let ops_dst = factory.create_candidates(&interpolation, &dst_components[0], context)?;
        for op_src in &ops_src {
            for op_dst in &ops_dst {
                if let Some(operation) = horiz_vertical_horiz_checked(
                    source,
                    target,
                    op_src,
                    vertical_transform,
                    op_dst,
                    &interpolation,
                    context,
                ) {
                    res.push(operation);
                }
            }
        }

        // `createOperationsCompoundToCompound`, coordinateoperationfactory.cpp:7769-7799: a single
        // ballpark vertical change between two projected horizontal parts (`EPSG:3912+EPSG:5779 to
        // EPSG:3794+EPSG:8690`). The registry operations of the horizontal pair are kept, each with
        // the vertical change, which writes nothing.
        if vertical_transforms.len() == 1
            && vertical_transform.has_ballpark_transformation()
            && matches!(src_components[0], Crs::Projected(_))
            && matches!(dst_components[0], Crs::Projected(_))
            && vertical_transform.name()
                == format!(
                    "Transformation from {} to {} (ballpark vertical transformation)",
                    src_components[1].name(),
                    dst_components[1].name()
                )
        {
            for horizontal in
                factory.registry_operations(&src_components[0], &dst_components[0], context)?
            {
                res.push(horizontal_null_vertical(
                    source,
                    target,
                    &horizontal,
                    vertical_transform,
                    Crs::Geographic(dst_geog.clone()),
                ));
            }
        }
    }

    for interpolation in &vector_interpolation {
        let ops_src: Vec<CoordinateOperation> = factory
            .create_candidates(source, interpolation, context)?
            .into_iter()
            .filter(|operation| !operation.has_ballpark_transformation())
            .collect();
        if ops_src.is_empty() {
            continue;
        }
        let ops_dst: Vec<CoordinateOperation> = factory
            .create_candidates(interpolation, target, context)?
            .into_iter()
            .filter(|operation| !operation.has_ballpark_transformation())
            .collect();
        for op1 in &ops_src {
            for op2 in &ops_dst {
                res.extend(compute_metadata(vec![op1.clone(), op2.clone()], context));
            }
        }
    }

    if vertical_transforms.is_empty() {
        for operation in
            factory.create_candidates(&src_components[0], &dst_components[0], context)?
        {
            res.push(with_endpoints(operation, source, target));
        }
    }
    Ok(res)
}

/// `createOperationsCompoundToCompound`'s intermediate CRS step: the operations from the source
/// to `interm` and from `interm` to the target, each pair chained.
fn through_crs(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    interm: &Crs,
    context: &CoordinateOperationContext,
    res: &mut Vec<CoordinateOperation>,
) -> Result<(), FactoryError> {
    let ops1 = factory.create_candidates(source, interm, context)?;
    let ops2 = factory.create_candidates(interm, target, context)?;
    for op1 in &ops1 {
        for op2 in &ops2 {
            res.extend(compute_metadata(vec![op1.clone(), op2.clone()], context));
        }
    }
    Ok(())
}

/// `createOperationsCompoundToCompound`'s two-step helper: each pair of `ops1` and `ops2`
/// chained. The first pass leaves out the ballpark operations, the second takes them.
fn two_steps(
    ops1: &[CoordinateOperation],
    ops2: &[CoordinateOperation],
    context: &CoordinateOperationContext,
    res: &mut Vec<CoordinateOperation>,
    best: &mut (f64, usize),
) {
    let mut res2 = Vec::new();
    let mut best2 = (-1.0_f64, 0_usize);
    for pass in 0..=1 {
        if !res2.is_empty() {
            break;
        }
        for op1 in ops1 {
            if (pass == 0 && op1.has_ballpark_transformation()) || is_ballpark_vertical(op1) {
                continue;
            }
            for op2 in ops2 {
                if (pass == 0 && op2.has_ballpark_transformation()) || is_ballpark_vertical(op2) {
                    continue;
                }
                if let Some(operation) = compute_metadata(vec![op1.clone(), op2.clone()], context) {
                    let accuracy = get_accuracy(&operation);
                    let steps = get_step_count(&operation);
                    if improves(accuracy, steps, best2) {
                        best2 = (accuracy, steps);
                    }
                    res2.push(operation);
                }
            }
        }
    }
    if improves(best2.0, best2.1, *best) {
        *best = best2;
        *res = res2;
    }
}

/// `CoordinateOperation::interpolationCRS`: set on a single operation only.
fn interpolation_of(operation: &CoordinateOperation) -> Option<&Crs> {
    match operation {
        CoordinateOperation::Conversion(conversion) => conversion.interpolation_crs(),
        CoordinateOperation::Transformation(transformation) => transformation.interpolation_crs(),
        CoordinateOperation::Concatenated(_) => None,
    }
}

fn is_ballpark_vertical(operation: &CoordinateOperation) -> bool {
    operation.name().contains(BALLPARK_VERTICAL_TRANSFORMATION)
}

/// The best-candidate rule of PROJ's compound-to-compound selection: a non-negative accuracy
/// that is lower than the best so far, or equal with fewer steps.
fn improves(accuracy: f64, steps: usize, best: (f64, usize)) -> bool {
    accuracy >= 0.0 && (best.0 < 0.0 || accuracy < best.0 || (accuracy == best.0 && steps < best.1))
}

/// Whether an operation has a grid it needs available, as `gridsNeeded` with the known grids
/// taken as available.
fn has_known_grid(db: &DatabaseContext, operation: &CoordinateOperation) -> bool {
    operation
        .grids_needed(db, GridAvailabilityUse::KnownAvailable)
        .is_ok_and(|grids| grids.iter().any(|grid| grid.info.available))
}

/// `hasNonTrivialTransf`: the first operation is not ballpark, or it has a known grid.
fn has_nontrivial_transformation(db: &DatabaseContext, operations: &[CoordinateOperation]) -> bool {
    operations
        .first()
        .is_some_and(|first| !first.has_ballpark_transformation() || has_known_grid(db, first))
}

/// The CGVD28 to CGVD2013a height change that one HT2 grid covers, from `vertical` to `other`.
fn cgvd_transition(vertical: &VerticalCRS, other: &VerticalCRS) -> bool {
    let grid = vertical.geoid_models().first().map(GeoidModel::name);
    let to_2013a = |year: &str| {
        other.name() == format!("CGVD2013a({year}) height") && other.geoid_models().is_empty()
    };
    vertical.name() == "CGVD28 height"
        && match grid {
            Some("HT2_1997") => to_2013a("1997"),
            Some("HT2_2002") => to_2013a("2002"),
            Some("HT2_2010") => to_2013a("2010"),
            _ => false,
        }
}

/// `ConcatenatedOperation::createComputeMetadata`: the steps, flattened, named by joining theirs,
/// with the intersection of their extents and the sum of their accuracies. `None` when the
/// extents do not meet and the context disallows that, which PROJ skips silently.
fn compute_metadata(
    operations: Vec<CoordinateOperation>,
    context: &CoordinateOperationContext,
) -> Option<CoordinateOperation> {
    if operations.len() == 1 {
        return operations.into_iter().next();
    }
    let steps = super::derived::remove_consecutive_inverse(flatten_steps(operations));
    if steps.len() == 1 {
        return steps.into_iter().next();
    }
    let extent = match joint_extent(&steps) {
        Some(extent) => extent,
        None if context.disallows_empty_intersection() => return None,
        None => None,
    };
    let accuracy = get_accuracy_of_steps(&steps);
    let name = steps
        .iter()
        .map(CoordinateOperation::name)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = steps
        .first()
        .and_then(operation_source)
        .cloned()
        .map(Box::new);
    let target = steps
        .last()
        .and_then(operation_target)
        .cloned()
        .map(Box::new);
    let mut identified = Identified::named(name);
    identified.domains = extent.into_iter().collect();
    Some(CoordinateOperation::Concatenated(ConcatenatedOperation {
        identified,
        steps: steps
            .into_iter()
            .map(|operation| ConcatenatedStep {
                operation,
                reverse: false,
            })
            .collect(),
        source,
        target,
        accuracy: (accuracy >= 0.0).then(|| stored_accuracy(accuracy)),
        version: None,
        computed_name: true,
        source_epoch: None,
        target_epoch: None,
        proj_based: None,
    }))
}

/// The steps `ConcatenatedOperation::createComputeMetadata` keeps, flattened: a concatenated
/// operation contributes its own steps (`concatenatedoperation.cpp:686-697`). A PROJ-based
/// operation is a single operation, not flattened (`createPROJBased` returns one operation,
/// `coordinateoperationfactory.cpp:2633`), so it keeps its name and is one step.
pub(crate) fn flatten_steps(operations: Vec<CoordinateOperation>) -> Vec<CoordinateOperation> {
    let mut steps = Vec::new();
    for operation in operations {
        match operation {
            CoordinateOperation::Concatenated(concatenated)
                if concatenated.proj_based.is_none() =>
            {
                steps.extend(
                    concatenated
                        .steps
                        .iter()
                        .map(ConcatenatedStep::oriented_operation),
                );
            }
            other => steps.push(other),
        }
    }
    steps
}

/// `getAccuracy(const std::vector<CoordinateOperationNNPtr> &)` (`oputils.cpp:587-612`): the sum
/// of the accuracies of the steps, or -1 when one of them is unknown. Two neighbouring steps that
/// both have an accuracy, and that the ETRS89 ensemble case covers, count as their larger
/// accuracy, once (`oputils.cpp:600-636`, see [`is_etrs89_ensemble_pair`]).
pub(crate) fn get_accuracy_of_steps(operations: &[CoordinateOperation]) -> f64 {
    let mut accuracy = -1.0_f64;
    let mut index = 0;
    while index < operations.len() {
        let subops_accuracy = get_accuracy(&operations[index]);
        if subops_accuracy < 0.0 {
            return -1.0;
        }
        if accuracy < 0.0 {
            accuracy = 0.0;
        }
        if subops_accuracy > 0.0 && index + 1 < operations.len() {
            let first = &operations[index];
            let second = &operations[index + 1];
            let next_accuracy = get_accuracy(second);
            let largest = subops_accuracy.max(next_accuracy);
            if next_accuracy > 0.0
                && is_etrs89_ensemble_pair(first, second, largest)
                && (is_noop(first) || is_noop(second))
            {
                accuracy += largest;
                index += 2;
                continue;
            }
        }
        accuracy += subops_accuracy;
        index += 1;
    }
    accuracy
}

/// The ETRS89 case of `getAccuracy` (`oputils.cpp:600-636`): `first` goes from an `ETRS89-XXX`
/// CRS to the ETRS89 or ETRF CRS that `second` goes on from, and `second` ends on an `ETRS89-YYY`
/// CRS. The larger accuracy of the two, `largest`, must be at most 0.1 m.
fn is_etrs89_ensemble_pair(
    first: &CoordinateOperation,
    second: &CoordinateOperation,
    largest: f64,
) -> bool {
    const ACCURACY_ENSEMBLE_ETRS89: f64 = 0.1;
    let (Some(crs1), Some(middle), Some(crs2)) = (
        operation_source(first),
        operation_target(first),
        operation_target(second),
    ) else {
        return false;
    };
    largest <= ACCURACY_ENSEMBLE_ETRS89
        && (middle.name() == "ETRS89" || middle.name().starts_with("ETRF"))
        && crs1.name().starts_with("ETRS89-")
        && crs2.name().starts_with("ETRS89-")
}

/// `IsNoOp` of `oputils.cpp`: the operation exports as `+proj=noop`. An operation whose PROJ string
/// cannot be written is not a no-op, as PROJ catches the exception and answers false.
fn is_noop(operation: &CoordinateOperation) -> bool {
    let exported = match operation {
        // A conversion has accuracy 0, so the ETRS89 case never asks for it.
        CoordinateOperation::Conversion(_) => return false,
        CoordinateOperation::Transformation(transformation) => {
            transformation.export_to_proj_string(ProjLines::Single)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.export_to_proj_string(ProjLines::Single)
        }
    };
    exported.is_ok_and(|text| text == "+proj=noop")
}

/// The accuracy a concatenated operation stores: PROJ writes `toString(accuracy)` as its accuracy
/// string (`concatenatedoperation.cpp:754-758`), `%.15g` (`internal.cpp:343-357`, 14 digits when
/// the 15 read `9999999999`), and `getAccuracy` reads that string back. A sum of 0.1 and 0.11 is
/// therefore stored as 0.21, not as 0.21000000000000002.
pub(crate) fn stored_accuracy(accuracy: f64) -> f64 {
    let fifteen = format!("{accuracy:.14e}");
    let text = if fifteen.contains("9999999999") {
        format!("{accuracy:.13e}")
    } else {
        fifteen
    };
    text.parse().unwrap_or(accuracy)
}

/// `getAccuracy(const CoordinateOperationNNPtr &)` (`oputils.cpp:560-585`): 0 for a conversion,
/// else the stated accuracy, or else the sum over the steps of a concatenated operation. -1 when
/// unknown.
pub(crate) fn get_accuracy(operation: &CoordinateOperation) -> f64 {
    match operation {
        CoordinateOperation::Conversion(_) => 0.0,
        _ => match operation.accuracy() {
            Some(accuracy) => accuracy,
            None => match operation {
                CoordinateOperation::Concatenated(concatenated) => {
                    let steps: Vec<CoordinateOperation> = concatenated
                        .steps
                        .iter()
                        .map(|step| step.operation.clone())
                        .collect();
                    get_accuracy_of_steps(&steps)
                }
                _ => -1.0,
            },
        },
    }
}

fn get_step_count(operation: &CoordinateOperation) -> usize {
    match operation {
        CoordinateOperation::Concatenated(concatenated) => concatenated.steps.len(),
        _ => 1,
    }
}

/// `createHorizVerticalHorizPROJBased`, which is skipped when the extents of its parts do not
/// meet and the context disallows that.
fn horiz_vertical_horiz_checked(
    source: &Crs,
    target: &Crs,
    first: &CoordinateOperation,
    vertical: &CoordinateOperation,
    last: &CoordinateOperation,
    interpolation: &Crs,
    context: &CoordinateOperationContext,
) -> Option<CoordinateOperation> {
    let mut steps = Vec::new();
    if !is_null_offset(first) {
        steps.push(first.clone());
    }
    steps.push(vertical.clone());
    if !is_null_offset(last) {
        steps.push(last.clone());
    }
    if context.disallows_empty_intersection() && joint_extent(&steps).is_none() {
        return None;
    }
    Some(proj_based_horiz_vertical_horiz(
        source,
        target,
        first,
        vertical,
        last,
        interpolation,
    ))
}

/// `setCRSs` on a copy: the source and target of the operation, with its identifiers kept.
fn with_endpoints(
    mut operation: CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> CoordinateOperation {
    match &mut operation {
        CoordinateOperation::Conversion(conversion) => {
            conversion.source = Some(Box::new(source.clone()));
            conversion.target = Some(Box::new(target.clone()));
        }
        CoordinateOperation::Transformation(transformation) => {
            transformation.source = Some(Box::new(source.clone()));
            transformation.target = Some(Box::new(target.clone()));
        }
        CoordinateOperation::Concatenated(concatenated) => {
            concatenated.source = Some(Box::new(source.clone()));
            concatenated.target = Some(Box::new(target.clone()));
        }
    }
    operation
}

/// `createOperationsCompoundToGeog` for a 2D geographic target: the operations of the
/// horizontal part to the target, the vertical part dropped. A null offset when there are none.
fn horizontal_only(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    horizontal: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if geographic_of(horizontal).is_none() {
        return Ok(Vec::new());
    }
    let mut operations = factory.create_candidates(horizontal, target, context)?;
    if operations.is_empty() {
        operations.extend(null_offset(horizontal, target));
    }
    Ok(operations
        .iter()
        .map(|operation| retarget(operation, source, target))
        .collect())
}

/// `createHorizNullVerticalPROJBased` (coordinateoperationfactory.cpp:2884-2914): the horizontal
/// operation alone, named `H + V`, with the extent of both steps and no accuracy. A ballpark
/// vertical step makes the accuracy unknown.
fn horizontal_null_vertical(
    source: &Crs,
    target: &Crs,
    horizontal: &CoordinateOperation,
    vertical: &CoordinateOperation,
    interpolation: Crs,
) -> CoordinateOperation {
    let steps = vec![horizontal.clone(), vertical.clone()];
    let mut operation = proj_based_operation(
        format!("{} + {}", horizontal.name(), vertical.name()),
        source,
        target,
        steps.clone(),
        None,
        ProjBased {
            kind: ProjBasedKind::HorizNullVertical,
            horizontal: horizontal.clone(),
            vertical: vertical.clone(),
            last: None,
            interpolation,
            inverted: false,
        },
    );
    if let Some(Some(domain)) = super::derived::joint_extent(&steps) {
        if let CoordinateOperation::Concatenated(concatenated) = &mut operation {
            concatenated.identified.domains = vec![domain];
        }
    }
    operation
}

/// `createHorizVerticalPROJBased`: a null horizontal part leaves the vertical
/// operation as it is, under the compound source and the target. Otherwise the
/// two are chained.
fn combine(
    source: &Crs,
    target: &Crs,
    horizontal: &CoordinateOperation,
    vertical: &CoordinateOperation,
) -> CoordinateOperation {
    let horizontal_is_null =
        horizontal.name().starts_with(NULL_GEOGRAPHIC_OFFSET) && !horizontal.name().contains(" + ");
    if horizontal_is_null {
        return retarget(vertical, source, target);
    }
    CoordinateOperation::Concatenated(ConcatenatedOperation {
        identified: Identified::named(format!("{} + {}", horizontal.name(), vertical.name())),
        steps: vec![
            ConcatenatedStep {
                operation: horizontal.clone(),
                reverse: false,
            },
            ConcatenatedStep {
                operation: vertical.clone(),
                reverse: false,
            },
        ],
        source: Some(Box::new(source.clone())),
        target: Some(Box::new(target.clone())),
        accuracy: None,
        version: None,
        computed_name: true,
        source_epoch: None,
        target_epoch: None,
        proj_based: None,
    })
}

/// The copy PROJ builds from a vertical operation: same name, method and
/// parameters, the compound source and the target, and no identifier. Its
/// accuracy is written as a double, as `toString(accuracy)` does, so `1.0` reads `1`.
fn retarget(operation: &CoordinateOperation, source: &Crs, target: &Crs) -> CoordinateOperation {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            let mut copy = transformation.clone();
            copy.identified.identifiers.clear();
            copy.source = Some(Box::new(source.clone()));
            copy.target = Some(Box::new(target.clone()));
            copy.accuracy_text = copy.accuracy.map(|value| format!("{value}"));
            CoordinateOperation::Transformation(copy)
        }
        CoordinateOperation::Concatenated(concatenated) => {
            let mut copy = concatenated.clone();
            copy.identified.identifiers.clear();
            copy.source = Some(Box::new(source.clone()));
            copy.target = Some(Box::new(target.clone()));
            CoordinateOperation::Concatenated(copy)
        }
        CoordinateOperation::Conversion(_) => operation.clone(),
    }
}

/// A compound whose horizontal part is projected, to a 3D geographic target. This is the
/// derived branch of PROJ's `createOperationsCompoundToGeog`: the first operation of the
/// projected CRS to its base, then the ballpark vertical change as `createHorizVerticalPROJBased`
/// with a null horizontal part. The target's axis steps wrap the vertical step, so the
/// pipeline optimizer cancels them against the horizontal tail and keeps the final swap.
fn projected_compound_to_geog(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    projected: &super::model::ProjectedCRS,
    vertical: &Crs,
    vertical_crs: &VerticalCRS,
    geog_dst: &GeographicCRS,
    context: &CoordinateOperationContext,
) -> Result<Option<CoordinateOperation>, FactoryError> {
    let base = Crs::Geodetic(projected.base_crs().clone());
    // A different datum keeps the general path. A registered vertical operation (sorted
    // first, not a ballpark) takes the place of the ballpark change of unit.
    if !same_geodetic_datum(&base, target) {
        return Ok(None);
    }
    let Some(base_geog) = geographic_of(&base) else {
        return Ok(None);
    };
    let horizontal_datum = geographic_datum_name(base_geog);
    let target_datum = geographic_datum_name(geog_dst);
    let vertical_ops = factory.create_operations(vertical, target, context)?;
    let registered = vertical_ops
        .iter()
        .find(|operation| !operation.has_ballpark_transformation())
        .cloned();
    let vertical_op = match registered {
        Some(operation) => operation,
        // PROJ keeps every vertical transformation of `createOperations(vertical, target)`,
        // the ballpark change of unit among them (createOperationsCompoundToGeog,
        // coordinateoperationfactory.cpp:6782-6785, 6921-6924).
        None if !vertical_ops.is_empty() => vertical_ops[0].clone(),
        None => {
            let Some(operation) = vertical_ballpark(
                vertical,
                vertical_crs,
                horizontal_datum.as_deref(),
                target_datum.as_deref(),
                geog_dst,
                target,
            ) else {
                return Ok(None);
            };
            operation
        }
    };
    let Some(horizontal_op) = factory
        .create_candidates(&Crs::Projected(projected.clone()), &base, context)?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let Some(null) = null_offset(&base, target) else {
        return Ok(None);
    };
    let last = proj_based_horiz_vertical(source, target, &null, &vertical_op, geog_dst);
    Ok(Some(chain(source, target, horizontal_op, last)))
}

fn chain(
    source: &Crs,
    target: &Crs,
    first: CoordinateOperation,
    second: CoordinateOperation,
) -> CoordinateOperation {
    let mut identified = Identified::named(format!("{} + {}", first.name(), second.name()));
    if let (Some(left), Some(right)) = (
        super::factory::operation_extent(&first),
        super::factory::operation_extent(&second),
    ) {
        if let Some(domain) = super::factory::intersect_extents(left, right) {
            identified.domains = vec![domain];
        }
    }
    CoordinateOperation::Concatenated(ConcatenatedOperation {
        identified,
        steps: vec![
            ConcatenatedStep {
                operation: first,
                reverse: false,
            },
            ConcatenatedStep {
                operation: second,
                reverse: false,
            },
        ],
        source: Some(Box::new(source.clone())),
        target: Some(Box::new(target.clone())),
        accuracy: None,
        version: None,
        computed_name: true,
        source_epoch: None,
        target_epoch: None,
        proj_based: None,
    })
}

/// The ballpark change of unit to a geographic CRS of another datum, as `vertical_geog_synthesized`
/// and `vertical_ballpark` name it.
pub(super) fn is_synthesized_ballpark(operation: &CoordinateOperation) -> bool {
    operation.name().contains(
        "(ballpark vertical transformation, without ellipsoid height to vertical height correction)",
    )
}

/// `createOperationsVertToGeogSynthetized` (`coordinateoperationfactory.cpp:6113`): the ballpark
/// change of vertical unit between a vertical CRS and a geographic CRS of another datum, in the
/// direction of the request. The reverse request inverts the operation, as PROJ does with
/// `applyInverse`. It is appended to the operations of a vertical source and a geographic
/// target (`createOperations`, `coordinateoperationfactory.cpp:3748-3751`, and `createOperationsVertToGeog`,
/// `:6107-6108`), and to the geoid operations before the registry (`:4266-4271`). `None` when the
/// datums are the same (PROJ builds a conversion there, not ported).
pub(super) fn vertical_geog_synthesized(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
    let (vertical, geog_crs, from_vertical) = match (source, target) {
        (Crs::Vertical(_), other) => (source, other, true),
        (other, Crs::Vertical(_)) => (target, other, false),
        _ => return None,
    };
    let Crs::Vertical(vertical_crs) = vertical else {
        return None;
    };
    let geog = geographic_of(geog_crs)?;
    let operation = vertical_ballpark(
        vertical,
        vertical_crs,
        None,
        geographic_datum_name(geog).as_deref(),
        geog,
        geog_crs,
    )?;
    Some(if from_vertical {
        operation
    } else {
        operation.inverse()
    })
}

/// `createOperationsVertToGeogSynthetized`: the ballpark change of vertical unit
/// from a vertical CRS to a 3D geographic CRS of another datum. `None` when the
/// datums are the same, where PROJ builds a conversion instead (not ported).
fn vertical_ballpark(
    vertical: &Crs,
    vertical_crs: &VerticalCRS,
    horizontal_datum: Option<&str>,
    target_datum: Option<&str>,
    geog_dst: &GeographicCRS,
    target: &Crs,
) -> Option<CoordinateOperation> {
    let scale = ballpark_scale(
        vertical_crs.coordinate_system().axis(),
        geog_dst.coordinate_system().axes().get(2),
    )?;

    let vertical_datum = vertical_datum_name(vertical_crs);
    let same_datum = match (vertical_datum.as_deref(), target_datum) {
        (Some("Ellipsoid"), Some(target)) => horizontal_datum == Some(target),
        (Some(vertical), Some(target)) => vertical == target,
        _ => false,
    };
    if same_datum {
        return None;
    }

    let name = format!(
        "Transformation from {} to {} (ballpark vertical transformation, without ellipsoid height to vertical height correction)",
        vertical.name(),
        target.name()
    );
    let domain = match (crs_extent(vertical), crs_extent(target)) {
        (Some(left), Some(right)) if left == right => left,
        _ => world_extent(),
    };
    Transformation::create_from_code(
        &OpProps::new().name(name),
        vertical.clone(),
        target.clone(),
        1069,
        vec![ParameterValue::scale(scale)],
        domain,
    )
    .ok()
    .map(CoordinateOperation::Transformation)
}

/// `createOperationsGeogToVertFromGeoid` for a vertical CRS with geoid models, and a 3D
/// geographic CRS in metres, either way round (`applyInverse` for a vertical source). Each
/// model gives its registered operation by identifier or by name, or a gravity-related height
/// transformation on its PROJ grid (`PROJ <grid>`). Only the operations whose target is the
/// vertical CRS are kept: the change of vertical unit and datum pivot of PROJ are not ported.
pub(super) fn geoid_vertical(
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (vertical, geog, from_vertical) = match (source, target) {
        (Crs::Vertical(vertical), geog) => (vertical, geog, true),
        (geog, Crs::Vertical(vertical)) => (vertical, geog, false),
        _ => return Ok(Vec::new()),
    };
    let Crs::Geographic(geog) = geog else {
        return Ok(Vec::new());
    };
    let geog_axes = geog.coordinate_system().axes();
    let vertical_axis = vertical.coordinate_system().axis();
    if geog_axes.len() != 3
        || geog_axes[2].unit().conversion_to_si() != 1.0
        || vertical_axis.unit().conversion_to_si() != 1.0
        || vertical_axis.direction() != "up"
    {
        return Ok(Vec::new());
    }
    let mut operations = Vec::new();
    for model in vertical.geoid_models() {
        let model_operations = match model.identifiers.first() {
            Some(id) => vec![super::factory::registered_operation(
                context,
                id.code_space(),
                id.code(),
            )?],
            None => match model.name().strip_prefix("PROJ ") {
                Some(grid) => proj_geoid_operation(vertical, geog, model, grid, context)?
                    .into_iter()
                    .collect(),
                None => super::factory::geoid_model_operations(context, model.name())?,
            },
        };
        for operation in model_operations {
            let to_vertical = match (operation.source_crs(), operation.target_crs()) {
                (Some(Crs::Vertical(_)), Some(Crs::Geographic(_))) => Some(operation),
                (Some(Crs::Geographic(_)), Some(Crs::Vertical(_))) => Some(operation.inverse()),
                _ => None,
            };
            let Some(operation) = to_vertical else {
                continue;
            };
            let kept = operation
                .target_crs()
                .is_some_and(|target| target.name() == vertical.name())
                || operation
                    .source_crs()
                    .is_some_and(|source| source.name() == vertical.name());
            if kept {
                operations.push(operation);
            }
        }
    }
    if from_vertical {
        return Ok(operations);
    }
    Ok(operations
        .iter()
        .map(CoordinateOperation::inverse)
        .collect())
}

/// `createOperationsFromDatabaseWithVertCRS` for a geographic CRS and a vertical CRS that no
/// registered operation links directly: through a vertical CRS of the same datum
/// (`createOperationsGeogToVertWithIntermediateVert`), then through the registered operations to
/// the vertical CRS from any geographic CRS (`createOperationsGeogToVertWithAlternativeGeog`).
pub(super) fn vertical_geog_fallback(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (geog, vertical, from_vertical) = match (source, target) {
        (Crs::Vertical(_), other) if geographic_of(other).is_some() => (other, source, true),
        (other, Crs::Vertical(_)) if geographic_of(other).is_some() => (other, target, false),
        _ => return Ok(Vec::new()),
    };
    // PROJ's guards (`createOperationsFromDatabaseWithVertCRS`, coordinateoperationfactory.cpp
    // 5152–5230) are: the registry gave nothing (the caller checks it), and the anti-recursion flag
    // of each step is not set. PROJ tests no identifier of the geographic CRS, so an identified one
    // reaches the same steps.
    let mut res = Vec::new();
    if !context.has_guard(GUARD_VERTICAL_INTERMEDIATE) {
        res = geog_to_vertical_intermediate(factory, geog, vertical, context)?;
    }
    if res.is_empty() && !context.has_guard(GUARD_ALTERNATIVE_GEOG) {
        res = geog_to_vertical_alternative(factory, geog, vertical, context)?;
    }
    if from_vertical {
        res = res.iter().map(CoordinateOperation::inverse).collect();
    }
    Ok(res)
}

/// `normalize2D3DInName`.
fn normalize_2d_3d_in_name(name: &str) -> String {
    [
        " (2D)",
        " (geographic3D horizontal)",
        " (geog2D)",
        " (geog3D)",
    ]
    .iter()
    .fold(name.to_owned(), |out, pattern| out.replace(pattern, ""))
}

/// `getOps`: the steps of a concatenated operation, nested ones included, or the operation.
fn flat_operations(operation: &CoordinateOperation) -> Vec<&CoordinateOperation> {
    match operation {
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .steps()
            .iter()
            .flat_map(|step| flat_operations(step.operation()))
            .collect(),
        other => vec![other],
    }
}

/// `useCompatibleTransformationsForSameSourceTarget`: two operations that go through the same
/// pair of CRS use the same transformation, in the same or the inverse direction.
fn compatible_transformations(first: &CoordinateOperation, second: &CoordinateOperation) -> bool {
    for step_a in flat_operations(first) {
        let CoordinateOperation::Transformation(_) = step_a else {
            continue;
        };
        let name_a = normalize_2d_3d_in_name(step_a.name());
        let (Some(source_a), Some(target_a)) = (step_a.source_crs(), step_a.target_crs()) else {
            continue;
        };
        if source_a.name() == "unknown" || target_a.name() == "unknown" {
            continue;
        }
        for step_b in flat_operations(second) {
            let CoordinateOperation::Transformation(_) = step_b else {
                continue;
            };
            let (Some(source_b), Some(target_b)) = (step_b.source_crs(), step_b.target_crs())
            else {
                continue;
            };
            if source_b.name() == "unknown" || target_b.name() == "unknown" {
                continue;
            }
            let both_null = name_a.starts_with(NULL_GEOGRAPHIC_OFFSET)
                && step_b.name().starts_with(NULL_GEOGRAPHIC_OFFSET);
            if source_a.name() == source_b.name() && target_a.name() == target_b.name() {
                if both_null {
                    continue;
                }
                if name_a != normalize_2d_3d_in_name(step_b.name()) {
                    return false;
                }
            } else if source_a.name() == target_b.name() && target_a.name() == source_b.name() {
                if both_null {
                    continue;
                }
                if name_a != normalize_2d_3d_in_name(step_b.inverse().name()) {
                    return false;
                }
            }
        }
    }
    true
}

/// The names of the geodetic datums of the transformations of an operation.
fn datum_names_used(operation: &CoordinateOperation) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for step in flat_operations(operation) {
        if !matches!(step, CoordinateOperation::Transformation(_)) {
            continue;
        }
        for crs in [step.source_crs(), step.target_crs()].into_iter().flatten() {
            if let Some(frame) = geodetic_frame(crs) {
                names.insert(frame.name().to_owned());
            }
        }
    }
    names
}

/// `createOperationsVertToVertWithIntermediateVert`: two pivot strategies through the vertical
/// CRS of the target's datum, then of the source's datum.
pub(super) fn vertical_pivot(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (Crs::Vertical(vsrc), Crs::Vertical(vdst)) = (source, target) else {
        return Ok(Vec::new());
    };
    if context.has_guard(GUARD_VERTICAL_PIVOT) {
        return Ok(Vec::new());
    }
    let mut inner = context.clone();
    inner.guards |= GUARD_VERTICAL_PIVOT;
    let candidates = |vertical: &VerticalCRS| -> Result<Vec<Crs>, FactoryError> {
        let Some(datum) = vertical.datum().and_then(|d| d.identifiers().first()) else {
            return Ok(Vec::new());
        };
        let rows = context.database().query(
            "SELECT auth_name, code FROM vertical_crs WHERE datum_auth_name = ? \
             AND datum_code = ? AND deprecated = 0",
            &[datum.code_space().to_owned(), datum.code().to_owned()],
        )?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                super::factory::AuthorityFactory::create(context.database(), row[0].text())
                    .create_coordinate_reference_system(&row[1].text())
                    .ok()
            })
            .collect())
    };
    let mut res = Vec::new();
    for candidate in candidates(vdst)? {
        if candidate.is_equivalent_to_with(target, Equivalence::Equivalent) {
            continue;
        }
        let first = factory.create_operations(source, &candidate, &inner)?;
        if first.is_empty() {
            continue;
        }
        let second = factory.create_operations(&candidate, target, &inner)?;
        let Some(second) = second.first() else {
            continue;
        };
        for op in first.iter().filter(|op| has_identifiers(op)) {
            res.extend(compute_metadata(vec![op.clone(), second.clone()], context));
        }
    }
    if res.is_empty() {
        for candidate in candidates(vsrc)? {
            if candidate.is_equivalent_to_with(source, Equivalence::Equivalent) {
                continue;
            }
            let second = factory.create_operations(&candidate, target, &inner)?;
            if second.is_empty() {
                continue;
            }
            let first = factory.create_operations(source, &candidate, &inner)?;
            let Some(first) = first.first() else {
                continue;
            };
            for op in second.iter().filter(|op| has_identifiers(op)) {
                res.extend(compute_metadata(vec![first.clone(), op.clone()], context));
            }
        }
    }
    Ok(res)
}

/// `hasIdentifiers`: the operation or one of its steps has an identifier.
fn has_identifiers(operation: &CoordinateOperation) -> bool {
    if !operation.identifiers().is_empty() {
        return true;
    }
    match operation {
        CoordinateOperation::Concatenated(concatenated) => concatenated
            .steps()
            .iter()
            .any(|step| has_identifiers(step.operation())),
        _ => false,
    }
}

fn geog_to_vertical_intermediate(
    factory: &CoordinateOperationFactory,
    geog: &Crs,
    vertical: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Crs::Vertical(vertical_crs) = vertical else {
        return Ok(Vec::new());
    };
    let Some(datum) = vertical_crs
        .datum()
        .and_then(|datum| datum.identifiers().first())
    else {
        return Ok(Vec::new());
    };
    let mut inner = context.clone();
    inner.guards |= GUARD_VERTICAL_INTERMEDIATE;
    let candidates = context.database().query(
        "SELECT auth_name, code FROM vertical_crs WHERE datum_auth_name = ? \
         AND datum_code = ? AND deprecated = 0",
        &[datum.code_space().to_owned(), datum.code().to_owned()],
    )?;
    let mut res = Vec::new();
    for row in &candidates {
        let Ok(candidate) =
            super::factory::AuthorityFactory::create(context.database(), row[0].text())
                .create_coordinate_reference_system(&row[1].text())
        else {
            continue;
        };
        let first = factory.create_candidates(geog, &candidate, &inner)?;
        if first.is_empty() {
            continue;
        }
        let second = factory.create_candidates(&candidate, vertical, &inner)?;
        let Some(second) = second.first() else {
            continue;
        };
        for op_first in &first {
            if !has_identifiers(op_first) {
                continue;
            }
            if candidate.is_equivalent_to_with(vertical, Equivalence::Equivalent) {
                res.push(op_first.clone());
            } else if let Some(operation) =
                compute_metadata(vec![op_first.clone(), second.clone()], context)
            {
                res.push(operation);
            }
        }
        if !res.is_empty() {
            break;
        }
    }
    Ok(res)
}

fn geog_to_vertical_alternative(
    factory: &CoordinateOperationFactory,
    geog: &Crs,
    vertical: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(id) = vertical.identifiers().first() else {
        return Ok(Vec::new());
    };
    let Some(geographic) = geographic_of(geog) else {
        return Ok(Vec::new());
    };
    let mut inner = context.clone();
    inner.guards |= GUARD_ALTERNATIVE_GEOG;
    let rows = context.database().query(
        "SELECT auth_name, code FROM coordinate_operation_view WHERE target_crs_auth_name = ? \
         AND target_crs_code = ? AND deprecated = 0",
        &[id.code_space().to_owned(), id.code().to_owned()],
    )?;
    let axes = geographic.coordinate_system().axes();
    let mut res = Vec::new();
    for row in &rows {
        let operation =
            super::factory::registered_operation(context, &row[0].text(), &row[1].text())?;
        // The registry search of PROJ (`findOpsInRegistryDirectTo`) drops, under the grid
        // availability of the context, an operation whose grid is missing.
        if context.discards_missing_grids()
            && !operation
                .grids_needed(context.database(), context.grid_availability)?
                .iter()
                .all(|grid| grid.info.available)
        {
            continue;
        }
        let Some(op_source) = operation.source_crs() else {
            continue;
        };
        if geographic_of(op_source).is_none() {
            continue;
        }
        if axes.len() == 3 && axes[2].unit().conversion_to_si() != 1.0 {
            let with_z = Crs::Geographic(promote_with_axis(
                &geographic_of(op_source)
                    .cloned()
                    .unwrap_or_else(|| geographic.clone()),
                Some(axes[2].clone()),
            ));
            let unit_change = factory.create_candidates(&with_z, op_source, &inner)?;
            let Some(unit_change) = unit_change.into_iter().next() else {
                continue;
            };
            if let Some(operation) = compute_metadata(vec![unit_change, operation], context) {
                res.push(operation);
            }
        } else {
            res.push(operation);
        }
    }
    Ok(res)
}

/// `getProjGeoidTransformation` for a model named `PROJ <grid>`: the gravity-related height
/// transformation from the vertical CRS to the geographic CRS on that grid. Among the
/// operations on the grid, those whose target has the datum of the vertical CRS are kept
/// when there are any. The most accurate one gives the accuracy and the area of use.
fn proj_geoid_operation(
    vertical: &VerticalCRS,
    geog: &GeographicCRS,
    model: &GeoidModel,
    grid: &str,
    context: &CoordinateOperationContext,
) -> Result<Option<CoordinateOperation>, FactoryError> {
    let on_grid = super::factory::grid_name_operations(context, grid)?;
    let vertical_datum = vertical_datum_name(vertical);
    let same_datum: Vec<CoordinateOperation> = on_grid
        .iter()
        .filter(|operation| match operation.target_crs() {
            Some(Crs::Vertical(target)) => {
                vertical_datum.is_some() && vertical_datum_name(target) == vertical_datum
            }
            _ => false,
        })
        .cloned()
        .collect();
    let candidates = if same_datum.is_empty() {
        on_grid
    } else {
        same_datum
    };
    let mut best: Option<usize> = None;
    let mut accuracy = -1.0_f64;
    for (index, operation) in candidates.iter().enumerate() {
        let candidate = operation.accuracy().unwrap_or(-1.0);
        if candidate - accuracy > 1e-10 {
            accuracy = candidate;
            best = Some(index);
        }
    }
    let domain = best
        .and_then(|index| candidates.get(index))
        .or_else(|| candidates.first())
        .and_then(|operation| operation.domains().first().cloned());

    let interpolation = match model.interpolation_crs.as_deref() {
        Some(Crs::Geographic(interpolation)) => interpolation.clone(),
        _ => geog.clone(),
    };
    let accuracy = (accuracy >= 0.0).then_some(accuracy);
    let mut identified = Identified::named(op_name(
        "Transformation",
        &Crs::Vertical(vertical.clone()),
        &Crs::Geographic(interpolation.clone()),
    ));
    identified.domains = domain.into_iter().collect();
    let transformation = Transformation {
        identified,
        method: OperationMethod::create("GravityRelatedHeight to Geographic3D", Vec::new()),
        parameters: vec![Parameter {
            name: "Geoid (height correction) model file".to_owned(),
            code_space: "EPSG".to_owned(),
            code: "8666".to_owned(),
            value: ParameterValue::filename(grid),
        }],
        source: Some(Box::new(Crs::Vertical(vertical.clone()))),
        target: Some(Box::new(Crs::Geographic(interpolation))),
        interpolation: None,
        accuracy,
        accuracy_text: accuracy.map(|value| format!("{value}")),
        version: None,
        point_motion: false,
        source_epoch: None,
        target_epoch: None,
        inverted: false,
    };
    Ok(Some(CoordinateOperation::Transformation(transformation)))
}

/// `createOperationsVertToVert`: a ballpark change of vertical unit between two vertical CRS
/// of different datums (method 1069, scale negated on a height/depth reversal), the same
/// change between two of the same datum, or a height/depth reversal (method 1068) when the
/// datum is the same and the unit is unchanged.
pub(super) fn vertical_to_vertical(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
    let (Crs::Vertical(src), Crs::Vertical(dst)) = (source, target) else {
        return None;
    };
    let scale = ballpark_scale(
        src.coordinate_system().axis(),
        Some(dst.coordinate_system().axis()),
    )?;
    let same_datum = vertical_datum_name(src) == vertical_datum_name(dst);
    let reversal = same_datum && scale == -1.0;
    let name = if same_datum {
        format!("Conversion from {} to {}", source.name(), target.name())
    } else {
        format!(
            "Transformation from {} to {} (ballpark vertical transformation)",
            source.name(),
            target.name()
        )
    };
    let domain = match (crs_extent(source), crs_extent(target)) {
        (Some(left), Some(right)) if left == right => left,
        _ => world_extent(),
    };
    let (code, values) = if reversal {
        (1068, Vec::new())
    } else {
        (1069, vec![ParameterValue::scale(scale)])
    };
    Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        code,
        values,
        domain,
    )
    .ok()
    .map(CoordinateOperation::Transformation)
}

/// The scale of a change of vertical unit from `src` to `dst`, where `dst` is `None` for a
/// 2D geographic target. Negated on a height/depth reversal. `None` when the target unit is 0.
fn ballpark_scale(src: &Axis, dst: Option<&Axis>) -> Option<f64> {
    let conv_src = src.unit().conversion_to_si();
    let conv_dst = dst.map_or(1.0, |axis| axis.unit().conversion_to_si());
    if conv_dst == 0.0 {
        return None;
    }
    let dst_up = dst.is_none_or(|axis| axis.direction() == "up");
    let dst_down = dst.is_some_and(|axis| axis.direction() == "down");
    let src_up = src.direction() == "up";
    let src_down = src.direction() == "down";
    let reversal = (src_up && dst_down) || (src_down && dst_up);
    let factor = conv_src / conv_dst;
    Some(if reversal { -factor } else { factor })
}

/// `createOperationsGeogToGeog` for two geographic CRS of the same datum, prime
/// meridian and vertical unit: a null geographic offset, which is `None` for any
/// other pair. Its name follows `buildOpName`, so two CRS of the same name get a
/// `(geog2D)` or `(geog3D)` qualifier.
pub(crate) fn null_offset(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
    let (src, dst) = (geographic_of(source)?, geographic_of(target)?);
    if !same_geodetic_datum(source, target) {
        return None;
    }
    let meridian =
        |crs: &Crs| geodetic_frame(crs).map(|frame| frame.prime_meridian().longitude().si_value());
    if meridian(source) != meridian(target) {
        return None;
    }
    let vertical_factor = |geog: &GeographicCRS| {
        geog.coordinate_system()
            .axes()
            .get(2)
            .map_or(1.0, |axis| axis.unit().conversion_to_si())
    };
    if vertical_factor(src) != vertical_factor(dst) {
        return None;
    }
    let three_d =
        src.coordinate_system().axes().len() == 3 || dst.coordinate_system().axes().len() == 3;
    let (code, values) = if three_d {
        (
            9660,
            vec![
                ParameterValue::angle(0.0),
                ParameterValue::angle(0.0),
                ParameterValue::length(0.0),
            ],
        )
    } else {
        (
            9619,
            vec![ParameterValue::angle(0.0), ParameterValue::angle(0.0)],
        )
    };
    let domain = match (crs_extent(source), crs_extent(target)) {
        (Some(left), Some(right)) if left == right => left,
        _ => world_extent(),
    };
    let name = op_name(NULL_GEOGRAPHIC_OFFSET, source, target);
    let mut operation = Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        code,
        values,
        domain,
    )
    .ok()?;
    operation.accuracy = Some(0.0);
    Some(CoordinateOperation::Transformation(operation))
}

/// `buildOpName`: `<prefix> from <source> to <target>`, qualified when the two
/// names are equal and the CRS differ in kind.
fn op_name(prefix: &str, source: &Crs, target: &Crs) -> String {
    let (source_name, target_name) = (source.name(), target.name());
    let (mut source_qualifier, mut target_qualifier) = ("", "");
    if source_name == target_name {
        source_qualifier = qualifier(source);
        target_qualifier = qualifier(target);
        if source_qualifier == target_qualifier {
            source_qualifier = "";
            target_qualifier = "";
        }
    }
    format!("{prefix} from {source_name}{source_qualifier} to {target_name}{target_qualifier}")
}

fn qualifier(crs: &Crs) -> &'static str {
    if let Some(geog) = geographic_of(crs) {
        return if geog.coordinate_system().axes().len() == 2 {
            " (geog2D)"
        } else {
            " (geog3D)"
        };
    }
    match crs {
        Crs::Geodetic(super::model::GeodeticCRS::Geocentric(_)) => " (geocentric)",
        _ => "",
    }
}

fn geographic_datum_name(geog: &GeographicCRS) -> Option<String> {
    geog.datum()
        .map(|datum| datum.name().to_owned())
        .or_else(|| {
            geog.datum_ensemble()
                .map(|ensemble| ensemble.name().to_owned())
        })
}

fn vertical_datum_name(vertical: &VerticalCRS) -> Option<String> {
    vertical
        .datum()
        .map(|datum| datum.name().to_owned())
        .or_else(|| {
            vertical
                .datum_ensemble()
                .map(|ensemble| ensemble.name().to_owned())
        })
}

/// `createOperationsCompoundToGeog` for a compound CRS with a bound component: the horizontal
/// and vertical operations are written as one pipeline, `createHorizVerticalHorizPROJBased` when
/// the vertical operation has an interpolation CRS and `createHorizVerticalPROJBased` when it has
/// none. Only the single source-to-interpolation and interpolation-to-target pair is considered;
/// PROJ's consistency filter between several pairs is not ported.
fn bound_compound_to_geog3d(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Crs::Compound(compound) = source else {
        return Ok(Vec::new());
    };
    let [horizontal, vertical] = compound.components() else {
        return Ok(Vec::new());
    };
    let Some(geog_dst) = geographic_of(target) else {
        return Ok(Vec::new());
    };
    if let Some(found) =
        bound_horizontal_and_vertical_to_other_datum(factory, source, target, context)?
    {
        return Ok(found);
    }
    let mut horizontal_ops = factory.create_candidates(horizontal, target, context)?;
    if horizontal_ops.is_empty() {
        horizontal_ops.extend(null_offset(horizontal, target));
    }
    let vertical_ops = factory.create_candidates(vertical, target, context)?;
    if horizontal_ops.is_empty() || vertical_ops.is_empty() {
        return Ok(horizontal_ops);
    }

    let mut found = Vec::new();
    let mut has_interpolated = false;
    for vertical_op in &vertical_ops {
        let Some(interpolation) = interpolation_geographic(vertical_op) else {
            continue;
        };
        let interpolation_crs = Crs::Geographic(interpolation.clone());
        let mut to_interpolation =
            factory.create_candidates(horizontal, &interpolation_crs, context)?;
        if to_interpolation.is_empty() {
            to_interpolation.extend(null_offset(horizontal, &interpolation_crs));
        }
        let height_axis = geog_dst.coordinate_system().axes().get(2).cloned();
        let interpolation_3d = Crs::Geographic(promote_with_axis(&interpolation, height_axis));
        let from_interpolation = if bound_cancels(horizontal, target, &interpolation) {
            to_interpolation
                .iter()
                .map(CoordinateOperation::inverse)
                .collect()
        } else {
            let mut candidates = factory.create_candidates(&interpolation_3d, target, context)?;
            if candidates.is_empty() {
                candidates.extend(null_offset(&interpolation_3d, target));
            }
            candidates
        };
        for first in &to_interpolation {
            for last in &from_interpolation {
                found.push(proj_based_horiz_vertical_horiz(
                    source,
                    target,
                    first,
                    vertical_op,
                    last,
                    &Crs::Geographic(interpolation.clone()),
                ));
                has_interpolated = true;
            }
        }
    }
    for vertical_op in &vertical_ops {
        if interpolation_geographic(vertical_op).is_none()
            && (!has_interpolated || vertical_op.has_ballpark_transformation())
        {
            for horizontal_op in &horizontal_ops {
                found.push(proj_based_horiz_vertical(
                    source,
                    target,
                    horizontal_op,
                    vertical_op,
                    geog_dst,
                ));
            }
        }
    }
    Ok(found)
}

/// `createOperationsCompoundToGeog` for `+proj=something +geoidgrids +nadgrids/+towgs84`: a
/// bound horizontal CRS and a bound vertical CRS whose hub is the horizontal base, to a CRS of
/// another datum. The vertical part goes to the hub of the vertical CRS, the horizontal bound
/// CRS, in 3D, to its own hub, and that hub to the target.
fn bound_horizontal_and_vertical_to_other_datum(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Option<Vec<CoordinateOperation>>, FactoryError> {
    let Crs::Compound(compound) = source else {
        return Ok(None);
    };
    let [Crs::Bound(horizontal), Crs::Bound(vertical)] = compound.components() else {
        return Ok(None);
    };
    let Some(horizontal_geog) = horizontal.source_crs().extract_geographic_crs() else {
        return Ok(None);
    };
    let Some(target_geog) = target.extract_geographic_crs() else {
        return Ok(None);
    };
    let db = context.database();
    let horizontal_geog_crs = Crs::Geographic(horizontal_geog);
    if !vertical
        .target_crs()
        .demote_to_2d(Some(db))
        .is_equivalent_to_with(&horizontal_geog_crs, Equivalence::Equivalent)
        || same_geodetic_datum(&horizontal_geog_crs, &Crs::Geographic(target_geog))
    {
        return Ok(None);
    }
    let vertical_dest = vertical.target_crs().clone();
    let first_source = CompoundCRS::create(
        "",
        vec![
            horizontal.source_crs().clone(),
            Crs::Bound(vertical.clone()),
        ],
    )?;
    let first = factory.create_candidates(&Crs::Compound(first_source), &vertical_dest, context)?;
    let horizontal_dest = horizontal.target_crs().promote_to_3d(Some(db));
    let bound_geog = Crs::Bound(BoundCRS::create(
        horizontal_geog_crs,
        horizontal.target_crs().clone(),
        horizontal.transformation().clone(),
    ))
    .promote_to_3d(Some(db));
    let second = factory.create_candidates(&bound_geog, &horizontal_dest, context)?;
    let third = factory.create_candidates(&horizontal_dest, target, context)?;
    let mut res = Vec::new();
    for op1 in &first {
        let op1 = with_endpoints(op1.clone(), source, &vertical_dest);
        for op2 in &second {
            let op2 = with_endpoints(op2.clone(), &vertical_dest, &horizontal_dest);
            for op3 in &third {
                res.extend(compute_metadata(
                    vec![op1.clone(), op2.clone(), op3.clone()],
                    context,
                ));
            }
        }
    }
    Ok(if res.is_empty() { None } else { Some(res) })
}

/// The base of a bound horizontal CRS is the target and its hub the interpolation CRS: the
/// horizontal operation to the interpolation CRS is then reused, inverted, after the vertical step.
fn bound_cancels(horizontal: &Crs, target: &Crs, interpolation: &GeographicCRS) -> bool {
    let Crs::Bound(bound) = horizontal else {
        return false;
    };
    let (Some(base), Some(target)) = (geographic_of(bound.source_crs()), geographic_of(target))
    else {
        return false;
    };
    let Some(hub) = geographic_of(bound.target_crs()) else {
        return false;
    };
    base.is_equivalent_to_with(&target.demote_to_2d(None), Equivalence::Equivalent)
        && hub
            .demote_to_2d(None)
            .is_equivalent_to_with(&interpolation.demote_to_2d(None), Equivalence::Equivalent)
}

/// `getInterpolationGeogCRS`: the 2D geographic CRS a vertical operation interpolates in.
fn interpolation_geographic(operation: &CoordinateOperation) -> Option<GeographicCRS> {
    let transformation = match operation {
        CoordinateOperation::Transformation(transformation) => Some(transformation),
        CoordinateOperation::Concatenated(concatenated) => {
            let mut transformations = concatenated.steps().iter().filter_map(|step| {
                match step.operation() {
                    // A same-datum change of unit is a conversion in PROJ.
                    CoordinateOperation::Transformation(transformation)
                        if !is_same_datum_unit_change(transformation) =>
                    {
                        Some(transformation)
                    }
                    _ => None,
                }
            });
            let first = transformations.next();
            if transformations.next().is_some() {
                None
            } else {
                first
            }
        }
        CoordinateOperation::Conversion(_) => None,
    }?;
    if operation.has_ballpark_transformation()
        || transformation
            .name()
            .to_ascii_lowercase()
            .contains("ballpark")
    {
        return None;
    }
    let interpolation = match transformation.interpolation_crs() {
        Some(crs) => geographic_of(crs)?.clone(),
        None => geographic_of(transformation.target_crs()?)?.clone(),
    };
    Some(interpolation.demote_to_2d(None))
}

fn is_same_datum_unit_change(transformation: &Transformation) -> bool {
    transformation.method().code() == "1069"
        && !transformation
            .name()
            .to_ascii_lowercase()
            .contains("ballpark")
}

/// `createOperationsGeogToGeog` when two geographic CRS of the same datum differ only by
/// their height unit: a change of vertical unit (method 1069). A 2D CRS counts as metres.
/// A prime-meridian shift or a lat/lon reversal takes PROJ's proj-based path, which is not
/// ported, and gives nothing here. A different datum is the ballpark path.
pub(super) fn geographic_height_change(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
    let (src, dst) = (geographic_of(source)?, geographic_of(target)?);
    if !same_geodetic_datum(source, target) || horizontal_axes_reversed(src, dst) {
        return None;
    }
    let src_factor = vertical_si(src);
    let dst_factor = vertical_si(dst);
    if src_factor == dst_factor || dst_factor == 0.0 {
        return None;
    }
    let name = format!("Transformation from {} to {}", source.name(), target.name());
    Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        1069,
        vec![ParameterValue::scale(src_factor / dst_factor)],
        world_extent(),
    )
    .ok()
    .map(CoordinateOperation::Transformation)
}

/// Height unit factor, metres for a 2D geographic CRS.
fn vertical_si(geographic: &GeographicCRS) -> f64 {
    geographic
        .coordinate_system()
        .axes()
        .get(2)
        .map_or(1.0, |axis| axis.unit().conversion_to_si())
}

/// Lat/lon against lon/lat. Height-axis direction is not an axis reversal.
fn horizontal_axes_reversed(src: &GeographicCRS, dst: &GeographicCRS) -> bool {
    let first = |geographic: &GeographicCRS| {
        geographic
            .coordinate_system()
            .axes()
            .first()
            .map(|axis| axis.direction().to_ascii_lowercase())
    };
    let (Some(src), Some(dst)) = (first(src), first(dst)) else {
        return false;
    };
    let latitude = |direction: &str| direction == "north" || direction == "south";
    let longitude = |direction: &str| direction == "east" || direction == "west";
    (latitude(&src) && longitude(&dst)) || (longitude(&src) && latitude(&dst))
}

/// `demoteTo2D` then `promoteTo3D` with the given height axis, metres when there is none.
fn promote_with_axis(geographic: &GeographicCRS, height: Option<Axis>) -> GeographicCRS {
    let mut promoted = geographic.demote_to_2d(None).promote_to_3d();
    if let Some(height) = height {
        if let Some(axis) = promoted.coordinate_system.axes.get_mut(2) {
            *axis = height;
        }
    }
    promoted
}

fn is_null_offset(operation: &CoordinateOperation) -> bool {
    operation.name().starts_with(NULL_GEOGRAPHIC_OFFSET) && !operation.name().contains(" + ")
}

fn accuracy_sum(operations: &[&CoordinateOperation]) -> Option<f64> {
    operations
        .iter()
        .map(|operation| match operation {
            CoordinateOperation::Conversion(_) => Some(0.0),
            other => other.accuracy(),
        })
        .sum::<Option<f64>>()
}

fn proj_based_operation(
    name: String,
    source: &Crs,
    target: &Crs,
    steps: Vec<CoordinateOperation>,
    accuracy: Option<f64>,
    proj_based: ProjBased,
) -> CoordinateOperation {
    CoordinateOperation::Concatenated(ConcatenatedOperation {
        identified: Identified::named(name),
        steps: steps
            .into_iter()
            .map(|operation| ConcatenatedStep {
                operation,
                reverse: false,
            })
            .collect(),
        source: Some(Box::new(source.clone())),
        target: Some(Box::new(target.clone())),
        accuracy,
        version: None,
        computed_name: false,
        source_epoch: None,
        target_epoch: None,
        proj_based: Some(Box::new(proj_based)),
    })
}

/// `createHorizVerticalHorizPROJBased`.
fn proj_based_horiz_vertical_horiz(
    source: &Crs,
    target: &Crs,
    first: &CoordinateOperation,
    vertical: &CoordinateOperation,
    last: &CoordinateOperation,
    interpolation: &Crs,
) -> CoordinateOperation {
    let mut steps = Vec::new();
    if !is_null_offset(first) {
        steps.push(first.clone());
    }
    steps.push(vertical.clone());
    if !is_null_offset(last) {
        steps.push(last.clone());
    }
    let cancels = steps.len() == 3 && last.inverse().name() == first.name();
    let name = if cancels {
        let helper = if first.name().starts_with("Inverse of") {
            last.name()
        } else {
            first.name()
        };
        format!("{} using {helper}", vertical.name())
    } else {
        steps
            .iter()
            .map(CoordinateOperation::name)
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let accuracy = accuracy_sum(&steps.iter().collect::<Vec<_>>());
    // The extent is the intersection of the steps' (`getExtent(ops, false, ...)`,
    // `createHorizVerticalHorizPROJBased`, coordinateoperationfactory.cpp:2851-2864).
    let domain = joint_extent(&steps).flatten();
    let mut operation = proj_based_operation(
        name,
        source,
        target,
        steps,
        accuracy,
        ProjBased {
            kind: ProjBasedKind::HorizVerticalHoriz,
            horizontal: first.clone(),
            vertical: vertical.clone(),
            last: Some(last.clone()),
            interpolation: interpolation.clone(),
            inverted: false,
        },
    );
    // A name joined from the steps is rebuilt from the inverted steps by the inverse.
    if let CoordinateOperation::Concatenated(concatenated) = &mut operation {
        concatenated.computed_name = !cancels;
        concatenated.identified.domains = domain.into_iter().collect();
    }
    operation
}

/// `createHorizVerticalPROJBased`.
fn proj_based_horiz_vertical(
    source: &Crs,
    target: &Crs,
    horizontal: &CoordinateOperation,
    vertical: &CoordinateOperation,
    geog_dst: &GeographicCRS,
) -> CoordinateOperation {
    let (name, steps) = if is_null_offset(horizontal) {
        (vertical.name().to_owned(), vec![vertical.clone()])
    } else {
        (
            format!("{} + {}", horizontal.name(), vertical.name()),
            vec![horizontal.clone(), vertical.clone()],
        )
    };
    let accuracy = accuracy_sum(&steps.iter().collect::<Vec<_>>());
    proj_based_operation(
        name,
        source,
        target,
        steps,
        accuracy,
        ProjBased {
            kind: ProjBasedKind::HorizVertical,
            horizontal: horizontal.clone(),
            vertical: vertical.clone(),
            last: None,
            interpolation: Crs::Geographic(geog_dst.clone()),
            inverted: false,
        },
    )
}
