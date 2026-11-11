//! PROJ's `createOperationsCompoundToGeog` (`coordinateoperationfactory.cpp`): a compound CRS of a
//! horizontal CRS and a vertical CRS, to a geographic CRS, with its derived horizontal branch,
//! the NAD83(CSRS)v7 epoch case, the only-vertical-ballpark intermediate and the interpolation
//! pairs. Also `createOperationsCompoundToCompound` for two compound CRS, and the ballpark of
//! `createOperationsVertToVert`.
//!
//! Not ported here: the bound-and-bound branch of `createOperationsCompoundToGeog` (6570-6640),
//! which `bound_compound_to_geog3d` approximates, the same-datum change of vertical unit as a
//! conversion in `createOperationsVertToGeogSynthetized`, and the NAD83(CSRS)v7 hack of
//! `createOperationsCompoundToCompound`.

use super::bound::{extract_geographic, HORIZONTAL_SHIFT_GTIFF};
use super::bound_ops::{operation_source, operation_target};
use super::buildop::{concatenation_interpolation, interpolation_crs_of, OpProps};
use super::derived::joint_extent;
use super::equiv::{geodetic_frame, same_geodetic_datum, Equivalence};
use super::factory::{
    crs_extent, find_ops_in_registry_direct_to, geographic_of, point_motion_candidates,
    sort_extents, world_extent, CoordinateOperationContext, CoordinateOperationFactory,
    DatabaseContext, Epochs, GridAvailabilityUse, SortExtents, GUARD_ALTERNATIVE_GEOG,
    GUARD_SKIP_HORIZONTAL, GUARD_VERTICAL_INTERMEDIATE, GUARD_VERTICAL_PIVOT,
};
use super::model::{
    Axis, BoundCRS, CompoundCRS, ConcatenatedOperation, ConcatenatedStep, Conversion,
    CoordinateOperation, Crs, GeographicCRS, GeoidModel, Identified, OperationMethod, Parameter,
    ParameterValue, ProjBased, ProjBasedKind, Transformation, UnitOfMeasure, VerticalCRS,
    VerticalCS,
};
use super::pipeline::ProjLines;
use super::FactoryError;

const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";
const BALLPARK_VERTICAL_TRANSFORMATION: &str = "ballpark vertical transformation";

/// `createOperationsCompoundToGeog` (coordinateoperationfactory.cpp:6454-7172): a compound CRS of
/// a horizontal and a vertical CRS to a geographic CRS. Empty for any pair that is not a
/// two-component compound to a geographic CRS.
pub(super) fn compound_to_geog3d(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    epochs: Epochs,
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
    // A derived horizontal part (coordinateoperationfactory.cpp:6473-6502): the source to the
    // compound of its base and the vertical part, then that compound to the target.
    // The CRS extents of the outermost call reach every sub-call, as PROJ's `Context` keeps
    // `extent1` and `extent2`.
    let inherited = context.with_inherited_extents(source, target);
    if let Some(base) = super::derived::derived_base(horizontal) {
        return derived_horizontal_to_geog(
            factory, source, target, &base, vertical, epochs, &inherited,
        );
    }
    if let Some(found) = nad83_csrs_v7_epoch_hack(
        factory, source, target, horizontal, vertical, geog_dst, epochs, &inherited,
    )? {
        return Ok(found);
    }
    // A bound CRS of a derived CRS (coordinateoperationfactory.cpp:6539-6568): the same, through
    // the compound of the bound CRS of its base, with the same hub and transformation.
    if let Crs::Bound(bound) = horizontal {
        if let Some(base) = super::derived::derived_base(bound.source_crs()) {
            let bound_base = Crs::Bound(BoundCRS::create(
                base,
                bound.target_crs().clone(),
                bound.transformation().clone(),
            ));
            return derived_horizontal_to_geog(
                factory,
                source,
                target,
                &bound_base,
                vertical,
                epochs,
                &inherited,
            );
        }
    }
    if geog_dst.coordinate_system().axes().len() != 3 {
        return horizontal_only(factory, source, target, horizontal, context);
    }
    match vertical {
        Crs::Vertical(_) => {}
        Crs::Bound(bound) if matches!(bound.source_crs(), Crs::Vertical(_)) => {}
        _ => return Ok(Vec::new()),
    }
    if matches!(horizontal, Crs::Bound(_)) || matches!(vertical, Crs::Bound(_)) {
        return bound_compound_to_geog3d(factory, source, target, context);
    }
    let Some(geog_src) = extract_geographic(horizontal) else {
        return Ok(Vec::new());
    };
    compound_horizontal_vertical(
        factory, source, target, horizontal, vertical, geog_src, geog_dst, &inherited,
    )
}

/// `createOperationsCompoundToGeog`, derived horizontal part (coordinateoperationfactory.cpp:
/// 6473-6502): the first operation of the source to the compound of the base CRS and the vertical
/// part, chained with each operation of that compound to the target.
fn derived_horizontal_to_geog(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    base: &Crs,
    vertical: &Crs,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let interm = Crs::Compound(CompoundCRS::create(
        format!("{} + {}", base.name(), vertical.name()),
        vec![base.clone(), vertical.clone()],
    )?);
    let at_source = Epochs {
        source: epochs.source,
        target: epochs.source,
    };
    let ops_first = factory.candidates_at_epochs(source, at_source, &interm, context)?;
    // PROJ asserts that `opsFirst` is not empty.
    let Some(first) = ops_first.into_iter().next() else {
        return Ok(Vec::new());
    };
    let ops_last = factory.candidates_at_epochs(&interm, epochs, target, context)?;
    Ok(ops_last
        .into_iter()
        .filter_map(|last| compute_metadata(vec![first.clone(), last], context))
        .collect())
}

/// The NAD83(CSRS)v7 case of `createOperationsCompoundToGeog` (coordinateoperationfactory.cpp:
/// 6504-6566): `NAD83(CSRS)v7 + CGVD2013a(1997) height` at epoch 1997 to `NAD83(CSRS)v7` 3D at the
/// same epoch goes through `NAD83(CSRS)v3 + CGVD2013a(1997) height` to `NAD83(CSRS)v3`, and the
/// 2002 case through `NAD83(CSRS)v4`. The operations are those of the intermediate pair, with the
/// source and the target of the call. `None` when the case does not apply.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of one compound search, as PROJ passes them"
)]
fn nad83_csrs_v7_epoch_hack(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    horizontal: &Crs,
    vertical: &Crs,
    geog_dst: &GeographicCRS,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Result<Option<Vec<CoordinateOperation>>, FactoryError> {
    let Some(geog_src) = geographic_of(horizontal) else {
        return Ok(None);
    };
    let (Some(source_epoch), Some(target_epoch)) = (epochs.source, epochs.target) else {
        return Ok(None);
    };
    if geog_src.name() != "NAD83(CSRS)v7"
        || geog_dst.coordinate_system().axes().len() != 3
        || geog_dst.name() != geog_src.name()
        || source_epoch != target_epoch
    {
        return Ok(None);
    }
    let is_1997 = (source_epoch - 1997.0).abs() < 1e-10;
    let is_2002 = (source_epoch - 2002.0).abs() < 1e-10;
    let epsg = super::factory::AuthorityFactory::create(context.database(), "EPSG");
    let (Ok(v7_2d), Ok(v7_3d)) = (
        epsg.create_coordinate_reference_system("8255"),
        epsg.create_coordinate_reference_system("8254"),
    ) else {
        return Ok(None);
    };
    if !horizontal.is_equivalent_to_with(&v7_2d, Equivalence::Equivalent)
        || !Crs::Geographic(geog_dst.clone()).is_equivalent_to_with(&v7_3d, Equivalence::Equivalent)
        || !((is_1997 && vertical.name() == "CGVD2013a(1997) height")
            || (is_2002 && vertical.name() == "CGVD2013a(2002) height"))
    {
        return Ok(None);
    }
    let (Ok(interm_2d), Ok(interm_3d)) = (
        epsg.create_coordinate_reference_system(if is_1997 { "8240" } else { "8246" }),
        epsg.create_coordinate_reference_system(if is_1997 { "8239" } else { "8244" }),
    ) else {
        return Ok(None);
    };
    let new_compound = Crs::Compound(CompoundCRS::create(
        format!("{} + {}", interm_2d.name(), vertical.name()),
        vec![interm_2d, vertical.clone()],
    )?);
    let at_source = Epochs {
        source: Some(source_epoch),
        target: Some(source_epoch),
    };
    let ops = factory.candidates_at_epochs(&new_compound, at_source, &interm_3d, context)?;
    Ok(Some(
        ops.into_iter()
            .map(|operation| with_endpoints(operation, source, target))
            .collect(),
    ))
}

/// `createOperationsCompoundToGeog` for a geographic horizontal part and a vertical part, to a 3D
/// geographic CRS (coordinateoperationfactory.cpp:6683-7170).
///
/// The horizontal operations go to the full target. The vertical operations go to the target with
/// the horizontal transformation skipped, and to the geographic CRS of the source in 3D when it
/// differs (`geogCRSTmp`). When all the vertical operations are ballpark, the geographic CRS of
/// the horizontal operations are tried as intermediates (issue 4618). A vertical operation that
/// names an interpolation CRS is chained with the horizontal operations to and from that CRS
/// (`createHorizVerticalHorizPROJBased`); the others take the horizontal operations to the target
/// (`createHorizVerticalPROJBased`) when no operation of the first kind exists or the vertical one
/// is ballpark.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of one compound search, as PROJ passes them"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one PROJ function, kept in its order"
)]
fn compound_horizontal_vertical(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    horizontal: &Crs,
    vertical: &Crs,
    geog_src: &GeographicCRS,
    geog_dst: &GeographicCRS,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let db = context.database();
    let ignore_grids = context.grid_availability == GridAvailabilityUse::IgnoreGridAvailability;
    let horiz_transforms = factory.create_candidates(horizontal, target, context)?;
    let has_all_grids = |operations: &[CoordinateOperation]| -> (bool, bool) {
        let mut registered = false;
        for operation in operations {
            if operation.identifiers().is_empty() {
                continue;
            }
            registered = true;
            if ignore_grids
                || operation
                    .grids_needed(db, GridAvailabilityUse::KnownAvailable)
                    .is_ok_and(|grids| grids.iter().all(|grid| grid.info.available))
            {
                return (true, true);
            }
        }
        (false, registered)
    };
    // `SetGeogCRSOfVertCRS` (coordinateoperationfactory.cpp:6760-6777): the datum of the
    // geographic CRS of the source is what an `Ellipsoid` vertical datum means.
    let horizontal_datum = geographic_datum_name(geog_src);
    let mut res = Vec::new();
    let mut vertical_transforms = Vec::new();
    if let Some(vertical_crs) = vertical.extract_vertical_crs() {
        // `SetSkipHorizontalTransform` (coordinateoperationfactory.cpp:6745-6784).
        let skip_horizontal = skip_horizontal_context(context);
        vertical_transforms = vertical_to_geog_candidates(
            factory,
            vertical,
            &vertical_crs,
            target,
            horizontal_datum.as_deref(),
            &skip_horizontal,
        )?;
        let (found_with_all_grids, _) = has_all_grids(&vertical_transforms);
        let src_geographic = Crs::Geographic(geog_src.clone());
        let dst_geographic = Crs::Geographic(geog_dst.clone());
        if !src_geographic.is_equivalent_to_with(&dst_geographic, Equivalence::Equivalent)
            && !geog_src.is_2d_part_of_3d(geog_dst)
        {
            let geog_tmp = Crs::Geographic(
                geog_src
                    .demote_to_2d(Some(db))
                    .promote_to_3d_with_axis(Some(db), geog_dst.coordinate_system().axes().get(2)),
            );
            let vertical_tmp = vertical_to_geog_candidates(
                factory,
                vertical,
                &vertical_crs,
                &geog_tmp,
                horizontal_datum.as_deref(),
                &skip_horizontal,
            )?;
            let (found_with_all_grids_2, found_registered) = has_all_grids(&vertical_tmp);
            if found_with_all_grids_2 && !found_with_all_grids {
                vertical_transforms = vertical_tmp;
            } else if found_registered {
                vertical_transforms.extend(vertical_tmp);
            }
        }

        // Case for EPSG:5550+7651 to EPSG:9754 (https://github.com/OSGeo/PROJ/issues/4618,
        // coordinateoperationfactory.cpp:6837-6905): when the vertical operations are all
        // ballpark, the geographic CRS of the horizontal operations that has a vertical
        // operation that is not ballpark is an intermediate.
        if vertical_transforms
            .iter()
            .all(CoordinateOperation::has_ballpark_transformation)
        {
            let mut candidates: std::collections::BTreeMap<String, Crs> =
                std::collections::BTreeMap::new();
            for operation in &horiz_transforms {
                if operation.has_ballpark_transformation() {
                    continue;
                }
                let CoordinateOperation::Concatenated(concatenated) = operation else {
                    continue;
                };
                for step in concatenated.steps() {
                    let Some(sub_target) = operation_target(step.operation()) else {
                        continue;
                    };
                    let tmp = sub_target.promote_to_3d(Some(db));
                    if !tmp.identifiers().is_empty()
                        && !tmp.is_equivalent_to_with(target, Equivalence::Equivalent)
                    {
                        candidates.entry(tmp.name().to_owned()).or_insert(tmp);
                    }
                }
            }
            for geog_tmp in candidates.values() {
                let vertical_tmp = vertical_to_geog_candidates(
                    factory,
                    vertical,
                    &vertical_crs,
                    geog_tmp,
                    horizontal_datum.as_deref(),
                    context,
                )?;
                if vertical_tmp
                    .iter()
                    .all(CoordinateOperation::has_ballpark_transformation)
                {
                    continue;
                }
                let ops1 = factory.create_candidates(source, geog_tmp, context)?;
                let ops2 = factory.create_candidates(geog_tmp, target, context)?;
                for op1 in ops1.iter().filter(|op| !op.has_ballpark_transformation()) {
                    for op2 in ops2.iter().filter(|op| !op.has_ballpark_transformation()) {
                        res.extend(compute_metadata(vec![op1.clone(), op2.clone()], context));
                    }
                }
            }
        }
    }

    if horiz_transforms.is_empty() || vertical_transforms.is_empty() {
        return Ok(horiz_transforms);
    }

    let target_2d = target.demote_to_2d(Some(db));
    let same_geog = horizontal.is_equivalent_to_with(&target_2d, Equivalence::Equivalent);
    // The operations to and from an interpolation CRS, by its identifier (PROJ's
    // `cacheHorizToInterpAndInterpToTarget`).
    let mut cache: std::collections::BTreeMap<
        String,
        (Vec<CoordinateOperation>, Vec<CoordinateOperation>),
    > = std::collections::BTreeMap::new();
    let mut with_interpolation = false;
    for vertical_transform in &vertical_transforms {
        let Some(interpolation_geog) = interpolation_geographic(vertical_transform, Some(db))
        else {
            continue;
        };
        let key = interpolation_geog
            .identifiers()
            .first()
            .map(|identifier| format!("{}:{}", identifier.code_space(), identifier.code()));
        let cached = key.as_ref().and_then(|key| cache.get(key)).cloned();
        let (src_to_interp, interp_to_target) = match cached {
            Some(pair) => pair,
            None => {
                let pair = interpolation_legs(
                    factory,
                    horizontal,
                    target,
                    &interpolation_geog,
                    geog_dst,
                    context,
                )?;
                if let Some(key) = key {
                    cache.insert(key, pair.clone());
                }
                pair
            }
        };
        let interpolation = Crs::Geographic(interpolation_geog);
        let has_only_one_op = src_to_interp.len() == 1 && interp_to_target.len() == 1;
        for first in &src_to_interp {
            for last in &interp_to_target {
                if !has_only_one_op
                    && ((same_geog && datum_names_used(first) != datum_names_used(last))
                        || !compatible_transformations(first, last))
                {
                    continue;
                }
                if let Some(operation) = horiz_vertical_horiz_checked(
                    source,
                    target,
                    first,
                    vertical_transform,
                    last,
                    &interpolation,
                    context,
                ) {
                    res.push(operation);
                    with_interpolation = true;
                }
            }
        }
    }

    for vertical_transform in &vertical_transforms {
        if interpolation_geographic(vertical_transform, Some(db)).is_none()
            && (!with_interpolation || vertical_transform.has_ballpark_transformation())
        {
            for horizontal_transform in &horiz_transforms {
                res.extend(combine(
                    source,
                    target,
                    horizontal_transform,
                    vertical_transform,
                    context,
                ));
            }
        }
    }
    Ok(res)
}

/// `createOperations(vertical, geographic)` for the vertical part of a compound: the candidates,
/// and the ballpark change of unit of `createOperationsVertToGeogSynthetized`, whose `Ellipsoid`
/// vertical datum is the datum `horizontal_datum` of the compound's geographic CRS
/// (`geogCRSOfVertCRSStack`).
fn vertical_to_geog_candidates(
    factory: &CoordinateOperationFactory,
    vertical: &Crs,
    vertical_crs: &VerticalCRS,
    geographic: &Crs,
    horizontal_datum: Option<&str>,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let mut operations = factory.create_candidates(vertical, geographic, context)?;
    if !operations.iter().any(is_synthesized_ballpark) {
        if let Some(geog) = geographic_of(geographic) {
            operations.extend(vertical_ballpark(
                vertical,
                vertical_crs,
                horizontal_datum,
                geographic_datum_name(geog).as_deref(),
                geog,
                geographic,
            ));
        }
    }
    Ok(operations)
}

/// The operations of the horizontal part to the interpolation CRS, in 2D, and those from the
/// interpolation CRS in 3D, with the height axis of the target, to the target
/// (coordinateoperationfactory.cpp:6949-7006). The GTiff horizontal shifts are dropped from the
/// second.
fn interpolation_legs(
    factory: &CoordinateOperationFactory,
    horizontal: &Crs,
    target: &Crs,
    interpolation: &GeographicCRS,
    geog_dst: &GeographicCRS,
    context: &CoordinateOperationContext,
) -> Result<(Vec<CoordinateOperation>, Vec<CoordinateOperation>), FactoryError> {
    let db = context.database();
    let src_to_interp =
        factory.create_candidates(horizontal, &Crs::Geographic(interpolation.clone()), context)?;
    let interp_3d = Crs::Geographic(
        interpolation
            .demote_to_2d(Some(db))
            .promote_to_3d_with_axis(Some(db), geog_dst.coordinate_system().axes().get(2)),
    );
    let interp_to_target = factory
        .create_candidates(&interp_3d, target, context)?
        .into_iter()
        .filter(|operation| !is_gtiff_shift(operation, context))
        .collect();
    Ok((src_to_interp, interp_to_target))
}

/// A single operation whose method is PROJ's GTiff horizontal shift
/// (`PROJ_WKT2_NAME_METHOD_HORIZONTAL_SHIFT_GTIFF`). PROJ reads the registry operations with its
/// own grid names by default, which turns an NTv2 or NADCON grid with a GeoTIFF alternative into
/// that method (`substitutePROJAlternativeGridNames`); the candidates here keep the database
/// method, so the substitution is made for the test.
fn is_gtiff_shift(operation: &CoordinateOperation, context: &CoordinateOperationContext) -> bool {
    let method_is_gtiff = |name: &str| name.contains(HORIZONTAL_SHIFT_GTIFF);
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            if method_is_gtiff(transformation.method().name()) {
                return true;
            }
            context.uses_proj_grid_names()
                && super::proj_grid_substitution::substitute_proj_alternative_grid_names(
                    transformation,
                    context.database(),
                )
                .ok()
                .flatten()
                .is_some_and(|substituted| method_is_gtiff(substituted.method().name()))
        }
        CoordinateOperation::Conversion(conversion) => method_is_gtiff(conversion.method().name()),
        CoordinateOperation::Concatenated(_) => false,
    }
}

/// The reverse of [`compound_to_geog3d`], for a geographic source and a compound
/// target (`applyInverse` of PROJ's dispatch).
pub(super) fn geog_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    if !matches!(target, Crs::Compound(_)) || geographic_of(source).is_none() {
        return Ok(Vec::new());
    }
    let reversed = Epochs {
        source: epochs.target,
        target: epochs.source,
    };
    Ok(
        compound_to_geog3d(factory, target, source, reversed, context)?
            .iter()
            .map(CoordinateOperation::inverse)
            .collect(),
    )
}

/// The point motion block of `createOperationsCompoundToCompound` (coordinateoperationfactory.cpp:
/// 7228-7290): two compound CRS whose horizontal parts are the same geographic CRS, at two
/// different epochs, when that datum has a point motion. Each candidate chains the operations from
/// the source to the geographic 3D CRS, the point motion of that CRS between the two epochs
/// (`candidates_at_epochs`), and the operations from it to the target, with no ballpark operation
/// at any of the three places. Empty when the block does not apply.
pub(super) fn compound_point_motion(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    epochs: Epochs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (Crs::Compound(src), Crs::Compound(dst)) = (source, target) else {
        return Ok(Vec::new());
    };
    let Some((source_epoch, target_epoch)) = epochs.differing() else {
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
    if !src_geog.is_equivalent_to_with(&dst_geog, Equivalence::Equivalent) {
        return Ok(Vec::new());
    }
    let src_geog = Crs::Geographic(src_geog);
    if point_motion_candidates(&src_geog, context)?.is_empty() {
        return Ok(Vec::new());
    }
    let geog3d = src_geog.promote_to_3d(Some(context.database()));
    let ops_first = factory.candidates_at_epochs(
        source,
        Epochs {
            source: Some(source_epoch),
            target: Some(source_epoch),
        },
        &geog3d,
        context,
    )?;
    let ops_middle = factory.candidates_at_epochs(
        &geog3d,
        Epochs {
            source: Some(source_epoch),
            target: Some(target_epoch),
        },
        &geog3d,
        context,
    )?;
    let ops_last = factory.candidates_at_epochs(
        &geog3d,
        Epochs {
            source: Some(target_epoch),
            target: Some(target_epoch),
        },
        target,
        context,
    )?;
    let mut res = Vec::new();
    for first in ops_first
        .iter()
        .filter(|op| !op.has_ballpark_transformation())
    {
        for middle in ops_middle
            .iter()
            .filter(|op| !op.has_ballpark_transformation())
        {
            for last in ops_last
                .iter()
                .filter(|op| !op.has_ballpark_transformation())
            {
                res.extend(compute_metadata(
                    vec![first.clone(), middle.clone(), last.clone()],
                    context,
                ));
            }
        }
    }
    Ok(res)
}

/// `createOperationsCompoundToCompound` for two compound CRS of the same number of components.
/// The horizontal parts go through the geographic CRS of each, promoted to 3D: the registry
/// operations between them, the vertical operations between the vertical parts, and the
/// intermediate geographic CRS when the vertical operation is ballpark or a geoid trigger
/// applies. Only the first of PROJ's branches that gives a result is kept. The point motion block
/// comes first (`compound_point_motion`, called by the caller); the epochs reach the geographic
/// pair of the intermediate step.
pub(super) fn compound_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    epochs: Epochs,
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
    // `createOperations(intermGeogSrc, sourceEpoch, intermGeogDst, targetEpoch)`: the epochs
    // reach the geographic pair, whose point motion is read at them (coordinateoperationfactory.
    // cpp:7255-7257).
    let geog_ops = factory.candidates_at_epochs(&inter_src, epochs, &inter_dst, context)?;
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
        if let Some(interpolation_crs) = interpolation_crs_of(vertical_transform) {
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
pub(super) fn compute_metadata(
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
    let interpolation = concatenation_interpolation(&steps).map(Box::new);
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
        interpolation,
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

/// `setCRSs` of the factory on a copy: the source and target of the operation, with its
/// identifiers kept. It is `CoordinateOperation::setCRSsUpdateInverse`
/// (coordinateoperationfactory.cpp:3127-3132): for a concatenated operation the source also goes
/// to its first step and the target to its last one (singleoperation.cpp:257-270), so the angular
/// unit and axis steps a step writes follow the CRS the concatenation was given.
fn with_endpoints(
    mut operation: CoordinateOperation,
    source: &Crs,
    target: &Crs,
) -> CoordinateOperation {
    set_crss_update_inverse(&mut operation, source, target);
    operation
}

fn set_crss_update_inverse(operation: &mut CoordinateOperation, source: &Crs, target: &Crs) {
    match operation {
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
            if let Some(first) = concatenated.steps.first_mut() {
                update_step_end(first, EndOfStep::Source, source);
            }
            if let Some(last) = concatenated.steps.last_mut() {
                update_step_end(last, EndOfStep::Target, target);
            }
        }
    }
}

/// The end of a step in the direction it runs.
#[derive(Clone, Copy)]
enum EndOfStep {
    Source,
    Target,
}

/// `first->setCRSsUpdateInverse(sourceCRSIn, firstTarget)` and
/// `last->setCRSsUpdateInverse(lastSource, targetCRSIn)`: one end of the step is replaced, the
/// other one is kept, and nothing changes when the step has no other end. A step that runs
/// reversed stores its operation the other way round.
fn update_step_end(step: &mut ConcatenatedStep, end: EndOfStep, crs: &Crs) {
    let stored_source = operation_source(&step.operation).cloned();
    let stored_target = operation_target(&step.operation).cloned();
    let (running_source, running_target) = if step.reverse {
        (stored_target, stored_source)
    } else {
        (stored_source, stored_target)
    };
    let (new_source, new_target) = match end {
        EndOfStep::Source => (Some(crs.clone()), running_target),
        EndOfStep::Target => (running_source, Some(crs.clone())),
    };
    let (Some(new_source), Some(new_target)) = (new_source, new_target) else {
        return;
    };
    if step.reverse {
        set_crss_update_inverse(&mut step.operation, &new_target, &new_source);
    } else {
        set_crss_update_inverse(&mut step.operation, &new_source, &new_target);
    }
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
    with_remarks(operation, &[horizontal, vertical])
}

/// `createHorizVerticalPROJBased` (coordinateoperationfactory.cpp:2717-2793): a null horizontal
/// part leaves the vertical operation as it is, under the compound source and the target.
/// Otherwise the two are chained as a proj-based operation written by
/// `MyPROJStringExportableHorizVertical`. `None` when the extents of the two do not meet and the
/// context refuses it (PROJ throws there and the candidate is dropped).
fn combine(
    source: &Crs,
    target: &Crs,
    horizontal: &CoordinateOperation,
    vertical: &CoordinateOperation,
    context: &CoordinateOperationContext,
) -> Option<CoordinateOperation> {
    let geog_dst = geographic_of(target)?;
    if is_null_offset(horizontal) {
        // `horizTransformIsNoOp`: the name, extent and accuracy of the vertical operation, still
        // written by the `HorizVertical` exportable.
        let mut operation =
            proj_based_horiz_vertical(source, target, horizontal, vertical, geog_dst);
        if let CoordinateOperation::Concatenated(concatenated) = &mut operation {
            concatenated.identified.domains = super::factory::operation_extent(vertical)
                .into_iter()
                .collect();
        }
        return Some(operation);
    }
    let steps = [horizontal.clone(), vertical.clone()];
    let domain = match joint_extent(&steps) {
        Some(domain) => domain,
        None if context.disallows_empty_intersection() => return None,
        None => None,
    };
    let mut operation = proj_based_horiz_vertical(source, target, horizontal, vertical, geog_dst);
    if let CoordinateOperation::Concatenated(concatenated) = &mut operation {
        concatenated.identified.domains = domain.into_iter().collect();
    }
    Some(operation)
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
        (Some(left), Some(right)) if left.is_equivalent_to(&right) => left,
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
/// transformation on its PROJ grid (`PROJ <grid>`).
///
/// A vertical CRS in metres keeps only the operations whose vertical side is the CRS itself, as
/// before. Any other vertical unit takes PROJ's `useTransf` path: the operations are on the metre
/// copy of the CRS (`vertCRSMetre`), joined to the CRS by a change of unit, or a ballpark change
/// when the datums differ (`geoid_vertical_non_metre`).
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
        || vertical_axis.direction() != "up"
    {
        return Ok(Vec::new());
    }
    if vertical_axis.unit().conversion_to_si() != 1.0 {
        let operations = geoid_vertical_non_metre(vertical, geog, context)?;
        if from_vertical {
            return Ok(operations);
        }
        return Ok(operations
            .iter()
            .map(CoordinateOperation::inverse)
            .collect());
    }
    let mut operations = Vec::new();
    for model in vertical.geoid_models() {
        for operation in model_operations(vertical, geog, model, context)? {
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

/// The operations of one geoid model between `vertical_metre` and `geog`, in the vertical to
/// geographic orientation: the registered operation of its identifier, or of its name, or the
/// gravity-related height transformation on its PROJ grid (`getProjGeoidTransformation`).
fn model_operations(
    vertical_metre: &VerticalCRS,
    geog: &GeographicCRS,
    model: &GeoidModel,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    Ok(match model.identifiers.first() {
        Some(id) => vec![super::factory::registered_operation(
            context,
            id.code_space(),
            id.code(),
        )?],
        None => match model.name().strip_prefix("PROJ ") {
            Some(grid) => proj_geoid_operation(vertical_metre, geog, model, grid, context)?
                .into_iter()
                .collect(),
            None => super::factory::geoid_model_operations(context, model.name())?,
        },
    })
}

/// `createOperationsGeogToVertFromGeoid` (coordinateoperationfactory.cpp:4629-4871) for a
/// vertical CRS whose unit is not the metre, or whose axis is not up in metres. The geoid
/// operations go to the metre copy of the CRS, `vertCRSMetre`. Each one whose vertical side is
/// not the CRS is joined to it by a conversion (`createOperationsVertToVert`), from the CRS to
/// that side, as `useTransf` appends it. The operations run from the vertical CRS to the
/// geographic one; the caller inverts them for the reverse request.
///
/// The geographic CRS with a height unit other than the metre is not ported: `geoid_vertical`
/// returns before this function for it.
fn geoid_vertical_non_metre(
    vertical: &VerticalCRS,
    geog: &GeographicCRS,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let vertical_crs = Crs::Vertical(vertical.clone());
    let vertical_metre = vertical_metre_crs(vertical);
    let mut operations = Vec::new();
    for model in vertical.geoid_models() {
        for operation in model_operations(&vertical_metre, geog, model, context)? {
            let to_vertical = match (operation.source_crs(), operation.target_crs()) {
                (Some(Crs::Vertical(_)), Some(Crs::Geographic(_))) => Some(operation),
                (Some(Crs::Geographic(_)), Some(Crs::Vertical(_))) => Some(operation.inverse()),
                _ => None,
            };
            let Some(operation) = to_vertical else {
                continue;
            };
            let Some(Crs::Vertical(operation_vertical)) = operation.source_crs().cloned() else {
                continue;
            };
            if Crs::Vertical(operation_vertical.clone()).is_equivalent_to(&vertical_crs) {
                operations.push(operation);
                continue;
            }
            // `createOperationsVertToVert(op->targetCRS(), targetCRS)` in the request direction:
            // the conversion runs from this CRS to the operation's vertical side.
            let Some(conversion) =
                vertical_to_vertical(&vertical_crs, &Crs::Vertical(operation_vertical))
            else {
                continue;
            };
            let steps = vec![conversion, operation];
            operations.push(CoordinateOperation::Concatenated(
                concatenate_vertical_steps(steps, &vertical_crs, geog),
            ));
        }
    }
    Ok(operations)
}

/// `ConcatenatedOperation::createComputeMetadata` of the two steps of a vertical CRS to a
/// geographic CRS, with the vertical CRS as its source and the geographic one as its target.
fn concatenate_vertical_steps(
    steps: Vec<CoordinateOperation>,
    vertical: &Crs,
    geog: &GeographicCRS,
) -> ConcatenatedOperation {
    let mut concatenated = super::factory::pivot_concatenation(steps);
    concatenated.source = Some(Box::new(vertical.clone()));
    concatenated.target = Some(Box::new(Crs::Geographic(geog.clone())));
    concatenated
}

/// `getNameVertCRSMetre` (coordinateoperationfactory.cpp:4697-4717): the name of the metre copy
/// of a vertical CRS. A trailing unit is dropped, a depth becomes a height, and `(metre)` is
/// appended when the name had no unit.
fn vertical_metre_name(name: &str) -> String {
    if name.is_empty() {
        return "unnamed".to_owned();
    }
    let mut ret = name.to_owned();
    let mut have_original_unit = false;
    if ret.ends_with(')') {
        if let Some(pos) = ret.rfind(" (") {
            have_original_unit = true;
            ret.truncate(pos);
        }
    }
    if let Some(pos) = ret.rfind(" depth") {
        ret.truncate(pos);
        ret.push_str(" height");
    }
    if !have_original_unit {
        ret.push_str(" (metre)");
    }
    ret
}

/// `vertCRSMetre` of `createOperationsGeogToVertFromGeoid`: the vertical CRS in metres, up, with
/// the datum of `vertical` and the name of `vertical_metre_name`. It has no geoid model.
fn vertical_metre_crs(vertical: &VerticalCRS) -> VerticalCRS {
    VerticalCRS {
        identified: Identified::named(vertical_metre_name(vertical.name())),
        datum: vertical.datum.clone(),
        ensemble: vertical.ensemble.clone(),
        coordinate_system: VerticalCS::create_gravity_related_height(UnitOfMeasure::metre()),
        geoid_models: Vec::new(),
    }
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
        let extents = sort_extents(context, source, target);
        res = geog_to_vertical_alternative(factory, geog, vertical, context, &extents)?;
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
        if !is_transformation_step(step_a) {
            continue;
        }
        let name_a = normalize_2d_3d_in_name(step_a.name());
        let (Some(source_a), Some(target_a)) = (step_a.source_crs(), step_a.target_crs()) else {
            continue;
        };
        if source_a.name() == "unknown" || target_a.name() == "unknown" {
            continue;
        }
        for step_b in flat_operations(second) {
            if !is_transformation_step(step_b) {
                continue;
            }
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

/// `dynamic_cast<const Transformation *>` of PROJ's loops over the steps of an operation. The
/// geographic/geocentric conversion (EPSG:9602) is a `Conversion` in PROJ, which those loops skip;
/// here it is kept as a transformation, so it is excluded by its method. A point motion operation
/// is a `PointMotionOperation` in PROJ, a `SingleOperation` that is not a `Transformation`, so it
/// is excluded too.
fn is_transformation_step(operation: &CoordinateOperation) -> bool {
    matches!(
        operation,
        CoordinateOperation::Transformation(transformation)
            if transformation.method().code() != "9602" && !transformation.is_point_motion()
    )
}

/// The names of the geodetic datums of the transformations of an operation.
fn datum_names_used(operation: &CoordinateOperation) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for step in flat_operations(operation) {
        if !is_transformation_step(step) {
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

/// `createOperationsGeogToVertWithAlternativeGeog` (coordinateoperationfactory.cpp:5073-5137):
/// the registered operations to the vertical CRS from any geographic CRS, found by
/// `findOpsInRegistryDirectTo`. A source with a third axis in another unit than the metre is
/// preceded by a unit change of the geographic CRS.
fn geog_to_vertical_alternative(
    factory: &CoordinateOperationFactory,
    geog: &Crs,
    vertical: &Crs,
    context: &CoordinateOperationContext,
    extents: &SortExtents,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let Some(geographic) = geographic_of(geog) else {
        return Ok(Vec::new());
    };
    let mut inner = context.clone();
    inner.guards |= GUARD_ALTERNATIVE_GEOG;
    let axes = geographic.coordinate_system().axes();
    let mut res = Vec::new();
    for operation in find_ops_in_registry_direct_to(context, vertical, extents)? {
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
/// when there are any. The accuracy is the model's when it has one, as `modelAccuracies` is
/// used in place of the search; otherwise the most accurate registered operation on the grid
/// gives it. The area of use is that operation's, or the first candidate's when the model's
/// accuracy is used.
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
    if model.accuracy.is_none() {
        for (index, operation) in candidates.iter().enumerate() {
            let candidate = operation.accuracy().unwrap_or(-1.0);
            if candidate - accuracy > 1e-10 {
                accuracy = candidate;
                best = Some(index);
            }
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
    let accuracy = model
        .accuracy
        .or_else(|| (accuracy >= 0.0).then_some(accuracy));
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
        ballpark: false,
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
    let domain = match (crs_extent(source), crs_extent(target)) {
        (Some(left), Some(right)) if left.is_equivalent_to(&right) => left,
        _ => world_extent(),
    };
    if vertical_datum_name(src) != vertical_datum_name(dst) {
        return Transformation::create_from_code(
            &OpProps::new().name(format!(
                "Transformation from {} to {} (ballpark vertical transformation)",
                source.name(),
                target.name()
            )),
            source.clone(),
            target.clone(),
            1069,
            vec![ParameterValue::scale(scale)],
            domain,
        )
        .ok()
        .map(CoordinateOperation::Transformation);
    }
    // Same datum: a Conversion in PROJ (`createOperationsVertToVert`, coordinateoperationfactory.cpp
    // :6050-6064). A height/depth reversal of the same unit is method 1068; otherwise a change of
    // vertical unit, whose factor is negated on a reversal.
    let (src_axis, dst_axis) = (
        src.coordinate_system().axis(),
        dst.coordinate_system().axis(),
    );
    let reversal = (src_axis.direction() == "up" && dst_axis.direction() == "down")
        || (src_axis.direction() == "down" && dst_axis.direction() == "up");
    let unit_changes = src_axis.unit().conversion_to_si() != dst_axis.unit().conversion_to_si();
    let props = OpProps::new().name(format!(
        "Conversion from {} to {}",
        source.name(),
        target.name()
    ));
    let conversion = if unit_changes || !reversal {
        Conversion::create_change_vertical_unit(&props, scale)
    } else {
        Conversion::create_height_depth_reversal(&props)
    };
    let mut conversion = conversion
        .ok()?
        .with_endpoints(source.clone(), target.clone());
    conversion.identified.domains = vec![domain];
    Some(CoordinateOperation::Conversion(conversion))
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
        (Some(left), Some(right)) if left.is_equivalent_to(&right) => left,
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
    let vertical_ops =
        factory.create_candidates(vertical, target, &skip_horizontal_context(context))?;
    if horizontal_ops.is_empty() || vertical_ops.is_empty() {
        return Ok(horizontal_ops);
    }

    let mut found = Vec::new();
    let mut has_interpolated = false;
    for vertical_op in &vertical_ops {
        let Some(interpolation) = interpolation_geographic_2d(vertical_op) else {
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
        if interpolation_geographic_2d(vertical_op).is_none()
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

/// `context` with `skipHorizontalTransformation` set, as `SetSkipHorizontalTransform` does.
fn skip_horizontal_context(context: &CoordinateOperationContext) -> CoordinateOperationContext {
    let mut inner = context.clone();
    inner.guards |= GUARD_SKIP_HORIZONTAL;
    inner
}

/// `getInterpolationGeogCRS`: the 2D geographic CRS a vertical operation interpolates in, demoted
/// without the database.
fn interpolation_geographic_2d(operation: &CoordinateOperation) -> Option<GeographicCRS> {
    interpolation_geographic(operation, None)
}

/// `getInterpolationGeogCRS`: the 2D geographic CRS a vertical operation interpolates in, demoted
/// with `db` (`demoteTo2D(std::string(), dbContext)`).
fn interpolation_geographic(
    operation: &CoordinateOperation,
    db: Option<&DatabaseContext>,
) -> Option<GeographicCRS> {
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
    Some(interpolation.demote_to_2d(db))
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
pub(super) fn promote_with_axis(geographic: &GeographicCRS, height: Option<Axis>) -> GeographicCRS {
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
        interpolation: None,
    })
}

/// `getRemarks` (coordinateoperationfactory.cpp:2668-2713): the remarks of the operations that have
/// some, each introduced by `For <name> (<authority>:<code>): `, one per line. The prefix of an
/// inverse operation, in its name and in its authority, is dropped.
fn get_remarks(operations: &[&CoordinateOperation]) -> String {
    let mut remarks = String::new();
    for operation in operations {
        let own = operation.remarks();
        if own.is_empty() {
            continue;
        }
        if !remarks.is_empty() {
            remarks.push('\n');
        }
        let name = operation.name();
        remarks.push_str("For ");
        remarks.push_str(name.strip_prefix("Inverse of ").unwrap_or(name));
        if let Some(identifier) = operation.identifiers().first() {
            let mut authority = identifier.code_space();
            if let Some(inner) = authority
                .strip_prefix("INVERSE(")
                .and_then(|text| text.strip_suffix(')'))
            {
                authority = inner;
            }
            if let Some(inner) = authority
                .strip_prefix("DERIVED_FROM(")
                .and_then(|text| text.strip_suffix(')'))
            {
                authority = inner;
            }
            remarks.push_str(&format!(" ({authority}:{})", identifier.code()));
        }
        remarks.push_str(": ");
        remarks.push_str(own);
    }
    remarks
}

/// The operation with the remarks of `operations` set (`REMARKS_KEY` of the properties).
fn with_remarks(
    mut operation: CoordinateOperation,
    operations: &[&CoordinateOperation],
) -> CoordinateOperation {
    if let CoordinateOperation::Concatenated(concatenated) = &mut operation {
        concatenated.identified.remarks = get_remarks(operations);
    }
    operation
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
    // `opGeogCRStoDstCRS->inverse()->_isEquivalentTo(opSrcCRSToGeogCRS, EQUIVALENT)`
    // (coordinateoperationfactory.cpp:2821-2823): the method and the parameter values, not the
    // name. A 2D ballpark offset and its 3D inverse differ by their method.
    let cancels = steps.len() == 3
        && last
            .inverse()
            .is_equivalent_to_with(first, Equivalence::Equivalent);
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
    // `opsForRemarks`: the two cancelling operations leave the vertical one and the first one
    // (coordinateoperationfactory.cpp:2818-2844).
    let remarks = if cancels {
        get_remarks(&[first, vertical])
    } else {
        get_remarks(&steps.iter().collect::<Vec<_>>())
    };
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
        concatenated.identified.remarks = remarks;
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
    let remarks_of: Vec<CoordinateOperation> = steps.clone();
    with_remarks(
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
        ),
        &remarks_of.iter().collect::<Vec<_>>(),
    )
}
