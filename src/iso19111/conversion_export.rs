//! PROJ strings of a conversion that carries its CRS pair: the projection of
//! `Conversion::_exportToPROJString`, written from the source and target CRS the
//! operation factory sets on it.
//!
//! The projection is the one [`super::projstring::projection_steps`] writes for
//! the pair. A projected source runs the projection backwards, so its steps sit
//! between `startInversion` and `stopInversion`, as PROJ writes them. An inverted
//! conversion is the same thing with the pair read target to source.
//!
//! A conversion without its pair has no PROJ string here. The factory sets the
//! pair only where the operation is a projection of a known base CRS, so the
//! conversions read from the database (and the inverse of a bare one) keep their
//! refusal.

use super::model::{Conversion, Crs, FactoryError, ParameterValue};
use super::pipeline::{PipelineFormatter, ProjLines};
use super::projstring::projection_steps;
use super::transform_export::{
    axis_order_reversal, cartesian_unit_and_axis_swap, change_vertical_unit_by_factor,
    export_vertical_unit_factor, vertical_offset, vertical_offset_and_slope,
};

impl Conversion {
    /// `_exportToPROJString` of a conversion with its CRS pair, as the steps of a
    /// concatenated operation or of a single pipeline.
    ///
    /// # Errors
    ///
    /// The conversion has no CRS pair, or the pair is not a projection that
    /// [`projection_steps`] writes.
    pub(crate) fn export_steps(
        &self,
        formatter: &mut PipelineFormatter,
    ) -> Result<(), FactoryError> {
        let (source, target) = if self.inverted {
            (self.target_crs(), self.source_crs())
        } else {
            (self.source_crs(), self.target_crs())
        };
        if super::transform_export::is_geographic_offset(self) {
            let (Some(source), Some(target)) = (source, target) else {
                return Err(FactoryError::Format(
                    "a conversion step of a concatenated operation has no PROJ string in this crate"
                        .to_owned(),
                ));
            };
            if self.inverted {
                formatter.start_inversion();
            }
            super::transform_export::geographic_offsets(self, source, target, formatter)?;
            if self.inverted {
                formatter.stop_inversion();
            }
            return Ok(());
        }
        if self.inverted {
            formatter.start_inversion();
        }
        if !(export_proj_based(self, source, target, formatter)?
            || export_body(self, source, target, formatter)?)
        {
            let (Some(source), Some(target)) = (source, target) else {
                if self.inverted {
                    formatter.stop_inversion();
                }
                return Err(FactoryError::Format(
                    "a conversion step of a concatenated operation has no PROJ string in this crate"
                        .to_owned(),
                ));
            };
            export_projection(source, target, formatter)?;
        }
        if self.inverted {
            formatter.stop_inversion();
        }
        Ok(())
    }

    /// `exportToPROJString` of a conversion with its CRS pair, as one pipeline.
    ///
    /// # Errors
    ///
    /// As [`Self::export_steps`].
    pub(crate) fn export_pipeline(&self, lines: ProjLines) -> Result<String, FactoryError> {
        let mut formatter = PipelineFormatter::with_grid_names(Vec::new());
        self.export_steps(&mut formatter)?;
        Ok(formatter.finish(lines))
    }
}

/// Axis-order reversal, a vertical unit change, and height-depth reversal.
/// Other methods are projections and return `false`.
fn export_body(
    conversion: &Conversion,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    match conversion.method().code() {
        "1068" => {
            formatter.add_step("axisswap");
            formatter.add_param("order", "1,2,-3");
            Ok(true)
        }
        "1104" => {
            export_vertical_unit_factor(vertical_unit_factor(source, target)?, formatter);
            Ok(true)
        }
        "9843" | "9844" => {
            axis_order_reversal(source, target, formatter)?;
            Ok(true)
        }
        "9616" => {
            vertical_offset(conversion.parameters(), source, target, formatter)?;
            Ok(true)
        }
        "1046" => {
            vertical_offset_and_slope(conversion.parameters(), source, target, formatter)?;
            Ok(true)
        }
        "1069" => {
            let factor = conversion
                .parameters()
                .iter()
                .find(|parameter| parameter.code() == "1051")
                .and_then(|parameter| match parameter.value() {
                    ParameterValue::Measure(measure) => Some(measure.si_value()),
                    _ => None,
                })
                .unwrap_or(0.0);
            change_vertical_unit_by_factor(factor, formatter);
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Source-unit SI factor over the target-unit SI factor, for EPSG:1104.
fn vertical_unit_factor(source: Option<&Crs>, target: Option<&Crs>) -> Result<f64, FactoryError> {
    let (Some(Crs::Vertical(source)), Some(Crs::Vertical(target))) = (source, target) else {
        return Err(FactoryError::Format(
            "Export of EPSG_CODE_METHOD_CHANGE_VERTICAL_UNIT_NO_CONV_FACTOR conversion to a PROJ string requires an input and output vertical CRS"
                .to_owned(),
        ));
    };
    Ok(source.coordinate_system().axis().unit().conversion_to_si()
        / target.coordinate_system().axis().unit().conversion_to_si())
}

/// `PROJ-based operation method: ` and the PROJ string after it.
const PROJ_BASED_PREFIX: &str = "PROJ-based operation method: ";

/// The CRS a conversion stands for: a compound CRS stands for its first component (`horiz` of
/// `Conversion::_exportToPROJString`, conversion.cpp:3886-3900 and 4420-4428), and a bound first
/// component for its base (`boundCRS->baseCRS()`, conversion.cpp:3896-3900).
fn horizontal_component(crs: &Crs) -> Crs {
    match crs {
        Crs::Compound(compound) => match compound.components().first() {
            Some(Crs::Bound(bound)) => bound.source_crs().clone(),
            Some(first) => first.clone(),
            None => crs.clone(),
        },
        other => other.clone(),
    }
}

/// A PROJ-based method from a projected source to a projected or derived projected target:
/// the inverse of the source's unit and axis order, the method's PROJ string, then the target's
/// unit and axis order (`Conversion::_exportToPROJString`, conversion.cpp:3908-3915 and
/// 4422-4450, with `SingleOperation::exportToPROJStringGeneric` for the string).
/// `false` for the other pairs, which keep the projection path.
fn export_proj_based(
    conversion: &Conversion,
    source: Option<&Crs>,
    target: Option<&Crs>,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let Some(text) = conversion.method().name().strip_prefix(PROJ_BASED_PREFIX) else {
        return Ok(false);
    };
    let (Some(source), Some(target)) = (source, target) else {
        return Ok(false);
    };
    let (source, target) = (horizontal_component(source), horizontal_component(target));
    let Crs::Projected(source) = &source else {
        return Ok(false);
    };
    let target_axes = match &target {
        Crs::Projected(projected) => projected.coordinate_system().axes().to_vec(),
        Crs::DerivedProjected(derived) => derived
            .coordinate_system()
            .axes()
            .into_iter()
            .cloned()
            .collect(),
        _ => return Ok(false),
    };
    formatter.start_inversion();
    cartesian_unit_and_axis_swap(source.coordinate_system().axes(), formatter);
    formatter.stop_inversion();
    formatter.ingest_proj_string(text);
    cartesian_unit_and_axis_swap(&target_axes, formatter);
    Ok(true)
}

/// The forward direction from `source` to `target`. A projected `source` is the
/// inverse of the projection from its base, which `target` names.
fn export_projection(
    source: &Crs,
    target: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    let (source, target) = (&horizontal_component(source), &horizontal_component(target));
    if matches!(source, Crs::Projected(_)) {
        formatter.start_inversion();
        ingest(&projection_steps(target, source)?, formatter);
        formatter.stop_inversion();
    } else {
        ingest(&projection_steps(source, target)?, formatter);
    }
    Ok(())
}

fn ingest(steps: &[String], formatter: &mut PipelineFormatter) {
    for step in steps {
        formatter.ingest_proj_string(step);
    }
}
