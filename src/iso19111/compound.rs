//! The vertical branch of PROJ's `createOperationsCompoundToGeog`
//! (`coordinateoperationfactory.cpp`): a compound CRS of a geographic CRS and a
//! vertical CRS, to a 3D geographic CRS. Also `createOperationsCompoundToCompound`
//! for two compound CRS with the same geographic part, and the ballpark of
//! `createOperationsVertToVert`.
//!
//! Not ported: the interpolation CRS of a vertical operation
//! (`createHorizVerticalHorizPROJBased`, which the vertical operation then
//! skips), the geographic CRS of a horizontal operation that differs from the
//! target (`geogCRSTmp`), the change of vertical unit when the two datums are
//! the same, the extent check of a concatenated candidate, and the geoid
//! trigger and three-pass selection of `createOperationsCompoundToCompound`.

use super::bound::extract_geographic;
use super::buildop::OpProps;
use super::equiv::{geodetic_frame, same_geodetic_datum, Equivalence};
use super::factory::{
    crs_extent, geographic_of, world_extent, CoordinateOperationContext, CoordinateOperationFactory,
};
use super::model::{
    Axis, ConcatenatedOperation, ConcatenatedStep, CoordinateOperation, Crs, GeographicCRS,
    GeoidModel, Identified, OperationMethod, Parameter, ParameterValue, ProjBased, ProjBasedKind,
    Transformation, VerticalCRS,
};
use super::FactoryError;

const NULL_GEOGRAPHIC_OFFSET: &str = "Null geographic offset";

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

    let mut horizontal_ops = factory.create_operations(horizontal, target, context)?;
    if horizontal_ops.is_empty() {
        horizontal_ops.extend(null_offset(horizontal, target));
    }
    if horizontal_ops.is_empty() {
        return Ok(Vec::new());
    }

    let mut vertical_ops = factory.create_operations(vertical, target, context)?;
    let horizontal_datum = geographic_datum_name(geog_src);
    let target_datum = geographic_datum_name(geog_dst);
    if let Some(vertical_crs) = vertical_crs {
        vertical_ops.extend(vertical_ballpark(
            vertical,
            vertical_crs,
            horizontal_datum.as_deref(),
            target_datum.as_deref(),
            geog_dst,
            target,
        ));
    }
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

/// `createOperationsCompoundToCompound`, for two compound CRS whose geographic parts are
/// equivalent: each vertical operation, with its horizontal part null. Only a vertical
/// operation with no interpolation CRS, or one whose interpolation CRS is that geographic
/// CRS, is kept. The others need `createHorizVerticalHorizPROJBased`, not ported.
pub(super) fn compound_to_compound(
    factory: &CoordinateOperationFactory,
    source: &Crs,
    target: &Crs,
    context: &CoordinateOperationContext,
) -> Result<Vec<CoordinateOperation>, FactoryError> {
    let (Crs::Compound(src), Crs::Compound(dst)) = (source, target) else {
        return Ok(Vec::new());
    };
    let ([src_horizontal, src_vertical], [dst_horizontal, dst_vertical]) =
        (src.components(), dst.components())
    else {
        return Ok(Vec::new());
    };
    let (Some(geog_src), Some(geog_dst)) =
        (geographic_of(src_horizontal), geographic_of(dst_horizontal))
    else {
        return Ok(Vec::new());
    };
    if !geog_src.is_equivalent_to_with(geog_dst, Equivalence::Equivalent)
        || src_vertical.name() == dst_vertical.name()
    {
        return Ok(Vec::new());
    }

    let vertical_ops = factory.create_candidates(src_vertical, dst_vertical, context)?;
    let Some(null) = null_offset(src_horizontal, dst_horizontal) else {
        return Ok(Vec::new());
    };
    Ok(vertical_ops
        .iter()
        .filter(|operation| horizontal_is_null(operation, geog_src))
        .map(|operation| {
            proj_based_horiz_vertical(
                source,
                target,
                &null,
                &retarget(operation, source, target),
                geog_dst,
            )
        })
        .collect())
}

/// Whether a vertical operation leaves the horizontal part null: no interpolation CRS, or
/// the geographic CRS of the horizontal part. A conversion never has one.
fn horizontal_is_null(operation: &CoordinateOperation, geog: &GeographicCRS) -> bool {
    match operation {
        CoordinateOperation::Transformation(transformation) => {
            match transformation.interpolation_crs() {
                None => true,
                Some(Crs::Geographic(interpolation)) => {
                    interpolation.is_equivalent_to_with(geog, Equivalence::Equivalent)
                }
                Some(_) => false,
            }
        }
        CoordinateOperation::Conversion(_) => true,
        CoordinateOperation::Concatenated(_) => false,
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
    let registered = factory
        .create_operations(vertical, target, context)?
        .into_iter()
        .find(|operation| !operation.has_ballpark_transformation());
    let vertical_op = match registered {
        Some(operation) => operation,
        None if factory
            .create_operations(vertical, target, context)?
            .is_empty() =>
        {
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
        None => return Ok(None),
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
        proj_based: None,
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
                    &interpolation,
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
    interpolation: &GeographicCRS,
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
    proj_based_operation(
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
            interpolation: Crs::Geographic(interpolation.clone()),
            inverted: false,
        },
    )
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
