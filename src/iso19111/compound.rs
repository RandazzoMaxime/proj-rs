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

use super::buildop::OpProps;
use super::equiv::{geodetic_frame, same_geodetic_datum, Equivalence};
use super::factory::{
    crs_extent, geographic_of, world_extent, CoordinateOperationContext, CoordinateOperationFactory,
};
use super::model::{
    Axis, ConcatenatedOperation, ConcatenatedStep, CoordinateOperation, Crs, GeographicCRS,
    Identified, ParameterValue, Transformation, VerticalCRS,
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
    let (Some(geog_src), Crs::Vertical(vertical_crs)) = (geographic_of(horizontal), vertical)
    else {
        return Ok(Vec::new());
    };

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
    vertical_ops.extend(vertical_ballpark(
        vertical,
        vertical_crs,
        horizontal_datum.as_deref(),
        target_datum.as_deref(),
        geog_dst,
        target,
    ));
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
    Ok(vertical_ops
        .iter()
        .filter(|operation| horizontal_is_null(operation, geog_src))
        .map(|operation| retarget(operation, source, target))
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

/// `createOperationsVertToVert`: the ballpark change of vertical unit between two vertical
/// CRS of different datums. Two CRS of the same datum, where PROJ builds a conversion, are
/// not ported.
pub(super) fn vertical_to_vertical(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
    let (Crs::Vertical(src), Crs::Vertical(dst)) = (source, target) else {
        return None;
    };
    if vertical_datum_name(src) == vertical_datum_name(dst) {
        return None;
    }
    let scale = ballpark_scale(
        src.coordinate_system().axis(),
        Some(dst.coordinate_system().axis()),
    )?;
    let name = format!(
        "Transformation from {} to {} (ballpark vertical transformation)",
        source.name(),
        target.name()
    );
    let domain = match (crs_extent(source), crs_extent(target)) {
        (Some(left), Some(right)) if left == right => left,
        _ => world_extent(),
    };
    Transformation::create_from_code(
        &OpProps::new().name(name),
        source.clone(),
        target.clone(),
        1069,
        vec![ParameterValue::scale(scale)],
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
fn null_offset(source: &Crs, target: &Crs) -> Option<CoordinateOperation> {
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
