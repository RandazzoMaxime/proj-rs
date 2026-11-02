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
    axis_order_reversal, change_vertical_unit_by_factor, export_vertical_unit_factor,
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

/// The forward direction from `source` to `target`. A projected `source` is the
/// inverse of the projection from its base, which `target` names.
fn export_projection(
    source: &Crs,
    target: &Crs,
    formatter: &mut PipelineFormatter,
) -> Result<(), FactoryError> {
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
