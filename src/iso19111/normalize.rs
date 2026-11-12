//! `CoordinateOperation::normalizeForVisualization`: the operation with the axis order a map
//! program expects, as `proj_normalize_for_visualization` builds it for an operation.
//!
//! A source or target whose first two axes are north then east gets an axis order reversal
//! step, and the operation, its name and its CRS pair follow. libproj's
//! `proj_create_crs_to_crs` result goes through it, so the operation `proj_as_projjson` and
//! `proj_as_wkt` write for a pair is this one.

use super::model::{
    ConcatenatedOperation, ConcatenatedStep, Conversion, CoordinateOperation, CoordinateSystem,
    Crs, GeodeticCRS, GeographicCRS, Identified, Identifier, OperationMethod,
};
use super::parammap;

/// `NORMALIZED_AXIS_ORDER_SUFFIX_STR`.
const NORMALIZED_SUFFIX: &str = " (with axis order normalized for visualization)";

/// EPSG code of the method "Axis Order Reversal (2D)", and of the conversion that uses it.
const AXIS_ORDER_REVERSAL_2D: i32 = 9843;
const AXIS_ORDER_CHANGE_2D: &str = "15498";
const AXIS_ORDER_CHANGE_2D_NAME: &str = "axis order change (2D)";

impl CoordinateOperation {
    /// `normalizeForVisualization`: north/east axes become east/north.
    ///
    /// `None` when a side that needs the reversal cannot be reversed.
    #[must_use]
    pub fn normalize_for_visualization(&self) -> Option<Self> {
        normalize_for_visualization(self)
    }
}

/// The operation `normalizeForVisualization` returns, or `None` when it cannot be built: a
/// side that needs the reversal is a kind this crate cannot reverse (a compound CRS), or the
/// operation has no CRS pair.
pub(crate) fn normalize_for_visualization(
    operation: &CoordinateOperation,
) -> Option<CoordinateOperation> {
    let (source, target) = operation_crs_pair(operation)?;
    let swap_source = must_axis_order_be_switched(source);
    let swap_target = must_axis_order_be_switched(target);
    if !swap_source && !swap_target {
        return Some(operation.clone());
    }
    let mut steps = Vec::new();
    let mut new_source = source.clone();
    let mut new_target = target.clone();
    if swap_source {
        new_source = reversed(source)?;
        steps.push(reversal_step(new_source.clone(), source.clone())?);
    }
    match operation {
        CoordinateOperation::Concatenated(concatenated) => {
            steps.extend(concatenated.steps().iter().cloned());
        }
        other => steps.push(ConcatenatedStep {
            operation: other.clone(),
            reverse: false,
        }),
    }
    if swap_target {
        new_target = reversed(target)?;
        steps.push(reversal_step(target.clone(), new_target.clone())?);
    }
    Some(CoordinateOperation::Concatenated(concatenated_of(
        operation, steps, new_source, new_target,
    )))
}

fn concatenated_of(
    operation: &CoordinateOperation,
    steps: Vec<ConcatenatedStep>,
    source: Crs,
    target: Crs,
) -> ConcatenatedOperation {
    let mut identified = Identified::named(format!("{}{NORMALIZED_SUFFIX}", operation.name()));
    identified.domains = operation.domains().to_vec();
    identified.remarks = remarks_of(operation).to_owned();
    identified.identifiers = operation
        .identifiers()
        .iter()
        .map(|identifier| {
            Identifier::new(
                format!("DERIVED_FROM({})", identifier.code_space()),
                identifier.code(),
            )
        })
        .collect();
    let (accuracy, computed_name) = match operation {
        CoordinateOperation::Concatenated(concatenated) => {
            (concatenated.accuracy(), concatenated.computed_name)
        }
        other => (other.accuracy(), false),
    };
    let interpolation = super::buildop::concatenation_interpolation(
        &steps
            .iter()
            .map(|step| step.operation.clone())
            .collect::<Vec<_>>(),
    )
    .map(Box::new);
    ConcatenatedOperation {
        identified,
        steps,
        source: Some(Box::new(source)),
        target: Some(Box::new(target)),
        accuracy,
        version: None,
        // Only selects how the accuracy text is spelled, see `accuracy_text`.
        computed_name,
        proj_based: None,
        source_epoch: None,
        target_epoch: None,
        interpolation,
    }
}

fn remarks_of(operation: &CoordinateOperation) -> &str {
    match operation {
        CoordinateOperation::Conversion(operation) => &operation.identified.remarks,
        CoordinateOperation::Transformation(operation) => &operation.identified.remarks,
        CoordinateOperation::Concatenated(operation) => &operation.identified.remarks,
    }
}

pub(crate) fn operation_crs_pair(operation: &CoordinateOperation) -> Option<(&Crs, &Crs)> {
    match operation {
        CoordinateOperation::Conversion(operation) => {
            Some((operation.source_crs()?, operation.target_crs()?))
        }
        CoordinateOperation::Transformation(operation) => {
            Some((operation.source_crs()?, operation.target_crs()?))
        }
        CoordinateOperation::Concatenated(operation) => operation.crs_pair(),
    }
}

/// `Conversion::createAxisOrderReversal(false)` between `source` and `target`.
fn reversal_step(source: Crs, target: Crs) -> Option<ConcatenatedStep> {
    let mapped = parammap::method_by_code(AXIS_ORDER_REVERSAL_2D)?;
    let conversion = Conversion {
        identified: Identified::named(AXIS_ORDER_CHANGE_2D_NAME)
            .with_id("EPSG", AXIS_ORDER_CHANGE_2D),
        method: OperationMethod {
            name: mapped.wkt2_name.to_owned(),
            code_space: "EPSG".to_owned(),
            code: AXIS_ORDER_REVERSAL_2D.to_string(),
            parameters: Vec::new(),
        },
        parameters: Vec::new(),
        source: Some(Box::new(source)),
        target: Some(Box::new(target)),
        version: None,
        inverted: false,
        interpolation: None,
    };
    Some(ConcatenatedStep {
        operation: CoordinateOperation::Conversion(conversion),
        reverse: false,
    })
}

/// `CRS::applyAxisOrderReversal` (crs.cpp:1015-1125) for a geographic or projected CRS: the two
/// horizontal axes are swapped whatever their order, and the CRS has no identifier of its own.
/// `None` for any other kind.
pub(crate) fn reversed(crs: &Crs) -> Option<Crs> {
    Some(match crs {
        Crs::Geographic(geographic) => Crs::Geographic(reversed_geographic(geographic)),
        Crs::Geodetic(GeodeticCRS::Geographic(geographic)) => {
            Crs::Geodetic(GeodeticCRS::Geographic(reversed_geographic(geographic)))
        }
        Crs::Projected(projected) => {
            let mut projected = projected.clone();
            projected.coordinate_system.axes.swap(0, 1);
            reverse_identified(&mut projected.identified);
            Crs::Projected(projected)
        }
        Crs::DerivedProjected(derived) => {
            let mut derived = derived.clone();
            if let CoordinateSystem::Cartesian(cs) = &mut derived.coordinate_system {
                cs.axes.swap(0, 1);
            }
            reverse_identified(&mut derived.identified);
            Crs::DerivedProjected(derived)
        }
        _ => return None,
    })
}

fn reversed_geographic(geographic: &GeographicCRS) -> GeographicCRS {
    let mut reversed = geographic.clone();
    reversed.coordinate_system.axes.swap(0, 1);
    reverse_identified(&mut reversed.identified);
    reversed
}

/// The identified part of `createProperties` (crs.cpp:1017-1062): the name toggles the
/// normalized suffix (or the "axis order reversed" one), the domains stay, the remarks say which
/// identifier the axes come from, and no identifier is kept.
fn reverse_identified(identified: &mut Identified) {
    const REVERSED_SUFFIX: &str = " (with axis order reversed)";
    const REMARKS_PREFIX: &str = "Axis order reversed compared to ";
    let name = identified.name.clone();
    identified.name = if let Some(base) = name.strip_suffix(NORMALIZED_SUFFIX) {
        base.to_owned()
    } else if let Some(base) = name.strip_suffix(REVERSED_SUFFIX) {
        base.to_owned()
    } else {
        format!("{name}{NORMALIZED_SUFFIX}")
    };
    let remarks = std::mem::take(&mut identified.remarks);
    identified.remarks = match identified.identifiers.as_slice() {
        [_] if remarks.starts_with(REMARKS_PREFIX) => String::new(),
        [identifier] => {
            let mut text = format!(
                "{REMARKS_PREFIX}{}:{}",
                identifier.code_space(),
                identifier.code()
            );
            if !remarks.is_empty() {
                text.push_str(". ");
                text.push_str(&remarks);
            }
            text
        }
        _ => remarks,
    };
    identified.identifiers.clear();
}

/// `CRS::mustAxisOrderBeSwitchedForVisualization`: the first two axes are north then east. A
/// compound CRS follows its first component.
pub(crate) fn must_axis_order_be_switched(crs: &Crs) -> bool {
    let axes = match crs {
        Crs::Compound(compound) => {
            return compound
                .components()
                .first()
                .is_some_and(must_axis_order_be_switched)
        }
        Crs::Geographic(crs) | Crs::Geodetic(GeodeticCRS::Geographic(crs)) => {
            crs.coordinate_system().axes()
        }
        Crs::Projected(crs) => crs.coordinate_system().axes(),
        Crs::DerivedProjected(crs) => {
            let axes = crs.coordinate_system().axes();
            return matches!(axes.as_slice(), [first, second, ..]
                if first.direction() == "north" && second.direction() == "east");
        }
        _ => return false,
    };
    matches!(axes, [first, second, ..]
        if first.direction() == "north" && second.direction() == "east")
}
