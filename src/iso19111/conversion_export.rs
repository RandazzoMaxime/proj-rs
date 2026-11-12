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

use super::model::{Axis, Conversion, Crs, FactoryError, ParameterValue, UnitKind};
use super::niche::DerivedKind;
use super::pipeline::{proj_number, PipelineFormatter, ProjLines};
use super::projstring::projection_steps;
use super::transform_export::{
    axes_unit_and_axis_swap, axis_order_reversal, change_vertical_unit_by_factor,
    export_vertical_unit_factor,
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
        if !export_body(self, source, target, formatter)? {
            let (Some(source), Some(target)) = (source, target) else {
                if self.inverted {
                    formatter.stop_inversion();
                }
                return Err(FactoryError::Format(
                    "a conversion step of a concatenated operation has no PROJ string in this crate"
                        .to_owned(),
                ));
            };
            if !export_proj_method(self, source, target, formatter)? {
                export_projection(source, target, formatter)?;
            }
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

/// `Conversion::_exportToPROJString` of a PROJ method (`PROJ <step> <params>`) between a
/// projected or vertical source and a derived projected or vertical target. Such a derived CRS
/// has no base projection to invert, so PROJ writes the step itself: the source inverted with its
/// unit and axis order, the step, its parameter values in SI, then the target's unit and axis
/// order (conversion.cpp:3738-3745, 3666-3700, 3906-3922 and 4467-4475). `false` for any other
/// pair, which keeps the projection path.
///
/// # Errors
///
/// A parameter value is not a linear measure, which this crate does not write for a PROJ method.
fn export_proj_method(
    conversion: &Conversion,
    source: &Crs,
    target: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<bool, FactoryError> {
    let Some(method) = conversion.method().name().strip_prefix("PROJ ") else {
        return Ok(false);
    };
    let is_vertical = |crs: &Crs| {
        matches!(crs, Crs::Vertical(_))
            || matches!(crs, Crs::Derived(derived) if derived.kind() == DerivedKind::Vertical)
    };
    let source_axes: Option<Vec<Axis>> = match source {
        Crs::Projected(projected) => Some(projected.coordinate_system().axes().to_vec()),
        other if is_vertical(other) => None,
        _ => return Ok(false),
    };
    let target_axes: Option<Vec<Axis>> = match target {
        Crs::DerivedProjected(derived) => Some(
            derived
                .coordinate_system()
                .axes()
                .into_iter()
                .cloned()
                .collect(),
        ),
        other if is_vertical(other) => None,
        _ => return Ok(false),
    };
    let mut values = Vec::new();
    for parameter in conversion.parameters() {
        let ParameterValue::Measure(measure) = parameter.value() else {
            return Err(FactoryError::Format(
                "a PROJ method with a parameter that is not a measure has no PROJ string in this crate"
                    .to_owned(),
            ));
        };
        if measure.unit().kind() != UnitKind::Linear {
            return Err(FactoryError::Format(
                "a PROJ method with an angular parameter has no PROJ string in this crate"
                    .to_owned(),
            ));
        }
        values.push((parameter.name().to_owned(), measure.si_value()));
    }
    if let Some(axes) = &source_axes {
        formatter.start_inversion();
        axes_unit_and_axis_swap(axes, formatter);
        formatter.stop_inversion();
    }
    let mut tokens = method.split(' ');
    formatter.add_step(tokens.next().unwrap_or_default());
    for token in tokens {
        match token.split_once('=') {
            Some((key, value)) => formatter.add_param(key, value),
            None => formatter.add_flag(token),
        }
    }
    for (name, value) in values {
        formatter.add_param(&name, &proj_number(value));
    }
    if let Some(axes) = &target_axes {
        axes_unit_and_axis_swap(axes, formatter);
    }
    Ok(true)
}

/// The forward direction from `source` to `target`. A projected `source` is the
/// inverse of the projection from its base, which `target` names.
fn export_projection(
    source: &Crs,
    target: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
    // A compound CRS stands for its first component (`horiz` of `Conversion::_exportToPROJString`,
    // conversion.cpp:3886-3900 and 4420-4428): the factory sets compound CRS on the conversion
    // of a compound's horizontal part.
    // A bound first component stands for its base (`boundCRS->baseCRS()`, conversion.cpp:3896-3900).
    let horizontal = |crs: &'_ Crs| -> Crs {
        match crs {
            Crs::Compound(compound) => match compound.components().first() {
                Some(Crs::Bound(bound)) => bound.source_crs().clone(),
                Some(first) => first.clone(),
                None => crs.clone(),
            },
            other => other.clone(),
        }
    };
    let (source, target) = (&horizontal(source), &horizontal(target));
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
