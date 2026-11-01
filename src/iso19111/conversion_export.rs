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

use super::model::{Conversion, Crs, FactoryError};
use super::pipeline::{PipelineFormatter, ProjLines};
use super::projstring::projection_steps;

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
        let (Some(source), Some(target)) = (self.source_crs(), self.target_crs()) else {
            return Err(FactoryError::Format(
                "a conversion step of a concatenated operation has no PROJ string in this crate"
                    .to_owned(),
            ));
        };
        if self.inverted {
            formatter.start_inversion();
            export_projection(target, source, formatter)?;
            formatter.stop_inversion();
            Ok(())
        } else {
            export_projection(source, target, formatter)
        }
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
