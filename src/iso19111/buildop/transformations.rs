//! Transformation constructors of PROJ `Transformation::create*`: the Helmert
//! family (geocentric translations, position vector, coordinate frame rotation,
//! and their time-dependent forms) and the Molodensky pair.
//!
//! The method, parameter order and EPSG codes come from [`super::super::parammap`].
//! Values are in the units of the EPSG parameters, as PROJ takes them: metres,
//! arc-seconds, and parts per million for the scale difference.

use std::f64::consts::PI;

use super::OpProps;
use crate::iso19111::model::{
    Crs, FactoryError, Measure, ObjectDomain, ParameterValue, Transformation, UnitKind,
    UnitOfMeasure,
};

/// Seven Helmert values in EPSG order: three translations (metres), three
/// rotations (arc-seconds), and the scale difference (parts per million).
fn helmert_values(translation: [f64; 3], rotation: [f64; 3], scale: f64) -> Vec<ParameterValue> {
    let mut values: Vec<ParameterValue> = translation
        .iter()
        .map(|value| ParameterValue::length(*value))
        .collect();
    values.extend(rotation.iter().map(|value| arc_second(*value)));
    values.push(parts_per_million(scale));
    values
}

fn arc_second(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: UnitOfMeasure::new("arc-second", PI / 648_000.0, UnitKind::Angular),
    })
}

fn parts_per_million(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: UnitOfMeasure::new("parts per million", 1e-6, UnitKind::Scale),
    })
}

/// Rates of change, per year: metres, arc-seconds, and parts per million.
fn rate_values(rates: [f64; 7]) -> Vec<ParameterValue> {
    let mut values = Vec::with_capacity(7);
    for (index, value) in rates.into_iter().enumerate() {
        values.push(match index {
            0..=2 => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new("metre per year", 1.0, UnitKind::Linear),
            }),
            3..=5 => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new("arc-second per year", PI / 648_000.0, UnitKind::Angular),
            }),
            _ => ParameterValue::Measure(Measure {
                value,
                unit: UnitOfMeasure::new("parts per million per year", 1e-6, UnitKind::Scale),
            }),
        });
    }
    values
}

/// Geocentric CRS get the geocentric method, geographic CRS the 2D one.
fn is_geocentric(crs: &Crs) -> bool {
    matches!(crs, Crs::Geodetic(_))
}

fn empty_domain() -> ObjectDomain {
    ObjectDomain {
        scope: None,
        area: None,
        south: None,
        west: None,
        north: None,
        east: None,
    }
}

/// Build the transformation with the EPSG method `code`, then remove the
/// domain that `create_from_code` records, as PROJ does not give one.
fn build(
    props: &OpProps,
    source: Crs,
    target: Crs,
    code: i32,
    values: Vec<ParameterValue>,
    accuracy: Option<f64>,
) -> Result<Transformation, FactoryError> {
    let mut transformation =
        Transformation::create_from_code(props, source, target, code, values, empty_domain())?;
    transformation.identified.domains.clear();
    transformation.accuracy = accuracy;
    Ok(transformation)
}

impl Transformation {
    /// Geocentric translations, Molodensky and Cartesian Grid Offsets: PROJ's inverse
    /// negates the values.
    pub(super) fn has_translation_inverse(&self) -> bool {
        matches!(
            self.method.code.parse::<i32>(),
            Ok(1031 | 9603 | 9604 | 9605 | 9656)
        )
    }

    /// Negate every measured value. `0.0 - v` keeps a zero positive.
    pub(super) fn negate_values(&mut self) {
        for parameter in &mut self.parameters {
            if let ParameterValue::Measure(measure) = &mut parameter.value {
                measure.value = 0.0 - measure.value;
            }
        }
    }

    /// PROJ `getTOWGS84Parameters(true)`: the seven Helmert values of the
    /// `+towgs84` convention, or `None` when the method is not a Helmert one. A
    /// translation has zero rotations and scale.
    ///
    /// # Errors
    ///
    /// A Helmert method with a missing parameter.
    pub fn towgs84_parameters(&self) -> Result<Option<[f64; 7]>, FactoryError> {
        self.towgs84_strict()
    }

    /// Geocentric translations (EPSG:9603 for geographic CRS, 1031 for geocentric).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_geocentric_translations(
        props: &OpProps,
        source: Crs,
        target: Crs,
        x: f64,
        y: f64,
        z: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1031 } else { 9603 };
        let values = vec![
            ParameterValue::length(x),
            ParameterValue::length(y),
            ParameterValue::length(z),
        ];
        build(props, source, target, code, values, accuracy)
    }

    /// Position vector transformation (EPSG:9606, or 1033 for geocentric CRS).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_position_vector(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1033 } else { 9606 };
        let values = helmert_values(
            [helmert[0], helmert[1], helmert[2]],
            [helmert[3], helmert[4], helmert[5]],
            helmert[6],
        );
        build(props, source, target, code, values, accuracy)
    }

    /// Coordinate frame rotation (EPSG:9607, or 1034 for geocentric CRS).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_coordinate_frame_rotation(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let code = if is_geocentric(&source) { 1034 } else { 9607 };
        let values = helmert_values(
            [helmert[0], helmert[1], helmert[2]],
            [helmert[3], helmert[4], helmert[5]],
            helmert[6],
        );
        build(props, source, target, code, values, accuracy)
    }

    /// Time-dependent position vector (EPSG:1054): seven Helmert values, their
    /// seven rates of change, then the parameter reference epoch.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_time_dependent_position_vector(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        rates: [f64; 7],
        epoch: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let values = time_dependent_values(helmert, rates, epoch);
        build(props, source, target, 1054, values, accuracy)
    }

    /// Time-dependent coordinate frame rotation (EPSG:1057).
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_time_dependent_coordinate_frame_rotation(
        props: &OpProps,
        source: Crs,
        target: Crs,
        helmert: [f64; 7],
        rates: [f64; 7],
        epoch: f64,
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        let values = time_dependent_values(helmert, rates, epoch);
        build(props, source, target, 1057, values, accuracy)
    }

    /// Molodensky (EPSG:9604): three translations, the semi-major axis
    /// difference, and the flattening difference.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_molodensky(
        props: &OpProps,
        source: Crs,
        target: Crs,
        shifts: [f64; 5],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        molodensky(props, source, target, 9604, shifts, accuracy)
    }

    /// Abridged Molodensky (EPSG:9605), with the same five values.
    ///
    /// # Errors
    ///
    /// The method map is missing.
    pub fn create_abridged_molodensky(
        props: &OpProps,
        source: Crs,
        target: Crs,
        shifts: [f64; 5],
        accuracy: Option<f64>,
    ) -> Result<Self, FactoryError> {
        molodensky(props, source, target, 9605, shifts, accuracy)
    }
}

/// The Helmert values, their rates, then the parameter reference epoch, in the
/// order of the time-dependent methods.
fn time_dependent_values(helmert: [f64; 7], rates: [f64; 7], epoch: f64) -> Vec<ParameterValue> {
    let mut values = helmert_values(
        [helmert[0], helmert[1], helmert[2]],
        [helmert[3], helmert[4], helmert[5]],
        helmert[6],
    );
    values.extend(rate_values(rates));
    values.push(ParameterValue::Measure(Measure {
        value: epoch,
        unit: UnitOfMeasure::new("year", 1.0, UnitKind::Time),
    }));
    values
}

fn molodensky(
    props: &OpProps,
    source: Crs,
    target: Crs,
    code: i32,
    shifts: [f64; 5],
    accuracy: Option<f64>,
) -> Result<Transformation, FactoryError> {
    let values = vec![
        ParameterValue::length(shifts[0]),
        ParameterValue::length(shifts[1]),
        ParameterValue::length(shifts[2]),
        ParameterValue::length(shifts[3]),
        ParameterValue::scale(shifts[4]),
    ];
    build(props, source, target, code, values, accuracy)
}
